use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, cut_err, repeat_till},
    error::StrContext,
};

use crate::{
    AstBlockItem, AstComment, AstConditionalBlock, AstExpression, AstNode, AstOperator, AstString,
    conditional_block, expression, get_comments, opt_trailing_comment, opt_ws_and_comments,
    script_value,
};

use super::AstValue;

#[derive(PartialEq, Eq, Debug)]
pub struct AstEntity<'a> {
    pub items: Vec<AstEntityItem<'a>>,
    pub span: Range<usize>,

    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comment: Option<AstComment<'a>>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum AstEntityItem<'a> {
    /// Key value pairs in the entity, like { a = b } or { a > b }
    Expression(Box<AstExpression<'a>>),

    /// Array items in the entity, like { a b c }
    Item(Box<AstValue<'a>>),

    /// Conditional blocks in the entity, like [[CONDITION] { a b c }]
    Conditional(Box<AstConditionalBlock<'a>>),
}

impl<'a> AstEntity<'a> {
    pub fn new(span: Range<usize>) -> Self {
        Self {
            items: Vec::new(),
            span,
            leading_comments: vec![],
            trailing_comment: None,
        }
    }

    pub fn with_property(
        mut self,
        key: AstString<'a>,
        operator: AstOperator<'a>,
        value: AstValue<'a>,
    ) -> Self {
        self.items
            .push(AstEntityItem::Expression(Box::new(AstExpression::new(
                key, operator, value,
            ))));
        self
    }

    pub fn with_leading_comment(mut self, comment: AstComment<'a>) -> Self {
        self.leading_comments.push(comment);
        self
    }

    pub fn with_trailing_comment(mut self, comment: AstComment<'a>) -> Self {
        self.trailing_comment = Some(comment);
        self
    }

    pub fn with_item(mut self, item: AstValue<'a>) -> Self {
        self.items.push(AstEntityItem::Item(Box::new(item)));
        self
    }

    pub fn with_conditional_block(mut self, conditional_block: AstConditionalBlock<'a>) -> Self {
        self.items
            .push(AstEntityItem::Conditional(Box::new(conditional_block)));
        self
    }

    /// Find all properties with the given key name
    pub fn find_properties(&self, key: &str) -> Vec<&AstExpression<'a>> {
        self.items
            .iter()
            .filter_map(|item| match item {
                AstEntityItem::Expression(prop) if prop.key.raw_value() == key => {
                    Some(prop.as_ref())
                }
                _ => None,
            })
            .collect()
    }

    /// Find the first property with the given key name
    pub fn find_property(&self, key: &str) -> Option<&AstExpression<'a>> {
        self.items.iter().find_map(|item| match item {
            AstEntityItem::Expression(prop) if prop.key.raw_value() == key => Some(prop.as_ref()),
            _ => None,
        })
    }

    /// Get all properties in the entity
    pub fn properties(&self) -> impl Iterator<Item = &AstExpression<'a>> {
        self.items.iter().filter_map(|item| match item {
            AstEntityItem::Expression(prop) => Some(prop.as_ref()),
            _ => None,
        })
    }

    /// Get all array items in the entity
    pub fn array_items(&self) -> impl Iterator<Item = &AstValue<'a>> {
        self.items.iter().filter_map(|item| match item {
            AstEntityItem::Item(value) => Some(value.as_ref()),
            _ => None,
        })
    }

    /// Get all conditional blocks in the entity
    pub fn conditional_blocks(&self) -> impl Iterator<Item = &AstConditionalBlock<'a>> {
        self.items.iter().filter_map(|item| match item {
            AstEntityItem::Conditional(cond) => Some(cond.as_ref()),
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

impl<'a> AstNode<'a> for AstEntity<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &self.leading_comments
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        self.trailing_comment.as_ref()
    }
}

pub(crate) fn entity<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstEntity<'a>> {
    let leading_comments = opt_ws_and_comments.parse_next(input)?;

    let start = '{'.span().parse_next(input)?;

    let ((expressions, _), span): ((Vec<_>, _), _) = cut_err(repeat_till(
        0..,
        alt((
            expression
                .map(AstBlockItem::Expression)
                .context(StrContext::Label("expression entity item")),
            script_value
                .map(AstBlockItem::ArrayItem)
                .context(StrContext::Label("array item entity item")),
            conditional_block
                .map(AstBlockItem::Conditional)
                .context(StrContext::Label("conditional block entity item")),
        )),
        (opt_ws_and_comments, '}'),
    ))
    .with_span()
    .context(StrContext::Label("expression"))
    .parse_next(input)?;

    let mut trailing_comment = opt_trailing_comment.parse_next(input)?;

    let span = start.start..span.end;

    let mut items = vec![];

    for expression in expressions {
        match expression {
            AstBlockItem::Expression(expression) => {
                items.push(AstEntityItem::Expression(Box::new(AstExpression::new(
                    expression.key,
                    expression.operator,
                    expression.value,
                ))));
            }
            AstBlockItem::ArrayItem(value) => {
                items.push(AstEntityItem::Item(Box::new(value)));
            }
            AstBlockItem::Conditional(conditional_block) => {
                items.push(AstEntityItem::Conditional(Box::new(conditional_block)));
            }
            AstBlockItem::Whitespace(whitespace) => {
                // For now... out of place comments are ignored
            }
        }
    }

    Ok(AstEntity {
        items,
        span,
        leading_comments: get_comments(&leading_comments),
        trailing_comment,
    })
}
