use std::{
    ops::Range,
    path::{Path, PathBuf},
};

use path_slash::PathBufExt;
use winnow::{
    LocatingSlice, ModalResult, Parser,
    combinator::{alt, eof, opt, repeat_till},
    error::StrContext,
    stream::Location,
    token::literal,
};

use crate::{
    AstBlockItem, AstEntityItem, AstNode, AstProperty, Operator, expression, script_value,
    with_opt_trailing_ws, ws_and_comments,
};

use anyhow::anyhow;

#[derive(Debug, PartialEq)]
pub struct AstModule<'a> {
    pub filename: String,
    pub namespace: String,
    pub items: Vec<AstEntityItem<'a>>,
    pub span: Range<usize>,
}

impl<'a> AstModule<'a> {
    pub fn new(namespace: &str, module_name: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
            filename: module_name.to_string(),
            items: Vec::new(),
            span: 0..0,
        }
    }

    pub fn parse_input(&'a mut self, input: &'a str) -> Result<(), anyhow::Error> {
        let mut input = LocatingSlice::new(input);

        let (items, span) = module(&mut input, &self.filename)
            .map_err(|e| anyhow!("Failed to parse module {}: {}", self.filename, e))?;

        self.items = items;
        self.span = span;

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
    module_name: &'a str,
) -> ModalResult<(Vec<AstEntityItem<'a>>, Range<usize>)> {
    if module_name.contains("99_README") {
        return Ok((Vec::new(), 0..0));
    }

    let start = input.current_token_start();

    opt(literal("\u{feff}")).parse_next(input)?;
    opt(ws_and_comments)
        .context(StrContext::Label("module start whitespace"))
        .parse_next(input)?;

    let (expressions, _): (Vec<AstBlockItem>, _) = repeat_till(
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
    .context(StrContext::Label("module"))
    .parse_next(input)?;

    let mut items = Vec::new();

    let span = start..input.current_token_start();

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
