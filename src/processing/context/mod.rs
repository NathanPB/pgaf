mod eval;
mod gen;
mod value;

use crate::config;
use crate::sites::Site;
pub use gen::*;
use std::path::PathBuf;
pub use value::*;

/// Holds the information about the execution of a single run on a specific site with its bound run configurations.
#[derive(Debug, Clone)]
pub struct Context {
    #[allow(dead_code)]
    // The part of the code that uses this is not yet implemented, so it's not dead code.
    pub site: Site,

    #[allow(dead_code)]
    // The part of the code that uses this is not yet implemented, so it's not dead code.
    pub run: config::runs::RunConfig,
}

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
            site: Site {
                id: 0,
                lon: GeoDeg::from(15.222),
                lat: GeoDeg::from(-15.23133),
            },
            run: config::runs::RunConfig {
                name: String::from("r1"),
                extra: HashMap::new(),
                template: PathBuf::from("dummy"),
            },
        };

        assert_eq!(ctx.dir(&wd), PathBuf::from("/tmp/r1/15_2220N/15_2313W"));
    }
}
