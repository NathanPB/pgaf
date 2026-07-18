use pgaf_sdk::context::{Context, PrimitiveContextValue};
use pgaf_sdk::pipeline::{Driver, PipelineStepType, PipelineStepTypeDriver};
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
pub struct DisplayConfig {
    pub label: String,
}

/// A pipeline step that prints the current context to stderr and forwards it unchanged.
pub struct Display;

fn format_prim(p: &PrimitiveContextValue) -> String {
    match p {
        PrimitiveContextValue::Bool(b) => b.to_string(),
        PrimitiveContextValue::Int(i) => i.to_string(),
        PrimitiveContextValue::Float(f) => f.to_string(),
        PrimitiveContextValue::String(s) => format!("{s:?}"),
        PrimitiveContextValue::Null => "null".to_string(),
    }
}

impl PipelineStepType<DisplayConfig> for Display {
    fn invoke(
        stream: Box<dyn Iterator<Item = (DisplayConfig, Context)>>,
    ) -> Box<dyn Iterator<Item = Context>> {
        Box::new(stream.map(|(config, ctx)| {
            let data: Vec<String> = ctx
                .data
                .iter()
                .map(|(k, v)| {
                    let formatted = match v.to_prim(&ctx) {
                        Ok(p) => format_prim(&p),
                        Err(e) => format!("<error: {e}>"),
                    };
                    format!("{k}={formatted}")
                })
                .collect();

            let data_str = if data.is_empty() {
                String::from("<empty>")
            } else {
                data.join(", ")
            };

            eprintln!(
                "[{}] id={} lon={} lat={} | {}",
                config.label, ctx.unit.id, ctx.unit.lon, ctx.unit.lat, data_str
            );
            ctx
        }))
    }
}

pub static DISPLAY_DRIVER: LazyLock<Driver> = LazyLock::new(|| {
    PipelineStepTypeDriver::<Display, DisplayConfig>::default().coerce_to_dynamic()
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{make_ctx, make_ctx_with_extras};
    use core::f64;
    use pgaf_sdk::context::ContextValue;
    use pgaf_sdk::pipeline::PipelineStepTypeArgs;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn label_args(label: &str) -> Arc<PipelineStepTypeArgs> {
        Arc::new(PipelineStepTypeArgs::Map(HashMap::from([(
            "label".into(),
            ContextValue::Prim(PrimitiveContextValue::String(label.into())),
        )])))
    }

    #[test]
    fn passes_through_unchanged() {
        let ctxs = vec![make_ctx(1), make_ctx(2), make_ctx(3)];
        let result: Vec<_> = DISPLAY_DRIVER
            .invoke(label_args("test"), Box::new(ctxs.into_iter()))
            .collect();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].unit.id, 1.into());
        assert_eq!(result[1].unit.id, 2.into());
        assert_eq!(result[2].unit.id, 3.into());
    }

    #[test]
    fn passes_through_with_data() {
        let extras = [
            (
                "foo".into(),
                ContextValue::Prim(PrimitiveContextValue::String("bar".into())),
            ),
            (
                "count".into(),
                ContextValue::Prim(PrimitiveContextValue::Int(42)),
            ),
        ]
        .into();
        let ctx = make_ctx_with_extras(7, extras);
        let result: Vec<_> = DISPLAY_DRIVER
            .invoke(label_args("test"), Box::new(vec![ctx].into_iter()))
            .collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].unit.id, 7.into());
    }

    #[test]
    fn evaluates_ident() {
        let extras = [(
            "score".into(),
            ContextValue::Prim(PrimitiveContextValue::Float(f64::consts::PI)),
        )]
        .into();
        let ctx = make_ctx_with_extras(5, extras);
        let result: Vec<_> = DISPLAY_DRIVER
            .invoke(label_args("test"), Box::new(vec![ctx].into_iter()))
            .collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].unit.id, 5.into());
    }
}
