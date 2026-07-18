mod validate; // TODO: Decouple validation/parsing from SDK

use crate::domain::{DomainGenerator, DomainGeneratorDriver};
use serde::Serialize;
use serde_inline_default::serde_inline_default;
use std::any::Any;
use std::error::Error;
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
    pub driver: DomainGeneratorDriver<Box<dyn DomainGenerator>, Box<dyn Any>>,
    pub sample_size: Option<usize>,
    pub args: serde_json::Value,
}

impl DomainConfig {
    pub fn build(&self) -> Result<Box<dyn DomainGenerator>, Box<dyn Error>> {
        let config = (self.driver.config_deserializer)(self.args.clone())?;
        (self.driver.create)(config)
    }
}

#[derive(Validate, Serialize, Clone, Debug)]
pub struct PipelineStep {
    #[validate(regex(path = *RE_PIPELINE_STEP_NAME, message = "Pipeline name must be alphanumeric and contain only underscores and dashes"))]
    pub name: String,

    #[serde(skip)]
    pub driver: crate::pipeline::Driver,

    #[serde(skip)]
    pub args: serde_json::Value,
}
