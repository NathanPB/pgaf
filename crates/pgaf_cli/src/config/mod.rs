mod deserialize;
mod validate;

use clap::Parser;
use deserialize::deserialize_public_identifier;
use pgaf_sdk::registry;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use std::path::{Path, PathBuf};
use validate::{RE_GENERAL_NAME, validate_unique_pipeline_names, validate_unique_sink_names};
use validator::{Validate, ValidationErrors};

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Workload file not found.")]
    WorkloadFileNotFound(PathBuf),
    #[error("Failed to validate workload: {0}")]
    WorkloadValidation(ValidationErrors),
    #[error("Failed to parse the workload file: {0}")]
    WorkloadJsonParse(serde_json::Error),
    #[error("Failed to parse the workload file: {0}")]
    WorkloadYamlParse(serde_saphyr::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
}

#[serde_inline_default]
#[derive(Validate, Debug, Deserialize, Clone)]
pub struct Workload {
    pub domain: Domain,

    #[validate(nested)]
    #[validate(custom(function = "validate_unique_pipeline_names"))]
    pub pipeline: Vec<PipelineStep>,

    #[validate(nested)]
    #[validate(custom(function = "validate_unique_sink_names"))]
    #[serde(default)]
    pub sink: Vec<Sink>,
}

#[derive(Validate, Debug, Deserialize, Clone)]
pub struct Domain {
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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to the JSON or YAML workload spec file.
    #[arg(short, long)]
    pub workload_file: PathBuf,

    /// Number of threads to use for parallel processing. If 0, will use all available cores.
    #[arg(short, long, default_value_t = 0)]
    pub threads: usize,

    /// Increase log verbosity (-v debug for pgaf, -vv trace for pgaf, -vvv trace for everything).
    #[arg(short, long, action = clap::ArgAction::Count, conflicts_with = "quiet")]
    pub verbose: u8,

    /// Only show warnings and errors.
    #[arg(short, long)]
    pub quiet: bool,
}

pub fn parse_args() -> Args {
    Args::parse()
}

#[tracing::instrument(level = "debug", skip_all, fields(path = %workload_file.display()))]
pub fn load_workload(workload_file: &Path) -> Result<Workload, ConfigError> {
    if !workload_file.exists() || !workload_file.is_file() {
        return Err(ConfigError::WorkloadFileNotFound(
            workload_file.to_path_buf(),
        ));
    }

    let workload_contents = std::fs::read_to_string(workload_file)?;

    let is_yaml = workload_file
        .extension()
        .map(|ext| ext == "yml" || ext == "yaml")
        .unwrap_or(false);

    let workload: Workload = if is_yaml {
        serde_saphyr::from_str(&workload_contents).map_err(ConfigError::WorkloadYamlParse)?
    } else {
        serde_json::de::from_str(&workload_contents).map_err(ConfigError::WorkloadJsonParse)?
    };

    workload
        .validate()
        .map_err(ConfigError::WorkloadValidation)?;

    Ok(workload)
}
