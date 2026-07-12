use crate::processing::context::{ContextValue, ContextValueDeserializeSeed};
use crate::registry::Registries;
use regex::Regex;
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use validator::{Validate, ValidationError};

static ERRCODE_RUN_NAME_DUPE: &str = "ERRCODE_RUN_NAME_DUPE";
static ERRCODE_TEMPLATE_FILE_NOT_FOUND: &str = "ERRCODE_TEMPLATE_FILE_NOT_FOUND";

static RE_VALID_RUN_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());

pub fn validate_unique_run_names(runs: &Vec<RunConfig>) -> Result<(), ValidationError> {
    let mut run_names = HashSet::new();

    for run in runs {
        if run_names.contains(&run.name) {
            let msg = format!("Run name {} is not unique", run.name);
            return Err(ValidationError::new(ERRCODE_RUN_NAME_DUPE).with_message(Cow::from(msg)));
        }
        run_names.insert(run.name.clone());
    }

    Ok(())
}

fn validate_template_file_exists(path: &Path) -> Result<(), ValidationError> {
    if !path.exists() || path.is_dir() {
        let msg = format!(
            "Template file {} does not exist or is not a file",
            path.display()
        );

        return Err(
            ValidationError::new(ERRCODE_TEMPLATE_FILE_NOT_FOUND).with_message(Cow::from(msg))
        );
    }
    Ok(())
}

#[derive(Validate, Serialize, Deserialize, Clone, Debug)]
pub struct RunConfig {
    #[validate(regex(path = *RE_VALID_RUN_NAME, message = "Run name must be alphanumeric and contain only underscores and dashes"))]
    pub name: String,

    #[validate(custom(function = "validate_template_file_exists"))]
    pub template: PathBuf,

    #[serde(flatten)]
    pub extra: HashMap<String, ContextValue>,
}

pub struct RunConfigDeserializerSeed<'a> {
    pub registries: &'a Registries,
    pub default_namespace: String,
}

impl<'de> DeserializeSeed<'de> for RunConfigDeserializerSeed<'de> {
    type Value = RunConfig;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(RunConfigVisitor {
            default_namespace: self.default_namespace,
            registries: self.registries,
        })
    }
}

struct RunConfigVisitor<'a> {
    default_namespace: String,
    registries: &'a Registries,
}

impl<'de> Visitor<'de> for RunConfigVisitor<'de> {
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
                    let value = map.next_value_seed(ContextValueDeserializeSeed {
                        default_namespace: self.default_namespace.clone(),
                        registries: self.registries,
                    })?;
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

pub struct RunConfigVecSeed<'a> {
    pub registries: &'a Registries,
    pub default_namespace: String,
}

impl<'de> DeserializeSeed<'de> for RunConfigVecSeed<'de> {
    type Value = Vec<RunConfig>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(RunConfigVecVisitor {
            default_namespace: self.default_namespace,
            registries: self.registries,
        })
    }
}

struct RunConfigVecVisitor<'a> {
    default_namespace: String,
    registries: &'a Registries,
}

impl<'de> Visitor<'de> for RunConfigVecVisitor<'de> {
    type Value = Vec<RunConfig>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence of RunConfig objects")
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let mut runs = Vec::with_capacity(seq.size_hint().unwrap_or(0));

        while let Some(run) = seq.next_element_seed(RunConfigDeserializerSeed {
            default_namespace: self.default_namespace.clone(),
            registries: self.registries,
        })? {
            runs.push(run);
        }

        Ok(runs)
    }
}
