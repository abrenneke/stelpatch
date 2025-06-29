use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, cut_err, repeat_till},
    error::StrContext,
};

use crate::{
    AstBlockItem, AstConditionalBlock, AstNode, AstOperator, AstProperty, AstString,
    conditional_block, expression, script_value, with_opt_trailing_ws,
};

use super::AstValue;

#[derive(PartialEq, Eq, Debug)]
pub struct AstEntity<'a> {
    pub items: Vec<AstEntityItem<'a>>,
    pub span: Range<usize>,
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
    pub fn new(span: Range<usize>) -> Self {
        Self {
            items: Vec::new(),
            span,
        }
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

    /// Find all properties with the given key name
    pub fn find_properties(&self, key: &str) -> Vec<&AstProperty<'a>> {
        self.items
            .iter()
            .filter_map(|item| match item {
                AstEntityItem::Property(prop) if prop.key.raw_value() == key => Some(prop),
                _ => None,
            })
            .collect()
    }

    /// Find the first property with the given key name
    pub fn find_property(&self, key: &str) -> Option<&AstProperty<'a>> {
        self.items.iter().find_map(|item| match item {
            AstEntityItem::Property(prop) if prop.key.raw_value() == key => Some(prop),
            _ => None,
        })
    }

    /// Get all properties in the entity
    pub fn properties(&self) -> impl Iterator<Item = &AstProperty<'a>> {
        self.items.iter().filter_map(|item| match item {
            AstEntityItem::Property(prop) => Some(prop),
            _ => None,
        })
    }

    /// Get all array items in the entity
    pub fn array_items(&self) -> impl Iterator<Item = &AstValue<'a>> {
        self.items.iter().filter_map(|item| match item {
            AstEntityItem::Item(value) => Some(value),
            _ => None,
        })
    }

    /// Get all conditional blocks in the entity
    pub fn conditional_blocks(&self) -> impl Iterator<Item = &AstConditionalBlock<'a>> {
        self.items.iter().filter_map(|item| match item {
            AstEntityItem::Conditional(cond) => Some(cond),
            _ => None,
        })
    }

    /// Check if the entity contains any items
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the number of items in the entity
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<'a> AstNode for AstEntity<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }
}

pub(crate) fn entity<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstEntity<'a>> {
    let start = with_opt_trailing_ws('{')
        .span()
        .context(StrContext::Label("opening bracket"))
        .parse_next(input)?;

    let ((expressions, _), span): ((Vec<_>, _), _) = cut_err(repeat_till(
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
    .with_span()
    .context(StrContext::Label("expression"))
    .parse_next(input)?;

    let span = start.start..span.end;

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

    Ok(AstEntity { items, span })
}
