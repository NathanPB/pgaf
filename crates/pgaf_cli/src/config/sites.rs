use crate::registry::resources::SiteGeneratorDriverResource;
use crate::registry::ResourceSeed;
use crate::sites::{SiteGenerator, SiteGeneratorDriver};
use serde::de::{DeserializeSeed, MapAccess, Visitor};
use serde_json::Map;
use std::any::Any;
use std::error::Error;
use std::fmt;
use validator::Validate;

#[derive(Validate, Clone)]
pub struct SiteSourceConfig {
    pub driver: SiteGeneratorDriver<Box<dyn SiteGenerator>, Box<dyn Any>>,
    pub sample_size: Option<usize>,
    args: serde_json::Value,
}

impl SiteSourceConfig {
    pub fn build(&self) -> Result<Box<dyn SiteGenerator>, Box<dyn Error>> {
        let config = (self.driver.config_deserializer)(self.args.clone())?;
        (self.driver.create)(config)
    }
}

#[derive(Clone)]
pub struct SiteSourceConfigSeed<'a> {
    pub resource_seed: ResourceSeed<'a, SiteGeneratorDriverResource>,
}

impl<'de> DeserializeSeed<'de> for SiteSourceConfigSeed<'de> {
    type Value = SiteSourceConfig;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(SiteSourceConfigVisitor { seed: self })
    }
}

struct SiteSourceConfigVisitor<'a> {
    seed: SiteSourceConfigSeed<'a>,
}

impl<'de> Visitor<'de> for SiteSourceConfigVisitor<'de> {
    type Value = SiteSourceConfig;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a SiteSourceConfig struct")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut resource: Option<SiteGeneratorDriverResource> = None;
        let mut sample_size = None;
        let mut args: Map<String, serde_json::Value> = Map::new();

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "type" => {
                    resource = Some(
                        map.next_value_seed(self.seed.resource_seed.clone())?
                            .resource,
                    )
                }
                "sample_size" => sample_size = Some(map.next_value()?),
                _ => {
                    args.insert(key.to_string(), map.next_value()?);
                }
            }
        }

        let resource = resource.ok_or_else(|| serde::de::Error::missing_field("type"))?;
        Ok(SiteSourceConfig {
            driver: resource.0,
            sample_size,
            args: serde_json::Value::Object(args),
        })
    }
}
