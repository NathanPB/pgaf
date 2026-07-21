pub mod builder;
mod serializer;

use crate::context::generator::ContextGenerator;
pub use builder::{ProcessorBuilder, ProcessorBuilderError};
use pgaf_sdk::context::Context;
use pgaf_sdk::registry::PublicIdentifier;
use pgaf_sdk::{pipeline, sink};
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use tracing::Span;

pub struct PipelineEntry {
    pub name: String,
    pub id: PublicIdentifier,
    pub driver: pipeline::Driver,
    pub args: Arc<pipeline::PipelineStepTypeArgs>,
}

pub struct SinkEntry {
    pub name: String,
    pub id: PublicIdentifier,
    pub driver: sink::Driver,
    pub args: serde_json::Value,
}

pub struct Processor {
    ctx_gen: ContextGenerator,
    pipeline: Vec<PipelineEntry>,
    sinks: Vec<SinkEntry>,
    workers: usize,
}

type ContextStream = Box<dyn Iterator<Item = Context>>;

/// Enters `span` around each pull, attributing per-item work inside a lazy pipeline stage
/// to that stage's span.
struct Spanned<I> {
    inner: I,
    span: Span,
}

impl<I: Iterator> Iterator for Spanned<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.span.in_scope(|| self.inner.next())
    }
}

impl Processor {
    pub fn start(self) {
        let Processor {
            ctx_gen,
            pipeline,
            sinks,
            workers,
        } = self;

        let run = Span::current();
        let started = Instant::now();

        tracing::debug!(
            workers,
            steps = pipeline.len(),
            sinks = sinks.len(),
            "pipeline started"
        );

        let pipeline = Arc::new(pipeline);
        let (tx, rx) = crossbeam_channel::bounded(workers * 2);

        let mut sink_txs = Vec::with_capacity(sinks.len());
        let sink_handles: Vec<_> = sinks
            .into_iter()
            .map(|entry| {
                let (s_tx, s_rx) = crossbeam_channel::bounded::<Context>(workers * 2);
                sink_txs.push(s_tx);
                let run = run.clone();
                thread::spawn(move || run_sink(run, entry, Box::new(s_rx.into_iter())))
            })
            .collect();
        let sink_txs = Arc::new(sink_txs);

        let handles: Vec<_> = (0..workers)
            .map(|worker_id| {
                let rx = rx.clone();
                let pipeline = Arc::clone(&pipeline);
                let sink_txs = Arc::clone(&sink_txs);
                let run = run.clone();
                thread::spawn(move || {
                    run_worker(
                        run,
                        worker_id,
                        Box::new(rx.into_iter()),
                        &pipeline,
                        &sink_txs,
                    )
                })
            })
            .collect();

        let units_sent = run_feed(ctx_gen, tx);

        for h in handles {
            h.join().ok();
        }

        drop(sink_txs);

        for h in sink_handles {
            h.join().ok();
        }

        tracing::info!(
            units.total = units_sent,
            elapsed_ms = started.elapsed().as_millis() as u64,
            "run complete",
        );
    }
}

#[tracing::instrument(name = "feed", level = "debug", skip_all)]
fn run_feed(ctx_gen: ContextGenerator, tx: crossbeam_channel::Sender<Context>) -> u64 {
    let mut units_sent = 0u64;
    for ctx in ctx_gen {
        units_sent += 1;
        tx.send(ctx).ok();
    }
    tracing::debug!(units.sent = units_sent, "feed complete");
    units_sent
}

#[tracing::instrument(
    name = "worker",
    level = "debug",
    parent = &run,
    skip_all,
    fields(worker.id = worker_id),
)]
fn run_worker(
    run: Span,
    worker_id: usize,
    stream: ContextStream,
    pipeline: &[PipelineEntry],
    sink_txs: &[crossbeam_channel::Sender<Context>],
) {
    let result = pipeline.iter().fold(stream, pipeline_step);

    let mut units_out = 0u64;
    for ctx in result {
        units_out += 1;
        for s_tx in sink_txs {
            s_tx.send(ctx.clone()).ok();
        }
    }

    tracing::debug!(units.out = units_out, "worker complete");
}

fn pipeline_step(input: ContextStream, entry: &PipelineEntry) -> ContextStream {
    // NOTE: Do not remove the Spanned wrapping.
    // Entering the span at the driver call site would measure construction, not execution.

    let span = tracing::debug_span!("step", step.name = %entry.name, step.r#type = %entry.id);
    Box::new(Spanned {
        inner: entry.driver.invoke(Arc::clone(&entry.args), input),
        span,
    })
}

#[tracing::instrument(
    name = "sink",
    level = "debug",
    parent = &run,
    skip_all,
    fields(sink.name = %entry.name, sink.r#type = %entry.id),
)]
fn run_sink(run: Span, entry: SinkEntry, stream: ContextStream) {
    if let Err(e) = entry.driver.invoke(entry.args, stream) {
        tracing::error!(error = %e, "sink failed");
    }
}
