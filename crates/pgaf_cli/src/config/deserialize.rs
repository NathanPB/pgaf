use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::path::PathBuf;

use pgaf_sdk::config::{Config, RunConfig, SiteSourceConfig};
use pgaf_sdk::context::ContextValue;
use pgaf_sdk::registry::{
    PublicIdentifierSeed, Registries, ResourceSeed, SiteGeneratorDriverResource,
};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::value::{Map, Value};

use crate::processing::context::ContextValueDeserializeSeed;

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

impl<'de> DeserializeSeed<'de> for ConfigDeserializeSeed<'de, SiteSourceConfig> {
    type Value = SiteSourceConfig;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(ConfigVisitor { seed: self })
    }
}

impl<'de> DeserializeSeed<'de> for ConfigDeserializeSeed<'de, Vec<RunConfig>> {
    type Value = Vec<RunConfig>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ConfigVisitor { seed: self })
    }
}

impl<'de> DeserializeSeed<'de> for ConfigDeserializeSeed<'de, RunConfig> {
    type Value = RunConfig;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(ConfigVisitor { seed: self })
    }
}

impl<'de> DeserializeSeed<'de> for ConfigDeserializeSeed<'de, SiteGeneratorDriverResource> {
    type Value = SiteGeneratorDriverResource;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ResourceSeed {
            registry: self.registries.reg_sitegen_drivers(),
            id_seed: PublicIdentifierSeed {
                default_namespace: self.default_namespace,
            },
        }
        .deserialize(deserializer)
        .map(|r| r.resource)
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

impl<'de> Visitor<'de> for ConfigVisitor<'de, Config> {
    type Value = Config;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a Config struct")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut sites: Option<SiteSourceConfig> = None;
        let mut runs = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "sites" => {
                    sites = Some(map.next_value_seed(self.seed.clone().cast::<SiteSourceConfig>())?)
                }
                "runs" => {
                    runs = Some(map.next_value_seed(self.seed.clone().cast::<Vec<RunConfig>>())?)
                }
                _ => return Err(serde::de::Error::unknown_field(&key, &["sites", "runs"])),
            }
        }

        let sites = sites.ok_or_else(|| serde::de::Error::missing_field("sites"))?;
        let runs = runs.ok_or_else(|| serde::de::Error::missing_field("runs"))?;

        Ok(Config { sites, runs })
    }
}

impl<'de> Visitor<'de> for ConfigVisitor<'de, SiteSourceConfig> {
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
                    resource =
                        Some(map.next_value_seed(
                            self.seed.clone().cast::<SiteGeneratorDriverResource>(),
                        )?)
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
            args: Value::Object(args),
        })
    }
}

impl<'de> Visitor<'de> for ConfigVisitor<'de, Vec<RunConfig>> {
    type Value = Vec<RunConfig>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence of RunConfig objects")
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let mut runs = Vec::with_capacity(seq.size_hint().unwrap_or(0));

        while let Some(run) = seq.next_element_seed(self.seed.clone().cast::<RunConfig>())? {
            runs.push(run);
        }

        Ok(runs)
    }
}

impl<'de> Visitor<'de> for ConfigVisitor<'de, RunConfig> {
    type Value = RunConfig;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a RunConfig object")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut name: Option<String> = None;
        let mut template: Option<PathBuf> = None;
        let mut extra = HashMap::new();

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "name" => {
                    if name.is_some() {
                        return Err(serde::de::Error::duplicate_field("name"));
                    }
                    name = Some(map.next_value()?);
                }
                "template" => {
                    if template.is_some() {
                        return Err(serde::de::Error::duplicate_field("template"));
                    }
                    template = Some(map.next_value()?);
                }
                _ => {
                    let value = map.next_value_seed(self.seed.clone().cast::<ContextValue>())?;
                    extra.insert(key, value);
                }
            }
        }

        let name = name.ok_or_else(|| serde::de::Error::missing_field("name"))?;
        let template = template.ok_or_else(|| serde::de::Error::missing_field("template"))?;

        Ok(RunConfig {
            name,
            template,
            extra,
        })
    }
}
