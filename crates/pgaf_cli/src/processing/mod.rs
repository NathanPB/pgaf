use crate::config::Args;
use crate::processing::template::TemplateEngine;
use context::ContextGenerator;
use pgaf_sdk::config::Config;
use pgaf_sdk::context::Context;
use pipeline::{create_pipeline_from_config, Pipeline, Pipelines};
use processor::unbatched::UnbatchedProcessor;
use std::path::PathBuf;
use std::sync::mpmc::sync_channel;
use std::sync::Arc;
use std::thread;

pub mod context;
mod pipeline;
mod processor;
mod template;

pub trait PipelineData: Sized + Send + Sync {}

pub struct ProcessingBuilder<'a> {
    pub config: &'a Config,
    pub args: &'a Args,
    pub workdir: PathBuf,
}

impl<'a> ProcessingBuilder<'a> {
    pub fn build(self) -> Result<Processing<Context>, Box<dyn std::error::Error>> {
        let sitegen = self.config.sites.build()?;

        let ctx_gen = ContextGenerator::new(
            Box::new(sitegen),
            self.config.runs.clone(),
            self.config.sites.sample_size,
        )?;

        let processor = UnbatchedProcessor {
            workdir: self.workdir,
        };

        let pipeline = create_pipeline_from_config(self.config, self.args.workers, processor)?;

        let mut templates = TemplateEngine::default();
        for run in &self.config.runs {
            templates.register(run.name.as_str(), &run.template)?;
        }

        Ok(Processing {
            pipeline,
            ctx_gen,
            templates,
            buffer_size: self.args.pipeline_buffer_size,
        })
    }
}

pub struct Processing<T: PipelineData> {
    pipeline: Pipelines<T>,
    ctx_gen: ContextGenerator,
    templates: TemplateEngine,
    buffer_size: usize,
}

impl<T: PipelineData + 'static> Processing<T> {
    pub fn start(self) {
        let ctx_gen = self.ctx_gen;
        let pipeline: Arc<dyn Pipeline<Output = T>> = match self.pipeline {
            Pipelines::Sync(pipeline) => Arc::new(pipeline),
            Pipelines::Threaded(pipeline) => Arc::new(pipeline),
        };

        thread::scope(|s| {
            let (tx, rx_conduct) = sync_channel::<Context>(self.buffer_size);
            let (tx_conduct, rx) = sync_channel::<T>(self.buffer_size);

            let tx_conduct2 = tx_conduct.clone();
            let t_conductor = s.spawn(move || {
                pipeline
                    .conduct(&tx_conduct2, &rx_conduct, &self.templates)
                    .unwrap()
            });
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
