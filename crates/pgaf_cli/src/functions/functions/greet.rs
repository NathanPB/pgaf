use serde::Deserialize;
use std::sync::LazyLock;

use crate::functions::{Driver, Function, FunctionDriver, FunctionRuntimeError};
use crate::processing::context::{Context, PrimitiveContextValue};

pub struct Greet;

#[derive(Deserialize)]
pub struct GreetArgs {
    first_name: String,
    last_name: String,
}

impl Function<GreetArgs> for Greet {
    fn invoke(
        args: GreetArgs,
        ctx: &Context,
    ) -> Result<PrimitiveContextValue, FunctionRuntimeError> {
        let result = format!(
            "Hello {} {}, we are on site {}.",
            args.first_name, args.last_name, ctx.site.id
        );
        Ok(PrimitiveContextValue::String(result))
    }
}

pub static GREET_DRIVER: LazyLock<Driver> =
    LazyLock::new(|| FunctionDriver::<Greet, GreetArgs>::new().coerce_to_dynamic());

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        config::runs::RunConfig, data::GeoDeg, processing::context::ContextValue, sites::Site,
    };

    use super::*;

    #[test]
    fn greet() {
        let ctx = Context {
            site: Site {
                id: 1,
                lon: GeoDeg::from(0.0),
                lat: GeoDeg::from(0.0),
            },
            run: RunConfig {
                name: "test".into(),
                extra: Default::default(),
                template: PathBuf::from("dummy"),
            },
        };

        let args = [
            (
                "first_name".into(),
                ContextValue::Prim(PrimitiveContextValue::String("Alice".into())),
            ),
            (
                "last_name".into(),
                ContextValue::Prim(PrimitiveContextValue::String("Smith".into())),
            ),
        ]
        .into();

        let result = GREET_DRIVER.invoke(&args, &ctx).unwrap();

        assert_eq!(
            result,
            PrimitiveContextValue::String("Hello Alice Smith, we are on site 1.".into())
        );
    }
}
