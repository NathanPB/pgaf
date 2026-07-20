mod deserialize;
mod validate;

use clap::Parser;
use deserialize::deserialize_public_identifier;
use pgaf_sdk::registry;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use std::path::PathBuf;
use validate::{RE_PIPELINE_STEP_NAME, validate_unique_pipeline_names};
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
    #[validate(regex(path = *RE_PIPELINE_STEP_NAME, message = "Pipeline name must be alphanumeric and contain only underscores and dashes"))]
    pub name: String,
    #[serde(deserialize_with = "deserialize_public_identifier")]
    pub r#type: registry::PublicIdentifier,
    pub args: serde_json::Value,
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
}

fn validate(args: &Args, config: &Config) -> Result<(), ConfigError> {
    args.validate()
        .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?;

    config
        .validate()
        .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?;

    Ok(())
}

pub fn init() -> Result<(Config, Args, PathBuf), ConfigError> {
    let args = Args::parse();
    let path = PathBuf::from(&args.config_file.clone());
    if !path.exists() || !path.is_file() {
        return Err(ConfigError::ConfigFileNotFound(path.clone()));
    }

    let json_str = std::fs::read_to_string(args.config_file.clone())
        .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?;

    let config: Config = serde_json::de::from_str(&json_str)
        .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?;

    validate(&args, &config)?;

    Ok((config, args, path))
}
