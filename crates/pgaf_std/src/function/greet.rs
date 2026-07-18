use serde::Deserialize;
use std::sync::LazyLock;

use pgaf_sdk::context::{Context, PrimitiveContextValue};
use pgaf_sdk::function::{Driver, Function, FunctionDriver, FunctionRuntimeError};

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
            "Hello {} {}, we are on unit {}.",
            args.first_name, args.last_name, ctx.unit.id
        );
        Ok(PrimitiveContextValue::String(result))
    }
}

pub static GREET_DRIVER: LazyLock<Driver> =
    LazyLock::new(|| FunctionDriver::<Greet, GreetArgs>::default().coerce_to_dynamic());

#[cfg(test)]
mod tests {
    use super::*;
    use pgaf_sdk::{context::ContextValue, data::GeoDeg, domain::ExecutionUnit};

    #[test]
    fn greet() {
        let ctx = Context {
            unit: ExecutionUnit {
                id: 1.into(),
                lon: GeoDeg::from(0.0),
                lat: GeoDeg::from(0.0),
            },
            data: Default::default(),
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
            PrimitiveContextValue::String("Hello Alice Smith, we are on unit 1.".into())
        );
    }
}
