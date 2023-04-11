use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::cw_model::Operator;
use nom::branch::alt;
use nom::bytes::complete::{escaped, is_not};
use nom::character::complete::{char, digit1, multispace1, one_of};
use nom::combinator::{cut, eof, map, map_res, opt, peek, recognize, value};
use nom::error::{FromExternalError, ParseError};
use nom::multi::{many0_count, many1_count, many_till};
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

impl<'a> ParserError<&'a str> for ErrorTree<&'a str> {}

impl<'a> ParserError<&'a str> for nom::error::VerboseError<&'a str> {}

impl<'a> ParserError<&'a str> for nom::error::Error<&'a str> {}

pub trait ParserError<I>:
    ParseError<I>
    + ContextError<I, &'static str>
    + TagError<I, &'static str>
    + FromExternalError<I, <f32 as FromStr>::Err>
    + FromExternalError<I, anyhow::Error>
    + std::fmt::Debug
{
}

#[derive(PartialEq, Eq, Debug)]
pub struct ParsedEntity<'a> {
    /// Array items in the entity, like { a b c }
    pub items: Vec<ParsedValue<'a>>,

    /// Key value pairs in the entity, like { a = b } or { a > b }
    pub properties: ParsedProperties<'a>,

    /// Conditional blocks in the entity, like [[CONDITION] { a b c }]
    pub conditional_blocks: HashMap<&'a str, ParsedConditionalBlock<'a>>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct ParsedProperties<'a> {
    pub kv: HashMap<&'a str, ParsedPropertyInfoList<'a>>,
    pub is_module: bool,
}

/// Info about the value of an entity's property. The property info contains the "= b" part of "a = b".
#[derive(PartialEq, Eq, Debug)]
pub struct ParsedPropertyInfo<'a> {
    pub operator: Operator,
    pub value: ParsedValue<'a>,
}

/// Since a property can have multiple values, we have to store them in a list.
/// For example, for an entity { key = value1 key = value2 }, "key" would have two property info items.
#[derive(PartialEq, Eq, Debug)]
pub struct ParsedPropertyInfoList<'a>(pub Vec<ParsedPropertyInfo<'a>>);

/// A value is anything after an =
#[derive(PartialEq, Eq, Debug)]
pub enum ParsedValue<'a> {
    String(&'a str),
    Number(&'a str),
    Boolean(bool),
    Entity(ParsedEntity<'a>),
    Color((&'a str, &'a str, &'a str, &'a str, Option<&'a str>)),
    Maths(&'a str),
}

/// A conditional block looks like [[PARAM_NAME] key = value] and is dumb
#[derive(Debug, PartialEq, Eq)]
pub struct ParsedConditionalBlock<'a> {
    pub key: (bool, &'a str),
    pub items: Vec<ParsedValue<'a>>,
    pub properties: ParsedProperties<'a>,
}

#[derive(Debug)]
struct ParsedExpression<'a> {
    key: &'a str,
    operator: Operator,
    value: ParsedValue<'a>,
}

enum ParsedBlockItem<'a> {
    Expression(ParsedExpression<'a>),
    ArrayItem(ParsedValue<'a>),
    Conditional(ParsedConditionalBlock<'a>),
}

#[derive(Debug, PartialEq)]
pub struct ParsedModule<'a> {
    pub filename: String,
    pub namespace: String,
    pub properties: ParsedProperties<'a>,
    pub values: Vec<ParsedValue<'a>>,
}

impl<'a> From<bool> for ParsedValue<'a> {
    fn from(v: bool) -> Self {
        Self::Boolean(v)
    }
}

impl<'a> From<ParsedEntity<'a>> for ParsedValue<'a> {
    fn from(v: ParsedEntity<'a>) -> Self {
        Self::Entity(v)
    }
}

impl<'a> From<ParsedValue<'a>> for ParsedPropertyInfoList<'a> {
    fn from(v: ParsedValue<'a>) -> Self {
        Self(vec![v.into()])
    }
}

impl<'a> From<ParsedValue<'a>> for ParsedPropertyInfo<'a> {
    fn from(v: ParsedValue<'a>) -> Self {
        Self {
            operator: Operator::Equals,
            value: v,
        }
    }
}

impl<'a> From<ParsedEntity<'a>> for ParsedPropertyInfo<'a> {
    fn from(v: ParsedEntity<'a>) -> Self {
        ParsedValue::Entity(v).into()
    }
}

impl<'a> From<ParsedEntity<'a>> for ParsedPropertyInfoList<'a> {
    fn from(e: ParsedEntity<'a>) -> Self {
        ParsedValue::Entity(e).into()
    }
}

impl<'a> From<ParsedPropertyInfoList<'a>> for Vec<ParsedPropertyInfo<'a>> {
    fn from(list: ParsedPropertyInfoList<'a>) -> Self {
        list.0
    }
}

impl<'a> ParsedPropertyInfoList<'a> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_property(mut self, operator: Operator, value: ParsedValue<'a>) -> Self {
        self.push(ParsedPropertyInfo { operator, value });
        self
    }

    pub fn push(&mut self, property: ParsedPropertyInfo<'a>) {
        self.0.push(property);
    }

    pub fn iter(&self) -> std::slice::Iter<ParsedPropertyInfo<'a>> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn into_vec(self) -> Vec<ParsedPropertyInfo<'a>> {
        self.0
    }

    pub fn retain(&mut self, f: impl Fn(&ParsedPropertyInfo<'a>) -> bool) {
        self.0.retain(f);
    }

    pub fn extend(&mut self, other: Vec<ParsedPropertyInfo<'a>>) {
        self.0.extend(other);
    }
}

impl<'a> ParsedEntity<'a> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            properties: ParsedProperties {
                kv: HashMap::new(),
                is_module: false,
            },
            conditional_blocks: HashMap::new(),
        }
    }

    pub fn with_property(mut self, key: &'a str, value: ParsedValue<'a>) -> Self {
        self.properties
            .kv
            .entry(key)
            .or_insert_with(ParsedPropertyInfoList::new)
            .0
            .push(ParsedPropertyInfo {
                operator: Operator::Equals,
                value,
            });
        self
    }

    pub fn with_property_values<I: IntoIterator<Item = ParsedValue<'a>>>(
        mut self,
        key: &'a str,
        values: I,
    ) -> Self {
        let items = self
            .properties
            .kv
            .entry(key)
            .or_insert_with(ParsedPropertyInfoList::new);
        for value in values {
            items.push(ParsedPropertyInfo {
                operator: Operator::Equals,
                value,
            });
        }
        self
    }

    pub fn with_property_with_operator(
        mut self,
        key: &'a str,
        operator: Operator,
        value: ParsedValue<'a>,
    ) -> Self {
        self.properties
            .kv
            .entry(key)
            .or_insert_with(ParsedPropertyInfoList::new)
            .0
            .push(ParsedPropertyInfo { operator, value });
        self
    }

    pub fn with_item(mut self, value: ParsedValue<'a>) -> Self {
        self.items.push(value);
        self
    }

    pub fn with_conditional(mut self, value: ParsedConditionalBlock<'a>) -> Self {
        self.conditional_blocks.insert(value.key.1, value);
        self
    }
}

/// A module for most intents and purposes is just an entity.
pub fn module<'a, E: ParserError<&'a str>>(
    input: &'a str,
    module_name: &'a str,
) -> IResult<&'a str, (ParsedProperties<'a>, Vec<ParsedValue<'a>>), E> {
    let original_input = input;
    let orig_slice: &'a str = &original_input;

    if module_name.contains("99_README") {
        return Ok((
            "",
            (
                ParsedProperties {
                    kv: HashMap::new(),
                    is_module: true,
                },
                Vec::new(),
            ),
        ));
    }

    let (input, _) = opt(tag("\u{feff}"))(orig_slice)?;
    let (input, _) = opt(ws_and_comments)
        .context("module start whitespace")
        .parse(input)?;
    let (input, (expressions, _)) = many_till(
        alt((
            map(
                with_opt_trailing_ws(expression),
                ParsedBlockItem::Expression,
            )
            .context("module expression"),
            map(
                with_opt_trailing_ws(script_value),
                ParsedBlockItem::ArrayItem,
            )
            .context("module script value"),
        )),
        eof,
    )
    .context("module")
    .parse(input)?;

    let mut properties = ParsedProperties {
        kv: HashMap::new(),
        is_module: true,
    };
    let mut values = Vec::new();

    for expression_or_value in expressions {
        match expression_or_value {
            ParsedBlockItem::Expression(expression) => {
                if expression.operator == Operator::Equals {
                    let items = properties
                        .kv
                        .entry(expression.key.clone())
                        .or_insert(ParsedPropertyInfoList::new());
                    items.push(ParsedPropertyInfo {
                        value: expression.value,
                        operator: expression.operator,
                    });
                }
            }
            ParsedBlockItem::ArrayItem(item) => {
                values.push(item);
            }
            ParsedBlockItem::Conditional(_) => {}
        }
    }

    Ok((input, (properties, values)))
}

impl<'a> ParsedModule<'a> {
    pub fn new(namespace: &str, module_name: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            filename: module_name.to_string(),
            properties: ParsedProperties {
                kv: HashMap::new(),
                is_module: true,
            },
            values: Vec::new(),
        }
    }

    pub fn parse_input(&'a mut self, input: &'a str) -> Result<(), anyhow::Error> {
        let (properties, values) = module::<nom::error::Error<_>>(&input, &self.filename)
            .map(|(_, module)| module)
            .map_err(|e| anyhow!(e.to_string()))?;

        self.properties = properties;
        self.values = values;

        Ok(())
    }

    pub fn get_module_info(file_path: &Path) -> (String, String) {
        let path = PathBuf::from(file_path);
        let mut namespace = String::new();
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
                namespace = cur_path
                    .strip_prefix(common_prefix)
                    .unwrap()
                    .parent()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                cur_path = cur_path.strip_prefix(common_prefix).unwrap().to_path_buf();
            }
        }

        namespace = ["common", &namespace]
            .iter()
            .collect::<PathBuf>()
            .to_slash_lossy()
            .to_string();

        let module_name = path.file_stem().unwrap().to_str().unwrap();

        (namespace, module_name.to_string())
    }
}

fn expression<'a, E: ParserError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, ParsedExpression<'a>, E> {
    let (input, key) = with_opt_trailing_ws(quoted_or_unquoted_string)
        .context("key")
        .parse(input)?;
    let (input, op) = map_res(with_opt_trailing_ws(operator), Operator::from_str)
        .context("operator")
        .parse(input)?;
    let (input, value) = cut(script_value).context("expression value").parse(input)?;

    Ok((
        input,
        ParsedExpression {
            key: key,
            operator: op,
            value,
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

fn entity<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, ParsedEntity<'a>, E> {
    let (input, _) = with_opt_trailing_ws(char('{'))
        .context("opening bracket")
        .parse(input)?;

    let (input, (expressions, _)) = cut(many_till(
        alt((
            with_opt_trailing_ws(map(expression, ParsedBlockItem::Expression))
                .context("expression entity item"),
            with_opt_trailing_ws(map(script_value, ParsedBlockItem::ArrayItem))
                .context("array item entity item"),
            with_opt_trailing_ws(map(conditional_block, ParsedBlockItem::Conditional))
                .context("conditional block entity item"),
        )),
        char('}').context("closing bracket"),
    ))
    .context("expression")
    .parse(input)?;

    let mut items = vec![];
    let mut properties = HashMap::new();
    let mut conditional_blocks = HashMap::new();

    for expression in expressions {
        match expression {
            ParsedBlockItem::Expression(expression) => {
                let items = properties
                    .entry(expression.key)
                    .or_insert(ParsedPropertyInfoList::new());
                items.push(ParsedPropertyInfo {
                    value: expression.value,
                    operator: expression.operator,
                });
            }
            ParsedBlockItem::ArrayItem(value) => {
                items.push(value);
            }
            ParsedBlockItem::Conditional(conditional_block) => {
                conditional_blocks.insert(conditional_block.key.1, conditional_block);
            }
        }
    }

    Ok((
        input,
        ParsedEntity {
            properties: ParsedProperties {
                kv: properties,
                is_module: false,
            },
            items,
            conditional_blocks,
        },
    ))
}

fn conditional_block<'a, E: ParserError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, ParsedConditionalBlock<'a>, E> {
    let (input, _) = with_opt_trailing_ws(tag("[["))(input)?;
    let (input, is_not) = opt(with_opt_trailing_ws(tag("!")))(input)?;
    let (input, key) = with_opt_trailing_ws(quoted_or_unquoted_string)(input)?;
    let (input, _) = with_opt_trailing_ws(char(']'))(input)?;

    let (input, (expressions, _)) = cut(many_till(
        alt((
            with_opt_trailing_ws(map(expression, ParsedBlockItem::Expression))
                .context("expression conditional item"),
            with_opt_trailing_ws(map(script_value, ParsedBlockItem::ArrayItem))
                .context("array item conditional item"),
        )),
        char(']').context("closing bracket"),
    ))
    .context("expression")
    .parse(input)?;

    let mut items = vec![];
    let mut properties = HashMap::new();

    for expression in expressions {
        match expression {
            ParsedBlockItem::Expression(expression) => {
                let items = properties
                    .entry(expression.key)
                    .or_insert(ParsedPropertyInfoList::new());
                items.push(ParsedPropertyInfo {
                    value: expression.value,
                    operator: expression.operator,
                });
            }
            ParsedBlockItem::ArrayItem(value) => {
                items.push(value);
            }
            // Nested conditionals possible???
            ParsedBlockItem::Conditional(_) => {}
        }
    }

    Ok((
        input,
        ParsedConditionalBlock {
            properties: ParsedProperties {
                kv: properties,
                is_module: false,
            },
            items,
            key: (is_not.is_some(), key),
        },
    ))
}

/// A color is either rgb { r g b a } or hsv { h s v a }. The a component is optional.
fn color<'a, E: ParserError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (&'a str, &'a str, &'a str, &'a str, Option<&'a str>), E> {
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

    Ok((input, (color_type, r, g, b, a)))
}

/// A number is a sequence of digits, optionally preceded by a sign and optionally followed by a decimal point and more digits, followed by whitespace.
fn number_val<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    let (input, v) = recognize(value(
        (),
        tuple((
            opt(alt((char('-'), char('+')))),
            digit1,
            opt(pair(char('.'), digit1)),
        )),
    ))
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
) -> IResult<&'a str, ParsedValue<'a>, E> {
    alt((
        map(color, ParsedValue::Color),
        map(entity, ParsedValue::Entity),
        map(number_val, ParsedValue::Number),
        map(quoted_or_unquoted_string, |v| ParsedValue::String(v)),
        map(inline_maths, |v| ParsedValue::Maths(v)),
    ))
    .context("script_value")
    .parse(input)
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
    value(
        (), // Output is thrown away.
        many1_count(alt((value((), multispace1), comment))),
    )
    .context("ws_and_comments")
    .parse(i)
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
    one_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789-$@")(input)
}

/// An unquoted string (i.e. identifier) - a sequence of valid identifier characters, spaces not allowed.
fn unquoted_string<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, &str, E> {
    terminated_value(recognize(pair(
        valid_identifier_start_char,
        many0_count(valid_identifier_char),
    )))
    .context("unquoted_string")
    .parse(input)
}

/// A string that is quoted with double quotes. Allows spaces and other characters that would otherwise be invalid in an unquoted string.
fn quoted_string<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, &str, E> {
    terminated_value(delimited(
        char('"'),
        recognize(many0_count(escaped(is_not("\\\""), '\\', is_not("\"")))),
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
    alt((ws_and_comments, value((), one_of("}=]")), value((), eof))).parse(input)
}

/// Insanity, inline math like @[x + 1], we don't really care about the formula inside, just that it's there.
fn inline_maths<'a, E: ParserError<&'a str>>(input: &'a str) -> IResult<&'a str, &str, E> {
    recognize(tuple((
        with_opt_trailing_ws(alt((tag("@["), tag("@\\[")))),
        many_till(is_not("]"), tag("]")),
    )))
    .context("inline_maths")
    .parse(input)
}
