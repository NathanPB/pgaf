mod gen;
mod value;

use crate::config;
use crate::sites::Site;
pub use gen::*;
pub use value::*;

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

    #[test]
    fn test_template_string() {
        let ctx = Context {
            site: Site {
                id: 0,
                lon: GeoDeg::from(15.222),
                lat: GeoDeg::from(-15.23133),
            },
            run: config::runs::RunConfig {
                name: String::from("r1"),
                template: PathBuf::from("dummy"),
                extra: [
                    (
                        "foo".to_string(),
                        ContextValue::Prim(PrimitiveContextValue::String("foo".to_string())),
                    ),
                    (
                        "bar".to_string(),
                        ContextValue::Prim(PrimitiveContextValue::String("bar".to_string())),
                    ),
                    (
                        "baz".to_string(),
                        ContextValue::TemplateString(
                            serde_json::from_str::<TemplateString>(r#""${foo}-${bar}""#).unwrap(),
                        ),
                    ),
                    (
                        "buz".to_string(),
                        ContextValue::TemplateString(
                            serde_json::from_str::<TemplateString>(r#""${baz}-baz-${baz}""#)
                                .unwrap(),
                        ),
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            },
        };

        assert_eq!(
            ctx.run.extra.get("baz").map(|v| v.to_prim(&ctx).unwrap()),
            Some(PrimitiveContextValue::String("foo-bar".to_string()))
        );
        assert_eq!(
            ctx.run.extra.get("buz").map(|v| v.to_prim(&ctx).unwrap()),
            Some(PrimitiveContextValue::String(
                "foo-bar-baz-foo-bar".to_string()
            ))
        );
    }
}

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
