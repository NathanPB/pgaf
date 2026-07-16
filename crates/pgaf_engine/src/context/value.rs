use std::collections::HashMap;

use crate::context::expr::Expr;
use crate::pipeline::PipelineData;
use pgaf_sdk::context::{Context, ContextEvaluationError, ContextValue, PrimitiveContextValue};
use pgaf_sdk::registry::{PublicIdentifier, Registries};
use serde::de::DeserializeSeed;
use serde::{Deserialize, Deserializer};

pub struct ContextValueDeserializeSeed<'a> {
    pub default_namespace: String,
    pub registries: &'a Registries,
}

impl<'de> DeserializeSeed<'de> for ContextValueDeserializeSeed<'de> {
    type Value = ContextValue;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        use super::expr::Expr;

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ParseStringFirstHelper {
            String(String),
            Other(PrimitiveContextValue),
        }

        match ParseStringFirstHelper::deserialize(deserializer)? {
            ParseStringFirstHelper::String(ts) => {
                let expr = Expr::try_from(ts.as_str()).unwrap_or(Expr::String(ts));
                expr_to_context_value(expr, self.registries, &self.default_namespace)
                    .map_err(serde::de::Error::custom)
            }
            ParseStringFirstHelper::Other(prim) => Ok(ContextValue::Prim(prim)),
        }
    }
}

#[allow(clippy::only_used_in_recursion)]
fn expr_to_context_value(
    expr: Expr,
    registries: &Registries,
    default_namespace: &str,
) -> Result<ContextValue, ContextEvaluationError> {
    Ok(match expr {
        Expr::Bool(v) => PrimitiveContextValue::Bool(v).into(),
        Expr::Int(v) => PrimitiveContextValue::Int(v).into(),
        Expr::Float(v) => PrimitiveContextValue::Float(v).into(),
        Expr::String(v) => PrimitiveContextValue::String(v).into(),
        Expr::Null => PrimitiveContextValue::Null.into(),
        Expr::StringTemplate(v) => {
            let frags: Result<_, _> = v
                .into_iter()
                .map(|expr| expr_to_context_value(expr, registries, default_namespace))
                .collect();
            ContextValue::StringTemplate(frags?)
        }
        Expr::FunctionCall { name, args } => {
            let id = PublicIdentifier::from_with_default_namespace(&name, default_namespace)
                .map_err(|err| ContextEvaluationError::IdentifierParse(name, err))?;

            let function = registries
                .reg_function_drivers()
                .get(&id)
                .ok_or(ContextEvaluationError::FunctionNotFound(id.clone()))?
                .clone();

            let args: HashMap<String, ContextValue> = args
                .into_iter()
                .map(|(k, expr)| {
                    let v = expr_to_context_value(expr, registries, default_namespace)?;
                    Ok((k, v))
                })
                .collect::<Result<HashMap<_, _>, ContextEvaluationError>>()?;

            ContextValue::Function {
                id,
                function: function.0,
                args,
            }
        }
        Expr::Ident(v) => ContextValue::Ident(v),
    })
}

impl PipelineData for Context {}

#[cfg(test)]
mod tests {
    use super::*;
    use pgaf_sdk::data::GeoDeg;
    use pgaf_sdk::domain::ExecutionUnit;
    use pgaf_sdk::function::{Driver, Function, FunctionDriver, FunctionRuntimeError};
    use pgaf_sdk::registry::FunctionDriverResource;
    use std::error::Error;
    use std::path::PathBuf;
    use std::sync::LazyLock;

    struct FnUppercase;

    #[derive(Deserialize)]
    struct FnUppercaseArgs {
        str: String,
    }

    impl Function<FnUppercaseArgs> for FnUppercase {
        fn invoke(
            args: FnUppercaseArgs,
            _: &Context,
        ) -> Result<PrimitiveContextValue, FunctionRuntimeError> {
            Ok(PrimitiveContextValue::String(args.str.to_uppercase()))
        }
    }

    static FN_UPPERCASE: LazyLock<Driver> =
        LazyLock::new(|| FunctionDriver::<FnUppercase, _>::default().coerce_to_dynamic());

    fn deserialize_context_value(
        json: &str,
        seed: Option<ContextValueDeserializeSeed>,
    ) -> Result<ContextValue, Box<dyn Error>> {
        let mut registries = Registries::default();
        let namespace = registries.claim_namespace("std")?;

        registries.regmut_function_drivers().register(
            &namespace,
            "uppercase",
            FunctionDriverResource(FN_UPPERCASE.clone()),
        )?;

        let seed = seed.unwrap_or(ContextValueDeserializeSeed {
            registries: &registries,
            default_namespace: "std".to_string(),
        });

        let mut de = serde_json::Deserializer::from_str(json);
        let value: ContextValue = seed.deserialize(&mut de)?;
        de.end()?;

        Ok(value)
    }

    #[test]
    fn deserialize_string_as_ast() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            deserialize_context_value(r#""${123}""#, None)?,
            ContextValue::StringTemplate(vec![ContextValue::Prim(PrimitiveContextValue::Int(123))])
        );

        Ok(())
    }

    #[test]
    fn deserialize_non_string_prims() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            deserialize_context_value(r#"123"#, None)?,
            ContextValue::Prim(PrimitiveContextValue::Int(123))
        );

        Ok(())
    }

    #[test]
    fn deserialize_invalid_ast_as_str() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            deserialize_context_value(r#""123""#, None)?,
            ContextValue::Prim(PrimitiveContextValue::String("123".to_string()))
        );

        Ok(())
    }

    #[test]
    fn deserialize_prims() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            deserialize_context_value(r#"true"#, None)?,
            ContextValue::Prim(PrimitiveContextValue::Bool(true))
        );
        assert_eq!(
            deserialize_context_value(r#"123"#, None)?,
            ContextValue::Prim(PrimitiveContextValue::Int(123))
        );
        assert_eq!(
            deserialize_context_value(r#"3.13"#, None)?,
            ContextValue::Prim(PrimitiveContextValue::Float(3.13))
        );
        assert_eq!(
            deserialize_context_value(r#""hello""#, None)?,
            ContextValue::Prim(PrimitiveContextValue::String("hello".to_string()))
        );
        assert_eq!(
            deserialize_context_value(r#"null"#, None)?,
            ContextValue::Prim(PrimitiveContextValue::Null)
        );

        Ok(())
    }

    #[test]
    fn deserialize_complex_ast() -> Result<(), Box<dyn Error>> {
        let json = r#""Hello ${uppercase(str: username)}, today is ${today} and we are vibing.""#;
        assert_eq!(
            deserialize_context_value(json, None)?,
            ContextValue::StringTemplate(vec![
                ContextValue::Prim(PrimitiveContextValue::String("Hello ".to_string())),
                ContextValue::Function {
                    id: PublicIdentifier::build("std", "uppercase").unwrap(),
                    function: FN_UPPERCASE.clone(),
                    args: [(
                        "str".to_string(),
                        ContextValue::Ident("username".to_string())
                    )]
                    .into()
                },
                ContextValue::Prim(PrimitiveContextValue::String(", today is ".to_string())),
                ContextValue::Ident("today".to_string()),
                ContextValue::Prim(PrimitiveContextValue::String(
                    " and we are vibing.".to_string()
                )),
            ])
        );

        Ok(())
    }

    #[test]
    fn evaluate_context_lookup() {
        let ctx = Context {
            unit: ExecutionUnit {
                id: 0,
                lon: GeoDeg::from(15.222),
                lat: GeoDeg::from(-15.23133),
            },
            run: pgaf_sdk::config::RunConfig {
                name: String::from("r1"),
                template: PathBuf::from("dummy"),
                extra: [
                    ("foo".to_string(), PrimitiveContextValue::Int(123).into()),
                    ("bar".to_string(), ContextValue::Ident("foo".to_string())),
                    (
                        "baz".to_string(),
                        ContextValue::Ident("invalid".to_string()),
                    ),
                ]
                .into(),
            },
        };

        assert_eq!(
            ctx.run.extra.get("foo").unwrap().to_prim(&ctx).unwrap(),
            PrimitiveContextValue::Int(123),
        );

        assert_eq!(
            ctx.run.extra.get("bar").unwrap().to_prim(&ctx).unwrap(),
            PrimitiveContextValue::Int(123),
        );

        assert_eq!(
            ctx.run.extra.get("baz").unwrap().to_prim(&ctx).unwrap_err(),
            ContextEvaluationError::IdentifierNotFound("invalid".to_string())
        );
    }
}
