use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, jsonrpc::Result};

use super::document_cache::DocumentCache;
use super::utils::position_to_offset;
use cw_parser::{AstEntity, AstExpression, AstNode, AstValue, AstVisitor};

/// A visitor that builds property paths for hover functionality
struct PropertyPathBuilder<'a> {
    position_offset: usize,
    current_path: Vec<String>,
    found_property: Option<String>,
    original_input: &'a str,
}

impl<'a> PropertyPathBuilder<'a> {
    fn new(position_offset: usize, input: &'a str) -> Self {
        Self {
            position_offset,
            current_path: Vec::new(),
            found_property: None,
            original_input: input,
        }
    }

    fn with_path_segment<F>(&mut self, segment: &str, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.current_path.push(segment.to_string());
        f(self);
        self.current_path.pop();
    }

    fn build_path(&self) -> String {
        if self.current_path.is_empty() {
            return "root".to_string();
        }
        self.current_path.join(".")
    }
}

impl<'a> AstVisitor<'a> for PropertyPathBuilder<'a> {
    fn visit_expression(&mut self, node: &AstExpression<'a>) -> () {
        let key_span = node.key.span(&self.original_input);

        // Check if the position is within this property's key
        if self.position_offset >= key_span.start.offset
            && self.position_offset <= key_span.end.offset
        {
            let full_path = if self.current_path.is_empty() {
                node.key.raw_value().to_string()
            } else {
                format!("{}.{}", self.build_path(), node.key.raw_value())
            };
            self.found_property = Some(full_path);
            return;
        }

        // If we're not in the key, check if we're in the value and it's an entity
        if let AstValue::Entity(entity) = &node.value {
            let entity_span = entity.span(&self.original_input);
            if self.position_offset >= entity_span.start.offset
                && self.position_offset <= entity_span.end.offset
            {
                // We're inside this property's entity value, so add this property to the path
                self.with_path_segment(node.key.raw_value(), |builder| {
                    builder.visit_entity(entity);
                });
            }
        }
    }

    fn walk_entity(&mut self, node: &AstEntity<'a>) -> () {
        for item in &node.items {
            self.visit_entity_item(item);
            if self.found_property.is_some() {
                break;
            }
        }
    }
}

pub async fn hover(
    _client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    document_cache: &DocumentCache,
    params: HoverParams,
) -> Result<Option<Hover>> {
    let uri = params
        .text_document_position_params
        .text_document
        .uri
        .to_string();
    let position = params.text_document_position_params.position;

    let documents = documents.read().await;
    let content = match documents.get(&uri) {
        Some(content) => content,
        None => return Ok(None),
    };

    // Convert position to byte offset
    let offset = position_to_offset(content, position);

    // Use document cache to get parsed AST
    let cached_document = document_cache.get(&uri).await;

    let cached_document = match cached_document {
        Some(cached_document) => cached_document,
        None => return Ok(None),
    };

    // Find the property at the given position
    let mut builder = PropertyPathBuilder::new(offset, cached_document.borrow_input());

    if let Ok(ast) = cached_document.borrow_ast() {
        builder.visit_module(ast);
    } else {
        return Ok(None);
    }

    if let Some(property_path) = builder.found_property {
        // Create hover content
        let hover_text = format!("`{}`", property_path);

        let hover = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_text,
            }),
            range: None, // We could calculate the exact range if needed
        };

        return Ok(Some(hover));
    }

    Ok(None)
}
