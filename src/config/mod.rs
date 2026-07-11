pub mod runs;
pub mod sites;

use crate::config::sites::{SiteSourceConfig, SiteSourceConfigSeed};
use crate::registry::{PublicIdentifierSeed, Registries, ResourceSeed};
use clap::Parser;
use runs::*;
use serde::de::{DeserializeSeed, MapAccess, Visitor};
use serde_inline_default::serde_inline_default;
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;
use validator::{Validate, ValidationError};

static ERRCODE_WORKDIR_NOT_DIR: &str = "ERRCODE_WORKDIR_NOT_DIR";
static ERRCODE_WORKDIR_NOT_EMPTY: &str = "ERRCODE_WORKDIR_NOT_EMPTY";

fn validate_workdir_is_directory(path: &PathBuf) -> Result<(), ValidationError> {
    if path.exists() && !path.is_dir() {
        return Err(
            ValidationError::new(ERRCODE_WORKDIR_NOT_DIR).with_message(Cow::from(format!(
                "Working directory {} is not a directory.",
                path.display()
            ))),
        );
    }

    Ok(())
}

fn validate_workdir_overrides(args: &Args) -> Result<(), ValidationError> {
    if let Some(path) = &args.workdir {
        if !args.clear_workdir {
            match path.read_dir() {
                Ok(entries) => {
                    if entries.count() > 0 {
                        let msg = format!("Working directory {} is not empty. Specify --clear-workdir to FORCEFULLY OVERWRITE it.", path.display());
                        return Err(ValidationError::new(ERRCODE_WORKDIR_NOT_EMPTY)
                            .with_message(Cow::from(msg)));
                    }
                }
                Err(err) => match err.kind() {
                    std::io::ErrorKind::NotFound => {}
                    _ => panic!(
                        "Unexpected error when checking workdir availability: {}",
                        err
                    ),
                },
            }
        }
    }
    Ok(())
}

#[derive(Validate, Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to the JSON configuration file.
    #[arg(short, long, default_value = "config.json")]
    pub config_file: String,

    /// Number of workers to use for parallel processing. If 0, will use all available cores.
    #[arg(short, long, default_value_t = 0)]
    pub workers: usize,

    /// Size of the buffer between each step of the processing pipeline. Defaults to 128.
    #[arg(short, long, default_value_t = 128)]
    pub pipeline_buffer_size: usize,

    /// Specify the working directory, created recursively if needed. If not specified, a temporary one will be created.
    /// Check --keep-workdir and --clear-workdir to control the behavior of the working directory.
    /// By default, the program will halt execution if the specified --workdir is not empty, unless --clear-workdir is specified.
    #[arg(short = 'd', long)]
    #[validate(custom(function = "validate_workdir_is_directory"))]
    pub workdir: Option<PathBuf>,

    /// Keeps the working directory after completed. Defaults to true if --workdir is specified.
    /// This option has NO effect if combined with --workdir (directory will always be kept).
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    pub keep_workdir: Option<bool>,

    #[arg(long, action = clap::ArgAction::SetTrue, default_value_t = false)]
    /// Overrides the working directory if it isn't already empty. This option has NO effect if not combined with --workdir (directory will always be kep).
    /// By default, the program will halt execution if the specified --workdir is not empty, unless --clear-workdir is specified.
    pub clear_workdir: bool,
}

#[serde_inline_default]
#[derive(Validate, Clone)]
pub struct Config {
    pub sites: SiteSourceConfig,

    #[validate(length(min = 1, message = "At least one run is required"))]
    #[validate(nested)]
    #[validate(custom(function = "validate_unique_run_names"))]
    pub runs: Vec<RunConfig>,
}

#[derive(Debug, Error)]
pub enum ConfigSeedBuilderError {
    #[error("Missing default namespace")]
    MissingDefaultNamespace,
    #[error("Missing registries")]
    MissingRegistries,
}

pub struct ConfigSeedBuilder<'a> {
    default_namespace: Option<String>,
    registries: Option<&'a Registries>,
}

impl<'a> Default for ConfigSeedBuilder<'a> {
    fn default() -> Self {
        Self {
            default_namespace: None,
            registries: None,
        }
    }
}

impl<'a> ConfigSeedBuilder<'a> {
    pub fn with_default_namespace(mut self, default_namespace: String) -> Self {
        self.default_namespace = Some(default_namespace);
        self
    }

    pub fn with_registries(mut self, registries: &'a Registries) -> Self {
        self.registries = Some(registries);
        self
    }

    pub fn build(self) -> Result<ConfigSeed<'a>, ConfigSeedBuilderError> {
        let registries = self
            .registries
            .ok_or(ConfigSeedBuilderError::MissingRegistries)?;

        let default_namespace = self
            .default_namespace
            .ok_or(ConfigSeedBuilderError::MissingDefaultNamespace)?;

        Ok(ConfigSeed {
            sites_seed: SiteSourceConfigSeed {
                resource_seed: ResourceSeed {
                    registry: registries.reg_sitegen_drivers(),
                    id_seed: PublicIdentifierSeed {
                        default_namespace: default_namespace.clone(),
                    },
                },
            },
            default_namespace,
            registries,
        })
    }
}

pub struct ConfigSeed<'a> {
    pub sites_seed: SiteSourceConfigSeed<'a>,
    pub default_namespace: String,
    pub registries: &'a Registries,
}

impl<'de> DeserializeSeed<'de> for ConfigSeed<'de> {
    type Value = Config;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(ConfigVisitor { seed: self })
    }
}

struct ConfigVisitor<'a> {
    pub seed: ConfigSeed<'a>,
}

impl<'de> Visitor<'de> for ConfigVisitor<'de> {
    type Value = Config;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a Config struct")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut sites = None;
        let mut runs = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "sites" => sites = Some(map.next_value_seed(self.seed.sites_seed.clone())?),
                "runs" => {
                    runs = Some(map.next_value_seed(RunConfigVecSeed {
                        default_namespace: self.seed.default_namespace.clone(),
                        registries: self.seed.registries,
                    })?)
                }
                _ => return Err(serde::de::Error::unknown_field(&key, &["sites"])),
            }
        }

        let sites = sites.ok_or_else(|| serde::de::Error::missing_field("sites"))?;
        let runs = runs.ok_or_else(|| serde::de::Error::missing_field("runs"))?;

        Ok(Config { sites, runs })
    }
}

fn validate(args: &Args, config: &Config) -> Result<(), ConfigError> {
    args.validate()
        .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?;

    validate_workdir_overrides(args).map_err(|e| ConfigError::ArgsValidationError(e))?;

    config
        .validate()
        .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Config file not found at path {0}")]
    ConfigFileNotFound(PathBuf),
    #[error("Config load failed: {0}")]
    ConfigLoadError(Box<dyn Error>),
    #[error("Arguments validation failed: {0}")]
    ArgsValidationError(ValidationError),
}

pub fn init(seed: ConfigSeed) -> Result<(Config, Args, PathBuf), ConfigError> {
    let args = Args::parse();
    let path = PathBuf::from(&args.config_file.clone());
    if !path.exists() || !path.is_file() {
        return Err(ConfigError::ConfigFileNotFound(path.clone()));
    }

    let json_str = std::fs::read_to_string(args.config_file.clone())
        .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?;

    let config: Config = seed
        .deserialize(&mut serde_json::Deserializer::from_str(&json_str))
        .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?;

    validate(&args, &config)?;

    Ok((config, args, path))
}
