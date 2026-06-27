use std::collections::HashMap;

use pest::Parser;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Null,
    Ident(String),
    FunctionCall {
        name: String,
        args: HashMap<String, Expr>,
    },
    StringTemplate(Vec<Expr>),
}

#[derive(pest_derive::Parser)]
#[grammar = "processing/context/eval.pest"]
struct EvalParser;

impl From<pest::iterators::Pair<'_, Rule>> for Expr {
    fn from(pair: pest::iterators::Pair<Rule>) -> Self {
        match pair.as_rule() {
            Rule::expr => pair.into_inner().next().unwrap().into(),
            Rule::null_val => Expr::Null,
            Rule::bool_val => Expr::Bool(pair.as_str().parse().unwrap()),
            Rule::int_val => Expr::Int(pair.as_str().parse().unwrap()),
            Rule::float_val => Expr::Float(pair.as_str().parse().unwrap()),
            Rule::ident => Expr::Ident(pair.as_str().to_string()),
            Rule::string_template => {
                let pieces: Vec<Expr> = pair.into_inner().map(Into::into).collect();
                if pieces.len() == 1 {
                    if let Some(Expr::String(s)) = pieces.first() {
                        return Expr::String(s.clone());
                    }
                }

                Expr::StringTemplate(pieces)
            }
            Rule::literal_text => Expr::String(pair.as_str().to_string()),
            Rule::interpolation => pair.into_inner().next().unwrap().into(),
            Rule::function_call => {
                let mut inner_rules = pair.into_inner();
                let name = inner_rules.next().unwrap().as_str().to_string();
                let mut args = HashMap::<String, Expr>::new();

                if let Some(args_pair) = inner_rules.next() {
                    for arg_pair in args_pair.into_inner() {
                        let mut arg_inner = arg_pair.into_inner();
                        let arg_name = arg_inner.next().unwrap().as_str().to_string();
                        let arg_value = arg_inner.next().unwrap().into();
                        args.insert(arg_name, arg_value);
                    }
                }

                Expr::FunctionCall { name, args }
            }
            _ => unreachable!("Rule {:?} is not covered.", pair.as_rule()),
        }
    }
}

impl<'a> TryFrom<&'a str> for Expr {
    type Error = pest::error::Error<Rule>;

    fn try_from(input: &'a str) -> Result<Self, Self::Error> {
        let mut pairs = EvalParser::parse(Rule::main, input)?;
        let top_level = pairs.next().expect("top_level ast member not found.");

        let mut inner_rules = top_level.into_inner();

        if let Some(first_pair) = inner_rules.next() {
            match first_pair.as_rule() {
                Rule::eval_block => Ok(first_pair.into_inner().next().unwrap().into()),
                _ => {
                    let mut pieces: Vec<Expr> = vec![first_pair.into()];
                    pieces.extend(inner_rules.map(Into::into));

                    if pieces.len() == 1 {
                        if let Some(Expr::String(s)) = pieces.first() {
                            return Ok(Expr::String(s.clone()));
                        }
                    }

                    Ok(Expr::StringTemplate(pieces))
                }
            }
        } else {
            Ok(Expr::String("".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn test_parse_null() {
        let expr: Expr = "$={null}".try_into().unwrap();
        assert_eq!(expr, Expr::Null);
    }

    #[test]
    fn test_parse_booleans() {
        let true_expr: Expr = "$={true}".try_into().unwrap();
        assert_eq!(true_expr, Expr::Bool(true));

        let false_expr: Expr = "$={false}".try_into().unwrap();
        assert_eq!(false_expr, Expr::Bool(false));
    }

    #[test]
    fn test_parse_integers() {
        let pos_int: Expr = "$={12345}".try_into().unwrap();
        assert_eq!(pos_int, Expr::Int(12345));

        let neg_int: Expr = "$={-99}".try_into().unwrap();
        assert_eq!(neg_int, Expr::Int(-99));
    }

    #[test]
    fn test_parse_floats() {
        let pos_float: Expr = "$={12.34}".try_into().unwrap();
        assert_eq!(pos_float, Expr::Float(12.34));

        let neg_float: Expr = "$={-0.56}".try_into().unwrap();
        assert_eq!(neg_float, Expr::Float(-0.56));
    }

    #[test]
    fn test_parse_quoted_strings() {
        // Inside an eval block, single quotes mean a plain string literal
        let simple_str: Expr = "$={'hello_world'}".try_into().unwrap();
        assert_eq!(simple_str, Expr::String("hello_world".to_string()));
    }

    #[test]
    fn test_parse_identifiers() {
        let id_expr: Expr = "$={my_variable_2}".try_into().unwrap();
        assert_eq!(id_expr, Expr::Ident("my_variable_2".to_string()));
    }

    #[test]
    fn test_parse_empty_function_call() {
        let func_expr: Expr = "$={get_version()}".try_into().unwrap();
        assert_eq!(
            func_expr,
            Expr::FunctionCall {
                name: "get_version".to_string(),
                args: HashMap::new()
            }
        );
    }

    #[test]
    fn test_parse_function_with_args() {
        let func_expr: Expr = "$={sum(a: 10, b: plrs)}".try_into().unwrap();
        assert_eq!(
            func_expr,
            Expr::FunctionCall {
                name: "sum".to_string(),
                args: HashMap::from([
                    ("a".to_string(), Expr::Int(10)),
                    ("b".to_string(), Expr::Ident("plrs".to_string()))
                ])
            }
        );
    }

    #[test]
    fn test_whitespace_tolerance() {
        let loose_expr: Expr = "$={   sum( \n a : 5 , \t b : 'hi' \r )   }"
            .try_into()
            .unwrap();

        assert_eq!(
            loose_expr,
            Expr::FunctionCall {
                name: "sum".to_string(),
                args: HashMap::from([
                    ("a".to_string(), Expr::Int(5)),
                    ("b".to_string(), Expr::String("hi".to_string()))
                ])
            }
        );
    }

    #[test]
    fn test_fallback_pure_literal_string() {
        let literal: Expr = "Just plain text without metadata".try_into().unwrap();
        assert_eq!(
            literal,
            Expr::String("Just plain text without metadata".to_string())
        );

        let empty_literal: Expr = "".try_into().unwrap();
        assert_eq!(empty_literal, Expr::String("".to_string()));
    }

    #[test]
    fn test_top_level_string_template() {
        // String templates at the root level of a JSON string
        let template: Expr = "Coordinates: ${lon}, ${lat}".try_into().unwrap();
        assert_eq!(
            template,
            Expr::StringTemplate(vec![
                Expr::String("Coordinates: ".to_string()),
                Expr::Ident("lon".to_string()),
                Expr::String(", ".to_string()),
                Expr::Ident("lat".to_string())
            ])
        );
    }

    #[test]
    fn test_deeply_nested_template_functions() {
        let input = "$={make_greeting(who: '${get_name(id: userId)} ${lastName}')}";
        let expr: Expr = input.try_into().unwrap();

        assert_eq!(
            expr,
            Expr::FunctionCall {
                name: "make_greeting".to_string(),
                args: HashMap::from([(
                    "who".to_string(),
                    Expr::StringTemplate(vec![
                        Expr::FunctionCall {
                            name: "get_name".to_string(),
                            args: HashMap::from([(
                                "id".to_string(),
                                Expr::Ident("userId".to_string())
                            )])
                        },
                        Expr::String(" ".to_string()),
                        Expr::Ident("lastName".to_string())
                    ])
                )])
            }
        );
    }

    #[test]
    fn test_syntax_errors() {
        // Unclosed explicit evaluation brackets
        // Since the parser can't parse the entrypoint, it fallsback as a String literal.
        let input = String::from("$={sum(a: 5)");
        let result: Result<Expr, _> = input.as_str().try_into();
        assert_eq!(result, Ok(Expr::String(input)));

        // Unclosed function arguments parenthesis
        // Since the parser can't parse the entrypoint, it fallsback as a String literal.
        let input = String::from("$={sum(a: 5}");
        let result: Result<Expr, _> = input.as_str().try_into();
        assert_eq!(result, Ok(Expr::String(input)));

        // Invalid identifier format (cannot start with a number)
        // Since the parser can't parse the entrypoint, it fallsback as a String literal.
        let input = String::from("$={1st_val}");
        let result: Result<Expr, _> = input.as_str().try_into();
        assert_eq!(result, Ok(Expr::String(input)));

        // Unclosed single-quoted string template inside evaluation
        let result: Result<Expr, _> = "$={concat(str: 'hello)}".try_into();
        assert!(result.is_err());
    }
}
