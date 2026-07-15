mod sync;
mod threaded;

use super::processor::Processor;
use super::template::TemplateEngine;
use pgaf_sdk::config::Config;
use pgaf_sdk::context::Context;
use std::error::Error;
use std::sync::mpmc::{Receiver, Sender};
pub use sync::*;
pub use threaded::*;

pub trait PipelineData: Sized + Send + Sync {}

pub trait Pipeline: Send + Sync {
    type Output: PipelineData;
    fn conduct(
        &self,
        tx: &Sender<Self::Output>,
        rx: &Receiver<Context>,
        templates: &TemplateEngine,
    ) -> Result<(), Box<dyn Error + Send>>;
}

pub fn create_pipeline_from_config<O: PipelineData + 'static>(
    _config: &Config,
    workers: usize,
    processor: impl Processor<Output = O> + 'static,
) -> Result<Pipelines<O>, Box<dyn Error>> {
    let worker_count = match workers {
        0 => num_cpus::get(),
        workers => workers,
    };

    let pipeline: Pipelines<O> = match workers {
        1 => Pipelines::Sync(SyncPipeline::new(processor)),
        _ => Pipelines::Threaded(ThreadedPipeline::new(processor, worker_count)?),
    };

    Ok(pipeline)
}

pub enum Pipelines<T: PipelineData> {
    Sync(SyncPipeline<T>),
    Threaded(ThreadedPipeline<T>),
}
