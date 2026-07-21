use crate::context::Context;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::marker::PhantomData;

type SinkStream = Box<dyn Iterator<Item = Context>>;
type SinkOutput = Result<(), Box<dyn Error>>;

/// A terminal consumer of a [`Context`] stream.
///
/// A sink is driven exactly once for the whole stream and returns nothing but success or
/// failure, it is the end of a pipeline, not a transformation within it.
///
/// Its argument `A` is deserialized **once**, before the stream is driven, and is considered
/// immutable for the lifetime of the invocation: a sink's args do **not** depend on any
/// individual [`Context`], so a plain [`serde_json::Value`] is deserialized straight into `A`
/// with no per-item, context-seeded evaluation.
pub trait SinkType<A>: Send + Sync {
    fn invoke(args: A, stream: SinkStream) -> SinkOutput;
}

pub struct SinkTypeDriver<F: SinkType<A>, A> {
    _marker_f: PhantomData<F>,
    _marker_a: PhantomData<A>,
}

impl<F: SinkType<A> + 'static, A: DeserializeOwned + 'static> Default for SinkTypeDriver<F, A> {
    fn default() -> Self {
        Self {
            _marker_f: PhantomData,
            _marker_a: PhantomData,
        }
    }
}

impl<F: SinkType<A> + 'static, A: DeserializeOwned + 'static> SinkTypeDriver<F, A> {
    pub fn coerce_to_dynamic(self) -> Driver {
        Driver(invoker_impl::<F, A>)
    }
}

type InvokeFn = fn(serde_json::Value, SinkStream) -> SinkOutput;

fn invoker_impl<F: SinkType<A>, A: DeserializeOwned>(
    args: serde_json::Value,
    stream: SinkStream,
) -> SinkOutput {
    F::invoke(serde_json::from_value(args)?, stream)
}

#[derive(Clone, Debug)]
pub struct Driver(InvokeFn);

impl Driver {
    /// Deserializes `args` into the sink's argument type **once**, then hands it the full
    /// context stream. The stream is driven to completion (or to the first error) by the sink
    /// itself; nothing here is lazy.
    pub fn invoke(&self, args: serde_json::Value, stream: SinkStream) -> SinkOutput {
        self.0(args, stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GeoDeg;
    use crate::domain::ExecutionUnit;
    use serde::Deserialize;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn make_ctx(id: i64) -> Context {
        Context {
            unit: ExecutionUnit {
                id: id.into(),
                lon: GeoDeg::from(0.0),
                lat: GeoDeg::from(0.0),
            },
            data: HashMap::default(),
        }
    }

    fn counting_stream(ids: Vec<i64>) -> (Arc<AtomicUsize>, SinkStream) {
        let pulled = Arc::new(AtomicUsize::new(0));
        let counter = pulled.clone();
        let stream = ids.into_iter().map(move |id| {
            counter.fetch_add(1, Ordering::SeqCst);
            make_ctx(id)
        });
        (pulled, Box::new(stream))
    }

    #[derive(Deserialize)]
    struct CollectArgs {
        prefix: String,
    }

    struct ExpectPrefixThenDrain;
    impl SinkType<CollectArgs> for ExpectPrefixThenDrain {
        fn invoke(args: CollectArgs, stream: SinkStream) -> SinkOutput {
            assert_eq!(
                args.prefix, "id-",
                "args should be deserialized from the JSON payload"
            );
            stream.for_each(drop);
            Ok(())
        }
    }

    #[test]
    fn deserializes_args_once_and_drains_stream() {
        let driver =
            SinkTypeDriver::<ExpectPrefixThenDrain, CollectArgs>::default().coerce_to_dynamic();
        let (pulled, stream) = counting_stream(vec![1, 2, 3]);

        driver.invoke(json!({ "prefix": "id-" }), stream).unwrap();

        assert_eq!(
            pulled.load(Ordering::SeqCst),
            3,
            "sink should drain the whole stream"
        );
    }

    #[test]
    fn bad_args_error_before_stream_is_touched() {
        let driver =
            SinkTypeDriver::<ExpectPrefixThenDrain, CollectArgs>::default().coerce_to_dynamic();
        let (pulled, stream) = counting_stream(vec![1, 2, 3]);

        let err = driver.invoke(json!({ "prefix": 42 }), stream).unwrap_err();

        assert!(
            err.to_string().contains("prefix") || err.to_string().contains("string"),
            "error should describe the arg mismatch: {err}"
        );
        assert_eq!(
            pulled.load(Ordering::SeqCst),
            0,
            "stream must not be pulled when arg deserialization fails"
        );
    }

    struct FailOnSecond;
    impl SinkType<()> for FailOnSecond {
        fn invoke(_args: (), stream: SinkStream) -> SinkOutput {
            for (n, _ctx) in stream.enumerate() {
                if n == 1 {
                    return Err("boom".into());
                }
            }
            Ok(())
        }
    }

    #[test]
    fn propagates_sink_runtime_error() {
        let driver = SinkTypeDriver::<FailOnSecond, ()>::default().coerce_to_dynamic();
        let (_pulled, stream) = counting_stream(vec![1, 2, 3]);

        let err = driver.invoke(json!(null), stream).unwrap_err();

        assert_eq!(err.to_string(), "boom");
    }
}
