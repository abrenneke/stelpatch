use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, opt, repeat_till},
    error::StrContext,
    token::literal,
};

use crate::{
    AstBlockItem, AstEntityItem, AstProperty, AstString, AstValue, expression,
    quoted_or_unquoted_string, script_value, with_opt_trailing_ws,
};

/// A conditional block looks like [[PARAM_NAME] key = value] and is dumb
#[derive(Debug, PartialEq, Eq)]
pub struct AstConditionalBlock<'a> {
    pub is_not: bool,
    pub key: AstString<'a>,
    pub items: Vec<AstEntityItem<'a>>,
}

impl<'a> AstConditionalBlock<'a> {
    pub fn new(is_not: bool, key: AstString<'a>, items: Vec<AstEntityItem<'a>>) -> Self {
        Self { is_not, key, items }
    }
}

pub(crate) fn conditional_block<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<AstConditionalBlock<'a>> {
    with_opt_trailing_ws(literal("[[")).parse_next(input)?;
    let is_not = opt(with_opt_trailing_ws(literal("!"))).parse_next(input)?;
    let key = with_opt_trailing_ws(quoted_or_unquoted_string).parse_next(input)?;
    with_opt_trailing_ws(']').parse_next(input)?;

    let (expressions, _): (Vec<_>, _) = repeat_till(
        0..,
        alt((
            with_opt_trailing_ws(expression.map(AstBlockItem::Expression))
                .context(StrContext::Label("expression conditional item")),
            with_opt_trailing_ws(script_value.map(AstBlockItem::ArrayItem))
                .context(StrContext::Label("array item conditional item")),
        )),
        ']'.context(StrContext::Label("closing bracket")),
    )
    .context(StrContext::Label("expression"))
    .parse_next(input)?;

    let mut items = vec![];

    for expression in expressions {
        match expression {
            AstBlockItem::Expression(expression) => {
                items.push(AstEntityItem::Property(AstProperty::new(
                    expression.key,
                    expression.operator,
                    expression.value,
                )));
            }
            AstBlockItem::ArrayItem(value) => {
                items.push(AstEntityItem::Item(value));
            }
            // Nested conditionals possible???
            AstBlockItem::Conditional(_) => {}
        }
    }

    Ok(AstConditionalBlock {
        is_not: is_not.is_some(),
        items,
        key,
    })
}
