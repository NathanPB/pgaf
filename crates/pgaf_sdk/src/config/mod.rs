mod validate; // TODO: Decouple validation/parsing from SDK

use crate::{domain, pipeline};
use serde::Serialize;
use serde_inline_default::serde_inline_default;
use validate::{RE_PIPELINE_STEP_NAME, validate_unique_pipeline_names};
use validator::Validate;

#[serde_inline_default]
#[derive(Validate, Clone)]
pub struct Config {
    pub domain: DomainConfig,

    #[validate(nested)]
    #[validate(custom(function = "validate_unique_pipeline_names"))]
    pub pipeline: Vec<PipelineStep>,
}

#[derive(Validate, Clone)]
pub struct DomainConfig {
    pub driver: domain::Driver,
    pub sample_size: Option<usize>,
    pub args: serde_json::Value,
}

#[derive(Validate, Serialize, Clone, Debug)]
pub struct PipelineStep {
    #[validate(regex(path = *RE_PIPELINE_STEP_NAME, message = "Pipeline name must be alphanumeric and contain only underscores and dashes"))]
    pub name: String,

    #[serde(skip)]
    pub driver: pipeline::Driver,

    #[serde(skip)]
    pub args: serde_json::Value,
}
