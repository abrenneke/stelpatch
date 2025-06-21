use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::model::Operator;
use winnow::ascii::{digit1, escaped, multispace1, till_line_ending};
use winnow::combinator::{alt, cut_err, delimited, eof, opt, peek, repeat_till};
use winnow::error::{ErrMode, ParserError, StrContext};
use winnow::token::{none_of, one_of, take_till, take_while};

use winnow::combinator::repeat;

use anyhow::anyhow;
use path_slash::PathExt;
use winnow::token::literal;
use winnow::{ModalResult, Parser};

mod tests;
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

    pub fn iter(&self) -> std::slice::Iter<'_, ParsedPropertyInfo<'a>> {
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
pub fn module<'a>(
    input: &mut &'a str,
    module_name: &'a str,
) -> ModalResult<(ParsedProperties<'a>, Vec<ParsedValue<'a>>)> {
    if module_name.contains("99_README") {
        return Ok((
            ParsedProperties {
                kv: HashMap::new(),
                is_module: true,
            },
            Vec::new(),
        ));
    }

    opt(literal("\u{feff}")).parse_next(input)?;
    opt(ws_and_comments)
        .context(StrContext::Label("module start whitespace"))
        .parse_next(input)?;

    let (expressions, _): (Vec<ParsedBlockItem>, _) = repeat_till(
        0..,
        alt((
            with_opt_trailing_ws(expression)
                .map(ParsedBlockItem::Expression)
                .context(StrContext::Label("module expression")),
            with_opt_trailing_ws(script_value)
                .map(ParsedBlockItem::ArrayItem)
                .context(StrContext::Label("module script value")),
        )),
        eof,
    )
    .context(StrContext::Label("module"))
    .parse_next(input)?;

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
                        .entry(expression.key)
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

    Ok((properties, values))
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
        let mut input = input;

        let (properties, values) = module(&mut input, &self.filename)
            .map_err(|e| anyhow!("Failed to parse module {}: {}", self.filename, e))?;

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

fn expression<'a>(input: &mut &'a str) -> ModalResult<ParsedExpression<'a>> {
    let key = with_opt_trailing_ws(quoted_or_unquoted_string)
        .context(StrContext::Label("key"))
        .parse_next(input)?;
    let op = with_opt_trailing_ws(operator)
        .try_map(Operator::from_str)
        .context(StrContext::Label("operator"))
        .parse_next(input)?;
    let value = cut_err(script_value)
        .context(StrContext::Label("expression value"))
        .parse_next(input)?;

    Ok(ParsedExpression {
        key: key,
        operator: op,
        value,
    })
}

fn operator<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    alt((
        literal(">="),
        literal("<="),
        literal("!="),
        literal("+="),
        literal("-="),
        literal("*="),
        literal("="),
        literal(">"),
        literal("<"),
    ))
    .parse_next(input)
}

fn entity<'a>(input: &mut &'a str) -> ModalResult<ParsedEntity<'a>> {
    with_opt_trailing_ws('{')
        .context(StrContext::Label("opening bracket"))
        .parse_next(input)?;

    let (expressions, _): (Vec<_>, _) = cut_err(repeat_till(
        0..,
        alt((
            with_opt_trailing_ws(expression.map(ParsedBlockItem::Expression))
                .context(StrContext::Label("expression entity item")),
            with_opt_trailing_ws(script_value.map(ParsedBlockItem::ArrayItem))
                .context(StrContext::Label("array item entity item")),
            with_opt_trailing_ws(conditional_block.map(ParsedBlockItem::Conditional))
                .context(StrContext::Label("conditional block entity item")),
        )),
        '}'.context(StrContext::Label("closing bracket")),
    ))
    .context(StrContext::Label("expression"))
    .parse_next(input)?;

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

    Ok(ParsedEntity {
        properties: ParsedProperties {
            kv: properties,
            is_module: false,
        },
        items,
        conditional_blocks,
    })
}

fn conditional_block<'a>(input: &mut &'a str) -> ModalResult<ParsedConditionalBlock<'a>> {
    with_opt_trailing_ws(literal("[[")).parse_next(input)?;
    let is_not = opt(with_opt_trailing_ws(literal("!"))).parse_next(input)?;
    let key = with_opt_trailing_ws(quoted_or_unquoted_string).parse_next(input)?;
    with_opt_trailing_ws(']').parse_next(input)?;

    let (expressions, _): (Vec<_>, _) = repeat_till(
        0..,
        alt((
            with_opt_trailing_ws(expression.map(ParsedBlockItem::Expression))
                .context(StrContext::Label("expression conditional item")),
            with_opt_trailing_ws(script_value.map(ParsedBlockItem::ArrayItem))
                .context(StrContext::Label("array item conditional item")),
        )),
        ']'.context(StrContext::Label("closing bracket")),
    )
    .context(StrContext::Label("expression"))
    .parse_next(input)?;

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

    Ok(ParsedConditionalBlock {
        properties: ParsedProperties {
            kv: properties,
            is_module: false,
        },
        items,
        key: (is_not.is_some(), key),
    })
}

/// A color is either rgb { r g b a } or hsv { h s v a }. The a component is optional.
fn color<'a>(
    input: &mut &'a str,
) -> ModalResult<(&'a str, &'a str, &'a str, &'a str, Option<&'a str>)> {
    let color_type = with_opt_trailing_ws(alt((literal("rgb"), literal("hsv"))))
        .context(StrContext::Label("color type"))
        .parse_next(input)?;

    let (r, g, b, a) = delimited(
        with_opt_trailing_ws('{'),
        cut_err((
            with_trailing_ws(number_val).context(StrContext::Label("color a")),
            with_trailing_ws(number_val).context(StrContext::Label("color b")),
            with_opt_trailing_ws(number_val).context(StrContext::Label("color c")),
            opt(with_opt_trailing_ws(number_val)).context(StrContext::Label("color d")),
        )),
        '}',
    )
    .context(StrContext::Label("color tuple"))
    .parse_next(input)?;

    Ok((color_type, r, g, b, a))
}

/// A number is a sequence of digits, optionally preceded by a sign and optionally followed by a decimal point and more digits, followed by whitespace.
fn number_val<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    let v = (opt(alt(('-', '+'))), digit1, opt(('.', digit1)))
        .take()
        .context(StrContext::Label("number_val"))
        .parse_next(input)?;

    peek(value_terminator)
        .context(StrContext::Label("number_val terminator"))
        .parse_next(input)?;

    Ok(v)
}

fn script_value<'a>(input: &mut &'a str) -> ModalResult<ParsedValue<'a>> {
    alt((
        color.map(ParsedValue::Color),
        entity.map(ParsedValue::Entity),
        number_val.map(ParsedValue::Number),
        quoted_or_unquoted_string.map(|v| ParsedValue::String(v)),
        inline_maths.map(|v| ParsedValue::Maths(v)),
    ))
    .context(StrContext::Label("script_value"))
    .parse_next(input)
}

/// A combinator that consumes trailing whitespace and comments after the inner parser. If there is no trailing whitespace, the parser succeeds.
fn with_opt_trailing_ws<'a, F, O, E>(mut inner: F) -> impl winnow::ModalParser<&'a str, O, E>
where
    F: winnow::ModalParser<&'a str, O, E>,
    E: ParserError<&'a str>,
    ErrMode<E>: From<ErrMode<winnow::error::ContextError>>,
{
    move |input: &mut &'a str| {
        let value = inner.parse_next(input)?;
        opt(ws_and_comments).parse_next(input)?;
        Ok(value)
    }
}

/// A combinator that consumes trailing whitespace and comments after the inner parser.
fn with_trailing_ws<'a, F, E>(mut inner: F) -> impl winnow::ModalParser<&'a str, &'a str, E>
where
    F: winnow::ModalParser<&'a str, &'a str, E>,
    E: ParserError<&'a str>,
    ErrMode<E>: From<ErrMode<winnow::error::ContextError>>,
{
    move |input: &mut &'a str| {
        let value = inner.parse_next(input)?;
        ws_and_comments.parse_next(input)?;
        Ok(value)
    }
}

/// Matches any amount of whitespace and comments.
fn ws_and_comments<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    repeat(1.., alt((multispace1, comment)))
        .map(|()| ())
        .take()
        .context(StrContext::Label("ws_and_comments"))
        .parse_next(input)
}

/// Comments using #
fn comment<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    ("#", till_line_ending)
        .take()
        .context(StrContext::Label("comment"))
        .parse_next(input)
}

const VALID_IDENTIFIER_CHARS: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_:.@-|/$'";

const VALID_IDENTIFIER_START_CHARS: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789-$@";

/// An unquoted string (i.e. identifier) - a sequence of valid identifier characters, spaces not allowed.
fn unquoted_string<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    terminated_value(
        (
            one_of(VALID_IDENTIFIER_START_CHARS),
            take_while(0.., VALID_IDENTIFIER_CHARS),
        )
            .take(),
    )
    .context(StrContext::Label("unquoted_string"))
    .parse_next(input)
}

/// A string that is quoted with double quotes. Allows spaces and other characters that would otherwise be invalid in an unquoted string.
fn quoted_string<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    terminated_value(delimited(
        '"',
        escaped(none_of(['\\', '"']), '\\', "\"".value("\""))
            .map(|()| ())
            .take(),
        '"',
    ))
    .context(StrContext::Label("quoted_string"))
    .parse_next(input)
}

/// A string that is either quoted or unquoted.
fn quoted_or_unquoted_string<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    alt((quoted_string, unquoted_string))
        .context(StrContext::Label("quoted_or_unquoted_string"))
        .parse_next(input)
}

/// Combinator that peeks ahead to see if a value is terminated correctly. Values can terminate with a space, }, etc.
fn terminated_value<'a, F, O, E>(mut inner: F) -> impl winnow::ModalParser<&'a str, O, E>
where
    F: winnow::ModalParser<&'a str, O, E>,
    E: ParserError<&'a str>,
    ErrMode<E>: From<ErrMode<winnow::error::ContextError>>,
{
    move |input: &mut &'a str| {
        let value = inner.parse_next(input)?;
        peek(value_terminator).parse_next(input)?;
        Ok(value)
    }
}

/// Characters that can terminate a value, like whitespace, a comma, or a closing brace.
fn value_terminator<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    alt((ws_and_comments.void(), one_of(b"}=]").void(), eof.void()))
        .take()
        .parse_next(input)
}

/// Insanity, inline math like @[x + 1], we don't really care about the formula inside, just that it's there.
fn inline_maths<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    (
        with_opt_trailing_ws(alt((literal("@["), literal("@\\[")))),
        take_till(0.., ']'),
        ']',
    )
        .take()
        .context(StrContext::Label("inline_maths"))
        .parse_next(input)
}
