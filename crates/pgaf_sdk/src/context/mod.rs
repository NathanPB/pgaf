mod value;

use crate::domain::ExecutionUnit;
use std::collections::HashMap;
pub use value::{ContextEvaluationError, ContextValue, PrimitiveContextValue};

/// The runtime state for a single pipeline invocation on one [`ExecutionUnit`].
///
/// A [`Context`] reresents a spatial target (the [`ExecutionUnit`] currently being processed)
/// throughout the execution of the pipeline, carrying additional runtime data.
#[derive(Debug, Clone)]
pub struct Context {
    pub unit: ExecutionUnit,
    pub data: HashMap<String, ContextValue>,
}

impl Context {
    pub fn get(&self, key: &str) -> Option<ContextValue> {
        // TODO: Remove "lng", rename "site_id" -> "id"
        match key {
            "site_id" => Some(ContextValue::Prim(PrimitiveContextValue::String(
                self.unit.id.to_string(),
            ))),
            "lng" => Some(ContextValue::Prim(PrimitiveContextValue::Float(
                self.unit.lon.as_f64(),
            ))),
            "lon" => Some(ContextValue::Prim(PrimitiveContextValue::Float(
                self.unit.lon.as_f64(),
            ))),
            "lat" => Some(ContextValue::Prim(PrimitiveContextValue::Float(
                self.unit.lat.as_f64(),
            ))),
            _ => self.data.get(key).cloned(),
        }
    }
}
