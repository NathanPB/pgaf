mod value;

use crate::config;
use crate::domain::ExecutionUnit;
use std::path::{Path, PathBuf};
pub use value::{ContextEvaluationError, ContextValue, PrimitiveContextValue};

/// The runtime state for a single pipeline invocation on one [`ExecutionUnit`].
///
/// A [`Context`] pairs a spatial target (the [`ExecutionUnit`] currently being processed) with
/// the [`RunConfig`](crate::config::RunConfig) that governs how that target
/// should be handled. It is the primary interface through which pipeline
/// functions read input parameters and resolve template variables.
#[derive(Debug, Clone)]
pub struct Context {
    pub unit: ExecutionUnit,
    pub run: config::RunConfig,
}

impl Context {
    pub fn get(&self, key: &str) -> Option<ContextValue> {
        // TODO: Remove this bunch of legacy backwards-compat stuff
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
            "name" => Some(ContextValue::Prim(PrimitiveContextValue::String(
                self.run.name.clone(),
            ))),
            _ => self.run.extra.get(key).cloned(),
        }
    }

    pub fn dir(&self, base: &Path) -> PathBuf {
        let mut path = base.to_path_buf();
        path.push(&self.run.name);
        path.push(self.unit.lon.ns(4));
        path.push(self.unit.lat.ew(4));
        path
    }

    pub fn tera(&self) -> Result<tera::Context, ContextEvaluationError> {
        let mut ctx = tera::Context::new();
        ctx.insert("site_id", &self.unit.id.to_string());
        ctx.insert("soil_id", &self.unit.id.to_string()); // Backwards compatibility. In the original Pythia, the site ID was the soil ID.
        ctx.insert("lng", &self.unit.lon.as_f32()); // Backwards compatibility, original Pythia impl used lat/lng instead of lon/lat.
        ctx.insert("lon", &self.unit.lon.as_f32());
        ctx.insert("lat", &self.unit.lat.as_f32());
        ctx.insert("name", &self.run.name);

        for (k, v) in &self.run.extra {
            ctx.insert(k, &v.to_prim(self)?);
        }

        Ok(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::data::GeoDeg;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_context_dir() {
        let wd = PathBuf::from("/tmp");
        let ctx = Context {
            unit: ExecutionUnit {
                id: 0.into(),
                lon: GeoDeg::from(15.222),
                lat: GeoDeg::from(-15.23133),
            },
            run: config::RunConfig {
                name: String::from("r1"),
                extra: HashMap::new(),
                template: PathBuf::from("dummy"),
            },
        };

        assert_eq!(ctx.dir(&wd), PathBuf::from("/tmp/r1/15_2220N/15_2313W"));
    }
}
