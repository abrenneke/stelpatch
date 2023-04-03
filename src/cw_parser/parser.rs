use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use nom::branch::alt;
use nom::bytes::complete::is_not;
use nom::character::complete::{char, digit1, multispace1, one_of};
use nom::combinator::{cut, eof, map, map_res, opt, peek, recognize, value};
use nom::error::{FromExternalError, ParseError};
use nom::multi::{many0, many1, many_till};
use nom::sequence::{delimited, pair, tuple};
use nom::{IResult, Parser};
use nom_supreme::context::ContextError;
use nom_supreme::error::ErrorTree;
use nom_supreme::tag::complete::tag;
use nom_supreme::tag::TagError;
use nom_supreme::ParserExt;

use anyhow::anyhow;
use path_slash::PathExt;

mod tests;

use crate::cw_model::{self, PropertyInfoList};

impl<'a> ParserError<&'a str> for ErrorTree<&'a str> {}

pub trait ParserError<I>:
    ParseError<I>
    + ContextError<I, &'static str>
    + TagError<I, &'static str>
    + FromExternalError<I, <f32 as FromStr>::Err>
    + FromExternalError<I, anyhow::Error>
    + std::fmt::Debug
{
}

/// A module is like an entity but also supports defines. A module is a whole file.
fn parse_module<'a, E: ParserError<&'a str>>(
    input: &'a str,
    type_path: &str,
    module_name: &str,
) -> IResult<&'a str, cw_model::Module, E> {
    let (input, _) = opt(ws_and_comments)
        .context("module start whitespace")
        .parse(input)?;
    let (input, (expressions, _)) = many_till(
        alt((
            with_opt_trailing_ws(expression).context("module expression"),
            with_opt_trailing_ws(define).context("module define"),
        )),
        eof,
    )
    .context("module")
    .parse(input)?;

    let mut defines = HashMap::new();
    let mut entities = HashMap::new();
    let mut properties = HashMap::new();

    for expression in expressions {
        if expression.operator == cw_model::Operator::Equals {
            if expression.is_define {
                defines.insert(expression.key, expression.value);
            } else {
                if expression.value.is_entity() {
                    entities.insert(expression.key, expression.value);
                } else {
                    let items = properties
                        .entry(expression.key.clone())
                        .or_insert(PropertyInfoList::new());
                    items.push(cw_model::PropertyInfo {
                        value: expression.value,
                        operator: expression.operator,
                    });
                }
            }
        }
    }

    Ok((
        input,
        cw_model::Module {
            type_path: type_path.to_string(),
            filename: module_name.to_string(),
            entities,
            defines,
            properties,
        },
    ))
}

impl cw_model::Module {
    /// Parses a cw module from a string.
    pub fn parse(input: String, type_path: &str, module_name: &str) -> Result<Self, anyhow::Error> {
        parse_module::<ErrorTree<_>>(&input, type_path, module_name)
            .map(|(_, module)| module)
            .map_err(|e| anyhow!(e.to_string()))
    }

    /// Parses a cw module from a file.
    pub async fn parse_from_file(file_path: &str) -> Result<Self, anyhow::Error> {
        let path = PathBuf::from(file_path);
        let mut type_path = String::new();
        let mut cur_path = path.clone();

        while let Some(common_index) = cur_path
            .components()
            .position(|c| c.as_os_str() == "common")
        {
            if let Some(common_prefix) = cur_path
                .components()
                .take(common_index + 1)
                .collect::<PathBuf>()
                .to_str()
            {
                type_path = cur_path
                    .strip_prefix(common_prefix)
                    .unwrap()
                    .parent()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                cur_path = cur_path.strip_prefix(common_prefix).unwrap().to_path_buf();
            }
        }

        type_path = ["common", &type_path]
            .iter()
            .collect::<PathBuf>()
            .to_slash_lossy()
            .to_string();

        let module_name = path.file_stem().unwrap().to_str().unwrap();
        let input = tokio::fs::read_to_string(file_path).await?;
        cw_model::Module::parse(input, &type_path, module_name)
    }
}

impl FromStr for cw_model::Module {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        cw_model::Module::parse(input.to_string(), "", "")
    }
}

fn expression<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, Expression, E> {
    let (input, key) = with_opt_trailing_ws(quoted_or_unquoted_string)
        .context("key")
        .parse(input)?;
    let (input, op) = map_res(with_opt_trailing_ws(operator), cw_model::Operator::from_str)
        .context("operator")
        .parse(input)?;
    let (input, value) = cut(script_value).context("expression value").parse(input)?;

    Ok((
        input,
        Expression {
            key: key.to_string(),
            operator: op,
            value,
            is_define: false,
        },
    ))
}

fn operator<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, &str, E> {
    alt((
        tag(">="),
        tag("<="),
        tag("!="),
        tag("+="),
        tag("-="),
        tag("*="),
        tag("="),
        tag(">"),
        tag("<"),
    ))(input)
}

enum ExpressionOrArrayItem {
    Expression(Expression),
    ArrayItem(cw_model::Value),
}

fn entity<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, cw_model::Entity, E> {
    let (input, _) = with_opt_trailing_ws(char('{'))
        .context("opening bracket")
        .parse(input)?;

    let (input, (expressions, _)) = cut(many_till(
        alt((
            with_opt_trailing_ws(map(expression, ExpressionOrArrayItem::Expression))
                .context("expression entity item"),
            with_opt_trailing_ws(map(script_value, ExpressionOrArrayItem::ArrayItem))
                .context("array item entity item"),
        )),
        char('}').context("closing bracket"),
    ))
    .context("expression")
    .parse(input)?;

    let mut items = vec![];
    let mut properties = HashMap::new();

    for expression in expressions {
        match expression {
            ExpressionOrArrayItem::Expression(expression) => {
                let items = properties
                    .entry(expression.key)
                    .or_insert(PropertyInfoList::new());
                items.push(cw_model::PropertyInfo {
                    value: expression.value,
                    operator: expression.operator,
                });
            }
            ExpressionOrArrayItem::ArrayItem(value) => {
                items.push(value);
            }
        }
    }

    Ok((input, cw_model::Entity { properties, items }))
}

#[derive(Debug)]
struct Expression {
    is_define: bool,
    key: String,
    operator: cw_model::Operator,
    value: cw_model::Value,
}

/// A color is either rgb { r g b a } or hsv { h s v a }. The a component is optional.
fn color<'a, E: ParserError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (String, f32, f32, f32, Option<f32>), E> {
    let (input, color_type) = with_opt_trailing_ws(alt((tag("rgb"), tag("hsv"))))
        .context("color tag")
        .parse(input)?;

    let (input, (r, g, b, a)) = delimited(
        with_opt_trailing_ws(char('{')),
        cut(with_opt_trailing_ws(tuple((
            with_trailing_ws(number_val).context("color a"),
            with_trailing_ws(number_val).context("color b"),
            with_opt_trailing_ws(number_val).context("color c"),
            opt(number_val).context("color d"),
        ))))
        .context("color tuple"),
        char('}'),
    )
    .context("color")
    .parse(input)?;

    Ok((input, (color_type.to_string(), r, g, b, a)))
}

/// A number is a sequence of digits, optionally preceded by a sign and optionally followed by a decimal point and more digits, followed by whitespace.
fn number_val<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, f32, E> {
    let (input, v) = map_res(
        recognize(value(
            (),
            tuple((
                opt(alt((char('-'), char('+')))),
                digit1,
                opt(pair(char('.'), digit1)),
            )),
        )),
        |v: &str| v.parse::<f32>(),
    )
    .context("number")
    .parse(input)?;
    // Could be followed by whitespace or comment, or could close a block like 1.23}, but can't be like 1.23/ etc.
    let (input, _) = peek(value_terminator)
        .context("number_terminator")
        .parse(input)?;
    Ok((input, v))
}

fn script_value<'a, E: ParserError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, cw_model::Value, E> {
    alt((
        map(color, cw_model::Value::Color),
        map(entity, cw_model::Value::Entity),
        map(number_val, cw_model::Value::Number),
        map(quoted_or_unquoted_string, |v| {
            cw_model::Value::String(v.to_string())
        }),
    ))
    .context("script_value")
    .parse(input)
}

fn define_identifier<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, &str, E> {
    recognize(pair(char('@'), unquoted_string))
        .context("define_identifier")
        .parse(input)
}

fn define<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, Expression, E> {
    let (input, key) = with_opt_trailing_ws(define_identifier)(input)?; // @identifier_name
    let (input, _) = with_opt_trailing_ws(char('='))
        .context("define_equals")
        .parse(input)?;
    let (input, value) = script_value::<E>.context("define_value").parse(input)?;

    Ok((
        input,
        Expression {
            key: key.to_string(),
            operator: cw_model::Operator::Equals,
            value,
            is_define: true,
        },
    ))
}

/// A combinator that consumes trailing whitespace and comments after the inner parser. If there is no trailing whitespace, the parser succeeds.
fn with_opt_trailing_ws<'a, F, O, E>(mut inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
    E: ParserError<&'a str>,
{
    move |input| {
        let (input, value) = inner(input)?;
        let (input, _) = opt(ws_and_comments)(input)?;
        Ok((input, value))
    }
}

/// A combinator that consumes trailing whitespace and comments after the inner parser.
fn with_trailing_ws<'a, F, O, E>(mut inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
    E: ParserError<&'a str>,
{
    move |input| {
        let (input, value) = inner(input)?;
        let (input, _) = ws_and_comments(input)?;
        Ok((input, value))
    }
}

/// Matches any amount of whitespace and comments.
fn ws_and_comments<'a, E: ParserError<&'a str>>(i: &'a str) -> IResult<&'a str, (), E> {
    let (input, _) = value(
        (), // Output is thrown away.
        many1(alt((value((), multispace1), comment))),
    )
    .context("ws_and_comments")
    .parse(i)?;
    Ok((input, ()))
}

/// Comments using #
fn comment<'a, E: ParserError<&'a str>>(i: &'a str) -> IResult<&'a str, (), E> {
    let (input, _) = value(
        (), // Output is thrown away.
        pair(char('#'), opt(is_not("\n\r"))),
    )
    .context("comment")
    .parse(i)?;
    Ok((input, ()))
}

fn valid_identifier_char<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, char, E> {
    one_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_:.@-|/$'")(input)
}

fn valid_identifier_start_char<'a, E: ParserError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, char, E> {
    one_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789")(input)
}

/// An unquoted string (i.e. identifier) - a sequence of valid identifier characters, spaces not allowed.
fn unquoted_string<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, &str, E> {
    terminated_value(recognize(pair(
        valid_identifier_start_char,
        many0(valid_identifier_char),
    )))
    .context("unquoted_string")
    .parse(input)
}

/// A string that is quoted with double quotes. Allows spaces and other characters that would otherwise be invalid in an unquoted string.
fn quoted_string<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, &str, E> {
    terminated_value(delimited(
        char('"'),
        recognize(many1(alt((valid_identifier_char, char(' '))))),
        char('"'),
    ))
    .context("quoted_string")
    .parse(input)
}

/// A string that is either quoted or unquoted.
fn quoted_or_unquoted_string<'a, E: ParserError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, &str, E> {
    alt((quoted_string, unquoted_string))
        .context("quoted_or_unquoted_string")
        .parse(input)
}

/// Combinator that peeks ahead to see if a value is terminated correctly. Values can terminate with a space, }, etc.
fn terminated_value<'a, F, O, E>(mut inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
    E: ParserError<&'a str>,
{
    move |input| {
        let (input, value) = inner(input)?;
        let (input, _) = peek(value_terminator)(input)?;
        Ok((input, value))
    }
}

/// Characters that can terminate a value, like whitespace, a comma, or a closing brace.
fn value_terminator<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
    alt((ws_and_comments, value((), tag("}")), value((), eof))).parse(input)
}
