use super::super::Site;
use crate::data::GeoDeg;
use gdal::raster::{Buffer, GdalDataType};
use gdal::{Dataset, GeoTransformEx};
use std::fmt;
use std::rc::Rc;

/// Represents an error meaning that the desired data type of the raster band is not supported.
#[derive(Debug, Clone)]
struct InvalidRasterDataTypeError {
    expected: GdalDataType,
    actual: GdalDataType,
}

impl InvalidRasterDataTypeError {
    fn new(actual: GdalDataType) -> Self {
        InvalidRasterDataTypeError {
            expected: GdalDataType::Int32,
            actual,
        }
    }
}

impl fmt::Display for InvalidRasterDataTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid raster data type. Expected {}, got {}.",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for InvalidRasterDataTypeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

/// Implementation of SiteGenerator that allows streaming from a GDAL raster dataset.
/// Works only on bands of data type Int32.
///
/// Example usage with https://dataverse.harvard.edu/dataset.xhtml?persistentId=doi:10.7910/DVN/1PEEY0:
///
/// Take a raster dataset. Instructions on how to rasterize can be found at [testdata/DSSAT-Soils.tif](testdata/README.md#dssat-soilstif).
///
/// ```rs
/// match RasterSiteGenerator::new("Point5m_SoilGrids-for-DSSAT-10km_v1.tif", 0) {
///     Ok(gen) => for site in gen {
///         println!("{:?}", site);
///     },
///     Err(e) => println!("{}", e),
/// }
/// ```
pub struct RasterSiteGenerator {
    ds: Rc<Dataset>,
    no_data_value: i32,
    band_index: usize,
    px_size_x: f64,
    px_size_y: f64,
    x_size: usize,
    y_size: usize,
    block_x_size: usize,
    block_y_size: usize,
    curr_block_x: usize,
    curr_block_y: usize,
    buffer: Option<Buffer<i32>>,
    buffer_x_size: usize,
    buffer_y_size: usize,
    px_idx: usize,
}

impl RasterSiteGenerator {
    /// Constructs a new RasterSiteGenerator.
    /// Parameter "path" is the GDAL-valid path to the raster dataset.
    /// Parameter "band_index" is the **ZERO-BASED** index of the band to use.
    pub fn new(path: &str, band_index: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let ds = Rc::new(Dataset::open(path)?);
        let band = ds.rasterband(band_index + 1)?;
        let (x_size, y_size) = band.size();

        let band_type = band.band_type();
        if band_type != GdalDataType::Int32 {
            return Err(Box::new(InvalidRasterDataTypeError::new(band_type)));
        }

        let (block_x_size, block_y_size) = band.block_size();
        let no_data_value = band.no_data_value().unwrap_or(0.0) as i32;

        // https://gdal.org/en/stable/tutorials/geotransforms_tut.html
        let geo_transform = ds.geo_transform()?;
        let px_size_x = geo_transform[1];
        let px_size_y = -geo_transform[5];

        let mut gen = Self {
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

        gen.load_next_block();
        Ok(gen)
    }

    fn load_next_block(&mut self) -> bool {
        if (self.curr_block_y * self.block_y_size) >= self.y_size
            || (self.curr_block_x * self.block_x_size) >= self.x_size
        {
            return false;
        }

        match self
            .ds
            .rasterband(self.band_index)
            .unwrap()
            .read_block((self.curr_block_x, self.curr_block_y))
        {
            Ok(buffer) => {
                self.buffer_x_size = self
                    .block_x_size
                    .min(self.x_size - self.curr_block_x * self.block_x_size);
                self.buffer_y_size = self
                    .block_y_size
                    .min(self.y_size - self.curr_block_y * self.block_y_size);
                self.buffer = Some(buffer);
                self.px_idx = 0;
                true
            }
            Err(_) => false,
        }
    }
}

impl Iterator for RasterSiteGenerator {
    type Item = Site;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref buffer) = self.buffer {
                if self.px_idx < self.buffer_x_size * self.buffer_y_size {
                    let x_offset = self.px_idx % self.buffer_x_size;
                    let y_offset = self.px_idx / self.buffer_x_size;
                    let value = buffer.data()[self.px_idx];
                    self.px_idx += 1;
                    if value == self.no_data_value {
                        continue;
                    }

                    let x = (self.curr_block_x * self.block_x_size + x_offset) as f64;
                    let y = (self.curr_block_y * self.block_y_size + y_offset) as f64;
                    let gt = self.ds.geo_transform().unwrap();
                    let (lon, lat) = gt.apply(x, y);

                    return Some(Site {
                        id: value,
                        lon: GeoDeg::from(lon + (self.px_size_x / 2.0)),
                        lat: GeoDeg::from(lat - (self.px_size_y / 2.0)),
                    });
                }
            }

            self.curr_block_x += 1;
            if self.curr_block_x * self.block_x_size >= self.x_size {
                self.curr_block_x = 0;
                self.curr_block_y += 1;
            }

            if !self.load_next_block() {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raster_site_generator() {
        let gen = RasterSiteGenerator::new("testdata/DSSAT-Soils.tif", 0).unwrap();

        let expected = vec![
            Site {
                id: 3894630,
                lon: GeoDeg::from(12.5418),
                lat: GeoDeg::from(14.875),
            },
            Site {
                id: 3898947,
                lon: GeoDeg::from(12.2919),
                lat: GeoDeg::from(14.7917),
            },
            Site {
                id: 3898948,
                lon: GeoDeg::from(12.3752),
                lat: GeoDeg::from(14.7917),
            },
            Site {
                id: 3898949,
                lon: GeoDeg::from(12.4585),
                lat: GeoDeg::from(14.7917),
            },
            Site {
                id: 3898975,
                lon: GeoDeg::from(14.6243),
                lat: GeoDeg::from(14.7917),
            },
            Site {
                id: 3898976,
                lon: GeoDeg::from(14.7076),
                lat: GeoDeg::from(14.7917),
            },
            Site {
                id: 3903264,
                lon: GeoDeg::from(12.042),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903265,
                lon: GeoDeg::from(12.1253),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903266,
                lon: GeoDeg::from(12.2086),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903267,
                lon: GeoDeg::from(12.2919),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903268,
                lon: GeoDeg::from(12.3752),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903269,
                lon: GeoDeg::from(12.4585),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903271,
                lon: GeoDeg::from(12.6251),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903273,
                lon: GeoDeg::from(12.7917),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903274,
                lon: GeoDeg::from(12.875),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903279,
                lon: GeoDeg::from(13.2915),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903280,
                lon: GeoDeg::from(13.3748),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903284,
                lon: GeoDeg::from(13.708),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903286,
                lon: GeoDeg::from(13.8746),
                lat: GeoDeg::from(14.7084),
            },
            Site {
                id: 3903293,
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
        for site in gen {
            if i < len {
                assert_eq!(site, expected[i]);
            }

            min_lon = min_lon.min(site.lon.as_f32());
            max_lon = max_lon.max(site.lon.as_f32());
            min_lat = min_lat.min(site.lat.as_f32());
            max_lat = max_lat.max(site.lat.as_f32());
            i += 1;
        }

        assert_eq!(i, 1157);
        assert_eq!(min_lon, 12.042);
        assert_eq!(max_lon, 14.9575);
        assert_eq!(min_lat, 12.0428);
        assert_eq!(max_lat, 14.875);
    }
}
