use std::fmt;
use std::marker::PhantomData;

use pgaf_engine::context::value::ContextValueDeserializeSeed;
use pgaf_sdk::config::{Config, DomainConfig, PipelineStep};
use pgaf_sdk::context::ContextValue;
use pgaf_sdk::registry::{
    DomainGeneratorDriverResource, PublicIdentifierSeed, Registries, ResourceSeed,
};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::value::{Map, Value};

pub struct ConfigDeserializeSeed<'a, T = Config> {
    pub default_namespace: String,
    pub registries: &'a Registries,
    _marker: PhantomData<T>,
}

impl<'a, T> ConfigDeserializeSeed<'a, T> {
    fn cast<U>(self) -> ConfigDeserializeSeed<'a, U> {
        ConfigDeserializeSeed {
            default_namespace: self.default_namespace,
            registries: self.registries,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> Clone for ConfigDeserializeSeed<'a, T> {
    fn clone(&self) -> Self {
        ConfigDeserializeSeed {
            default_namespace: self.default_namespace.clone(),
            registries: self.registries,
            _marker: PhantomData,
        }
    }
}

impl<'a> ConfigDeserializeSeed<'a, Config> {
    pub fn new(registries: &'a Registries, default_namespace: &str) -> Self {
        Self {
            default_namespace: default_namespace.to_string(),
            registries,
            _marker: PhantomData,
        }
    }
}

struct ConfigVisitor<'a, T> {
    seed: ConfigDeserializeSeed<'a, T>,
}

impl<'de> DeserializeSeed<'de> for ConfigDeserializeSeed<'de, Config> {
    type Value = Config;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(ConfigVisitor { seed: self })
    }
}

impl<'de> DeserializeSeed<'de> for ConfigDeserializeSeed<'de, DomainConfig> {
    type Value = DomainConfig;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(ConfigVisitor { seed: self })
    }
}

impl<'de> DeserializeSeed<'de> for ConfigDeserializeSeed<'de, DomainGeneratorDriverResource> {
    type Value = DomainGeneratorDriverResource;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ResourceSeed {
            registry: self.registries.reg_domaingen_drivers(),
            id_seed: PublicIdentifierSeed {
                default_namespace: self.default_namespace,
            },
        }
        .deserialize(deserializer)
        .map(|r| r.resource)
    }
}

impl<'de> DeserializeSeed<'de> for ConfigDeserializeSeed<'de, pgaf_sdk::pipeline::Driver> {
    type Value = pgaf_sdk::pipeline::Driver;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ResourceSeed {
            registry: self.registries.reg_pipelinestep_drivers(),
            id_seed: PublicIdentifierSeed {
                default_namespace: self.default_namespace,
            },
        }
        .deserialize(deserializer)
        .map(|r| r.resource.0)
    }
}

impl<'de> DeserializeSeed<'de> for ConfigDeserializeSeed<'de, ContextValue> {
    type Value = ContextValue;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ContextValueDeserializeSeed {
            default_namespace: self.default_namespace,
            registries: self.registries,
        }
        .deserialize(deserializer)
    }
}

impl<'de> DeserializeSeed<'de> for ConfigDeserializeSeed<'de, PipelineStep> {
    type Value = PipelineStep;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(ConfigVisitor { seed: self })
    }
}

impl<'de> DeserializeSeed<'de> for ConfigDeserializeSeed<'de, Vec<PipelineStep>> {
    type Value = Vec<PipelineStep>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ConfigVisitor { seed: self })
    }
}

impl<'de> Visitor<'de> for ConfigVisitor<'de, Config> {
    type Value = Config;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("{ domain: {...}, pipeline: {...}[], runs: {...}[] }")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut domain: Option<DomainConfig> = None;
        let mut pipeline: Option<Vec<PipelineStep>> = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "domain" => {
                    domain = Some(map.next_value_seed(self.seed.clone().cast::<DomainConfig>())?)
                }
                "pipeline" => {
                    pipeline =
                        Some(map.next_value_seed(self.seed.clone().cast::<Vec<PipelineStep>>())?)
                }
                _ => {
                    return Err(serde::de::Error::unknown_field(
                        &key,
                        &["domain", "pipeline"],
                    ));
                }
            }
        }

        Ok(Config {
            domain: domain.ok_or_else(|| serde::de::Error::missing_field("domain"))?,
            pipeline: pipeline.ok_or_else(|| serde::de::Error::missing_field("pipeline"))?,
        })
    }
}

impl<'de> Visitor<'de> for ConfigVisitor<'de, DomainConfig> {
    type Value = DomainConfig;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a DomainConfig struct")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut resource: Option<DomainGeneratorDriverResource> = None;
        let mut sample_size = None;
        let mut args: Map<String, serde_json::Value> = Map::new();

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "type" => {
                    resource = Some(map.next_value_seed(
                        self.seed.clone().cast::<DomainGeneratorDriverResource>(),
                    )?)
                }
                "sample_size" => sample_size = Some(map.next_value()?),
                _ => {
                    args.insert(key.to_string(), map.next_value()?);
                }
            }
        }

        let resource = resource.ok_or_else(|| serde::de::Error::missing_field("type"))?;
        Ok(DomainConfig {
            driver: resource.0,
            sample_size,
            args: Value::Object(args),
        })
    }
}

impl<'de> Visitor<'de> for ConfigVisitor<'de, PipelineStep> {
    type Value = PipelineStep;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("{name: string, type: string, args: any}")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut name: Option<String> = None;
        let mut driver: Option<pgaf_sdk::pipeline::Driver> = None;
        let mut args: Option<serde_json::Value> = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "name" => name = Some(map.next_value()?),
                "type" => {
                    let seed = self.seed.clone().cast::<pgaf_sdk::pipeline::Driver>();
                    driver = Some(map.next_value_seed(seed)?)
                }
                "args" => args = Some(map.next_value()?),
                _ => {
                    return Err(serde::de::Error::unknown_field(
                        &key,
                        &["name", "type", "args"],
                    ));
                }
            }
        }

        Ok(PipelineStep {
            name: name.ok_or_else(|| serde::de::Error::missing_field("name"))?,
            driver: driver.ok_or_else(|| serde::de::Error::missing_field("type"))?,
            args: args.ok_or_else(|| serde::de::Error::missing_field("args"))?,
        })
    }
}

impl<'de> Visitor<'de> for ConfigVisitor<'de, Vec<PipelineStep>> {
    type Value = Vec<PipelineStep>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("{...}[]")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut steps = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(step) = seq.next_element_seed(self.seed.clone().cast::<PipelineStep>())? {
            steps.push(step);
        }

        Ok(steps)
    }
}
