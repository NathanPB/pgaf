use crate::{domain, function, pipeline};

use super::Resource;

#[derive(Clone)]
pub struct DomainGeneratorDriverResource(pub domain::Driver);
impl Resource for DomainGeneratorDriverResource {}

#[derive(Clone)]
pub struct FunctionDriverResource(pub function::Driver);
impl Resource for FunctionDriverResource {}

#[derive(Clone)]
pub struct PipelineStepTypeDriverResource(pub pipeline::Driver);
impl Resource for PipelineStepTypeDriverResource {}
