pub mod unbatched;

use crate::context::generator::ContextGenerator;
use crate::context::value::ContextValueDeserializeSeed;
use crate::pipeline::{Pipeline, PipelineData, PipelineKind, create_pipeline_from_config};
use pgaf_sdk::config::Config;
use pgaf_sdk::context::Context;
use pgaf_sdk::registry::Registries;
use std::error::Error;
use std::sync::{
    Arc,
    mpmc::{Receiver, Sender, sync_channel},
};
use std::thread;
use unbatched::UnbatchedProcessor;

pub trait Processor: Send + Sync {
    type Output: PipelineData;

    fn process(
        &self,
        tx: &Sender<Self::Output>,
        rx: &Receiver<Context>,
    ) -> Result<(), Box<dyn Error + Send>>;
}

pub struct ProcessingBuilder<'a> {
    pub config: &'a Config,
    pub workers: usize,
    pub pipeline_buffer_size: usize,
    pub registries: &'a Registries,
    pub std_namespace: String,
}

impl<'a> ProcessingBuilder<'a> {
    pub fn build(self) -> Result<Processing<Context>, Box<dyn std::error::Error>> {
        let domaingen = self.config.domain.build()?;

        let ctx_gen = ContextGenerator::new(Box::new(domaingen), self.config.domain.sample_size)?;

        let deserializer = ContextValueDeserializeSeed {
            registries: self.registries,
            default_namespace: self.std_namespace,
        };
        let processor = UnbatchedProcessor::new(self.config, &deserializer);
        let pipeline = create_pipeline_from_config(self.config, self.workers, processor)?;

        Ok(Processing {
            pipeline,
            ctx_gen,
            buffer_size: self.pipeline_buffer_size,
        })
    }
}

pub struct Processing<T: PipelineData> {
    pipeline: PipelineKind<T>,
    ctx_gen: ContextGenerator,
    buffer_size: usize,
}

impl<T: PipelineData + 'static> Processing<T> {
    pub fn start(self) {
        let ctx_gen = self.ctx_gen;
        let pipeline: Arc<dyn Pipeline<Output = T>> = match self.pipeline {
            PipelineKind::Sync(pipeline) => Arc::new(pipeline),
            PipelineKind::Threaded(pipeline) => Arc::new(pipeline),
        };

        thread::scope(|s| {
            let (tx, rx_conduct) = sync_channel::<Context>(self.buffer_size);
            let (tx_conduct, rx) = sync_channel::<T>(self.buffer_size);

            let tx_conduct2 = tx_conduct.clone();
            let t_conductor = s.spawn(move || pipeline.conduct(&tx_conduct2, &rx_conduct).unwrap());
            let t_sink = s.spawn(move || {
                for _ in rx { /* noop */ }
            });

            for ctx in ctx_gen {
                tx.send(ctx).unwrap();
            }

            drop(tx);
            t_conductor.join().unwrap();

            drop(tx_conduct);
            t_sink.join().unwrap();
        })
    }
}
