use std::io::IsTerminal;

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Filter policy — precedence: flags > `RUST_LOG` > default; `RUST_LOG` only
/// applies when no verbose/quiet options are configured.
fn env_filter(verbose: u8, quiet: bool) -> EnvFilter {
    let dirs = match (quiet, verbose) {
        (true, _) => "error,pgaf=warn",
        (_, 0) => "warn,pgaf=info",
        (_, 1) => "warn,pgaf=debug",
        (_, 2) => "info,pgaf=trace",
        (_, _) => "trace",
    };

    match std::env::var("RUST_LOG") {
        Ok(env) if verbose == 0 && !quiet => EnvFilter::builder().parse(&env).unwrap_or_else(|e| {
            eprintln!("warning: invalid RUST_LOG ({e}); using defaults");
            EnvFilter::new(dirs)
        }),
        _ => EnvFilter::new(dirs),
    }
}

/// Initializes the global tracing subscriber pointing to stderr.
pub fn init(verbose: u8, quiet: bool) {
    let filter = env_filter(verbose, quiet);
    let ansi = std::io::stderr().is_terminal();

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(ansi),
        )
        .init();
}
