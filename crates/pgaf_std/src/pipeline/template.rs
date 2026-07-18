use std::fmt;
use std::sync::LazyLock;

use pgaf_sdk::context::{Context, ContextEvaluationError, ContextValue, PrimitiveContextValue};
use pgaf_sdk::pipeline::{Driver, PipelineStepType, PipelineStepTypeDriver};
use serde::Deserializer;
use serde::de::{IgnoredAny, MapAccess, Visitor};

/// Where the template string comes from.
pub enum TemplateSource {
    /// Read the template from a file at this path.
    File(String),
    /// Use this string directly as the template.
    Inline(String),
}

/// Arguments for the [`Template`] pipeline step. `template_file` and `template` are mutually
/// exclusive. `output_file` and `output` are independent and may both be set simultaneously.
pub struct TemplateArgs {
    /// Template source. `template_file` and `template` are mutually exclusive.
    pub source: TemplateSource,
    /// Write the rendered output to this file path.
    pub output_file: Option<String>,
    /// Store the rendered output as a string in `ctx.run.extra` under this key.
    pub output: Option<String>,
}

impl<'de> serde::Deserialize<'de> for TemplateArgs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(TemplateArgsVisitor)
    }
}

struct TemplateArgsVisitor;

impl<'de> Visitor<'de> for TemplateArgsVisitor {
    type Value = TemplateArgs;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a map of template step arguments")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<TemplateArgs, A::Error> {
        let mut template_file: Option<String> = None;
        let mut template_inline: Option<String> = None;
        let mut output_file: Option<String> = None;
        let mut output: Option<String> = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "template_file" => template_file = Some(map.next_value()?),
                "template" => template_inline = Some(map.next_value()?),
                "output_file" => output_file = Some(map.next_value()?),
                "output" => output = Some(map.next_value()?),
                _ => {
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        let source = match (template_file, template_inline) {
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "`template_file` and `template` are mutually exclusive",
                ));
            }
            (Some(f), None) => TemplateSource::File(f),
            (None, Some(s)) => TemplateSource::Inline(s),
            (None, None) => {
                return Err(serde::de::Error::missing_field(
                    "template_file` or `template",
                ));
            }
        };

        Ok(TemplateArgs {
            source,
            output_file,
            output,
        })
    }
}

#[derive(Debug)]
enum TemplateError {
    Io(std::io::Error),
    Tera(tera::Error),
    Context(ContextEvaluationError),
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateError::Io(e) => write!(f, "io error: {e}"),
            TemplateError::Tera(e) => write!(f, "tera error: {e}"),
            TemplateError::Context(e) => write!(f, "context evaluation error: {e}"),
        }
    }
}

impl From<std::io::Error> for TemplateError {
    fn from(e: std::io::Error) -> Self {
        TemplateError::Io(e)
    }
}

impl From<tera::Error> for TemplateError {
    fn from(e: tera::Error) -> Self {
        TemplateError::Tera(e)
    }
}

impl From<ContextEvaluationError> for TemplateError {
    fn from(e: ContextEvaluationError) -> Self {
        TemplateError::Context(e)
    }
}

/// A pipeline step that renders a Tera template with the current context and routes the output
/// to a file, a context key, or both simultaneously.
///
/// All values in `ctx.run.extra` (plus `site_id`, `lon`, `lat`, `name`) are available inside
/// the template via their key names — the same variables exposed by [`Context::tera`].
///
/// | Config key      | Type   | Required | Description |
/// |-----------------|--------|----------|-------------|
/// | `template_file` | string | †        | Load template from this file path *(exclusive with `template`)* |
/// | `template`      | string | †        | Use this string as the inline template *(exclusive with `template_file`)* |
/// | `output_file`   | string | ✗        | Write rendered output to this file path |
/// | `output`        | string | ✗        | Store rendered output in this context key |
///
/// † Exactly one of `template_file` or `template` is required.
pub struct Template;

impl PipelineStepType<TemplateArgs> for Template {
    fn invoke(
        stream: Box<dyn Iterator<Item = (TemplateArgs, Context)>>,
    ) -> Box<dyn Iterator<Item = Context>> {
        Box::new(stream.filter_map(|(args, mut ctx)| {
            let unit_id = ctx.unit.id.clone();
            match render(args, &mut ctx) {
                Ok(()) => Some(ctx),
                Err(e) => {
                    eprintln!("template: error at unit {unit_id}: {e}");
                    None
                }
            }
        }))
    }
}

fn build_tera_ctx(ctx: &Context) -> Result<tera::Context, TemplateError> {
    let id = ctx.unit.id.to_string();
    let lon = ctx.unit.lon.as_f64();
    let lat = ctx.unit.lat.as_f64();
    let name = ctx.run.name.clone();
    let extras: Vec<(String, serde_json::Value)> = ctx
        .run
        .extra
        .iter()
        .map(|(k, v)| {
            let prim = v.to_prim(ctx).map_err(TemplateError::Context)?;
            let val = serde_json::to_value(prim).unwrap_or(serde_json::Value::Null);
            Ok((k.clone(), val))
        })
        .collect::<Result<_, TemplateError>>()?;

    let mut tera_ctx = tera::Context::new();
    tera_ctx.insert("id", &id);
    tera_ctx.insert("lon", &lon);
    tera_ctx.insert("lat", &lat);
    tera_ctx.insert("name", &name);
    for (k, v) in extras {
        tera_ctx.insert(k, &v);
    }
    Ok(tera_ctx)
}

fn render(args: TemplateArgs, ctx: &mut Context) -> Result<(), TemplateError> {
    let tera_ctx = build_tera_ctx(ctx)?;

    let template_str = match args.source {
        TemplateSource::File(path) => std::fs::read_to_string(path)?,
        TemplateSource::Inline(s) => s,
    };

    let rendered = tera::Tera::one_off(&template_str, &tera_ctx, false)?;

    if let Some(path) = args.output_file {
        std::fs::write(path, &rendered)?;
    }

    if let Some(key) = args.output {
        ctx.run.extra.insert(
            key,
            ContextValue::Prim(PrimitiveContextValue::String(rendered)),
        );
    }

    Ok(())
}

pub static TEMPLATE_DRIVER: LazyLock<Driver> = LazyLock::new(|| {
    PipelineStepTypeDriver::<Template, TemplateArgs>::default().coerce_to_dynamic()
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::make_ctx_with_extras;
    use pgaf_sdk::context::{ContextValue, PrimitiveContextValue};
    use pgaf_sdk::pipeline::PipelineStepTypeArgs;
    use std::collections::HashMap;

    fn map_args(
        pairs: impl IntoIterator<Item = (&'static str, ContextValue)>,
    ) -> PipelineStepTypeArgs {
        PipelineStepTypeArgs::Map(
            pairs
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect::<HashMap<_, _>>(),
        )
    }

    fn str_val(s: &str) -> ContextValue {
        ContextValue::Prim(PrimitiveContextValue::String(s.into()))
    }

    fn extras(
        pairs: impl IntoIterator<Item = (&'static str, &'static str)>,
    ) -> HashMap<String, ContextValue> {
        pairs
            .into_iter()
            .map(|(k, v)| (k.to_string(), str_val(v)))
            .collect()
    }

    #[test]
    fn inline_template_renders_to_key() {
        let args = map_args([
            ("template", str_val("hello world")),
            ("output", str_val("out")),
        ]);

        let ctx = make_ctx_with_extras(1, extras([]));
        let ctx = TEMPLATE_DRIVER
            .invoke(args, Box::new(vec![ctx].into_iter()))
            .next()
            .unwrap();

        assert_eq!(
            ctx.run.extra.get("out"),
            Some(&ContextValue::Prim(PrimitiveContextValue::String(
                "hello world".into()
            )))
        );
    }

    #[test]
    fn context_vars_available_in_template() {
        let args = map_args([
            ("template", str_val("{{ greeting }}")),
            ("output", str_val("out")),
        ]);

        let ctx = make_ctx_with_extras(1, extras([("greeting", "hi there")]));
        let ctx = TEMPLATE_DRIVER
            .invoke(args, Box::new(vec![ctx].into_iter()))
            .next()
            .unwrap();

        assert_eq!(
            ctx.run.extra.get("out"),
            Some(&ContextValue::Prim(PrimitiveContextValue::String(
                "hi there".into()
            )))
        );
    }

    #[test]
    fn template_file_renders() {
        let path = std::env::temp_dir().join("pgaf_test_template_input.tera");
        std::fs::write(&path, "file: {{ val }}").unwrap();

        let args = map_args([
            ("template_file", str_val(path.to_str().unwrap())),
            ("output", str_val("out")),
        ]);

        let ctx = make_ctx_with_extras(1, extras([("val", "ok")]));
        let ctx = TEMPLATE_DRIVER
            .invoke(args, Box::new(vec![ctx].into_iter()))
            .next()
            .unwrap();

        assert_eq!(
            ctx.run.extra.get("out"),
            Some(&ContextValue::Prim(PrimitiveContextValue::String(
                "file: ok".into()
            )))
        );
    }

    #[test]
    fn output_written_to_file() {
        let path = std::env::temp_dir().join("pgaf_test_template_output.txt");
        let args = map_args([
            ("template", str_val("written")),
            ("output_file", str_val(path.to_str().unwrap())),
        ]);

        let ctx = make_ctx_with_extras(1, extras([]));
        let result: Vec<_> = TEMPLATE_DRIVER
            .invoke(args, Box::new(vec![ctx].into_iter()))
            .collect();

        assert_eq!(result.len(), 1);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "written");
    }

    #[test]
    fn both_outputs_work_simultaneously() {
        let path = std::env::temp_dir().join("pgaf_test_template_both.txt");
        let args = map_args([
            ("template", str_val("dual")),
            ("output_file", str_val(path.to_str().unwrap())),
            ("output", str_val("out")),
        ]);

        let ctx = make_ctx_with_extras(1, extras([]));
        let ctx = TEMPLATE_DRIVER
            .invoke(args, Box::new(vec![ctx].into_iter()))
            .next()
            .unwrap();

        assert_eq!(
            ctx.run.extra.get("out"),
            Some(&ContextValue::Prim(PrimitiveContextValue::String(
                "dual".into()
            )))
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "dual");
    }

    #[test]
    fn mutual_exclusivity_drops_context() {
        let args = map_args([
            ("template", str_val("a")),
            ("template_file", str_val("/dev/null")),
            ("output", str_val("out")),
        ]);
        let ctx = make_ctx_with_extras(1, extras([]));
        let result: Vec<_> = TEMPLATE_DRIVER
            .invoke(args, Box::new(vec![ctx].into_iter()))
            .collect();

        assert!(result.is_empty());
    }

    #[test]
    fn missing_source_drops_context() {
        let args = map_args([("output", str_val("out"))]);
        let ctx = make_ctx_with_extras(1, extras([]));
        let result: Vec<_> = TEMPLATE_DRIVER
            .invoke(args, Box::new(vec![ctx].into_iter()))
            .collect();
        assert!(result.is_empty());
    }
}
