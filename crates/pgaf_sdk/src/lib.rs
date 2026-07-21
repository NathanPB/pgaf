//! ## Tracing contract for plugin authors
//!
//! [`pipeline::PipelineStepType`], [`sink::SinkType`], [`function::Function`],
//! and [`domain::DomainGeneratorCreate`] implementations become the
//! plugin-ecosystem's diagnostic surface, so they must follow the workspace's
//! tracing conventions:
//!
//! 1. emit only through `tracing` macros (`trace!`, `debug!`, `info!`, `warn!`,
//!    `error!`) — never `println!`/`eprintln!`. stdout is the data plane;
//!    stderr belongs to whatever subscriber the host process installs.
//! 2. per-unit success paths are `trace!`; per-unit recoverable failures are
//!    `warn!`; a failure that aborts the whole stream is `error!` and must
//!    also surface as `Err`.
//! 3. step/sink identity (`step.name`, `step.type`, `sink.name`, `sink.type`)
//!    is provided by the engine's enclosing span — do not re-add it as a
//!    field.
//! 4. messages are lowercase, punctuation-free, verb-phrase constants of at
//!    most four words; all variance lives in fields, never interpolated into
//!    the message.
//! 5. guard expensive field computation behind
//!    `tracing::enabled!(Level::TRACE)` — field expressions inside a disabled
//!    macro call are already free.
//! 6. never initialize a subscriber.
//!
//! ### Fields you attach yourself
//!
//! There is deliberately no ambient per-unit span anywhere in the pipeline —
//! at the 10⁵–10⁸-unit range this tool targets, one would cost real
//! allocation on every item even when disabled. So `unit.id` does not arrive
//! for free the way step/sink identity does: read it off the `ctx: &Context`
//! already in scope and attach it to your own event:
//! `warn!(unit.id = %ctx.unit.id, error = %e, "cmd failed")`.
//!
//! | field | type | meaning |
//! |---|---|---|
//! | `unit.id` | display | [`domain::ExecutionUnit`] id |
//! | `unit.lon`, `unit.lat` | f64 | unit coordinates (trace-level detail only) |
//! | `error` | display (`%e`) | the error, `Display`-formatted |
//!
//! Beyond these, any other field is yours: dot-namespaced, lowercase, `%`
//! (Display) for ids/errors/paths, raw values for numbers. Never log a raw
//! [`context::Context::data`] map wholesale.
//!
//! ### Ambient and internal fields — never set these yourself
//!
//! `step.name`, `step.type`, `sink.name`, `sink.type` arrive already attached
//! via the engine's enclosing span (rule 3) — any event or span you open
//! inside a step or sink invocation inherits them automatically. `value.type`
//! ([`context::ContextValue::to_prim`]) and `namespace`/`id` ([`registry`])
//! belong to this crate's own boundary code; plugin authors never emit them
//! and don't need to know they exist.

pub mod context;
pub mod data;
pub mod domain;
pub mod function;
pub mod pipeline;
pub mod registry;
pub mod sink;
