pub mod builder;
mod serializer;

use crate::context::generator::ContextGenerator;
pub use builder::{ProcessorBuilder, ProcessorBuilderError};
use pgaf_sdk::context::Context;
use pgaf_sdk::{pipeline, sink};
use std::sync::Arc;
use std::thread;

pub struct Processor {
    ctx_gen: ContextGenerator,
    pipeline: Vec<(pipeline::Driver, Arc<pipeline::PipelineStepTypeArgs>)>,
    sinks: Vec<(sink::Driver, serde_json::Value)>,
    workers: usize,
}

impl Processor {
    pub fn start(self) {
        let pipeline = Arc::new(self.pipeline);
        let (tx, rx) = crossbeam_channel::bounded(self.workers * 2);

        let mut sink_txs = Vec::with_capacity(self.sinks.len());
        let sink_handles: Vec<_> = self
            .sinks
            .into_iter()
            .map(|(driver, args)| {
                let (s_tx, s_rx) = crossbeam_channel::bounded::<Context>(self.workers * 2);
                sink_txs.push(s_tx);
                thread::spawn(move || {
                    let stream = Box::new(s_rx.into_iter()) as Box<dyn Iterator<Item = _>>;
                    if let Err(e) = driver.invoke(args, stream) {
                        eprintln!("Sink error: {e}");
                    }
                })
            })
            .collect();
        let sink_txs = Arc::new(sink_txs);

        let handles: Vec<_> = (0..self.workers)
            .map(|_| {
                let rx = rx.clone();
                let pipeline = Arc::clone(&pipeline);
                let sink_txs = Arc::clone(&sink_txs);
                thread::spawn(move || {
                    let stream = Box::new(rx.into_iter()) as Box<dyn Iterator<Item = _>>;
                    let result = pipeline.iter().fold(stream, |s, (driver, args)| {
                        driver.invoke(Arc::clone(args), s)
                    });

                    for ctx in result {
                        for s_tx in sink_txs.iter() {
                            s_tx.send(ctx.clone()).ok();
                        }
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

        drop(sink_txs);

        for h in sink_handles {
            h.join().ok();
        }
    }
}
