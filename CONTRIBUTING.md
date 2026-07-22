# Contributing to pgaf

Thanks for your interest in hacking on pgaf. The project is a Cargo workspace
with four crates:

- `pgaf_cli` — the binary. Owns argument parsing, workload loading, and the
  tracing subscriber.
- `pgaf_engine` — the processor: fan-out/fan-in over `std::thread` and
  crossbeam channels.
- `pgaf_sdk` — the trait surface steps, sinks, functions, and domains are
  built against. This is the future plugin-author contract, so its public
  docs are load-bearing.
- `pgaf_std` — the standard library of steps, sinks, and domains.

`rust-toolchain.toml` pins the toolchain; a Nix flake is available
(`nix develop`) if you want the full environment including GDAL. The usual
`cargo build` / `cargo test` / `cargo clippy` apply.

# On AI Usage

*LLM-generated code is **(partially)** allowed.*

However, "allowed" doesn't mean "prompt anything and submit PR". I **will not** allow sloppery, and I **will not** even try to review billion-line PRs or something that screams "low effort". If you are using AI to contribute to this repository, make sure to keep the code-quality decent, small and well-localized PRs and commits, and be sure to understand your changes and the implications they bring before submitting something.

> AI is a tool, just like other tools we use.  And it's clearly a useful one.
>
> Yes, it can also be a somewhat painful tool \[...\]
>
> But the solution is not to put your head in the sand and sing "La La La, I can't hear you" at the top of your voice like some people seem to do.
>
> -- [Linus Torvalds](https://lore.kernel.org/linux-media/CAHk-=wi4zC+Ze8e+p3tMv8TtG_80KzsZ1syL9anBtmEh5Z40vg@mail.gmail.com/)

# Logging and Instrumentation

pgaf uses [`tracing`](https://docs.rs/tracing). How it's used is deliberate
and fairly strict, because diagnostics are part of the CLI's contract with
users and part of the SDK's contract with future plugin authors.

## The three output planes

For this repository, nothing here is negotiable, and most other rules fall out of it:

1. **stdout is reserved.** No log line, ever.
2. **stderr is where all tracing output lands**, via the subscriber that `pgaf_cli` installs.
3. **`std:display` also writes to stderr, directly** — it's a user-configured
   pipeline step whose *product* is printing contexts, not a diagnostic. It
   stays on `eprintln!` on purpose; don't "fix" it by routing it through
   `tracing`, or `-q` would silence output the user explicitly asked for.

## Who depends on what

Library crates (`pgaf_sdk`, `pgaf_engine`, `pgaf_std`) depend on the
`tracing` facade only — from `[workspace.dependencies]`, never with a
subscriber in sight. Only `pgaf_cli` depends on `tracing-subscriber`, and
only `pgaf_cli/src/trace.rs` initializes it. The libraries have to work
under any subscriber a host process installs (think wasm or Python hosts
later), so a subscriber dependency in a library crate is a bug even if it
compiles fine.

Verbosity is driven by `-v`/`-q`, with `RUST_LOG` honored when neither flag
is given. The defaults live in `trace.rs::env_filter`; `pgaf` is a string
prefix of every workspace crate name, so a single `pgaf=debug` directive
covers all of them.

## Levels

- `trace` — per-unit success-path detail. This is the only level allowed to
  fire once per execution unit on the happy path.
- `debug` — lifecycle: steps configured, workers started, feed complete.
- `info` — the handful of lines a user sees by default. Currently that's
  roughly "initialize pgaf" and the end-of-run summary. Adding an `info`
  event means adding a line to every default run; think twice.
- `warn` — per-unit recoverable failures (a cmd that failed for one unit, a
  template that didn't render). The run continues.
- `error` — something aborted a stream or the run. These pair with an `Err`
  return or a nonzero exit; an `error!` for a condition the code then
  ignores is lying to the user.

## Writing events

Messages are stable identifiers, not sentences. Must be lowercase, no punctuation,
at about four words, and all variance goes in fields:

```rust
tracing::warn!(unit.id = %ctx.unit.id, error = %e, "cmd failed");     // yes
tracing::warn!("Command failed for {}: {}", ctx.unit.id, e);          // no
```

Fields are dot-namespaced and lowercase: `unit.id`, `step.name`,
`units.sent`, `elapsed_ms`. Use `%` (Display) for ids, errors, and paths;
raw values for numbers. The registry of established field names is in the
`pgaf_sdk` crate-level rustdoc — extend it there when you introduce a new
one rather than inventing a near-duplicate. Never log a `Context.data` map
wholesale, or anything else of user-controlled size; log its length if the
size is the interesting part.

## Spans, identity, and the hot path

The engine maintains a fixed span topology; its shape, and the rules to
follow when changing it, live in the `pgaf_engine::processor` rustdoc. The
consequence that matters day to day: **step and sink identity is
ambient.** Any event emitted inside a step or sink implementation inherits
those fields from the enclosing span, so implementations must not re-attach
them.

`unit.id` is the opposite case: there is intentionally no per-unit span, so
attach it yourself from the `ctx` in scope. The reason there isn't one is
the hot path — workers process 10⁵–10⁸ units per run, and the per-unit
tracing budget is the span enter/exit in `Spanned` plus disabled-callsite
checks, a few tens of nanoseconds. To keep it that way:

- no per-unit events at `debug` or above, and no per-unit spans at all;
- aggregate counters locally and emit one summary event per run (or per
  worker at `debug`), like `feed complete` and `worker complete` do today;
- if computing a field value is expensive *before* the macro call, guard it
  with `tracing::enabled!(Level::TRACE)`. Field expressions inside a
  disabled macro are already never evaluated, so this is only for real
  precomputation.

One more rule reaches beyond the engine: never put `#[instrument]` (or an
entered span) on a function that returns a lazy iterator, such as the
`invoke` implementations — it would measure construction, not execution.
Where `#[instrument]` is fine (setup-phase functions), always use
`skip_all` and name the fields you want explicitly. Auto-capturing `Debug`
args is how a workload's user data ends up in a log line.

## Testing

Assert on return values, not log output — logs are diagnostics, not API.
The exception is code whose observable behavior *is* the log line (e.g.
`std:filter` warning on a non-boolean arg); there, use a scoped subscriber
(`tracing::subscriber::with_default`) so parallel tests don't bleed into
each other.
