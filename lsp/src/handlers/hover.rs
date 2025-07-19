use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, jsonrpc::Result};

use crate::handlers::cache::types::TypeInfo;
use crate::handlers::cache::{
    EntityRestructurer, GameDataCache, TypeCache, TypeFormatter, get_entity_property_type_from_ast,
};

use super::document_cache::DocumentCache;
use super::scoped_type::PropertyNavigationResult;
use super::utils::{extract_namespace_from_uri, position_to_offset};
use cw_model::entity_from_module_ast;
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
    found_container_key: Option<String>,
    found_entity_key: Option<String>,
    original_input: &'a str,
}

impl<'a, 'ast> PropertyPathBuilder<'a, 'ast> {
    fn new(position_offset: usize, input: &'a str) -> Self {
        Self {
            position_offset,
            current_path: Vec::new(),
            found_property: None,
            found_entity_context: None,
            found_container_key: None,
            found_entity_key: None,
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

            // Set container and entity context based on the path level
            if self.current_path.is_empty() {
                // Top-level key - this is both container and entity key for normal entities
                self.found_container_key = Some(node.key.raw_value().to_string());
                self.found_entity_key = Some(node.key.raw_value().to_string());
            } else {
                // Nested property - the container/entity is the first element in current_path
                if let Some(first_path_element) = self.current_path.first() {
                    self.found_container_key = Some(first_path_element.clone());
                    self.found_entity_key = Some(first_path_element.clone()); // Same for normal entities
                }
            }

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

    // Check if required caches are initialized
    if !TypeCache::is_initialized()
        || !GameDataCache::is_initialized()
        || !EntityRestructurer::is_initialized()
    {
        return Ok(None);
    }

    let type_cache = TypeCache::get().unwrap();

    // Extract namespace from URI to get type information
    let namespace = match extract_namespace_from_uri(&uri) {
        Some(namespace) => namespace,
        None => return Ok(None),
    };

    if namespace.starts_with("common/inline_scripts") {
        // These are special, they don't have a type
        return Ok(None);
    }

    // Get namespace type information
    let namespace_type = match type_cache.get_namespace_type(&namespace, Some(&uri)) {
        Some(info) => info,
        None => return Ok(None),
    };

    // Find the property at the given position
    let mut builder = PropertyPathBuilder::new(offset, cached_document.borrow_input());

    if let Ok(ast) = cached_document.borrow_ast() {
        builder.visit_module(ast);
    } else {
        return Ok(None);
    }

    if let Some(property_path) = builder.found_property.as_ref() {
        // Check if this is a top-level key (entity name) or a nested property
        let is_top_level_key = !property_path.contains('.');

        // Build the base hover content
        let mut hover_content = String::new();

        let type_info = if is_top_level_key {
            // For top-level keys, we don't want to show namespace type as it's confusing
            // Top-level keys are entity names which can be anything, so no type info
            None
        } else {
            // For nested properties, we need to find the correct entity context first
            let property_parts: Vec<&str> = property_path.split('.').collect();
            if property_parts.len() > 1 {
                // Skip the first part (entity name) and join the rest
                let actual_property_path = property_parts[1..].join(".");

                // Use the entity context found by PropertyPathBuilder
                if let Some(entity_context) = builder.found_entity_context {
                    // Implement the exact same flow as provider.rs for sophisticated type resolution
                    let container_key = property_parts[0];
                    let entity_key = container_key; // For normal entities, these are the same

                    // Step 1: Filter union types based on the ROOT KEY (type_key_filter)
                    let filtered_namespace_type =
                        type_cache.filter_union_types_by_key(namespace_type.clone(), entity_key);

                    // Step 2: Get the effective entity for subtype narrowing
                    let (_effective_key, effective_entity) =
                        EntityRestructurer::get_effective_entity_for_subtype_narrowing(
                            &namespace,
                            container_key,
                            entity_key,
                            entity_context,
                        );

                    // Step 4: Perform subtype narrowing with the effective entity data
                    let matching_subtypes = type_cache.get_resolver().determine_matching_subtypes(
                        filtered_namespace_type.clone(),
                        &effective_entity,
                    );

                    let validation_type = if !matching_subtypes.is_empty() {
                        Arc::new(filtered_namespace_type.with_subtypes(matching_subtypes))
                    } else {
                        filtered_namespace_type
                    };

                    // Step 5: Navigate to the specific property within the narrowed type
                    let path_parts: Vec<&str> = actual_property_path.split('.').collect();
                    let mut current_type = validation_type;

                    for part in path_parts.iter() {
                        // Resolve the current type to its actual type before navigation
                        current_type = type_cache.get_resolver().resolve_type(current_type);

                        match type_cache
                            .get_resolver()
                            .navigate_to_property(current_type, part)
                        {
                            PropertyNavigationResult::Success(property_type) => {
                                current_type = property_type;
                            }
                            PropertyNavigationResult::NotFound => {
                                return Ok(None);
                            }
                            PropertyNavigationResult::ScopeError(_) => {
                                return Ok(None);
                            }
                        }
                    }

                    Some(TypeInfo {
                        property_path: actual_property_path.clone(),
                        scoped_type: Some(current_type),
                        documentation: None,
                        source_info: None,
                    })
                } else {
                    // Fallback: try to find the entity in the AST manually
                    let mut found_entity_type = None;
                    if let Ok(ast) = cached_document.borrow_ast() {
                        let entity_name = property_parts[0];

                        // Find the entity in the AST that matches our container key
                        for item in &ast.items {
                            if let cw_parser::AstEntityItem::Expression(expr) = item {
                                if expr.key.raw_value() == entity_name {
                                    if let AstValue::Entity(ast_entity) = &expr.value {
                                        // Found the right entity context - use AST-based resolution
                                        found_entity_type = get_entity_property_type_from_ast(
                                            &namespace,
                                            ast_entity,
                                            &actual_property_path,
                                            Some(&uri),
                                        );
                                    }
                                    break; // Found the key, stop searching
                                }
                            }
                        }
                    }

                    // Only use string-based fallback if AST lookup completely failed
                    found_entity_type
                }
            } else {
                None
            }
        };

        if let Some(type_info) = type_info {
            // Format the type information using TypeFormatter
            if let Some(scoped_type) = &type_info.scoped_type {
                let formatter = TypeFormatter::new(&type_cache.get_resolver(), 30);
                let property_parts: Vec<&str> = type_info.property_path.split('.').collect();
                let formatted_type = formatter.format_type(
                    scoped_type.clone(),
                    property_parts.last().copied(), // Pass the last part as property name
                );
                // Add type information in a clean format
                hover_content.push_str(&format!("```\n{}\n```", formatted_type));
            }

            // Add brief documentation if available
            if let Some(documentation) = &type_info.documentation {
                if !documentation.trim().is_empty() {
                    hover_content.push_str(&format!("\n\n{}", documentation.trim()));
                }
            }

            // Add source info if available
            if let Some(source_info) = &type_info.source_info {
                if !source_info.is_empty() {
                    hover_content.push_str(&format!("\n\n*{}*", source_info));
                }
            }
        }

        if !hover_content.is_empty() {
            let hover = Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: hover_content,
                }),
                range: None, // We could calculate the exact range if needed
            };

            return Ok(Some(hover));
        }
    }

    Ok(None)
}
