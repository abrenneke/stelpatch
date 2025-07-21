use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, jsonrpc::Result};

use crate::handlers::cache::types::TypeInfo;
use crate::handlers::cache::{TypeCache, get_entity_property_type_from_ast};
use crate::handlers::common_validation::{
    NamespaceValidationResult, build_hover_response, detect_skip_root_key_container,
    filter_and_narrow_entity_type, find_entity_in_module, find_nested_entity_in_container,
    validate_namespace_and_caches,
};

use super::document_cache::DocumentCache;
use super::scoped_type::PropertyNavigationResult;
use super::utils::position_to_offset;
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
            // Always update to the innermost entity containing the position
            self.found_entity_context = Some(node);
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

    // Validate namespace and caches using common validation
    let validation_context = match validate_namespace_and_caches(&uri) {
        NamespaceValidationResult::Valid(context) => context,
        NamespaceValidationResult::CachesNotInitialized
        | NamespaceValidationResult::NamespaceNotFound
        | NamespaceValidationResult::InlineScript
        | NamespaceValidationResult::UnknownNamespace => return Ok(None),
    };

    let namespace = validation_context.namespace;
    let namespace_type = validation_context.namespace_type;
    let type_cache = TypeCache::get().unwrap();

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

        let type_info = if is_top_level_key {
            // For top-level keys, show contextual information about the entity type
            let entity_name = property_path;

            // Check if this is a skip_root_key container using common validation
            let skip_root_key_result = detect_skip_root_key_container(&namespace_type, entity_name);
            let container_info = if skip_root_key_result.is_skip_root_key_container {
                if let Some(type_name) = skip_root_key_result.matching_type_name {
                    Some(format!("Container for {} entities", type_name))
                } else {
                    Some("Container entity".to_string())
                }
            } else {
                None
            };

            if let Some(info) = container_info {
                // This is a skip_root_key container
                Some(TypeInfo {
                    property_path: property_path.clone(),
                    scoped_type: None,
                    documentation: Some(info),
                    source_info: Some(format!("From namespace: {}", namespace)),
                })
            } else {
                // Regular entity - show the namespace context
                Some(TypeInfo {
                    property_path: property_path.clone(),
                    scoped_type: None,
                    documentation: Some(format!("Entity in {} namespace", namespace)),
                    source_info: None,
                })
            }
        } else {
            // For nested properties, we need to find the correct entity context first
            let property_parts: Vec<&str> = property_path.split('.').collect();
            if property_parts.len() > 1 {
                // Skip the first part (entity name) and join the rest
                let actual_property_path = property_parts[1..].join(".");

                // Use the entity context found by PropertyPathBuilder
                if let Some(entity_context) = builder.found_entity_context {
                    let container_key = property_parts[0];

                    // Check for skip_root_key using common validation
                    let skip_root_key_result =
                        detect_skip_root_key_container(&namespace_type, container_key);
                    let is_skip_root_key_container =
                        skip_root_key_result.is_skip_root_key_container;

                    let (validation_type, final_property_path) = if is_skip_root_key_container
                        && property_parts.len() >= 2
                    {
                        // Skip root key: filter by nested entity key directly, not container key
                        let nested_entity_key = property_parts[1];
                        let nested_property_path = if property_parts.len() > 2 {
                            property_parts[2..].join(".")
                        } else {
                            String::new() // Empty path when hovering over the entity name itself
                        };

                        // Find the nested entity in the AST using common utilities
                        let nested_entity_context = if let Ok(ast) = cached_document.borrow_ast() {
                            if let Some(container_lookup) =
                                find_entity_in_module(&ast, container_key).ast_entity
                            {
                                find_nested_entity_in_container(container_lookup, nested_entity_key)
                                    .ast_entity
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(nested_ast_entity) = nested_entity_context {
                            let validation_type = filter_and_narrow_entity_type(
                                namespace_type.clone(),
                                &namespace,
                                container_key,
                                nested_entity_key,
                                nested_ast_entity,
                            );

                            (validation_type, nested_property_path)
                        } else {
                            return Ok(None);
                        }
                    } else {
                        // Normal case: filter by container key using common function
                        let entity_key = container_key;
                        let validation_type = filter_and_narrow_entity_type(
                            namespace_type.clone(),
                            &namespace,
                            container_key,
                            entity_key,
                            entity_context,
                        );

                        (validation_type, actual_property_path)
                    };

                    // Navigate to the specific property within the narrowed type
                    let mut current_type = validation_type;

                    if !final_property_path.is_empty() {
                        let path_parts: Vec<&str> = final_property_path.split('.').collect();

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
                    }

                    Some(TypeInfo {
                        property_path: final_property_path.clone(),
                        scoped_type: Some(current_type),
                        documentation: None,
                        source_info: None,
                    })
                } else {
                    // Fallback: try to find the entity in the AST using common utilities
                    let mut found_entity_type = None;
                    if let Ok(ast) = cached_document.borrow_ast() {
                        let entity_name = property_parts[0];

                        // Find the entity using common utility function
                        if let Some(ast_entity) =
                            find_entity_in_module(&ast, entity_name).ast_entity
                        {
                            // Found the right entity context - use AST-based resolution
                            found_entity_type = get_entity_property_type_from_ast(
                                &namespace,
                                ast_entity,
                                &actual_property_path,
                                Some(&uri),
                            );
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
            // Use common hover response builder
            if let Some(hover) = build_hover_response(type_info, &type_cache) {
                return Ok(Some(hover));
            }
        }
    }

    Ok(None)
}
