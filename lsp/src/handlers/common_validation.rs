use cw_model::{CwtType, Entity, SimpleType};
use cw_parser::{AstEntity, AstEntityItem, AstModule, AstValue};
use lasso::Spur;
use std::sync::Arc;

use super::document_cache::DocumentCache;
use crate::handlers::cache::TypeFormatter;
use crate::handlers::cache::types::TypeInfo;
use crate::handlers::cache::{EntityRestructurer, GameDataCache, TypeCache};
use crate::handlers::scoped_type::{CwtTypeOrSpecialRef, ScopedType};
use crate::handlers::utils::extract_namespace_from_uri;
use crate::interner::get_interner;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

/// Common validation context that both hover and diagnostics can use
pub struct ValidationContext {
    pub namespace: Spur,
    pub namespace_type: Arc<ScopedType>,
    pub uri: String,
}

/// Result of namespace validation
pub enum NamespaceValidationResult {
    Valid(ValidationContext),
    CachesNotInitialized,
    NamespaceNotFound,
    InlineScript, // Special case for inline scripts
    UnknownNamespace,
}

/// Validates namespace and initializes validation context
pub fn validate_namespace_and_caches(uri: &str) -> NamespaceValidationResult {
    // Check if required caches are initialized
    if !TypeCache::is_initialized()
        || !GameDataCache::is_initialized()
        || !EntityRestructurer::is_initialized()
    {
        return NamespaceValidationResult::CachesNotInitialized;
    }

    // Extract namespace from URI
    let namespace = match extract_namespace_from_uri(uri) {
        Some(namespace) => namespace,
        None => return NamespaceValidationResult::NamespaceNotFound,
    };

    if namespace.starts_with("common/inline_scripts") {
        return NamespaceValidationResult::InlineScript;
    }

    let type_cache = TypeCache::get().unwrap();

    let namespace = get_interner().get_or_intern(namespace);

    // Get namespace type information
    let namespace_type = match type_cache.get_namespace_type(namespace, Some(uri)) {
        Some(info) => info,
        None => return NamespaceValidationResult::UnknownNamespace,
    };

    NamespaceValidationResult::Valid(ValidationContext {
        namespace,
        namespace_type,
        uri: uri.to_string(),
    })
}

/// Result of skip root key detection
pub struct SkipRootKeyResult {
    pub is_skip_root_key_container: bool,
    pub matching_type_name: Option<Spur>,
}

/// Detects if a container key matches skip_root_key patterns in union types
pub fn detect_skip_root_key_container(
    namespace_type: &Arc<ScopedType>,
    container_key: Spur,
) -> SkipRootKeyResult {
    let type_cache = TypeCache::get().unwrap();
    let interner = get_interner();

    match namespace_type.cwt_type_for_matching() {
        CwtTypeOrSpecialRef::Union(union_types) => {
            for union_type in union_types {
                let type_name = union_type.get_type_name();
                if type_name.is_some() && !interner.resolve(&type_name.unwrap()).is_empty() {
                    if let Some(type_def) =
                        type_cache.get_cwt_analyzer().get_type(type_name.unwrap())
                    {
                        if let Some(skip_root_key) = &type_def.skip_root_key {
                            let should_skip = match skip_root_key {
                                cw_model::SkipRootKey::Specific(skip_key) => {
                                    interner.resolve(&container_key).to_lowercase()
                                        == interner.resolve(&skip_key).to_lowercase()
                                }
                                cw_model::SkipRootKey::Any => true,
                                cw_model::SkipRootKey::Except(exceptions) => {
                                    !exceptions.iter().any(|exception| {
                                        interner.resolve(&exception).to_lowercase()
                                            == interner.resolve(&container_key).to_lowercase()
                                    })
                                }
                                cw_model::SkipRootKey::Multiple(keys) => keys.iter().any(|k| {
                                    interner.resolve(&k).to_lowercase()
                                        == interner.resolve(&container_key).to_lowercase()
                                }),
                            };

                            if should_skip {
                                return SkipRootKeyResult {
                                    is_skip_root_key_container: true,
                                    matching_type_name: type_name,
                                };
                            }
                        }
                    }
                }
            }
        }
        CwtTypeOrSpecialRef::ScopedUnion(scoped_union_types) => {
            for scoped_type in scoped_union_types {
                let type_name = scoped_type.get_type_name();
                if type_name.is_some() && !interner.resolve(&type_name.unwrap()).is_empty() {
                    if let Some(type_def) =
                        type_cache.get_cwt_analyzer().get_type(type_name.unwrap())
                    {
                        if let Some(skip_root_key) = &type_def.skip_root_key {
                            let should_skip = match skip_root_key {
                                cw_model::SkipRootKey::Specific(skip_key) => {
                                    interner.resolve(&container_key).to_lowercase()
                                        == interner.resolve(&skip_key).to_lowercase()
                                }
                                cw_model::SkipRootKey::Any => true,
                                cw_model::SkipRootKey::Except(exceptions) => {
                                    !exceptions.iter().any(|exception| {
                                        interner.resolve(&exception).to_lowercase()
                                            == interner.resolve(&container_key).to_lowercase()
                                    })
                                }
                                cw_model::SkipRootKey::Multiple(keys) => keys.iter().any(|k| {
                                    interner.resolve(&k).to_lowercase()
                                        == interner.resolve(&container_key).to_lowercase()
                                }),
                            };

                            if should_skip {
                                return SkipRootKeyResult {
                                    is_skip_root_key_container: true,
                                    matching_type_name: type_name,
                                };
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    SkipRootKeyResult {
        is_skip_root_key_container: false,
        matching_type_name: None,
    }
}

/// Performs entity filtering and subtype narrowing pipeline
pub fn filter_and_narrow_entity_type(
    namespace_type: Arc<ScopedType>,
    namespace: Spur,
    container_key: Spur,
    entity_key: Spur,
    ast_entity: &AstEntity,
) -> Arc<ScopedType> {
    let type_cache = TypeCache::get().unwrap();

    // Filter union types by key
    let filtered_namespace_type =
        type_cache.filter_union_types_by_key(namespace_type.clone(), entity_key);

    // Get effective entity for subtype narrowing
    let (_effective_key, effective_entity) =
        EntityRestructurer::get_effective_entity_for_subtype_narrowing(
            namespace,
            container_key,
            entity_key,
            ast_entity,
        );

    // Perform subtype narrowing
    let matching_subtypes = type_cache
        .get_resolver()
        .determine_matching_subtypes(filtered_namespace_type.clone(), &effective_entity);

    if !matching_subtypes.is_empty() {
        type_cache.apply_subtype_scope_changes(filtered_namespace_type.clone(), matching_subtypes)
    } else {
        filtered_namespace_type
    }
}

/// Checks if a namespace should be treated as type_per_file
pub fn is_type_per_file_namespace(namespace_type: &Arc<ScopedType>) -> bool {
    if let Some(type_cache) = TypeCache::get() {
        if let Some(type_def) = namespace_type
            .get_type_name()
            .and_then(|type_name| type_cache.get_cwt_analyzer().get_type(type_name))
        {
            return type_def.options.type_per_file;
        }
    }
    false
}

/// Performs subtype narrowing for type_per_file namespaces
pub fn apply_file_level_subtype_narrowing(
    namespace_type: Arc<ScopedType>,
    entity: &Entity,
) -> Arc<ScopedType> {
    if let Some(type_cache) = TypeCache::get() {
        let matching_subtypes = type_cache
            .get_resolver()
            .determine_matching_subtypes(namespace_type.clone(), entity);

        if !matching_subtypes.is_empty() {
            type_cache.apply_subtype_scope_changes(namespace_type.clone(), matching_subtypes)
        } else {
            namespace_type.clone()
        }
    } else {
        namespace_type.clone()
    }
}

/// Creates a type for variable assignments (keys starting with "@")
pub fn create_variable_assignment_type(namespace_type: &Arc<ScopedType>) -> Arc<ScopedType> {
    let expected_type = Arc::new(CwtType::Union(vec![
        Arc::new(CwtType::Simple(SimpleType::Int)),
        Arc::new(CwtType::Simple(SimpleType::Float)),
        Arc::new(CwtType::Simple(SimpleType::Scalar)),
        Arc::new(CwtType::Simple(SimpleType::Bool)),
    ]));

    Arc::new(ScopedType::new_cwt(
        expected_type,
        namespace_type.scope_stack().clone(),
        namespace_type.in_scripted_effect_block().cloned(),
    ))
}

/// Result of entity lookup in AST
pub struct EntityLookupResult<'a> {
    pub found: bool,
    pub ast_entity: Option<&'a AstEntity<'a>>,
}

/// Finds an entity by key in the AST module
pub fn find_entity_in_module<'a>(
    module: &'a AstModule<'a>,
    entity_key: Spur,
) -> EntityLookupResult<'a> {
    let interner = get_interner();
    for item in &module.items {
        if let AstEntityItem::Expression(expr) = item {
            if expr.key.raw_value() == interner.resolve(&entity_key) {
                if let AstValue::Entity(ast_entity) = &expr.value {
                    return EntityLookupResult {
                        found: true,
                        ast_entity: Some(ast_entity),
                    };
                }
            }
        }
    }

    EntityLookupResult {
        found: false,
        ast_entity: None,
    }
}

/// Finds a nested entity within a container entity
pub fn find_nested_entity_in_container<'a>(
    container_entity: &'a AstEntity<'a>,
    nested_entity_key: Spur,
) -> EntityLookupResult<'a> {
    let interner = get_interner();
    for nested_item in &container_entity.items {
        if let AstEntityItem::Expression(nested_expr) = nested_item {
            if nested_expr.key.raw_value() == interner.resolve(&nested_entity_key) {
                if let AstValue::Entity(nested_ast_entity) = &nested_expr.value {
                    return EntityLookupResult {
                        found: true,
                        ast_entity: Some(nested_ast_entity),
                    };
                }
            }
        }
    }

    EntityLookupResult {
        found: false,
        ast_entity: None,
    }
}

/// Result of document content and AST access
pub struct DocumentAccessResult<'a> {
    pub content: &'a str,
    pub cached_document: std::sync::Arc<crate::handlers::document_cache::CachedDocument>,
}

/// Common pattern for accessing document content and cached AST
pub fn get_document_content_and_cache<'a>(
    documents: &'a std::collections::HashMap<String, String>,
    document_cache: &'a DocumentCache,
    uri: &str,
) -> Result<DocumentAccessResult<'a>, &'static str> {
    let content = match documents.get(uri) {
        Some(content) => content,
        None => return Err("Document not found"),
    };

    let cached_document = match document_cache.get(uri) {
        Some(cached_document) => cached_document,
        None => return Err("Cached document not found"),
    };

    Ok(DocumentAccessResult {
        content,
        cached_document,
    })
}

/// Builds a hover response from TypeInfo
pub fn build_hover_response(type_info: TypeInfo, type_cache: &TypeCache) -> Option<Hover> {
    let mut hover_content = String::new();
    let interner = get_interner();
    // Format the type information using TypeFormatter
    if let Some(scoped_type) = &type_info.scoped_type {
        let formatter = TypeFormatter::new(&type_cache.get_resolver(), 30);
        let property_parts: Vec<&str> = type_info.property_path.split('.').collect();
        let formatted_type = formatter.format_type(
            scoped_type.clone(),
            property_parts.last().map(|s| interner.get_or_intern(s)), // Pass the last part as property name
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

    if !hover_content.is_empty() {
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_content,
            }),
            range: None, // We could calculate the exact range if needed
        })
    } else {
        None
    }
}
