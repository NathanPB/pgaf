use super::Context;
use crate::{
    function,
    registry::{PublicIdentifier, PublicIdentifierError},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum PrimitiveContextValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Null,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContextValue {
    StringTemplate(Vec<ContextValue>),
    Prim(PrimitiveContextValue),
    Function {
        id: PublicIdentifier,
        function: function::Driver,
        args: function::FunctionArgs,
    },
    Ident(String),
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ContextEvaluationError {
    #[error("Identifier '{0}' could not be resolved.")]
    IdentifierNotFound(String),
    #[error("Function with identifier '{0}' could not be resolved.")]
    FunctionNotFound(PublicIdentifier),
    #[error("Failed to parse identifier '{0}': {1}")]
    IdentifierParse(String, PublicIdentifierError),
    #[error("Failed to invoke function '{0}': {1}")]
    FunctionInvoke(PublicIdentifier, function::FunctionRuntimeError),
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
    /// Variant name for the `value.type` tracing field. Used to group [`Self::to_prim`]
    /// span timings by the kind of expression evaluated.
    fn kind(&self) -> &'static str {
        match self {
            ContextValue::StringTemplate(_) => "template",
            ContextValue::Prim(_) => "prim",
            ContextValue::Function { .. } => "function",
            ContextValue::Ident(_) => "ident",
        }
    }

    /// Evaluates this expression to a [`PrimitiveContextValue`] against `ctx`.
    ///
    /// Recurses for [`ContextValue::Ident`] and [`ContextValue::StringTemplate`].
    ///
    /// ## Instrumentation
    ///
    /// Each recursion re-entering this same instrumented span with its own
    /// `value.type`, so nested busy time is attributed to the inner expression.
    #[instrument(level = "trace", skip_all, fields(value.type = self.kind()))]
    pub fn to_prim(&self, ctx: &Context) -> Result<PrimitiveContextValue, ContextEvaluationError> {
        match self {
            ContextValue::Prim(p) => Ok(p.clone()),
            ContextValue::Ident(i) => ctx
                .get(i)
                .ok_or(ContextEvaluationError::IdentifierNotFound(i.clone()))
                .and_then(|v| v.to_prim(ctx)),
            ContextValue::StringTemplate(s) => {
                let evaluated: Result<Vec<_>, _> = s
                    .iter()
                    .map(|it| it.to_prim(ctx).map(|prim| prim.as_string()))
                    .collect();

                Ok(PrimitiveContextValue::String(evaluated?.concat()))
            }
            ContextValue::Function { id, function, args } => function
                .invoke(args, ctx)
                .map_err(|e| ContextEvaluationError::FunctionInvoke(id.clone(), e)),
        }
    }
}

impl From<PrimitiveContextValue> for ContextValue {
    fn from(value: PrimitiveContextValue) -> Self {
        Self::Prim(value)
    }
}
