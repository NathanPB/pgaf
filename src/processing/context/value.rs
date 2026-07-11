use super::{Context, TemplateString};
use crate::processing::PipelineData;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum PrimitiveContextValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Null,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum ContextValue {
    TemplateString(TemplateString),
    Prim(PrimitiveContextValue),
}

#[derive(Debug, Error)]
pub enum ContextEvaluationError {
    #[error("Placeholder '{0}' could not be resolved.")]
    Interpolation(String),
}

impl PrimitiveContextValue {
    pub fn as_string(&self) -> String {
        match self {
            PrimitiveContextValue::Bool(b) => b.to_string(),
            PrimitiveContextValue::Int(i) => i.to_string(),
            PrimitiveContextValue::Float(f) => f.to_string(),
            PrimitiveContextValue::String(s) => s.clone(),
            PrimitiveContextValue::Null => "null".to_string(),
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

impl PipelineData for Context {}
