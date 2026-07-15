mod deserialize;

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

pub use deserialize::ConfigDeserializeSeed;

use clap::Parser;
use pgaf_sdk::config::Config;
use serde::de::DeserializeSeed;
use validator::{Validate, ValidationError};

static ERRCODE_WORKDIR_NOT_DIR: &str = "ERRCODE_WORKDIR_NOT_DIR";
static ERRCODE_WORKDIR_NOT_EMPTY: &str = "ERRCODE_WORKDIR_NOT_EMPTY";

fn validate_workdir_is_directory(path: &Path) -> Result<(), ValidationError> {
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
                        let msg = format!(
                            "Working directory {} is not empty. Specify --clear-workdir to FORCEFULLY OVERWRITE it.",
                            path.display()
                        );
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

fn validate(args: &Args, config: &Config) -> Result<(), ConfigError> {
    args.validate()
        .map_err(|e| ConfigError::ConfigLoadError(Box::new(e)))?;

    validate_workdir_overrides(args)?;

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
