use super::Context;
use crate::processing::context::expr::Expr;
use crate::processing::PipelineData;
use crate::registry::Registries;
use serde::de::DeserializeSeed;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum PrimitiveContextValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Null,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum ContextValue {
    StringTemplate(Vec<ContextValue>),
    Prim(PrimitiveContextValue),
    Ident(String),
}

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

// [`registries`] and [`default_namespace`] params will be using in function resolution
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
        Expr::FunctionCall { name, args: _ } => {
            todo!("Function calls not supported yet (attempting to parse {name})")
        }
        Expr::Ident(v) => ContextValue::Ident(v),
    })
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ContextEvaluationError {
    #[error("Identifier '{0}' could not be resolved.")]
    IdentifierNotFound(String),
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
        }
    }
}

impl From<PrimitiveContextValue> for ContextValue {
    fn from(value: PrimitiveContextValue) -> Self {
        Self::Prim(value)
    }
}

impl PipelineData for Context {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GeoDeg;
    use crate::sites::Site;
    use std::error::Error;
    use std::path::PathBuf;

    fn deserialize_context_value(
        json: &str,
        seed: Option<ContextValueDeserializeSeed>,
    ) -> Result<ContextValue, Box<dyn Error>> {
        let registries = Registries::new();
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
        // TODO: add an inner function call formatDate(date: today)
        let json = r#""Hello ${username}, today is ${today} and we are vibing.""#;
        assert_eq!(
            deserialize_context_value(json, None)?,
            ContextValue::StringTemplate(vec![
                ContextValue::Prim(PrimitiveContextValue::String("Hello ".to_string())),
                ContextValue::Ident("username".to_string()),
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
            site: Site {
                id: 0,
                lon: GeoDeg::from(15.222),
                lat: GeoDeg::from(-15.23133),
            },
            run: crate::config::runs::RunConfig {
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
