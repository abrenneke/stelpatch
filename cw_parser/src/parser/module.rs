use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, eof, opt, repeat_till},
    error::StrContext,
    token::literal,
};

use crate::{
    AstBlockItem, AstEntityItem, AstNode, AstProperty, Operator, expression, script_value,
    with_opt_trailing_ws, ws_and_comments,
};

use anyhow::anyhow;
use self_cell::self_cell;

#[derive(Debug, PartialEq)]
pub struct AstModule<'a> {
    pub items: Vec<AstEntityItem<'a>>,
    pub span: Range<usize>,
}

pub type AstModuleResult<'a> = Result<AstModule<'a>, anyhow::Error>;

self_cell!(
    pub struct AstModuleCell {
        owner: String,

        #[covariant]
        dependent: AstModuleResult,
    }

    impl {Debug, PartialEq}
);

impl AstModuleCell {
    pub fn from_input(input: String) -> Self {
        Self::new(input, |input| {
            let mut module = AstModule::new();
            module.parse_input(&input).map(|_| module)
        })
    }
}

impl<'a> AstModule<'a> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            span: 0..0,
        }
    }

    pub fn from_input(input: &'a str) -> Result<Self, anyhow::Error> {
        let mut module = Self::new();
        module.parse_input(input)?;
        Ok(module)
    }

    pub fn parse_input(&mut self, input: &'a str) -> Result<(), anyhow::Error> {
        let mut input = LocatingSlice::new(input);

        let (items, span) =
            module(&mut input).map_err(|e| anyhow!("Failed to parse module: {}", e))?;

        self.items = items;
        self.span = span;

        Ok(())
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

    /// Get all properties in the module
    pub fn properties(&self) -> impl Iterator<Item = &AstProperty<'a>> {
        self.items.iter().filter_map(|item| match item {
            AstEntityItem::Property(prop) => Some(prop),
            _ => None,
        })
    }

    /// Get all array items in the module
    pub fn array_items(&self) -> impl Iterator<Item = &crate::AstValue<'a>> {
        self.items.iter().filter_map(|item| match item {
            AstEntityItem::Item(value) => Some(value),
            _ => None,
        })
    }

    /// Check if the module contains any items
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the number of items in the module
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<'a> AstNode for AstModule<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }
}

/// A module for most intents and purposes is just an entity.
pub fn module<'a>(
    input: &mut LocatingSlice<&'a str>,
) -> ModalResult<(Vec<AstEntityItem<'a>>, Range<usize>)> {
    opt(literal("\u{feff}")).parse_next(input)?;
    opt(ws_and_comments)
        .context(StrContext::Label("module start whitespace"))
        .parse_next(input)?;

    let ((expressions, _), span): ((Vec<AstBlockItem>, _), _) = repeat_till(
        0..,
        alt((
            with_opt_trailing_ws(expression)
                .map(AstBlockItem::Expression)
                .context(StrContext::Label("module expression")),
            with_opt_trailing_ws(script_value)
                .map(AstBlockItem::ArrayItem)
                .context(StrContext::Label("module script value")),
        )),
        eof,
    )
    .with_span()
    .context(StrContext::Label("module"))
    .parse_next(input)?;

    let mut items = Vec::new();

    let span = 0..span.end;

    for expression_or_value in expressions {
        match expression_or_value {
            AstBlockItem::Expression(expression) => {
                if expression.operator.operator == Operator::Equals {
                    items.push(AstEntityItem::Property(AstProperty::new(
                        expression.key,
                        expression.operator,
                        expression.value,
                    )));
                }
            }
            AstBlockItem::ArrayItem(item) => {
                items.push(AstEntityItem::Item(item));
            }
            AstBlockItem::Conditional(_) => {}
        }
    }

    Ok((items, span))
}
