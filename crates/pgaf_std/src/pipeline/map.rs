use pgaf_sdk::context::{Context, ContextValue, PrimitiveContextValue};
use pgaf_sdk::pipeline::{Driver, PipelineStepType, PipelineStepTypeDriver};
use std::collections::HashMap;
use std::sync::LazyLock;

/// A pipeline step that upserts computed values into the [`Context`]'s run extras.
///
/// The argument must be a [`Map`](pgaf_sdk::pipeline::PipelineStepTypeArgs::Map) of
/// `String → ContextValue`. Each value is evaluated via `to_prim` against the current
/// context, then inserted (or overwritten) into `ctx.run.extra`. All contexts are
/// forwarded; this step never drops items.
pub struct Map;

impl PipelineStepType<HashMap<String, PrimitiveContextValue>> for Map {
    fn invoke(
        stream: Box<dyn Iterator<Item = (HashMap<String, PrimitiveContextValue>, Context)>>,
    ) -> Box<dyn Iterator<Item = Context>> {
        Box::new(stream.map(|(updates, mut ctx)| {
            for (key, value) in updates {
                ctx.data.insert(key, ContextValue::Prim(value));
            }
            ctx
        }))
    }
}

pub static MAP_DRIVER: LazyLock<Driver> = LazyLock::new(|| {
    PipelineStepTypeDriver::<Map, HashMap<String, PrimitiveContextValue>>::default()
        .coerce_to_dynamic()
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::make_ctx;
    use pgaf_sdk::pipeline::PipelineStepTypeArgs;
    use std::sync::Arc;

    #[test]
    fn upserts_values() {
        let ctx = make_ctx(1);
        let args = PipelineStepTypeArgs::Map(
            [
                (
                    "score".into(),
                    ContextValue::Prim(PrimitiveContextValue::Int(99)),
                ),
                (
                    "label".into(),
                    ContextValue::Prim(PrimitiveContextValue::String("ok".into())),
                ),
            ]
            .into(),
        );
        let mut result: Vec<_> = MAP_DRIVER
            .invoke(Arc::new(args), Box::new(vec![ctx].into_iter()))
            .collect();
        assert_eq!(result.len(), 1);
        let ctx = result.remove(0);
        assert_eq!(
            ctx.data.get("score"),
            Some(&ContextValue::Prim(PrimitiveContextValue::Int(99)))
        );
        assert_eq!(
            ctx.data.get("label"),
            Some(&ContextValue::Prim(PrimitiveContextValue::String(
                "ok".into()
            )))
        );
    }

    #[test]
    fn overwrites_existing_key() {
        let mut ctx = make_ctx(1);
        ctx.data.insert(
            "score".into(),
            ContextValue::Prim(PrimitiveContextValue::Int(0)),
        );
        let args = PipelineStepTypeArgs::Map(
            [(
                "score".into(),
                ContextValue::Prim(PrimitiveContextValue::Int(100)),
            )]
            .into(),
        );
        let mut result: Vec<_> = MAP_DRIVER
            .invoke(Arc::new(args), Box::new(vec![ctx].into_iter()))
            .collect();
        let ctx = result.remove(0);
        assert_eq!(
            ctx.data.get("score"),
            Some(&ContextValue::Prim(PrimitiveContextValue::Int(100)))
        );
    }
}
