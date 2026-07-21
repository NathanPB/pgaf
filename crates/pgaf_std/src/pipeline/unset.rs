use pgaf_sdk::context::{Context, PrimitiveContextValue};
use pgaf_sdk::pipeline::{Driver, PipelineStepType, PipelineStepTypeDriver};
use std::collections::HashMap;
use std::sync::LazyLock;

/// A pipeline step that conditionally removes keys from the [`Context`]'s run extras.
///
/// The argument must be a [`Map`](pgaf_sdk::pipeline::PipelineStepTypeArgs::Map) of
/// `String → ContextValue`, where each value acts as a predicate for the matching key.
/// `false` removes the key; `true` leaves it untouched. Any other primitive is a no-op
/// and a warning is emitted to stderr. All contexts are forwarded; this step never drops items.
pub struct Unset;

impl PipelineStepType<HashMap<String, PrimitiveContextValue>> for Unset {
    fn invoke(
        stream: Box<dyn Iterator<Item = (HashMap<String, PrimitiveContextValue>, Context)>>,
    ) -> Box<dyn Iterator<Item = Context>> {
        Box::new(stream.map(|(predicates, mut ctx)| {
            for (key, value) in predicates {
                match value {
                    PrimitiveContextValue::Bool(false) => {
                        ctx.data.remove(&key);
                    }
                    PrimitiveContextValue::Bool(true) => {}
                    _ => {
                        tracing::warn!(
                            unit.id = %ctx.unit.id,
                            key = %key,
                            "unset predicate not boolean"
                        );
                    }
                }
            }
            ctx
        }))
    }
}

pub static UNSET_DRIVER: LazyLock<Driver> = LazyLock::new(|| {
    PipelineStepTypeDriver::<Unset, HashMap<String, PrimitiveContextValue>>::default()
        .coerce_to_dynamic()
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::make_ctx_with_extras;
    use pgaf_sdk::context::ContextValue;
    use pgaf_sdk::pipeline::PipelineStepTypeArgs;
    use std::sync::Arc;

    fn prim(v: PrimitiveContextValue) -> ContextValue {
        ContextValue::Prim(v)
    }

    #[test]
    fn removes_key_when_false() {
        let ctx = make_ctx_with_extras(
            1,
            [("foo".into(), prim(PrimitiveContextValue::Int(42)))].into(),
        );
        let args = PipelineStepTypeArgs::Map(
            [("foo".into(), prim(PrimitiveContextValue::Bool(false)))].into(),
        );
        let mut result: Vec<_> = UNSET_DRIVER
            .invoke(Arc::new(args), Box::new(vec![ctx].into_iter()))
            .collect();
        let ctx = result.remove(0);
        assert!(!ctx.data.contains_key("foo"));
    }

    #[test]
    fn keeps_key_when_true() {
        let ctx = make_ctx_with_extras(
            1,
            [("foo".into(), prim(PrimitiveContextValue::Int(42)))].into(),
        );
        let args = PipelineStepTypeArgs::Map(
            [("foo".into(), prim(PrimitiveContextValue::Bool(true)))].into(),
        );
        let mut result: Vec<_> = UNSET_DRIVER
            .invoke(Arc::new(args), Box::new(vec![ctx].into_iter()))
            .collect();
        let ctx = result.remove(0);
        assert!(ctx.data.contains_key("foo"));
    }

    #[test]
    fn warns_and_keeps_on_non_bool() {
        let ctx = make_ctx_with_extras(
            1,
            [("foo".into(), prim(PrimitiveContextValue::Int(42)))].into(),
        );
        let args = PipelineStepTypeArgs::Map(
            [(
                "foo".into(),
                prim(PrimitiveContextValue::String("oops".into())),
            )]
            .into(),
        );
        let mut result: Vec<_> = UNSET_DRIVER
            .invoke(Arc::new(args), Box::new(vec![ctx].into_iter()))
            .collect();
        let ctx = result.remove(0);
        assert!(ctx.data.contains_key("foo"));
    }
}
