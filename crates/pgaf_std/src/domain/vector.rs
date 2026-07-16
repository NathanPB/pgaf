use gdal::Dataset;
use gdal::errors::GdalError;
use gdal::vector::{Feature, FeatureIterator, FieldValue, Layer, LayerAccess};
use pgaf_sdk::data::GeoDeg;
use pgaf_sdk::domain::{DomainGeneratorDriver, ExecutionUnit, UnitId};
use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::LazyLock;
use validator::Validate;

/// Implementation of [`pgaf_sdk::domain::DomainGenerator`] that allows streaming from a GDAL vector dataset.
/// Example usage with https://dataverse.harvard.edu/dataset.xhtml?persistentId=doi:10.7910/DVN/1PEEY0:
/// ```rs
/// match VectorDomainGenerator::new("Point5m_SoilGrids-for-DSSAT-10km_v1.shp.zip", "CELL5M".to_string()) {
///     Ok(gen) => for unit in gen {
///         println!("{:?}", unit);
///     },
///     Err(e) => println!("{}", e),
/// }
/// ```
pub struct VectorDomainGenerator {
    unit_id_key: String,
    ds: Rc<Dataset>,
    curr_layer: usize,
    layer: Option<Layer<'static>>,
    feat_iter: Box<Option<FeatureIterator<'static>>>,
}

#[serde_inline_default]
#[derive(Validate, Deserialize, Clone, Debug)]
pub struct VectorDomainGeneratorConfig {
    #[validate(length(min = 1, message = "Vector file path cannot be empty"))]
    pub file: String,

    #[serde_inline_default("ID".to_string())]
    #[validate(length(min = 1, message = "ExecutionUnit ID key cannot be empty"))]
    pub unit_id_key: String,
}

pub const VECTOR_DRIVER: LazyLock<
    DomainGeneratorDriver<VectorDomainGenerator, VectorDomainGeneratorConfig>,
> = LazyLock::new(|| DomainGeneratorDriver {
    create: Arc::new(|c: VectorDomainGeneratorConfig| {
        VectorDomainGenerator::new(c.file.as_str(), c.unit_id_key)
    }),
    config_deserializer: Arc::new(serde_json::from_value),
});

impl VectorDomainGenerator {
    /// Constructs from a GDAL vector dataset.
    /// Parameter "path" is the GDAL-valid path to the dataset.
    /// Parameter "unit_id_key" is the name of the field in the dataset that contains the unit ID. Must be an int32, otherwise the feature is skipped.
    pub fn new(path: &str, unit_id_key: String) -> Result<Self, Box<dyn std::error::Error>> {
        let ds = Rc::new(Dataset::open(path)?);
        Ok(VectorDomainGenerator {
            unit_id_key,
            ds,
            curr_layer: 0,
            layer: None,
            feat_iter: Box::new(None),
        })
    }
}

impl Iterator for VectorDomainGenerator {
    type Item = ExecutionUnit;

    fn next(&mut self) -> Option<Self::Item> {
        if self.feat_iter.is_none() {
            self.layer = self
                .ds
                .layer(self.curr_layer)
                .ok()
                .map(|l| unsafe { std::mem::transmute::<Layer, Layer<'static>>(l) });

            if let Some(layer) = self.layer.as_mut() {
                *self.feat_iter = Some(unsafe {
                    std::mem::transmute::<FeatureIterator, FeatureIterator<'static>>(
                        layer.features(),
                    )
                });
                return self.next();
            }
            return None;
        }

        match self.feat_iter.as_mut() {
            Some(feat_iter) => match feat_iter.next() {
                Some(feat) => feature_to_unit(&feat, &self.unit_id_key),
                None => {
                    self.curr_layer += 1;
                    *self.feat_iter = None;
                    self.next()
                }
            },
            None => None,
        }
    }
}

fn feature_to_unit(feature: &Feature, unit_id_key: &str) -> Option<ExecutionUnit> {
    // TODO: Fix the error handling of this nightmare.
    // Currently, errors are just being silently swallowed.

    if let Some(geometry) = feature.geometry() {
        if geometry.geometry_type() != gdal::vector::OGRwkbGeometryType::wkbPoint {
            return None;
        }

        let field_idx = feature.field_index(unit_id_key).ok()?;
        let id_result: Result<UnitId, _> = feature
            .field(field_idx)
            .and_then(|id| {
                id.ok_or(GdalError::NullPointer {
                    method_name: "pgaf_std::domain::vector::feature_to_unit",
                    msg: format!("Feature has no field '{unit_id_key}'."),
                })
            })
            .and_then(|id| {
                let ogr_type = id.ogr_field_type();
                match id {
                    FieldValue::IntegerValue(v) => Ok(v.into()),
                    FieldValue::Integer64Value(v) => Ok(v.into()),
                    FieldValue::StringValue(v) => Ok(v.into()),
                    FieldValue::RealValue(v) => Ok(v.into()),
                    _ => Err(GdalError::NullPointer {
                        method_name: "pgaf_std::domain::vector::feature_to_unit",
                        msg: format!(
                            "Feature ID is not of any compatible type. Found ORG type {ogr_type}.",
                        ),
                    }),
                }
            });

        if let Ok(id) = id_result {
            let (lon, lat, _) = geometry.get_point(0);
            return Some(ExecutionUnit {
                id,
                lon: GeoDeg::from(lon),
                lat: GeoDeg::from(lat),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_vector_domain_generator() {
        let testfile = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("testdata")
            .join("DSSAT-Soils.shp.zip")
            .into_string()
            .unwrap();

        let generator = VectorDomainGenerator::new(&testfile, "CELL5M".to_string()).unwrap();

        let expected = vec![
            ExecutionUnit {
                id: 3989689.into(),
                lon: GeoDeg::from(14.125),
                lat: GeoDeg::from(13.042),
            },
            ExecutionUnit {
                id: 3989690.into(),
                lon: GeoDeg::from(14.208),
                lat: GeoDeg::from(13.042),
            },
            ExecutionUnit {
                id: 3989691.into(),
                lon: GeoDeg::from(14.292),
                lat: GeoDeg::from(13.042),
            },
            ExecutionUnit {
                id: 3989692.into(),
                lon: GeoDeg::from(14.375),
                lat: GeoDeg::from(13.042),
            },
            ExecutionUnit {
                id: 3989693.into(),
                lon: GeoDeg::from(14.458),
                lat: GeoDeg::from(13.042),
            },
            ExecutionUnit {
                id: 3994009.into(),
                lon: GeoDeg::from(14.125),
                lat: GeoDeg::from(12.958),
            },
            ExecutionUnit {
                id: 3994010.into(),
                lon: GeoDeg::from(14.208),
                lat: GeoDeg::from(12.958),
            },
            ExecutionUnit {
                id: 3994011.into(),
                lon: GeoDeg::from(14.292),
                lat: GeoDeg::from(12.958),
            },
            ExecutionUnit {
                id: 3994012.into(),
                lon: GeoDeg::from(14.375),
                lat: GeoDeg::from(12.958),
            },
            ExecutionUnit {
                id: 3994013.into(),
                lon: GeoDeg::from(14.458),
                lat: GeoDeg::from(12.958),
            },
            ExecutionUnit {
                id: 3998329.into(),
                lon: GeoDeg::from(14.125),
                lat: GeoDeg::from(12.875),
            },
            ExecutionUnit {
                id: 3998330.into(),
                lon: GeoDeg::from(14.208),
                lat: GeoDeg::from(12.875),
            },
            ExecutionUnit {
                id: 3998331.into(),
                lon: GeoDeg::from(14.292),
                lat: GeoDeg::from(12.875),
            },
            ExecutionUnit {
                id: 3998332.into(),
                lon: GeoDeg::from(14.375),
                lat: GeoDeg::from(12.875),
            },
            ExecutionUnit {
                id: 3998333.into(),
                lon: GeoDeg::from(14.458),
                lat: GeoDeg::from(12.875),
            },
            ExecutionUnit {
                id: 3998334.into(),
                lon: GeoDeg::from(14.542),
                lat: GeoDeg::from(12.875),
            },
            ExecutionUnit {
                id: 4002650.into(),
                lon: GeoDeg::from(14.208),
                lat: GeoDeg::from(12.792),
            },
            ExecutionUnit {
                id: 4002651.into(),
                lon: GeoDeg::from(14.292),
                lat: GeoDeg::from(12.792),
            },
            ExecutionUnit {
                id: 4002652.into(),
                lon: GeoDeg::from(14.375),
                lat: GeoDeg::from(12.792),
            },
            ExecutionUnit {
                id: 4002653.into(),
                lon: GeoDeg::from(14.458),
                lat: GeoDeg::from(12.792),
            },
        ];

        let len = expected.len();

        let mut min_lon: f32 = 180.0;
        let mut max_lon: f32 = -180.0;
        let mut min_lat: f32 = 90.0;
        let mut max_lat: f32 = -90.0;

        let mut i = 0;
        for unit in generator {
            if i < len {
                assert_eq!(unit, expected[i]);
            }

            min_lon = min_lon.min(unit.lon.as_f32());
            max_lon = max_lon.max(unit.lon.as_f32());
            min_lat = min_lat.min(unit.lat.as_f32());
            max_lat = max_lat.max(unit.lat.as_f32());
            i += 1;
        }

        assert_eq!(i, 1157);
        assert_eq!(min_lon, 12.042);
        assert_eq!(max_lon, 14.958);
        assert_eq!(min_lat, 12.042);
        assert_eq!(max_lat, 14.875);
    }
}
