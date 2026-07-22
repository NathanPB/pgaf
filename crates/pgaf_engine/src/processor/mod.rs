//! ## Where the ambient tracing fields come from
//!
//! `pgaf_sdk`'s tracing contract promises `step.name`/`step.type` and
//! `sink.name`/`sink.type` to any event a [`pipeline::PipelineStepType`] or
//! [`sink::SinkType`] implementation emits, without that implementation ever
//! setting them itself. This module is where that promise is kept, and
//! nowhere else: [`PipelineEntry`]/[`SinkEntry`] retain `name`/`id` past
//! deserialization purely so this module's private `pipeline_step` and
//! `run_sink` functions can attach them to a span, which `Spanned` (for
//! steps) and `#[instrument]` (for sinks) then hold open around the driver's
//! `invoke` call. Any `warn!`/`error!`/span the driver's own code opens nests
//! inside and inherits the fields automatically — see each function's doc
//! for specifics.
//!
//! ## Tracing rules when changing this module
//!
//! The span topology is fixed: a `run` span (owned by the caller, typically
//! the CLI), with `feed` on the main thread, one `worker` span per worker
//! thread, per-worker `step` spans around each pipeline stage, and one
//! `sink` span per sink thread.
//!
//! - Spans do not follow `thread::spawn`. [`Processor::start`] captures the
//!   ambient `run` span and hands a clone into every worker and sink thread,
//!   where `#[instrument(parent = &run, ...)]` re-parents explicitly. A new
//!   thread must do the same handoff, or everything it emits detaches from
//!   the run.
//! - Never enter a span around a call that returns a lazy iterator —
//!   `Driver::invoke` only constructs the chain, so such a span would
//!   measure nanoseconds of setup and close before execution. Per-item
//!   attribution belongs to `Spanned`, which enters the step span around
//!   each `next()` call instead.
//! - There is deliberately no per-unit span and no per-unit event at
//!   `debug` or above: workers see 10⁵–10⁸ units per run, so counters
//!   aggregate locally and report once, like `feed complete` and
//!   `worker complete` do.

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

/// A configured pipeline step.
///
/// `name` and `id` survive past [`builder`] specifically so this module's
/// private `pipeline_step` function can attach them as the `step` span's
/// `step.name`/`step.type` fields — they have no other runtime use once the
/// driver is resolved.
pub struct PipelineEntry {
    pub name: String,
    pub id: PublicIdentifier,
    pub driver: pipeline::Driver,
    pub args: Arc<pipeline::PipelineStepTypeArgs>,
}

/// A configured sink.
///
/// `name` and `id` survive past [`builder`] specifically so this module's
/// private `run_sink` function can attach them as the `sink` span's
/// `sink.name`/`sink.type` fields — they have no other runtime use once the
/// driver is resolved.
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
///
/// This is the mechanism, not just the timing, that makes `step.name`/`step.type`
/// ambient: `span` is entered for the whole duration of each `next()` call, so
/// anything the wrapped stage's iterator does while producing an item —
/// including events the [`pipeline::PipelineStepType`] implementation emits
/// deep inside its own logic — happens with this span current, and inherits
/// its fields without the implementation referencing them.
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

/// Attaches `worker.id` once, here, via `#[instrument]`. Every `step` span
/// opened by [`pipeline_step`] during this worker's fold nests under this
/// span, so `worker.id` reaches those (and anything they contain) through
/// ordinary span-parent inheritance — nothing downstream needs to know its
/// own worker index.
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

/// Builds the `step` span carrying `step.name`/`step.type` and wraps the
/// driver's output in it via [`Spanned`]. This is the *only* place those two
/// fields get attached — [`pipeline::PipelineStepType`] implementations never
/// set them (e.g. `pgaf_sdk`'s own per-item arg-deserialization `warn!` picks
/// them up this way, with no `step.name` field of its own).
fn pipeline_step(input: ContextStream, entry: &PipelineEntry) -> ContextStream {
    // NOTE: Do not remove the Spanned wrapping.
    // Entering the span at the driver call site would measure construction, not execution.

    let span = tracing::debug_span!("step", step.name = %entry.name, step.r#type = %entry.id);
    Box::new(Spanned {
        inner: entry.driver.invoke(Arc::clone(&entry.args), input),
        span,
    })
}

/// Attaches `sink.name`/`sink.type` via `#[instrument]` — the sink-side
/// counterpart to [`pipeline_step`]. The driver's `invoke` call, and the
/// `error!` below, run with this span current, so [`sink::SinkType`]
/// implementations (and this function's own error report) get sink identity
/// without adding it themselves.
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
