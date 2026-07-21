mod deserializer;

use crate::context::{Context, ContextEvaluationError, ContextValue, PrimitiveContextValue};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;

pub type FunctionArgs = HashMap<String, ContextValue>;

#[derive(thiserror::Error, Debug, PartialEq)]
#[error("Runtime error: {0}")]
pub struct FunctionRuntimeError(pub String);

#[derive(thiserror::Error, Debug)]
pub enum FunctionArgParseError {
    #[error("Failed to evaluate: {0}")]
    Evaluation(#[from] ContextEvaluationError),
    #[error("Failed to parse: {0}")]
    Custom(String),
}

impl serde::de::Error for FunctionArgParseError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self::Custom(msg.to_string())
    }
}

/// See the [crate-level tracing contract](crate) for how implementations must
/// report diagnostics.
pub trait Function<A>: Send + Sync {
    fn invoke(args: A, ctx: &Context) -> Result<PrimitiveContextValue, FunctionRuntimeError>;
}

pub struct FunctionDriver<F: Function<A>, A> {
    _marker_f: PhantomData<F>,
    _marker_a: PhantomData<A>,
}

impl<F: Function<A> + 'static, A: DeserializeOwned + Send + Sync + 'static> Default
    for FunctionDriver<F, A>
{
    fn default() -> Self {
        Self {
            _marker_f: Default::default(),
            _marker_a: Default::default(),
        }
    }
}

impl<F: Function<A> + 'static, A: DeserializeOwned + Send + Sync + 'static> FunctionDriver<F, A> {
    pub fn coerce_to_dynamic(self) -> Driver {
        Driver(invoker_impl::<F, A>)
    }
}

type InvokeFn = fn(&FunctionArgs, &Context) -> Result<PrimitiveContextValue, FunctionRuntimeError>;

fn invoker_impl<F: Function<A>, A: DeserializeOwned + Send + Sync>(
    args: &FunctionArgs,
    ctx: &Context,
) -> Result<PrimitiveContextValue, FunctionRuntimeError> {
    let a = deserializer::deserialize_args::<A>(args.clone(), ctx)
        .map_err(|e| FunctionRuntimeError(e.to_string()))?;

    F::invoke(a, ctx)
}

#[derive(Clone, Debug)]
pub struct Driver(InvokeFn);

/// **WARNING:** [`PartialEq`] is implemented for [`Driver`] solely to fulfil a badly-placed `derive(PartialEq)`.
/// This comparison always results in `true`.
impl PartialEq for Driver {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Driver {
    pub fn invoke(
        &self,
        args: &FunctionArgs,
        ctx: &Context,
    ) -> Result<PrimitiveContextValue, FunctionRuntimeError> {
        self.0(args, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GeoDeg;
    use crate::domain::ExecutionUnit;
    use serde::Deserialize;

    struct StringFn;
    #[derive(Deserialize)]
    struct StringArgs {
        value: String,
    }
    impl Function<StringArgs> for StringFn {
        fn invoke(
            args: StringArgs,
            _ctx: &Context,
        ) -> Result<PrimitiveContextValue, FunctionRuntimeError> {
            Ok(PrimitiveContextValue::String(args.value))
        }
    }

    fn make_ctx() -> Context {
        make_ctx_with_data(HashMap::new())
    }

    fn make_ctx_with_data(data: HashMap<String, ContextValue>) -> Context {
        Context {
            unit: ExecutionUnit {
                id: 1.into(),
                lon: GeoDeg::from(0.0),
                lat: GeoDeg::from(0.0),
            },
            data,
        }
    }

    #[test]
    fn prim_string() {
        let ctx = make_ctx();
        let args = [(
            "value".into(),
            ContextValue::Prim(PrimitiveContextValue::String("hello".into())),
        )]
        .into();

        let result = FunctionDriver::<StringFn, StringArgs>::default()
            .coerce_to_dynamic()
            .invoke(&args, &ctx)
            .unwrap();

        assert_eq!(result, PrimitiveContextValue::String("hello".into()));
    }

    #[test]
    fn lazy_template_string() {
        let ctx = make_ctx_with_data(
            [(
                "user_name".into(),
                ContextValue::Prim(PrimitiveContextValue::String("Alice".into())),
            )]
            .into_iter()
            .collect(),
        );
        let ts = ContextValue::StringTemplate(vec![
            ContextValue::Prim(PrimitiveContextValue::String("Hello ".to_string())),
            ContextValue::Ident("user_name".to_string()),
        ]);
        let args = [("value".into(), ts)].into();

        let result = FunctionDriver::<StringFn, StringArgs>::default()
            .coerce_to_dynamic()
            .invoke(&args, &ctx)
            .unwrap();

        assert_eq!(result, PrimitiveContextValue::String("Hello Alice".into()));
    }

    #[test]
    fn missing_arg() {
        let ctx = make_ctx();
        let args = FunctionArgs::new();
        let err = FunctionDriver::<StringFn, StringArgs>::default()
            .coerce_to_dynamic()
            .invoke(&args, &ctx)
            .unwrap_err();

        assert!(
            err.to_string().contains("value"),
            "error should name the missing field: {err}"
        );
    }

    #[test]
    fn invalid_type() {
        let ctx = make_ctx();
        let args = [(
            "value".into(),
            ContextValue::Prim(PrimitiveContextValue::Int(42)),
        )]
        .into();

        let err = FunctionDriver::<StringFn, StringArgs>::default()
            .coerce_to_dynamic()
            .invoke(&args, &ctx)
            .unwrap_err();

        assert!(
            err.to_string().contains("invalid type"),
            "error should describe the type mismatch: {err}"
        );
    }
}
