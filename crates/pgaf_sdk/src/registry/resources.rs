use std::any::Any;

use crate::{domain, function};

use super::Resource;

#[derive(Clone)]
pub struct DomainGeneratorDriverResource(
    pub domain::DomainGeneratorDriver<Box<dyn domain::DomainGenerator>, Box<dyn Any>>,
);

impl Resource for DomainGeneratorDriverResource {}

#[derive(Clone)]
pub struct FunctionDriverResource(pub function::Driver);

impl Resource for FunctionDriverResource {}
