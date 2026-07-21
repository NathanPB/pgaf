use pgaf_sdk::context::{Context, PrimitiveContextValue};
use pgaf_sdk::function::{Driver, Function, FunctionDriver, FunctionRuntimeError};
use serde::Deserialize;
use std::sync::{LazyLock, Once};

/// **For load testing only, not intended to be used by end users.**
///
/// Computes the nth Fibonacci number using naive recursion (O(2^n) time).
/// The deliberate inefficiency makes it useful for saturating CPU cores
/// during pipeline load tests. Tune `n` to control per-unit work:
///
/// | n  | approx calls | approx wall time (single core) |
/// |----|--------------|-------------------------------|
/// | 30 | ~2M          | ~5 ms                         |
/// | 35 | ~29M         | ~100 ms                       |
/// | 40 | ~330M        | ~1 s                          |
/// | 45 | ~3.7B        | ~10 s                         |
///
/// `n` is capped at `u8::MAX` (255) by the type, but anything above ~50
/// will effectively hang a worker thread indefinitely.
///
/// If `debug_assertions` are disabled, a warning will be emitted.
pub struct DbgFib;

#[derive(Deserialize)]
pub struct DbgFibArgs {
    n: u8,
}

fn fib(n: u8) -> i64 {
    match n {
        0 => 0,
        1 => 1,
        n => fib(n - 1) + fib(n - 2),
    }
}

impl Function<DbgFibArgs> for DbgFib {
    fn invoke(
        args: DbgFibArgs,
        _ctx: &Context,
    ) -> Result<PrimitiveContextValue, FunctionRuntimeError> {
        if !cfg!(debug_assertions) {
            static WARNING: Once = Once::new();
            WARNING.call_once(|| {
                tracing::warn!("dbgfib in release build");
            });
        }

        Ok(PrimitiveContextValue::Int(fib(args.n)))
    }
}

pub static DBG_FIB_DRIVER: LazyLock<Driver> =
    LazyLock::new(|| FunctionDriver::<DbgFib, DbgFibArgs>::default().coerce_to_dynamic());

#[cfg(test)]
mod tests {
    use super::*;
    use pgaf_sdk::{context::ContextValue, data::GeoDeg, domain::ExecutionUnit};
    use std::collections::HashMap;

    fn ctx() -> Context {
        Context {
            unit: ExecutionUnit {
                id: 1.into(),
                lon: GeoDeg::from(0.0),
                lat: GeoDeg::from(0.0),
            },
            data: HashMap::default(),
        }
    }

    fn args(n: u8) -> HashMap<String, ContextValue> {
        [(
            "n".into(),
            ContextValue::Prim(PrimitiveContextValue::Int(n as i64)),
        )]
        .into()
    }

    #[test]
    fn base_cases() {
        assert_eq!(
            DBG_FIB_DRIVER.invoke(&args(0), &ctx()).unwrap(),
            PrimitiveContextValue::Int(0)
        );
        assert_eq!(
            DBG_FIB_DRIVER.invoke(&args(1), &ctx()).unwrap(),
            PrimitiveContextValue::Int(1)
        );
    }

    #[test]
    fn known_values() {
        assert_eq!(
            DBG_FIB_DRIVER.invoke(&args(10), &ctx()).unwrap(),
            PrimitiveContextValue::Int(55)
        );
        assert_eq!(
            DBG_FIB_DRIVER.invoke(&args(20), &ctx()).unwrap(),
            PrimitiveContextValue::Int(6765)
        );
    }
}
