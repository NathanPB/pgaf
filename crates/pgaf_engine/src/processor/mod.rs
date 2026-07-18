mod serializer;

use crate::context::generator::ContextGenerator;
use crate::context::value::ContextValueDeserializeSeed;
use pgaf_sdk::config::Config;
use pgaf_sdk::{pipeline, registry::Registries};
use serde::de::DeserializeSeed;
use serializer::PipelineStepTypeArgsDeserializer;
use std::error::Error;
use std::sync::Arc;
use std::thread;

pub struct ProcessorBuilder<'a> {
    pub config: &'a Config,
    pub workers: usize,
    pub registries: &'a Registries,
    pub std_namespace: String,
}

impl<'a> ProcessorBuilder<'a> {
    pub fn build(self) -> Result<Processor, Box<dyn Error>> {
        let domaingen = self
            .config
            .domain
            .driver
            .create(self.config.domain.args.clone())?;

        let ctx_gen = ContextGenerator::new(Box::new(domaingen), self.config.domain.sample_size)?;

        let deserializer = ContextValueDeserializeSeed {
            registries: self.registries,
            default_namespace: self.std_namespace,
        };
        let pipeline = self
            .config
            .pipeline
            .iter()
            .map(|it| {
                (
                    it.driver.clone(),
                    Arc::new(
                        PipelineStepTypeArgsDeserializer(deserializer.clone())
                            .deserialize(it.args.clone())
                            .expect("Failed to deserialize pipeline step arguments.")
                            .0,
                    ),
                )
            })
            .collect();

        let workers = if self.workers == 0 {
            thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        } else {
            self.workers
        };

        Ok(Processor {
            ctx_gen,
            pipeline,
            workers,
        })
    }
}

pub struct Processor {
    ctx_gen: ContextGenerator,
    pipeline: Vec<(pipeline::Driver, Arc<pipeline::PipelineStepTypeArgs>)>,
    workers: usize,
}

impl Processor {
    pub fn start(self) {
        let pipeline = Arc::new(self.pipeline);
        let (tx, rx) = crossbeam_channel::bounded(self.workers * 2);

        let handles: Vec<_> = (0..self.workers)
            .map(|_| {
                let rx = rx.clone();
                let pipeline = Arc::clone(&pipeline);
                thread::spawn(move || {
                    let stream = Box::new(rx.into_iter()) as Box<dyn Iterator<Item = _>>;
                    let result = pipeline.iter().fold(stream, |s, (driver, args)| {
                        driver.invoke(Arc::clone(args), s)
                    });

                    for _ in result {
                        // Let it sink
                    }
                })
            })
            .collect();

        for ctx in self.ctx_gen {
            tx.send(ctx).ok();
        }

        drop(tx);

        for h in handles {
            h.join().ok();
        }
    }
}
