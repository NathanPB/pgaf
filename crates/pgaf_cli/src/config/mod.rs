mod deserialize;
mod validate;

use clap::Parser;
use deserialize::deserialize_public_identifier;
use pgaf_sdk::registry;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use std::path::PathBuf;
use validate::{RE_GENERAL_NAME, validate_unique_pipeline_names, validate_unique_sink_names};
use validator::{Validate, ValidationError};

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found at path {0}")]
    ConfigFileNotFound(PathBuf),
    #[error("Config load failed: {0}")]
    ConfigLoadError(Box<dyn std::error::Error>),
    #[error("Arguments validation failed: {0}")]
    ArgsValidationError(#[from] ValidationError),
}

#[serde_inline_default]
#[derive(Validate, Debug, Deserialize, Clone)]
pub struct Config {
    pub domain: DomainConfig,

    #[validate(nested)]
    #[validate(custom(function = "validate_unique_pipeline_names"))]
    pub pipeline: Vec<PipelineStep>,

    #[validate(nested)]
    #[validate(custom(function = "validate_unique_sink_names"))]
    #[serde(default)]
    pub sink: Vec<Sink>,
}

#[derive(Validate, Debug, Deserialize, Clone)]
pub struct DomainConfig {
    #[serde(deserialize_with = "deserialize_public_identifier")]
    pub r#type: registry::PublicIdentifier,
    pub sample_size: Option<usize>,
    #[serde(flatten)]
    pub args: serde_json::Value,
}

#[derive(Validate, Serialize, Deserialize, Clone, Debug)]
pub struct PipelineStep {
    #[validate(regex(path = *RE_GENERAL_NAME, message = "Pipeline name must be alphanumeric and contain only underscores and dashes"))]
    pub name: String,
    #[serde(deserialize_with = "deserialize_public_identifier")]
    pub r#type: registry::PublicIdentifier,
    pub args: serde_json::Value,
}

#[derive(Validate, Serialize, Deserialize, Clone, Debug)]
pub struct Sink {
    #[validate(regex(path = *RE_GENERAL_NAME, message = "Sink name must be alphanumeric and contain only underscores and dashes"))]
    pub name: String,
    #[serde(deserialize_with = "deserialize_public_identifier")]
    pub r#type: registry::PublicIdentifier,
    pub args: serde_json::Value,
}

#[derive(Validate, Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to the JSON or YAML configuration file.
    #[arg(short, long)]
    pub config_file: PathBuf,

    /// Number of workers to use for parallel processing. If 0, will use all available cores.
    #[arg(short, long, default_value_t = 0)]
    pub workers: usize,
}

fn validate(args: &Args, config: &Config) -> Result<(), ConfigError> {
    args.validate()
        .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?;

    config
        .validate()
        .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?;

    Ok(())
}

pub fn init() -> Result<(Config, Args), ConfigError> {
    let args = Args::parse();
    if !args.config_file.exists() || !args.config_file.is_file() {
        return Err(ConfigError::ConfigFileNotFound(args.config_file.clone()));
    }

    let config_contents = std::fs::read_to_string(&args.config_file)
        .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?;

    let is_yaml = args
        .config_file
        .extension()
        .map(|ext| ext == "yml" || ext == "yaml")
        .unwrap_or(false);

    let config: Config = if is_yaml {
        serde_saphyr::from_str(&config_contents)
            .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?
    } else {
        serde_json::de::from_str(&config_contents)
            .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?
    };

    validate(&args, &config)?;

    Ok((config, args))
}
