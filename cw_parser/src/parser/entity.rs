use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, cut_err, repeat_till},
    error::StrContext,
};

use crate::{
    AstBlockItem, AstConditionalBlock, AstOperator, AstProperty, AstString, conditional_block,
    expression, script_value, with_opt_trailing_ws,
};

use super::AstValue;

#[derive(PartialEq, Eq, Debug)]
pub struct AstEntity<'a> {
    pub items: Vec<AstEntityItem<'a>>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum AstEntityItem<'a> {
    /// Key value pairs in the entity, like { a = b } or { a > b }
    Property(AstProperty<'a>),

    /// Array items in the entity, like { a b c }
    Item(AstValue<'a>),

    /// Conditional blocks in the entity, like [[CONDITION] { a b c }]
    Conditional(AstConditionalBlock<'a>),
}

impl<'a> AstEntity<'a> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn with_property(
        mut self,
        key: AstString<'a>,
        operator: AstOperator<'a>,
        value: AstValue<'a>,
    ) -> Self {
        self.items.push(AstEntityItem::Property(AstProperty::new(
            key, operator, value,
        )));
        self
    }

    pub fn with_item(mut self, item: AstValue<'a>) -> Self {
        self.items.push(AstEntityItem::Item(item));
        self
    }

    pub fn with_conditional_block(mut self, conditional_block: AstConditionalBlock<'a>) -> Self {
        self.items
            .push(AstEntityItem::Conditional(conditional_block));
        self
    }
}

pub(crate) fn entity<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstEntity<'a>> {
    with_opt_trailing_ws('{')
        .context(StrContext::Label("opening bracket"))
        .parse_next(input)?;

    let (expressions, _): (Vec<_>, _) = cut_err(repeat_till(
        0..,
        alt((
            with_opt_trailing_ws(expression.map(AstBlockItem::Expression))
                .context(StrContext::Label("expression entity item")),
            with_opt_trailing_ws(script_value.map(AstBlockItem::ArrayItem))
                .context(StrContext::Label("array item entity item")),
            with_opt_trailing_ws(conditional_block.map(AstBlockItem::Conditional))
                .context(StrContext::Label("conditional block entity item")),
        )),
        '}'.context(StrContext::Label("closing bracket")),
    ))
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
            AstBlockItem::Conditional(conditional_block) => {
                items.push(AstEntityItem::Conditional(conditional_block));
            }
        }
    }

    Ok(AstEntity { items })
}
