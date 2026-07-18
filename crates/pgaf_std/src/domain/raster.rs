use gdal::raster::{Buffer, GdalDataType};
use gdal::{Dataset, GeoTransformEx};
use pgaf_sdk::data::GeoDeg;
use pgaf_sdk::domain::{DomainGeneratorCreate, DomainGeneratorDriverTyped, ExecutionUnit, UnitId};
use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use std::fmt;
use std::rc::Rc;
use std::sync::LazyLock;
use validator::Validate;

enum RasterBuffer {
    UInt8(Buffer<u8>),
    Int8(Buffer<i8>),
    UInt16(Buffer<u16>),
    Int16(Buffer<i16>),
    UInt32(Buffer<u32>),
    Int32(Buffer<i32>),
    UInt64(Buffer<u64>),
    Int64(Buffer<i64>),
    Float32(Buffer<f32>),
    Float64(Buffer<f64>),
}

impl RasterBuffer {
    fn get(&self, idx: usize, no_data: Option<f64>) -> Option<UnitId> {
        macro_rules! buf_get {
            ($b:expr, $idx:expr, $($variant:ident),+) => {
                match $b {
                    $(RasterBuffer::$variant(b) => {
                        let v = *b.data().get($idx)?;
                        if no_data.is_some_and(|nd| v as f64 == nd) {
                            return None;
                        }
                        Some(v.into())
                    },)+
                }
            };
        }

        buf_get!(
            self, idx, UInt8, Int8, UInt16, Int16, UInt32, Int32, UInt64, Int64, Float32, Float64
        )
    }
}

/// Represents an error meaning that the data type of the raster band is not supported.
#[derive(Debug, Clone)]
struct InvalidRasterDataTypeError(GdalDataType);

impl std::error::Error for InvalidRasterDataTypeError {}

impl fmt::Display for InvalidRasterDataTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid raster data type {}.", self.0)
    }
}

/// Implementation of [`pgaf_sdk::domain::DomainGenerator`] that allows streaming from a GDAL raster dataset.
///
/// Example usage with https://dataverse.harvard.edu/dataset.xhtml?persistentId=doi:10.7910/DVN/1PEEY0:
///
/// Take a raster dataset. Instructions on how to rasterize can be found at [testdata/DSSAT-Soils.tif](testdata/README.md#dssat-soilstif).
///
/// ```rs
/// match RasterDomainGenerator::new("Point5m_SoilGrids-for-DSSAT-10km_v1.tif", 0) {
///     Ok(gen) => for unit in gen {
///         println!("{:?}", unit);
///     },
///     Err(e) => println!("{}", e),
/// }
/// ```
pub struct RasterDomainGenerator {
    ds: Rc<Dataset>,
    no_data_value: Option<f64>,
    band_index: usize,
    px_size_x: f64,
    px_size_y: f64,
    x_size: usize,
    y_size: usize,
    block_x_size: usize,
    block_y_size: usize,
    curr_block_x: usize,
    curr_block_y: usize,
    buffer: Option<RasterBuffer>,
    buffer_x_size: usize,
    buffer_y_size: usize,
    px_idx: usize,
}

#[serde_inline_default]
#[derive(Validate, Deserialize, Clone, Debug)]
pub struct RasterDomainGeneratorConfig {
    pub file: String,

    #[serde_inline_default(0)]
    pub layer_index: usize,
}

pub struct RasterDriver;

impl DomainGeneratorCreate<RasterDomainGeneratorConfig> for RasterDriver {
    type Generator = RasterDomainGenerator;

    fn create(
        config: RasterDomainGeneratorConfig,
    ) -> Result<Self::Generator, Box<dyn std::error::Error>> {
        RasterDomainGenerator::new(config.file.as_str(), config.layer_index)
    }
}

pub static RASTER_DRIVER: LazyLock<pgaf_sdk::domain::Driver> = LazyLock::new(|| {
    DomainGeneratorDriverTyped::<RasterDriver, RasterDomainGeneratorConfig>::default()
        .coerce_to_dynamic()
});

impl RasterDomainGenerator {
    /// Parameter "path" is the GDAL-valid path to the raster dataset.
    /// Parameter "band_index" is the **ZERO-BASED** index of the band to use.
    pub fn new(path: &str, band_index: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let ds = Rc::new(Dataset::open(path)?);
        let band = ds.rasterband(band_index + 1)?;
        let (x_size, y_size) = band.size();

        let (block_x_size, block_y_size) = band.block_size();
        let no_data_value = band.no_data_value();

        // https://gdal.org/en/stable/tutorials/geotransforms_tut.html
        let geo_transform = ds.geo_transform()?;
        let px_size_x = geo_transform[1];
        let px_size_y = -geo_transform[5];

        let mut generator = Self {
            ds,
            no_data_value,
            band_index: band_index + 1,
            px_size_x,
            px_size_y,
            x_size,
            y_size,
            block_x_size,
            block_y_size,
            curr_block_x: 0,
            curr_block_y: 0,
            buffer: None,
            buffer_x_size: 0,
            buffer_y_size: 0,
            px_idx: 0,
        };

        // TODO: Investigate if this is skipping the first pixel.
        generator.load_next_block()?;
        Ok(generator)
    }

    fn load_next_block(&mut self) -> Result<bool, InvalidRasterDataTypeError> {
        if (self.curr_block_y * self.block_y_size) >= self.y_size
            || (self.curr_block_x * self.block_x_size) >= self.x_size
        {
            return Ok(false);
        }

        let bidx = (self.curr_block_x, self.curr_block_y);
        let err_msg = "Failed to read gdal raster block.";
        let band = self.ds.rasterband(self.band_index).unwrap();
        let buffer = match band.band_type() {
            GdalDataType::Int8 => RasterBuffer::Int8(band.read_block(bidx).expect(err_msg)),
            GdalDataType::Int16 => RasterBuffer::Int16(band.read_block(bidx).expect(err_msg)),
            GdalDataType::Int32 => RasterBuffer::Int32(band.read_block(bidx).expect(err_msg)),
            GdalDataType::Int64 => RasterBuffer::Int64(band.read_block(bidx).expect(err_msg)),
            GdalDataType::UInt8 => RasterBuffer::UInt8(band.read_block(bidx).expect(err_msg)),
            GdalDataType::UInt16 => RasterBuffer::UInt16(band.read_block(bidx).expect(err_msg)),
            GdalDataType::UInt32 => RasterBuffer::UInt32(band.read_block(bidx).expect(err_msg)),
            GdalDataType::UInt64 => RasterBuffer::UInt64(band.read_block(bidx).expect(err_msg)),
            GdalDataType::Float32 => RasterBuffer::Float32(band.read_block(bidx).expect(err_msg)),
            GdalDataType::Float64 => RasterBuffer::Float64(band.read_block(bidx).expect(err_msg)),
            other => return Err(InvalidRasterDataTypeError(other)),
        };

        self.buffer_x_size = self
            .block_x_size
            .min(self.x_size - self.curr_block_x * self.block_x_size);

        self.buffer_y_size = self
            .block_y_size
            .min(self.y_size - self.curr_block_y * self.block_y_size);

        self.buffer = Some(buffer);
        self.px_idx = 0;
        Ok(true)
    }
}

impl Iterator for RasterDomainGenerator {
    type Item = ExecutionUnit;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref buffer) = self.buffer
                && self.px_idx < self.buffer_x_size * self.buffer_y_size
            {
                let x_offset = self.px_idx % self.buffer_x_size;
                let y_offset = self.px_idx / self.buffer_x_size;
                let value = buffer.get(self.px_idx, self.no_data_value);
                self.px_idx += 1;

                let Some(value) = value else {
                    continue;
                };

                let x = (self.curr_block_x * self.block_x_size + x_offset) as f64;
                let y = (self.curr_block_y * self.block_y_size + y_offset) as f64;
                let gt = self.ds.geo_transform().unwrap();
                let (lon, lat) = gt.apply(x, y);

                return Some(ExecutionUnit {
                    id: value,
                    lon: GeoDeg::from(lon + (self.px_size_x / 2.0)),
                    lat: GeoDeg::from(lat - (self.px_size_y / 2.0)),
                });
            }

            self.curr_block_x += 1;
            if self.curr_block_x * self.block_x_size >= self.x_size {
                self.curr_block_x = 0;
                self.curr_block_y += 1;
            }

            // TODO: stop silently swallowing the error.
            if !self.load_next_block().unwrap_or(false) {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_raster_domain_generator() {
        let testfile: String = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("testdata")
            .join("DSSAT-Soils.tif")
            .to_string_lossy()
            .to_string();

        dbg!(&testfile);

        let generator = RasterDomainGenerator::new(&testfile, 0).unwrap();

        let expected = vec![
            ExecutionUnit {
                id: 3894630.into(),
                lon: GeoDeg::from(12.5418),
                lat: GeoDeg::from(14.875),
            },
            ExecutionUnit {
                id: 3898947.into(),
                lon: GeoDeg::from(12.2919),
                lat: GeoDeg::from(14.7917),
            },
            ExecutionUnit {
                id: 3898948.into(),
                lon: GeoDeg::from(12.3752),
                lat: GeoDeg::from(14.7917),
            },
            ExecutionUnit {
                id: 3898949.into(),
                lon: GeoDeg::from(12.4585),
                lat: GeoDeg::from(14.7917),
            },
            ExecutionUnit {
                id: 3898975.into(),
                lon: GeoDeg::from(14.6243),
                lat: GeoDeg::from(14.7917),
            },
            ExecutionUnit {
                id: 3898976.into(),
                lon: GeoDeg::from(14.7076),
                lat: GeoDeg::from(14.7917),
            },
            ExecutionUnit {
                id: 3903264.into(),
                lon: GeoDeg::from(12.042),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903265.into(),
                lon: GeoDeg::from(12.1253),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903266.into(),
                lon: GeoDeg::from(12.2086),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903267.into(),
                lon: GeoDeg::from(12.2919),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903268.into(),
                lon: GeoDeg::from(12.3752),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903269.into(),
                lon: GeoDeg::from(12.4585),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903271.into(),
                lon: GeoDeg::from(12.6251),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903273.into(),
                lon: GeoDeg::from(12.7917),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903274.into(),
                lon: GeoDeg::from(12.875),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903279.into(),
                lon: GeoDeg::from(13.2915),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903280.into(),
                lon: GeoDeg::from(13.3748),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903284.into(),
                lon: GeoDeg::from(13.708),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903286.into(),
                lon: GeoDeg::from(13.8746),
                lat: GeoDeg::from(14.7084),
            },
            ExecutionUnit {
                id: 3903293.into(),
                lon: GeoDeg::from(14.4577),
                lat: GeoDeg::from(14.7084),
            },
        ];

        let len = expected.len();

        let mut min_lon: f32 = 180.0;
        let mut max_lon: f32 = -180.0;
        let mut min_lat: f32 = 90.0;
        let mut max_lat: f32 = -90.0;

        let mut i = 0;
        for domain in generator {
            if i < len {
                assert_eq!(domain, expected[i]);
            }

            min_lon = min_lon.min(domain.lon.as_f32());
            max_lon = max_lon.max(domain.lon.as_f32());
            min_lat = min_lat.min(domain.lat.as_f32());
            max_lat = max_lat.max(domain.lat.as_f32());
            i += 1;
        }

        assert_eq!(i, 1157);
        assert_eq!(min_lon, 12.042);
        assert_eq!(max_lon, 14.9575);
        assert_eq!(min_lat, 12.0428);
        assert_eq!(max_lat, 14.875);
    }
}
