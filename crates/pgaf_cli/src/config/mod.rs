mod deserialize;

use clap::Parser;
pub use deserialize::ConfigDeserializeSeed;
use pgaf_sdk::config::Config;
use serde::de::DeserializeSeed;
use std::path::PathBuf;
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

pub fn init(seed: ConfigDeserializeSeed) -> Result<(Config, Args, PathBuf), ConfigError> {
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
