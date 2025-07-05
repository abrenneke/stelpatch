use std::ops::Range;

use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, eof, opt, repeat_till},
    error::StrContext,
    token::literal,
};

use crate::{
    AstBlockItem, AstComment, AstEntityItem, AstExpression, AstNode, Operator, expression,
    get_comments, opt_ws_and_comments, script_value, with_opt_trailing_ws, ws_and_comments,
};

use anyhow::anyhow;
use self_cell::self_cell;

#[derive(Debug, PartialEq)]
pub struct AstModule<'a> {
    pub items: Vec<AstEntityItem<'a>>,
    pub span: Range<usize>,

    pub leading_comments: Vec<AstComment<'a>>,
    pub trailing_comments: Vec<AstComment<'a>>,
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

impl Clone for AstModuleCell {
    fn clone(&self) -> Self {
        let owner_clone = self.borrow_owner().to_owned();
        Self::from_input(owner_clone) // Have to re-parse on clone because the original AST points at the original string, can't just change all the references
    }
}

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
            leading_comments: vec![],
            trailing_comments: vec![],
        }
    }

    pub fn from_input(input: &'a str) -> Result<Self, anyhow::Error> {
        let mut module = Self::new();
        module.parse_input(input)?;
        Ok(module)
    }

    pub fn parse_input(&mut self, input: &'a str) -> Result<(), anyhow::Error> {
        let input = LocatingSlice::new(input);

        let module = module
            .parse(input)
            .map_err(|e| anyhow!("Failed to parse module: {}", e))?;

        self.items = module.items;
        self.span = module.span;
        self.leading_comments = module.leading_comments;
        self.trailing_comments = module.trailing_comments;

        Ok(())
    }

    /// Find all properties with the given key name
    pub fn find_properties(&self, key: &str) -> Vec<&AstExpression<'a>> {
        self.items
            .iter()
            .filter_map(|item| match item {
                AstEntityItem::Expression(prop) if prop.key.raw_value() == key => Some(prop),
                _ => None,
            })
            .collect()
    }

    /// Find the first property with the given key name
    pub fn find_property(&self, key: &str) -> Option<&AstExpression<'a>> {
        self.items.iter().find_map(|item| match item {
            AstEntityItem::Expression(prop) if prop.key.raw_value() == key => Some(prop),
            _ => None,
        })
    }

    /// Get all properties in the module
    pub fn properties(&self) -> impl Iterator<Item = &AstExpression<'a>> {
        self.items.iter().filter_map(|item| match item {
            AstEntityItem::Expression(prop) => Some(prop),
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

impl<'a> AstNode<'a> for AstModule<'a> {
    fn span_range(&self) -> Range<usize> {
        self.span.clone()
    }

    fn leading_comments(&self) -> &[AstComment<'a>] {
        &self.leading_comments
    }

    fn trailing_comment(&self) -> Option<&AstComment<'a>> {
        self.trailing_comments.last()
    }
}

/// A module for most intents and purposes is just an entity.
pub fn module<'a>(input: &mut LocatingSlice<&'a str>) -> ModalResult<AstModule<'a>> {
    opt(literal("\u{feff}")).parse_next(input)?;

    let leading_comments = opt_ws_and_comments(input)?;

    let ((expressions, _), span): ((Vec<AstBlockItem>, _), _) = repeat_till(
        0..,
        alt((
            expression
                .map(AstBlockItem::Expression)
                .context(StrContext::Label("module expression")),
            script_value
                .map(AstBlockItem::ArrayItem)
                .context(StrContext::Label("module script value")),
            ws_and_comments
                .map(AstBlockItem::Whitespace)
                .context(StrContext::Label("module whitespace")),
        )),
        eof,
    )
    .with_span()
    .context(StrContext::Label("module"))
    .parse_next(input)?;

    let mut items = Vec::new();

    let span = 0..span.end;

    let mut trailing_comments = Vec::new();

    for expression_or_value in expressions {
        match expression_or_value {
            AstBlockItem::Expression(expression) => {
                if expression.operator.operator == Operator::Equals {
                    items.push(AstEntityItem::Expression(AstExpression::new(
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
            AstBlockItem::Whitespace(whitespace) => {
                // For now... any out of place comments are just added to the end of the module
                trailing_comments.extend(whitespace.iter().map(|w| w.clone()));
            }
        }
    }

    Ok(AstModule {
        items,
        span,
        leading_comments: get_comments(&leading_comments),
        trailing_comments: get_comments(&trailing_comments),
    })
}
