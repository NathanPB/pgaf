use std::sync::LazyLock;

use pgaf_sdk::context::{Context, PrimitiveContextValue};
use pgaf_sdk::pipeline::{Driver, PipelineStepType, PipelineStepTypeDriver};

/// A pipeline step that evaluates its argument for side-effects and forwards every context
/// unchanged. Intended for calling functions when only their side-effects matter (e.g. writing a
/// file, triggering an external process) — the return value is evaluated and then discarded.
///
/// All contexts are forwarded regardless of what the argument evaluates to.
///
/// # Internals
///
/// The argument is evaluated (including any function side-effects) by `invoker_impl` in
/// [`pgaf_sdk::pipeline`] *before* [`Void::invoke`] is ever called.
/// [`invoke`](Void::invoke) itself is a plain passthrough and never drops items.
pub struct Void;

impl PipelineStepType<PrimitiveContextValue> for Void {
    fn invoke(
        stream: Box<dyn Iterator<Item = (PrimitiveContextValue, Context)>>,
    ) -> Box<dyn Iterator<Item = Context>> {
        Box::new(stream.map(|(_, ctx)| ctx))
    }
}

pub static VOID_DRIVER: LazyLock<Driver> = LazyLock::new(|| {
    PipelineStepTypeDriver::<Void, PrimitiveContextValue>::default().coerce_to_dynamic()
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{make_ctx, make_ctx_with_extras};
    use pgaf_sdk::context::{ContextValue, PrimitiveContextValue};
    use pgaf_sdk::pipeline::PipelineStepTypeArgs;
    use std::sync::Arc;

    fn one(v: PrimitiveContextValue) -> PipelineStepTypeArgs {
        PipelineStepTypeArgs::One(ContextValue::Prim(v))
    }

    #[test]
    fn null_passes_through() {
        let ctxs = vec![make_ctx(1), make_ctx(2)];
        let result: Vec<_> = VOID_DRIVER
            .invoke(
                Arc::new(one(PrimitiveContextValue::Null)),
                Box::new(ctxs.into_iter()),
            )
            .collect();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn bool_passes_through() {
        let ctxs = vec![make_ctx(1), make_ctx(2)];
        let result: Vec<_> = VOID_DRIVER
            .invoke(
                Arc::new(one(PrimitiveContextValue::Bool(true))),
                Box::new(ctxs.into_iter()),
            )
            .collect();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn string_passes_through() {
        let ctxs = vec![make_ctx(1)];
        let result: Vec<_> = VOID_DRIVER
            .invoke(
                Arc::new(one(PrimitiveContextValue::String("hello".into()))),
                Box::new(ctxs.into_iter()),
            )
            .collect();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn ident_resolving_to_non_null_passes_through() {
        let extras = [(
            "status".into(),
            ContextValue::Prim(PrimitiveContextValue::Int(42)),
        )]
        .into();
        let ctx = make_ctx_with_extras(1, extras);
        let args = PipelineStepTypeArgs::One(ContextValue::Ident("status".into()));
        let result: Vec<_> = VOID_DRIVER
            .invoke(Arc::new(args), Box::new(vec![ctx].into_iter()))
            .collect();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn ctx_is_unmodified() {
        let ctx = make_ctx(7);
        let result: Vec<_> = VOID_DRIVER
            .invoke(
                Arc::new(one(PrimitiveContextValue::Bool(false))),
                Box::new(vec![ctx].into_iter()),
            )
            .collect();
        assert_eq!(result[0].unit.id, 7.into());
    }
}
