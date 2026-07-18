mod sync;
mod threaded;

use super::processor::Processor;
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
    ) -> Result<(), Box<dyn Error + Send>>;
}

pub fn create_pipeline_from_config<O: PipelineData + 'static>(
    _config: &Config,
    workers: usize,
    processor: impl Processor<Output = O> + 'static,
) -> Result<PipelineKind<O>, Box<dyn Error>> {
    let worker_count = match workers {
        0 => num_cpus::get(),
        workers => workers,
    };

    let pipeline: PipelineKind<O> = match workers {
        1 => PipelineKind::Sync(SyncPipeline::new(processor)),
        _ => PipelineKind::Threaded(ThreadedPipeline::new(processor, worker_count)?),
    };

    Ok(pipeline)
}

pub enum PipelineKind<T: PipelineData> {
    Sync(SyncPipeline<T>),
    Threaded(ThreadedPipeline<T>),
}
