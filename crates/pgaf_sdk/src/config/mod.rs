mod validate; // TODO: Decouple validation/parsing from SDK

use crate::{
    context::ContextValue,
    site::{SiteGenerator, SiteGeneratorDriver},
};
use serde::Serialize;
use serde_inline_default::serde_inline_default;
use std::error::Error;
use std::{any::Any, collections::HashMap, path::PathBuf};
use validate::{RE_VALID_RUN_NAME, validate_template_file_exists, validate_unique_run_names};
use validator::Validate;

#[serde_inline_default]
#[derive(Validate, Clone)]
pub struct Config {
    pub sites: SiteSourceConfig,

    #[validate(length(min = 1, message = "At least one run is required"))]
    #[validate(nested)]
    #[validate(custom(function = "validate_unique_run_names"))]
    pub runs: Vec<RunConfig>,
}

#[derive(Validate, Clone)]
pub struct SiteSourceConfig {
    pub driver: SiteGeneratorDriver<Box<dyn SiteGenerator>, Box<dyn Any>>,
    pub sample_size: Option<usize>,
    pub args: serde_json::Value,
}

impl SiteSourceConfig {
    pub fn build(&self) -> Result<Box<dyn SiteGenerator>, Box<dyn Error>> {
        let config = (self.driver.config_deserializer)(self.args.clone())?;
        (self.driver.create)(config)
    }
}

#[derive(Validate, Serialize, Clone, Debug)]
pub struct RunConfig {
    #[validate(regex(path = *RE_VALID_RUN_NAME, message = "Run name must be alphanumeric and contain only underscores and dashes"))]
    pub name: String,

    #[validate(custom(function = "validate_template_file_exists"))]
    pub template: PathBuf,

    #[serde(skip)]
    pub extra: HashMap<String, ContextValue>,
}
