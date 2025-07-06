use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, jsonrpc::Result};

use super::document_cache::DocumentCache;
use super::type_cache::{TypeInfo, get_entity_property_type};
use super::utils::{extract_namespace_from_uri, position_to_offset};
use cw_model::{InferredType, PrimitiveType};
use cw_parser::{AstEntity, AstExpression, AstNode, AstValue, AstVisitor};

/// A visitor that finds the completion context at a given position
struct CompletionContextFinder<'a> {
    position_offset: usize,
    current_path: Vec<String>,
    context: Option<CompletionContext>,
    original_input: &'a str,
}

#[derive(Debug, Clone)]
enum CompletionContext {
    /// We're typing a property value (the string after the =)
    PropertyValue {
        property_path: String,
        /// The partial text that's been typed so far
        partial_text: String,
    },
    /// We're typing a property key
    PropertyKey {
        parent_path: String,
        partial_text: String,
    },
}

impl<'a> CompletionContextFinder<'a> {
    fn new(position_offset: usize, input: &'a str) -> Self {
        Self {
            position_offset,
            current_path: Vec::new(),
            context: None,
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

    /// Extract partial text from the input at the current position
    fn extract_partial_text(&self, start_offset: usize, _end_offset: usize) -> String {
        let start_pos = start_offset
            .min(self.position_offset)
            .min(self.original_input.len());

        // Find the start of the current token (go back to find quote or whitespace)
        let mut token_start = self.position_offset;
        let bytes = self.original_input.as_bytes();

        while token_start > start_pos {
            let ch = bytes[token_start - 1];
            if ch == b'"' || ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'=' {
                break;
            }
            token_start -= 1;
        }

        // Skip opening quote if present
        if token_start < self.original_input.len() && bytes[token_start] == b'"' {
            token_start += 1;
        }

        // Extract the text from token start to current position
        if token_start <= self.position_offset {
            self.original_input[token_start..self.position_offset].to_string()
        } else {
            String::new()
        }
    }

    /// Extract partial text after the equals sign
    fn extract_partial_text_after_equals(&self, start_offset: usize, end_offset: usize) -> String {
        let range_text = &self.original_input[start_offset..end_offset];

        // Find the '=' sign
        if let Some(equals_pos) = range_text.find('=') {
            let after_equals_start = start_offset + equals_pos + 1;

            // Skip whitespace after '='
            let mut token_start = after_equals_start;
            let bytes = self.original_input.as_bytes();

            while token_start < self.position_offset && token_start < self.original_input.len() {
                let ch = bytes[token_start];
                if ch != b' ' && ch != b'\t' && ch != b'\n' {
                    break;
                }
                token_start += 1;
            }

            // Skip opening quote if present
            if token_start < self.original_input.len() && bytes[token_start] == b'"' {
                token_start += 1;
            }

            // Extract from after whitespace/quote to cursor position
            if token_start <= self.position_offset {
                self.original_input[token_start..self.position_offset].to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    }
}

impl<'a> AstVisitor<'a> for CompletionContextFinder<'a> {
    fn visit_expression(&mut self, node: &AstExpression<'a>) -> () {
        if self.context.is_some() {
            return; // Already found context
        }

        let key_span = node.key.span(&self.original_input);
        let value_span = node.value.span(&self.original_input);

        // Debug logging
        eprintln!(
            "Visiting expression: key='{}', position_offset={}, key_span={:?}, value_span={:?}",
            node.key.raw_value(),
            self.position_offset,
            key_span,
            value_span
        );

        // Check if cursor is in the key
        if self.position_offset >= key_span.start.offset
            && self.position_offset <= key_span.end.offset
        {
            let partial_text =
                self.extract_partial_text(key_span.start.offset, key_span.end.offset);
            self.context = Some(CompletionContext::PropertyKey {
                parent_path: self.build_path(),
                partial_text,
            });
            return;
        }

        // Check if cursor is in the value
        if self.position_offset >= value_span.start.offset
            && self.position_offset <= value_span.end.offset
        {
            match &node.value {
                AstValue::String(s) => {
                    // For string values, this is our completion context
                    let full_path = if self.current_path.is_empty() {
                        node.key.raw_value().to_string()
                    } else {
                        format!("{}.{}", self.build_path(), node.key.raw_value())
                    };

                    let string_span = s.span(&self.original_input);
                    let partial_text =
                        self.extract_partial_text(string_span.start.offset, string_span.end.offset);

                    self.context = Some(CompletionContext::PropertyValue {
                        property_path: full_path,
                        partial_text,
                    });
                    return;
                }
                AstValue::Entity(entity) => {
                    // If we're in an entity value, recurse into it
                    self.with_path_segment(node.key.raw_value(), |finder| {
                        finder.visit_entity(entity);
                    });
                }
                _ => {
                    // For other value types, we don't provide completion
                }
            }
        }

        // Check if cursor is positioned right after the key, where a value is expected
        // This handles cases like "category = " where no value has been typed yet
        if self.position_offset > key_span.end.offset {
            // Look for the '=' sign between key and value
            let between_start = key_span.end.offset;
            let between_end = value_span.start.offset;

            if self.position_offset >= between_start && self.position_offset <= between_end {
                // Check if there's an '=' sign in this range
                let between_text = &self.original_input[between_start..between_end];
                if between_text.contains('=') {
                    let full_path = if self.current_path.is_empty() {
                        node.key.raw_value().to_string()
                    } else {
                        format!("{}.{}", self.build_path(), node.key.raw_value())
                    };

                    // Extract any partial text that might be typed after the '='
                    let partial_text =
                        self.extract_partial_text_after_equals(between_start, between_end);

                    self.context = Some(CompletionContext::PropertyValue {
                        property_path: full_path,
                        partial_text,
                    });
                    return;
                }
            }
        }
    }

    fn walk_entity(&mut self, node: &AstEntity<'a>) -> () {
        for item in &node.items {
            self.visit_entity_item(item);
            if self.context.is_some() {
                break;
            }
        }
    }
}

/// Extract completion items from type information
fn extract_completion_items(type_info: &TypeInfo, partial_text: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    if let Some(inferred_type) = &type_info.inferred_type {
        match inferred_type {
            InferredType::Literal(literal) => {
                if literal
                    .to_lowercase()
                    .contains(&partial_text.to_lowercase())
                {
                    items.push(CompletionItem {
                        label: literal.clone(),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: Some("Literal value".to_string()),
                        documentation: None,
                        insert_text: Some(format!("\"{}\"", literal)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    });
                }
            }
            InferredType::LiteralUnion(literals) => {
                for literal in literals {
                    if literal
                        .to_lowercase()
                        .contains(&partial_text.to_lowercase())
                    {
                        items.push(CompletionItem {
                            label: literal.clone(),
                            kind: Some(CompletionItemKind::VALUE),
                            detail: Some("Literal value".to_string()),
                            documentation: None,
                            insert_text: Some(format!("\"{}\"", literal)),
                            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                            ..Default::default()
                        });
                    }
                }
            }
            InferredType::Primitive(PrimitiveType::String) => {
                // For generic string types, we could potentially provide common values
                // but for now we'll just indicate it's a string
                items.push(CompletionItem {
                    label: "string".to_string(),
                    kind: Some(CompletionItemKind::TYPE_PARAMETER),
                    detail: Some("String type".to_string()),
                    documentation: Some(Documentation::String("Enter a string value".to_string())),
                    insert_text: Some("\"\"".to_string()),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                });
            }
            InferredType::PrimitiveUnion(primitives) => {
                if primitives.contains(&PrimitiveType::String) {
                    items.push(CompletionItem {
                        label: "string".to_string(),
                        kind: Some(CompletionItemKind::TYPE_PARAMETER),
                        detail: Some("String value".to_string()),
                        documentation: Some(Documentation::String(
                            "Enter a string value".to_string(),
                        )),
                        insert_text: Some("\"\"".to_string()),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    });
                }
            }
            InferredType::Union(types) => {
                // Check if any of the union types are string-like
                for union_type in types {
                    let sub_type_info = TypeInfo {
                        property_path: type_info.property_path.clone(),
                        type_description: String::new(),
                        inferred_type: Some(union_type.clone()),
                    };
                    items.extend(extract_completion_items(&sub_type_info, partial_text));
                }
            }
            _ => {
                // For other types, we don't provide completion
            }
        }
    }

    // Sort items by relevance (exact matches first, then by length)
    items.sort_by(|a, b| {
        let a_exact = a.label.eq_ignore_ascii_case(partial_text);
        let b_exact = b.label.eq_ignore_ascii_case(partial_text);

        if a_exact && !b_exact {
            std::cmp::Ordering::Less
        } else if !a_exact && b_exact {
            std::cmp::Ordering::Greater
        } else {
            a.label.len().cmp(&b.label.len())
        }
    });

    items
}

/// Attempt to repair incomplete syntax for completion by inserting placeholder values
fn repair_syntax_for_completion(content: &str, offset: usize) -> Option<(String, usize)> {
    // Find the current line
    let lines: Vec<&str> = content.lines().collect();
    let mut current_offset = 0;
    let mut current_line_index = 0;

    // Find which line the cursor is on
    for (i, line) in lines.iter().enumerate() {
        let line_end = current_offset + line.len();
        if offset <= line_end {
            current_line_index = i;
            break;
        }
        current_offset = line_end + 1; // +1 for the newline character
    }

    if current_line_index >= lines.len() {
        return None;
    }

    let current_line = lines[current_line_index];
    let line_start_offset = current_offset;
    let cursor_pos_in_line = offset - line_start_offset;

    // Look for incomplete expressions like "key = " or "key = \"partial"
    let line_up_to_cursor = &current_line[..cursor_pos_in_line.min(current_line.len())];

    // Check if we have a pattern like "key = " or "key = \"partial"
    if let Some(equals_pos) = line_up_to_cursor.rfind('=') {
        let after_equals = &line_up_to_cursor[equals_pos + 1..];
        let after_equals_trimmed = after_equals.trim();

        // Check if we need to add a placeholder value
        let needs_placeholder = if after_equals_trimmed.is_empty() {
            // Case: "key = " - add a placeholder
            true
        } else if after_equals_trimmed.starts_with('"') && !after_equals_trimmed.ends_with('"') {
            // Case: "key = \"partial" - close the quote
            false // We'll handle this by closing the quote
        } else {
            // Case: "key = partial" - add quotes around it
            false // We'll handle this by adding quotes
        };

        let mut repaired_content = String::new();
        let mut adjusted_offset = offset;

        // Add everything before the current line
        for (i, line) in lines.iter().enumerate() {
            if i < current_line_index {
                repaired_content.push_str(line);
                repaired_content.push('\n');
            } else if i == current_line_index {
                // Handle the current line
                let line_before_cursor =
                    &current_line[..cursor_pos_in_line.min(current_line.len())];
                repaired_content.push_str(line_before_cursor);

                if needs_placeholder {
                    // Add a placeholder value
                    repaired_content.push_str("\"__PLACEHOLDER__\"");
                    // The cursor should be positioned at the start of the placeholder
                    adjusted_offset = repaired_content.len() - "__PLACEHOLDER__".len() - 1; // -1 for the quote
                } else if after_equals_trimmed.starts_with('"')
                    && !after_equals_trimmed.ends_with('"')
                {
                    // Close the unclosed quote
                    repaired_content.push('"');
                    adjusted_offset = offset;
                } else if !after_equals_trimmed.is_empty() && !after_equals_trimmed.starts_with('"')
                {
                    // Add quotes around unquoted value
                    // Need to find where the unquoted value starts
                    let unquoted_start = line_start_offset + equals_pos + 1 + after_equals.len()
                        - after_equals_trimmed.len();
                    repaired_content = repaired_content
                        [..repaired_content.len() - after_equals_trimmed.len()]
                        .to_string();
                    repaired_content.push('"');
                    repaired_content.push_str(after_equals_trimmed);
                    repaired_content.push('"');
                    adjusted_offset = offset + 1; // +1 for the opening quote
                }

                // Add the rest of the current line
                let remaining_line = &current_line[cursor_pos_in_line.min(current_line.len())..];
                repaired_content.push_str(remaining_line);
                repaired_content.push('\n');
            } else {
                // Add remaining lines
                repaired_content.push_str(line);
                if i < lines.len() - 1 {
                    repaired_content.push('\n');
                }
            }
        }

        Some((repaired_content, adjusted_offset))
    } else {
        None
    }
}

pub async fn completion(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    document_cache: &DocumentCache,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let uri = params.text_document_position.text_document.uri.to_string();
    let position = params.text_document_position.position;

    // Debug logging
    client
        .log_message(
            tower_lsp::lsp_types::MessageType::INFO,
            format!(
                "Completion requested for URI: {} at position {}:{}",
                uri, position.line, position.character
            ),
        )
        .await;

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

    // Find the completion context at the given position
    let mut finder = CompletionContextFinder::new(offset, cached_document.borrow_input());

    match cached_document.borrow_ast() {
        Ok(ast) => {
            client
                .log_message(
                    tower_lsp::lsp_types::MessageType::INFO,
                    "AST parsed successfully".to_string(),
                )
                .await;
            finder.visit_module(ast);
        }
        Err(e) => {
            client
                .log_message(
                    tower_lsp::lsp_types::MessageType::INFO,
                    format!("AST parsing failed: {:?}", e),
                )
                .await;

            // Strategy: Try to repair the syntax by inserting placeholder values
            client
                .log_message(
                    tower_lsp::lsp_types::MessageType::INFO,
                    "Attempting syntax repair for completion".to_string(),
                )
                .await;

            if let Some((repaired_content, adjusted_offset)) =
                repair_syntax_for_completion(content, offset)
            {
                client
                    .log_message(
                        tower_lsp::lsp_types::MessageType::INFO,
                        "Syntax repaired, trying to parse again".to_string(),
                    )
                    .await;

                // Try to parse the repaired content
                if let Ok(repaired_ast) = cw_parser::AstModule::from_input(&repaired_content) {
                    client
                        .log_message(
                            tower_lsp::lsp_types::MessageType::INFO,
                            "Repaired AST parsed successfully".to_string(),
                        )
                        .await;

                    // Use the repaired AST for completion
                    let mut finder =
                        CompletionContextFinder::new(adjusted_offset, &repaired_content);
                    finder.visit_module(&repaired_ast);

                    if let Some(context) = finder.context {
                        client
                            .log_message(
                                tower_lsp::lsp_types::MessageType::INFO,
                                format!("Repaired AST found context: {:?}", context),
                            )
                            .await;

                        // Extract namespace and provide completion
                        let namespace = extract_namespace_from_uri(&uri);
                        if let Some(namespace) = namespace {
                            if crate::handlers::type_cache::TypeCache::is_initialized() {
                                if let CompletionContext::PropertyValue {
                                    property_path,
                                    partial_text,
                                } = context
                                {
                                    let type_info =
                                        get_entity_property_type(&namespace, &property_path).await;
                                    if let Some(type_info) = type_info {
                                        let items =
                                            extract_completion_items(&type_info, &partial_text);
                                        if !items.is_empty() {
                                            return Ok(Some(CompletionResponse::Array(items)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            return Ok(None);
        }
    }

    if let Some(context) = finder.context {
        client
            .log_message(
                tower_lsp::lsp_types::MessageType::INFO,
                format!("Found completion context: {:?}", context),
            )
            .await;

        // Extract namespace from URI to get type information
        let namespace = extract_namespace_from_uri(&uri);

        client
            .log_message(
                tower_lsp::lsp_types::MessageType::INFO,
                format!("Extracted namespace: {:?}", namespace),
            )
            .await;

        if let Some(namespace) = namespace {
            // Only provide completion if the type cache is initialized
            if !crate::handlers::type_cache::TypeCache::is_initialized() {
                client
                    .log_message(
                        tower_lsp::lsp_types::MessageType::INFO,
                        "Type cache not initialized yet".to_string(),
                    )
                    .await;
                return Ok(None);
            }

            client
                .log_message(
                    tower_lsp::lsp_types::MessageType::INFO,
                    "Type cache is initialized".to_string(),
                )
                .await;

            match context {
                CompletionContext::PropertyValue {
                    property_path,
                    partial_text,
                } => {
                    client
                        .log_message(
                            tower_lsp::lsp_types::MessageType::INFO,
                            format!(
                                "Processing property value completion for path: {}",
                                property_path
                            ),
                        )
                        .await;

                    // For property values, look up the type and provide string completions
                    let is_top_level_key = !property_path.contains('.');

                    client
                        .log_message(
                            tower_lsp::lsp_types::MessageType::INFO,
                            format!("Is top level key: {}", is_top_level_key),
                        )
                        .await;

                    let type_info = if is_top_level_key {
                        // For top-level keys, look up the type directly
                        client
                            .log_message(
                                tower_lsp::lsp_types::MessageType::INFO,
                                format!(
                                    "Looking up top-level property type: namespace={}, property={}",
                                    namespace, property_path
                                ),
                            )
                            .await;
                        get_entity_property_type(&namespace, &property_path).await
                    } else {
                        // For nested properties, strip the entity name and look up the property path
                        let property_parts: Vec<&str> = property_path.split('.').collect();
                        if property_parts.len() > 1 {
                            let actual_property_path = property_parts[1..].join(".");
                            client.log_message(
                                tower_lsp::lsp_types::MessageType::INFO,
                                format!("Looking up nested property type: namespace={}, property={}", namespace, actual_property_path)
                            ).await;
                            get_entity_property_type(&namespace, &actual_property_path).await
                        } else {
                            client
                                .log_message(
                                    tower_lsp::lsp_types::MessageType::INFO,
                                    "Invalid nested property path".to_string(),
                                )
                                .await;
                            return Ok(None);
                        }
                    };

                    if let Some(type_info) = type_info {
                        let items = extract_completion_items(&type_info, &partial_text);
                        client
                            .log_message(
                                tower_lsp::lsp_types::MessageType::INFO,
                                format!("Generated {} completion items", items.len()),
                            )
                            .await;
                        if !items.is_empty() {
                            return Ok(Some(CompletionResponse::Array(items)));
                        }
                    } else {
                        client
                            .log_message(
                                tower_lsp::lsp_types::MessageType::INFO,
                                "No type info found for property".to_string(),
                            )
                            .await;
                    }
                }
                CompletionContext::PropertyKey {
                    parent_path: _,
                    partial_text: _,
                } => {
                    // For now, we don't provide completion for property keys
                    // This could be implemented later by looking up the expected properties
                    // for the current object type
                    return Ok(None);
                }
            }
        }
    } else {
        client
            .log_message(
                tower_lsp::lsp_types::MessageType::INFO,
                "No completion context found".to_string(),
            )
            .await;
    }

    Ok(None)
}
