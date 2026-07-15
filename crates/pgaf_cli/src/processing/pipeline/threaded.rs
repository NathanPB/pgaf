use super::super::processor::Processor;
use super::super::template::TemplateEngine;
use super::{Pipeline, PipelineData};
use pgaf_sdk::context::Context;
use std::error::Error;
use std::sync::mpmc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::thread::ScopedJoinHandle;

#[derive(Debug, thiserror::Error)]
pub struct NotEnoughWorkersError;

impl std::fmt::Display for NotEnoughWorkersError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Not enough workers to use ThreadedPipelineBuilder. At least 2 workers are required."
        )
    }
}

impl<O: PipelineData> ThreadedPipeline<O> {
    pub fn new(
        processor: impl Processor<Output = O> + 'static,
        workers: usize,
    ) -> Result<ThreadedPipeline<O>, NotEnoughWorkersError> {
        if workers <= 1 {
            return Err(NotEnoughWorkersError);
        }

        Ok(ThreadedPipeline {
            workers,
            processor: Arc::new(processor),
        })
    }
}

pub struct ThreadedPipeline<O: Sized + Send + Sync> {
    workers: usize,
    processor: Arc<dyn Processor<Output = O>>,
}

impl<O: PipelineData + 'static> Pipeline for ThreadedPipeline<O> {
    type Output = O;

    fn conduct(
        &self,
        tx: &Sender<Self::Output>,
        rx: &Receiver<Context>,
        templates: &TemplateEngine,
    ) -> Result<(), Box<dyn Error + Send>> {
        thread::scope(|s| {
            let thread_pool: Vec<ScopedJoinHandle<()>> = (0..self.workers)
                .enumerate()
                .map(move |(i, _)| {
                    s.spawn(move || {
                        // TODO: better error recovery
                        if let Err(err) = self.processor.process(tx, rx, templates) {
                            panic!("ThreadedPipeline: Worker Thread {} crashed: {}", i, err);
                        }
                    })
                })
                .collect();

            thread_pool.into_iter().enumerate().for_each(|(i, t)| {
                t.join().unwrap_or_else(|_| {
                    panic!(
                        "ThreadedPipeline: Worker Thread {} crashed when joining.",
                        i
                    )
                });
            });

            Ok(())
        })
    }
}
