use std::any::Any;

use crate::{function, site};

use super::Resource;

#[derive(Clone)]
pub struct SiteGeneratorDriverResource(
    pub site::SiteGeneratorDriver<Box<dyn site::SiteGenerator>, Box<dyn Any>>,
);

impl Resource for SiteGeneratorDriverResource {}

#[derive(Clone)]
pub struct FunctionDriverResource(pub function::Driver);

impl Resource for FunctionDriverResource {}
