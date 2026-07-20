pub mod builder;
mod serializer;

use crate::context::generator::ContextGenerator;
pub use builder::{ProcessorBuilder, ProcessorBuilderError};
use pgaf_sdk::pipeline;
use std::sync::Arc;
use std::thread;

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
