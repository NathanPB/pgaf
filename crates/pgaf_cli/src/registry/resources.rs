use crate::functions;
use crate::registry::Resource;
use crate::sites::SiteGenerator;
use crate::sites::SiteGeneratorDriver;
use std::any::Any;

#[derive(Clone)]
pub struct SiteGeneratorDriverResource(
    pub SiteGeneratorDriver<Box<dyn SiteGenerator>, Box<dyn Any>>,
);

impl Resource for SiteGeneratorDriverResource {}

#[derive(Clone)]
pub struct FunctionDriverResource(pub functions::Driver);

impl Resource for FunctionDriverResource {}
