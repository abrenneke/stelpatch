use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, jsonrpc::Result};

use super::cache::{
    get_entity_property_type, get_entity_property_type_from_ast, get_namespace_entity_type,
};
use super::document_cache::DocumentCache;
use super::utils::{extract_namespace_from_uri, position_to_offset};
use cw_parser::{AstEntity, AstExpression, AstNode, AstValue, AstVisitor};

/// A visitor that builds property paths for hover functionality
struct PropertyPathBuilder<'a, 'ast>
where
    'a: 'ast,
{
    position_offset: usize,
    current_path: Vec<String>,
    found_property: Option<String>,
    found_entity_context: Option<&'ast AstEntity<'a>>,
    original_input: &'a str,
}

impl<'a, 'ast> PropertyPathBuilder<'a, 'ast> {
    fn new(position_offset: usize, input: &'a str) -> Self {
        Self {
            position_offset,
            current_path: Vec::new(),
            found_property: None,
            found_entity_context: None,
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

impl<'a, 'ast> AstVisitor<'a, 'ast> for PropertyPathBuilder<'a, 'ast>
where
    'a: 'ast,
{
    fn visit_expression(&mut self, node: &'ast AstExpression<'a>) -> () {
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

    fn visit_entity(&mut self, node: &'ast AstEntity<'a>) -> () {
        // Check if we're looking for a property within this entity
        let entity_span = node.span(&self.original_input);
        if self.position_offset >= entity_span.start.offset
            && self.position_offset <= entity_span.end.offset
        {
            // Store this entity as context for type resolution
            if self.found_entity_context.is_none() {
                self.found_entity_context = Some(node);
            }
        }

        // Continue with normal entity walking
        self.walk_entity(node);
    }

    fn walk_entity(&mut self, node: &'ast AstEntity<'a>) -> () {
        for item in &node.items {
            self.visit_entity_item(item);
            if self.found_property.is_some() {
                break;
            }
        }
    }
}

pub fn hover(
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

    let documents = documents.read().expect("Failed to read documents");
    let content = match documents.get(&uri) {
        Some(content) => content,
        None => return Ok(None),
    };

    // Convert position to byte offset
    let offset = position_to_offset(content, position);

    // Use document cache to get parsed AST
    let cached_document = document_cache.get(&uri);

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
        // Extract namespace from URI to get type information
        let namespace = extract_namespace_from_uri(&uri);

        // Check if this is a top-level key (entity name) or a nested property
        let is_top_level_key = !property_path.contains('.');

        // Build the base hover content
        let mut hover_content = String::new();

        // Add type information if we can determine the namespace
        if let Some(namespace) = namespace {
            // Only try to get type info if the type cache is initialized
            if crate::handlers::cache::TypeCache::is_initialized() {
                let type_info = if is_top_level_key {
                    // For top-level keys, show the namespace type (the structure of entities in this namespace)
                    get_namespace_entity_type(&namespace, Some(&uri))
                } else {
                    // For nested properties, use AST-based type resolution if we have entity context
                    let property_parts: Vec<&str> = property_path.split('.').collect();
                    if property_parts.len() > 1 {
                        // Skip the first part (entity name) and join the rest
                        let actual_property_path = property_parts[1..].join(".");

                        // Try to use AST-based resolution if we have entity context
                        if let Some(entity_context) = builder.found_entity_context {
                            get_entity_property_type_from_ast(
                                &namespace,
                                entity_context,
                                &actual_property_path,
                                Some(&uri),
                            )
                        } else {
                            // Fallback to string-based resolution
                            get_entity_property_type(&namespace, &actual_property_path, Some(&uri))
                        }
                    } else {
                        None
                    }
                };

                if let Some(type_info) = type_info {
                    // Add type information in a clean format
                    hover_content.push_str(&format!("```\n{}\n```", type_info.type_description));

                    // Add brief documentation if available
                    if let Some(documentation) = &type_info.documentation {
                        if !documentation.trim().is_empty() {
                            hover_content.push_str(&format!("\n\n{}", documentation.trim()));
                        }
                    }

                    // Add source info if available and it indicates subtype narrowing was applied
                    if let Some(source_info) = &type_info.source_info {
                        if source_info.contains("subtype narrowing") {
                            hover_content.push_str(&format!("\n\n*{}*", source_info));
                        }
                    }
                }
            }
        }

        let hover = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_content,
            }),
            range: None, // We could calculate the exact range if needed
        };

        return Ok(Some(hover));
    }

    Ok(None)
}
