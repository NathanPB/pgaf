use super::super::context::Context;
use super::super::processor::Processor;
use super::super::template::TemplateEngine;
use super::{Pipeline, PipelineData};
use std::error::Error;
use std::sync::mpmc::{Receiver, Sender};
use std::sync::Arc;

pub struct SyncPipeline<O: PipelineData> {
    processor: Arc<dyn Processor<Output = O>>,
}

impl<O: PipelineData> SyncPipeline<O> {
    pub fn new(processor: impl Processor<Output = O> + 'static) -> Self {
        Self {
            processor: Arc::new(processor),
        }
    }
}

impl<O: PipelineData> Pipeline for SyncPipeline<O> {
    type Output = O;

    fn conduct(
        &self,
        tx: &Sender<Self::Output>,
        rx: &Receiver<Context>,
        templates: &TemplateEngine,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.processor.process(tx, rx, templates)
    }
}
