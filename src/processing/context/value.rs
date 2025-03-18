use super::Context;
use crate::processing::PipelineData;
use serde::{Deserialize, Deserializer, Serialize};
use std::path::PathBuf;
use std::sync::LazyLock;
use thiserror::Error;

static RE_TEMPLATE_STRING: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(\$\{[^}]+}|[^$]+)").unwrap());

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum PrimitiveContextValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum ContextValue {
    TemplateString(TemplateString),
    Prim(PrimitiveContextValue),
}

#[derive(Clone, Debug)]
pub struct TemplateString(Vec<TemplateStringFragment>);

#[derive(Debug, Error)]
pub enum ContextEvaluationError {
    #[error("Placeholder '{0}' could not be resolved.")]
    Interpolation(String),
}

#[derive(Clone, Debug)]
enum TemplateStringFragment {
    Literal(String),
    Template(String),
}

impl PrimitiveContextValue {
    pub fn as_string(&self) -> String {
        match self {
            PrimitiveContextValue::Bool(b) => b.to_string(),
            PrimitiveContextValue::Int(i) => i.to_string(),
            PrimitiveContextValue::Float(f) => f.to_string(),
            PrimitiveContextValue::String(s) => s.clone(),
        }
    }
}

impl ContextValue {
    pub fn to_prim(&self, ctx: &Context) -> Result<PrimitiveContextValue, ContextEvaluationError> {
        match self {
            ContextValue::Prim(p) => Ok(p.clone()),
            ContextValue::TemplateString(s) => {
                Ok(PrimitiveContextValue::String(s.interpolate(ctx)?))
            }
        }
    }
}

impl TemplateString {
    pub fn interpolate(&self, ctx: &Context) -> Result<String, ContextEvaluationError> {
        let mut s = String::new();
        for fragment in &self.0 {
            match fragment {
                TemplateStringFragment::Literal(l) => s.push_str(l),
                TemplateStringFragment::Template(k) => {
                    let value = ctx
                        .get(k)
                        .ok_or(ContextEvaluationError::Interpolation(k.to_string()))?;
                    s.push_str(value.to_prim(ctx)?.as_string().as_str());
                }
            }
        }
        Ok(s)
    }
}

impl<'de> Deserialize<'de> for TemplateString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let fragments: Vec<TemplateStringFragment> = RE_TEMPLATE_STRING
            .captures_iter(&s)
            .map(|cap| {
                let matched = &cap[0];
                if matched.starts_with("${") && matched.ends_with('}') {
                    let placeholder = matched.trim_start_matches("${").trim_end_matches('}');
                    TemplateStringFragment::Template(placeholder.to_string())
                } else {
                    TemplateStringFragment::Literal(matched.to_string())
                }
            })
            .collect();

        if fragments.is_empty() {
            return Err(serde::de::Error::custom(format!(
                "Invalid template string: '{}' contains no valid fragments (expected at least one placeholder in the format '${{...}}')",
                s
            )));
        }

        Ok(TemplateString(fragments))
    }
}

impl Serialize for TemplateString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = String::new();
        for fragment in &self.0 {
            match fragment {
                TemplateStringFragment::Literal(l) => s.push_str(l),
                TemplateStringFragment::Template(t) => s.push_str(&format!("${{{}}}", t)),
            }
        }
        serializer.serialize_str(&s)
    }
}

impl PipelineData for Context {}

impl Context {
    pub fn get(&self, key: &str) -> Option<ContextValue> {
        match key {
            "site_id" => Some(ContextValue::Prim(PrimitiveContextValue::String(
                self.site.id.to_string(),
            ))),
            "lng" => Some(ContextValue::Prim(PrimitiveContextValue::Float(
                self.site.lon.as_f64().into(),
            ))),
            "lon" => Some(ContextValue::Prim(PrimitiveContextValue::Float(
                self.site.lon.as_f64().into(),
            ))),
            "lat" => Some(ContextValue::Prim(PrimitiveContextValue::Float(
                self.site.lat.as_f64().into(),
            ))),
            "name" => Some(ContextValue::Prim(PrimitiveContextValue::String(
                self.run.name.clone(),
            ))),
            _ => self.run.extra.get(key).cloned(),
        }
    }

    pub fn dir(&self, base: &PathBuf) -> PathBuf {
        let mut path = base.clone();
        path.push(&self.run.name);
        path.push(&self.site.lon.ns(4));
        path.push(&self.site.lat.ew(4));
        path
    }

    pub fn tera(&self) -> Result<tera::Context, ContextEvaluationError> {
        let mut ctx = tera::Context::new();
        ctx.insert("site_id", &self.site.id);
        ctx.insert("soil_id", &self.site.id); // Backwards compatibility. In the original Pythia, the site ID was the soil ID.
        ctx.insert("lng", &self.site.lon.as_f32()); // Backwards compatibility, original Pythia impl used lat/lng instead of lon/lat.
        ctx.insert("lon", &self.site.lon.as_f32());
        ctx.insert("lat", &self.site.lat.as_f32());
        ctx.insert("name", &self.run.name);

        for (k, v) in &self.run.extra {
            ctx.insert(k, &v.to_prim(self)?);
        }

        Ok(ctx)
    }
}
