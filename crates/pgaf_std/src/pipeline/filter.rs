use pgaf_sdk::context::{Context, PrimitiveContextValue};
use pgaf_sdk::pipeline::{Driver, PipelineStepType, PipelineStepTypeDriver};
use std::sync::LazyLock;

/// A pipeline step that keeps or drops [`Context`]s based on a boolean predicate.
///
/// The argument must be a [`One`](pgaf_sdk::pipeline::PipelineStepTypeArgs::One) value that
/// evaluates to a [`bool`]. `true` keeps the context; `false` drops it silently. Any other
/// primitive is treated as a no-op and a warning is emitted to stderr.
pub struct Filter;

impl PipelineStepType<PrimitiveContextValue> for Filter {
    fn invoke(
        stream: Box<dyn Iterator<Item = (PrimitiveContextValue, Context)>>,
    ) -> Box<dyn Iterator<Item = Context>> {
        Box::new(stream.filter_map(|(args, ctx)| {
            match args {
                    PrimitiveContextValue::Bool(true) => Some(ctx),
                    PrimitiveContextValue::Bool(false) => None,
                    _ => {
                        eprintln!(
                            "filter: supplied parameter on unit ID {} does not evaluate to boolean, nooping", ctx.unit.id
                        );
                        Some(ctx)
                    }
                }
        }))
    }
}

pub static FILTER_DRIVER: LazyLock<Driver> = LazyLock::new(|| {
    PipelineStepTypeDriver::<Filter, PrimitiveContextValue>::default().coerce_to_dynamic()
});

#[cfg(test)]
mod tests {
    use super::*;
    use pgaf_sdk::config::RunConfig;
    use pgaf_sdk::context::{ContextValue, PrimitiveContextValue};
    use pgaf_sdk::data::GeoDeg;
    use pgaf_sdk::domain::ExecutionUnit;
    use pgaf_sdk::pipeline::PipelineStepTypeArgs;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_ctx(id: i64) -> Context {
        Context {
            unit: ExecutionUnit {
                id: id.into(),
                lon: GeoDeg::from(0.0),
                lat: GeoDeg::from(0.0),
            },
            run: RunConfig {
                name: "test".into(),
                extra: HashMap::new(),
                template: PathBuf::from("dummy"),
            },
        }
    }

    fn prim(v: PrimitiveContextValue) -> ContextValue {
        ContextValue::Prim(v)
    }

    #[test]
    fn keeps_true() {
        let ctxs = vec![make_ctx(1), make_ctx(2)];
        let args = PipelineStepTypeArgs::One(prim(PrimitiveContextValue::Bool(true)));
        let result: Vec<_> = FILTER_DRIVER
            .invoke(args, Box::new(ctxs.into_iter()))
            .collect();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn drops_false() {
        let ctxs = vec![make_ctx(1), make_ctx(2)];
        let args = PipelineStepTypeArgs::One(prim(PrimitiveContextValue::Bool(false)));
        let result: Vec<_> = FILTER_DRIVER
            .invoke(args, Box::new(ctxs.into_iter()))
            .collect();
        assert!(result.is_empty());
    }

    #[test]
    fn drops_non_bool_with_warning() {
        let ctxs = vec![make_ctx(1)];
        let args = PipelineStepTypeArgs::One(prim(PrimitiveContextValue::Int(42)));
        let result: Vec<_> = FILTER_DRIVER
            .invoke(args, Box::new(ctxs.into_iter()))
            .collect();

        assert_eq!(result.len(), 1);
    }
}
