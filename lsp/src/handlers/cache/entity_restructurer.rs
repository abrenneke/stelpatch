use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use cw_model::{Entity, SkipRootKey, TypeKeyFilter, Value};

use crate::handlers::cache::{GameDataCache, TypeCache};

/// Post-processor that restructures entities according to type definitions
///
/// Handles two main restructuring scenarios:
/// 1. `skip_root_key`: Skip a container key and use nested keys as entity names
/// 2. `name_field`: Use a field within an entity as the key instead of structural key
///
/// These can be used independently or together.
///
/// # Examples
///
/// ## Skip Root Key Only
/// ```
/// // Original structure:
/// container_key = {
///     entity_a = { prop = value }
///     entity_b = { prop = value }
/// }
///
/// // After restructuring:
/// // entities["namespace"]["entity_a"] = { prop = value }
/// // entities["namespace"]["entity_b"] = { prop = value }
/// ```
///
/// ## Name Field Only
/// ```
/// // Original structure:
/// some_key = {
///     name = "actual_entity_name"
///     prop = value
/// }
///
/// // After restructuring:
/// // entities["namespace"]["actual_entity_name"] = { name = "actual_entity_name", prop = value }
/// ```
///
/// ## Both Skip Root Key and Name Field (like sprites)
/// ```
/// // Original structure:
/// spriteTypes = {
///     spriteType = {
///         name = "GFX_my_sprite"
///         textureFile = "path/to/texture.dds"
///     }
/// }
///
/// // After restructuring:
/// // entities["interface"]["GFX_my_sprite"] = { name = "GFX_my_sprite", textureFile = "..." }
/// ```
pub struct EntityRestructurer {
    game_data: &'static GameDataCache,
    type_cache: &'static TypeCache,
}

/// Result of entity restructuring
#[derive(Clone)]
pub struct RestructuredEntities {
    /// Namespace -> Entity Name -> Entity
    /// For special types, entities are indexed by their name_field value
    pub entities: HashMap<String, HashMap<String, Entity>>,
    /// Track which namespaces were restructured
    pub restructured_namespaces: HashMap<String, RestructureInfo>,
}

/// Information about how a namespace was restructured
#[derive(Clone)]
pub struct RestructureInfo {
    pub skip_root_key: Option<String>,
    pub name_field: Option<String>,
    pub original_entity_count: usize,
    pub restructured_entity_count: usize,
}

static RESTRUCTURED_ENTITIES: RwLock<Option<RestructuredEntities>> = RwLock::new(None);

impl EntityRestructurer {
    /// Create a new EntityRestructurer
    pub fn new(game_data: &'static GameDataCache, type_cache: &'static TypeCache) -> Self {
        Self {
            game_data,
            type_cache,
        }
    }

    /// Get the restructured entities result
    pub fn get() -> Option<RestructuredEntities> {
        RESTRUCTURED_ENTITIES.read().unwrap().clone()
    }

    /// Check if the restructurer has been initialized
    pub fn is_initialized() -> bool {
        RESTRUCTURED_ENTITIES.read().unwrap().is_some()
    }

    /// Reset the restructured entities cache, forcing re-initialization on next access
    pub fn reset() {
        eprintln!("Resetting EntityRestructurer cache");
        let mut cache = RESTRUCTURED_ENTITIES.write().unwrap();
        *cache = None;
    }

    /// Load and process all entities that need restructuring
    pub fn load(&self) {
        // Check if already initialized
        if Self::is_initialized() {
            return;
        }

        // Compute the result without holding the lock
        let start = std::time::Instant::now();

        let mut restructured = RestructuredEntities {
            entities: HashMap::new(),
            restructured_namespaces: HashMap::new(),
        };

        self.process_all_namespaces(&mut restructured);

        let duration = start.elapsed();
        eprintln!("Entity restructuring completed in {:?}", duration);
        eprintln!(
            "Restructured {} namespaces: {}",
            restructured.restructured_namespaces.len(),
            restructured
                .restructured_namespaces
                .keys()
                .cloned()
                .collect::<Vec<String>>()
                .join(", "),
        );

        // Now acquire the lock only to store the result
        let mut cache = RESTRUCTURED_ENTITIES.write().unwrap();

        // Double-check after acquiring write lock
        if cache.is_none() {
            *cache = Some(restructured);
        }
    }

    /// Process all namespaces that need restructuring
    fn process_all_namespaces(&self, restructured: &mut RestructuredEntities) {
        // Get type definitions that need special handling
        let types_with_special_loading = self.get_types_needing_restructure();

        eprintln!(
            "Found {} types needing restructure: {}",
            types_with_special_loading.len(),
            types_with_special_loading
                .keys()
                .cloned()
                .collect::<Vec<String>>()
                .join(", ")
        );

        for (namespace, type_defs) in types_with_special_loading {
            if let Some(namespace_data) = self.game_data.get_namespaces().get(&namespace) {
                let (entities, info) = self.process_namespace(&type_defs, namespace_data);

                restructured.entities.insert(namespace.clone(), entities);
                restructured
                    .restructured_namespaces
                    .insert(namespace.clone(), info);
            } else {
                eprintln!(
                    "WARN: Namespace {} not found in game data, skipping",
                    namespace
                );
            }
        }
    }

    /// Get type definitions that need restructuring
    fn get_types_needing_restructure(&self) -> HashMap<String, Vec<TypeDefinitionInfo>> {
        let mut result: HashMap<String, Vec<TypeDefinitionInfo>> = HashMap::new();

        for (_type_name, type_def) in self.type_cache.get_cwt_analyzer().get_types() {
            // Check if this type needs any kind of restructuring
            if type_def.skip_root_key.is_some() || type_def.name_field.is_some() {
                if let Some(path) = &type_def.path {
                    // Extract namespace from path (e.g., "game/interface" -> "interface")
                    let namespace = if let Some(stripped) = path.strip_prefix("game/") {
                        stripped.to_string()
                    } else {
                        path.clone()
                    };

                    let type_info = TypeDefinitionInfo {
                        skip_root_key: type_def.skip_root_key.clone(),
                        name_field: type_def.name_field.clone(),
                        type_key_filter: type_def.options.type_key_filter.clone(),
                    };

                    result
                        .entry(namespace)
                        .or_insert_with(Vec::new)
                        .push(type_info);
                }
            }
        }

        result
    }

    /// Process a single namespace according to its type definition
    fn process_namespace(
        &self,
        type_defs: &Vec<TypeDefinitionInfo>,
        namespace_data: &crate::handlers::cache::Namespace,
    ) -> (HashMap<String, Entity>, RestructureInfo) {
        let mut restructured_entities = HashMap::new();
        let mut original_count = 0;

        // Process each module in the namespace individually to avoid key overwrites
        for (_module_name, module) in &namespace_data.modules {
            // Process each property in the module
            for (key, property_list) in &module.properties.kv {
                if let Some(first_property) = property_list.0.first() {
                    if let Value::Entity(entity) = &first_property.value {
                        original_count += 1;

                        // Check if this entity passes the type_key_filter
                        if !self.passes_type_key_filter(key, type_defs) {
                            continue;
                        }

                        // Check if this key should be skipped
                        if type_defs
                            .iter()
                            .any(|type_def| self.should_skip_root_key(key, &type_def.skip_root_key))
                        {
                            // Skip this root key and process nested entities
                            // Find the name_field from the matching type definition
                            let matching_type_def = type_defs.iter().find(|type_def| {
                                self.should_skip_root_key(key, &type_def.skip_root_key)
                            });
                            let name_field = matching_type_def.and_then(|t| t.name_field.as_ref());
                            let extracted_entities = self.extract_entities_from_container(
                                entity,
                                &name_field.cloned(),
                                type_defs,
                            );
                            restructured_entities.extend(extracted_entities);
                        } else if type_defs
                            .iter()
                            .any(|type_def| type_def.name_field.is_some())
                        {
                            // Use name field to determine entity key, but don't skip root
                            let name_field_type_def = type_defs
                                .iter()
                                .find(|type_def| type_def.name_field.is_some());
                            if let Some(entity_name) = name_field_type_def.and_then(|type_def| {
                                self.extract_name_from_entity(entity, &type_def.name_field)
                            }) {
                                restructured_entities.insert(entity_name, entity.clone());
                            } else {
                                // Fallback to original key if name field not found
                                restructured_entities.insert(key.clone(), entity.clone());
                            }
                        } else {
                            // Standard processing - use the key as-is
                            restructured_entities.insert(key.clone(), entity.clone());
                        }
                    }
                }
            }
        }

        let info = RestructureInfo {
            skip_root_key: type_defs
                .iter()
                .find(|type_def| type_def.skip_root_key.is_some())
                .and_then(|type_def| type_def.skip_root_key.as_ref())
                .and_then(|s| match s {
                    SkipRootKey::Specific(key) => Some(key.clone()),
                    _ => None,
                }),
            name_field: type_defs
                .iter()
                .find(|type_def| type_def.name_field.is_some())
                .and_then(|type_def| type_def.name_field.clone()),
            original_entity_count: original_count,
            restructured_entity_count: restructured_entities.len(),
        };

        (restructured_entities, info)
    }

    /// Check if a root key should be skipped
    fn should_skip_root_key(&self, key: &str, skip_config: &Option<SkipRootKey>) -> bool {
        match skip_config {
            Some(SkipRootKey::Specific(skip_key)) => key == skip_key,
            Some(SkipRootKey::Any) => true,
            Some(SkipRootKey::Except(exceptions)) => !exceptions.contains(&key.to_string()),
            Some(SkipRootKey::Multiple(keys)) => keys.contains(&key.to_string()),
            None => false,
        }
    }

    /// Check if a key passes the type_key_filter
    fn passes_type_key_filter(&self, key: &str, type_defs: &Vec<TypeDefinitionInfo>) -> bool {
        // If no type_key_filter is defined, allow all keys
        let type_key_filters: Vec<&TypeKeyFilter> = type_defs
            .iter()
            .filter_map(|type_def| type_def.type_key_filter.as_ref())
            .collect();

        if type_key_filters.is_empty() {
            return true;
        }

        // Check if the key matches any of the type_key_filters
        for filter in type_key_filters {
            if self.matches_type_key_filter(key, filter) {
                return true;
            }
        }

        false
    }

    /// Check if a key matches a specific type_key_filter
    fn matches_type_key_filter(&self, key: &str, filter: &TypeKeyFilter) -> bool {
        match filter {
            TypeKeyFilter::Specific(required_key) => key == required_key,
            TypeKeyFilter::OneOf(required_keys) => required_keys.contains(&key.to_string()),
            TypeKeyFilter::Not(excluded_key) => key != excluded_key,
        }
    }

    /// Extract entities from a container entity (when skipping root key)
    fn extract_entities_from_container(
        &self,
        container_entity: &Entity,
        name_field: &Option<String>,
        type_defs: &Vec<TypeDefinitionInfo>,
    ) -> HashMap<String, Entity> {
        let mut result = HashMap::new();

        // Look for child entities in the container
        for (child_key, child_property_list) in &container_entity.properties.kv {
            for property_info in &child_property_list.0 {
                if let Value::Entity(child_entity) = &property_info.value {
                    // Check if this child entity passes the type_key_filter
                    if !self.passes_type_key_filter(child_key, type_defs) {
                        continue;
                    }

                    // Determine the entity name based on whether we have a name field
                    let entity_name = if name_field.is_some() {
                        // Use name field if available, fallback to structural key
                        self.extract_name_from_entity(child_entity, name_field)
                            .unwrap_or_else(|| child_key.clone())
                    } else {
                        // No name field, use the structural key
                        child_key.clone()
                    };
                    result.insert(entity_name, child_entity.clone());
                }
            }
        }

        result
    }

    /// Extract the name field value from an entity
    fn extract_name_from_entity(
        &self,
        entity: &Entity,
        name_field: &Option<String>,
    ) -> Option<String> {
        if let Some(field_name) = name_field {
            if let Some(property_list) = entity.properties.kv.get(field_name) {
                if let Some(first_property) = property_list.0.first() {
                    return Some(first_property.value.to_string());
                }
            }
        }
        None
    }

    /// Get an entity by name from a namespace, handling special loading rules
    pub fn get_entity(namespace: &str, entity_name: &str) -> Option<Entity> {
        // Check restructured entities first
        if let Some(restructured) = Self::get() {
            if let Some(entities) = restructured.entities.get(namespace) {
                if let Some(entity) = entities.get(entity_name) {
                    return Some(entity.clone());
                }
            }
        }

        // Check mod data
        if let Some(entity) = super::ModDataCache::get_entity(namespace, entity_name) {
            return Some(entity);
        }

        // Fall back to original GameDataCache
        if let Some(game_data) = GameDataCache::get() {
            if let Some(namespace_data) = game_data.get_namespaces().get(namespace) {
                namespace_data.entities.get(entity_name).cloned()
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get all entities in a namespace as a HashMap
    pub fn get_namespace_entities_map(namespace: &str) -> Option<HashMap<String, Entity>> {
        Self::get()?.entities.get(namespace).cloned()
    }

    /// Check if a namespace was restructured
    pub fn was_restructured(namespace: &str) -> bool {
        Self::get()
            .map(|r| r.restructured_namespaces.contains_key(namespace))
            .unwrap_or(false)
    }

    /// Get restructure info for a namespace
    pub fn get_restructure_info(namespace: &str) -> Option<RestructureInfo> {
        Self::get()?.restructured_namespaces.get(namespace).cloned()
    }

    /// Get entity keys for a namespace, using restructured keys if available
    pub fn get_namespace_entity_keys(namespace: &str) -> Option<Vec<String>> {
        let mut all_keys = HashSet::new();

        // Add restructured entity keys if available
        if let Some(restructured) = Self::get() {
            if let Some(entities) = restructured.entities.get(namespace) {
                all_keys.extend(entities.keys().cloned());
            }
        }

        // Add mod entity keys
        if let Some(mod_keys) = super::game_data::ModDataCache::get_namespace_entity_keys(namespace)
        {
            all_keys.extend(mod_keys.iter().cloned());
        }

        // Add original entity keys from GameDataCache
        if let Some(game_data) = GameDataCache::get() {
            if let Some(original_keys) = game_data.get_namespace_entity_keys(namespace) {
                all_keys.extend(original_keys.iter().cloned());
            }
        }

        if all_keys.is_empty() {
            None
        } else {
            Some(all_keys.into_iter().collect())
        }
    }

    /// Get entities for a namespace as a vector of (key, entity) tuples
    pub fn get_namespace_entities(namespace: &str) -> Option<Vec<(String, Entity)>> {
        let mut all_entities = HashMap::new();

        // Add restructured entities if available
        if let Some(restructured) = Self::get() {
            if let Some(entities) = restructured.entities.get(namespace) {
                all_entities.extend(entities.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }

        // Add original entities from GameDataCache
        if let Some(game_data) = GameDataCache::get() {
            if let Some(namespace_data) = game_data.get_namespaces().get(namespace) {
                all_entities.extend(
                    namespace_data
                        .entities
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone())),
                );
            }
        }

        if all_entities.is_empty() {
            None
        } else {
            Some(all_entities.into_iter().collect())
        }
    }

    /// Get entity keys for a namespace as a HashSet, using restructured keys if available
    pub fn get_namespace_entity_keys_set(namespace: &str) -> Option<Arc<HashSet<String>>> {
        let mut all_keys = HashSet::new();

        // Add restructured entity keys if available
        if let Some(restructured) = Self::get() {
            if let Some(entities) = restructured.entities.get(namespace) {
                all_keys.extend(entities.keys().cloned());
            }
        } else {
            eprintln!("WARN: EntityRestructurer not initialized");
        }

        // Add mod entity keys
        if let Some(mod_keys_set) =
            super::game_data::ModDataCache::get_namespace_entity_keys_set(namespace)
        {
            all_keys.extend(mod_keys_set.iter().cloned());
        }

        // Add original entity keys from GameDataCache
        if let Some(game_data) = GameDataCache::get() {
            if let Some(original_keys_set) = game_data.get_namespace_entity_keys_set(namespace) {
                all_keys.extend(original_keys_set.iter().cloned());
            }
        }

        if all_keys.is_empty() {
            None
        } else {
            Some(Arc::new(all_keys))
        }
    }

    /// Get a specific entity from a namespace, using restructured entities if available
    pub fn get_namespace_entity(namespace: &str, entity_name: &str) -> Option<Entity> {
        if let Some(restructured) = Self::get() {
            if let Some(entities) = restructured.entities.get(namespace) {
                // Use restructured entities
                return entities.get(entity_name).cloned();
            }
        }

        // Check mod data next
        if let Some(mod_entity) = super::game_data::ModDataCache::get_entity(namespace, entity_name)
        {
            return Some(mod_entity);
        }

        // Fall back to original GameDataCache
        if let Some(game_data) = GameDataCache::get() {
            if let Some(namespace_data) = game_data.get_namespaces().get(namespace) {
                namespace_data.entities.get(entity_name).cloned()
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get all entities in a namespace, using restructured entities if available
    pub fn get_all_namespace_entities(namespace: &str) -> Option<HashMap<String, Entity>> {
        if let Some(restructured) = Self::get() {
            if let Some(entities) = restructured.entities.get(namespace) {
                // Use restructured entities
                return Some(entities.clone());
            }
        }

        // Fall back to original GameDataCache
        if let Some(game_data) = GameDataCache::get() {
            if let Some(namespace_data) = game_data.get_namespaces().get(namespace) {
                Some(namespace_data.entities.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Check if a namespace has restructured entities
    pub fn has_restructured_entities(namespace: &str) -> bool {
        Self::get()
            .map(|r| r.entities.contains_key(namespace))
            .unwrap_or(false)
    }

    /// Get scripted variables for a namespace (always from original GameDataCache)
    pub fn get_namespace_scripted_variables(namespace: &str) -> Option<HashMap<String, Value>> {
        let mut all_variables = HashMap::new();

        // Add base game variables first
        if let Some(game_data) = GameDataCache::get() {
            if let Some(namespace_data) = game_data.get_namespaces().get(namespace) {
                all_variables.extend(
                    namespace_data
                        .scripted_variables
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone())),
                );
            }
        }

        // Add mod variables (can override base game variables)
        if let Some(mod_variables) =
            super::game_data::ModDataCache::get_namespace_scripted_variables(namespace)
        {
            all_variables.extend(mod_variables);
        }

        if all_variables.is_empty() {
            None
        } else {
            Some(all_variables)
        }
    }

    /// Get all entities in a namespace, using restructured entities if available
    pub fn get_all_entities(namespace: &str) -> Option<Vec<Entity>> {
        let mut all_entities = Vec::new();

        // Add restructured entities if available
        if let Some(restructured) = Self::get() {
            if let Some(entities) = restructured.entities.get(namespace) {
                all_entities.extend(entities.values().cloned());
            }
        }

        // Add mod entities
        let mod_namespaces = super::game_data::ModDataCache::get_namespaces();
        if let Some(mod_namespace) = mod_namespaces.get(namespace) {
            all_entities.extend(mod_namespace.entities.values().cloned());
        }

        // Add original entities from GameDataCache
        if let Some(game_data) = GameDataCache::get() {
            if let Some(namespace_data) = game_data.get_namespaces().get(namespace) {
                all_entities.extend(namespace_data.entities.values().cloned());
            }
        }

        if all_entities.is_empty() {
            None
        } else {
            Some(all_entities)
        }
    }

    /// Get all entities in a namespace as a HashMap, using restructured entities if available
    pub fn get_all_entities_map(namespace: &str) -> Option<HashMap<String, Entity>> {
        let mut all_entities = HashMap::new();

        // Add original entities from GameDataCache first
        if let Some(game_data) = GameDataCache::get() {
            if let Some(namespace_data) = game_data.get_namespaces().get(namespace) {
                all_entities.extend(
                    namespace_data
                        .entities
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone())),
                );
            }
        }

        // Add mod entities (can override base game entities)
        let mod_namespaces = super::game_data::ModDataCache::get_namespaces();
        if let Some(mod_namespace) = mod_namespaces.get(namespace) {
            all_entities.extend(
                mod_namespace
                    .entities
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone())),
            );
        }

        // Add restructured entities if available (can override both base and mod entities)
        if let Some(restructured) = Self::get() {
            if let Some(entities) = restructured.entities.get(namespace) {
                all_entities.extend(entities.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }

        if all_entities.is_empty() {
            None
        } else {
            Some(all_entities)
        }
    }
}

/// Simplified type definition info for restructuring
#[derive(Debug, Clone)]
struct TypeDefinitionInfo {
    pub skip_root_key: Option<SkipRootKey>,
    pub name_field: Option<String>,
    pub type_key_filter: Option<TypeKeyFilter>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_functionality() {
        // Test that reset clears the cache and allows reinitialization

        // First, simulate that the cache is already initialized
        {
            let mut cache = RESTRUCTURED_ENTITIES.write().unwrap();
            *cache = Some(RestructuredEntities {
                entities: HashMap::new(),
                restructured_namespaces: HashMap::new(),
            });
        }

        // Verify it's initialized
        assert!(EntityRestructurer::is_initialized());

        // Reset the cache
        EntityRestructurer::reset();

        // Verify it's no longer initialized
        assert!(!EntityRestructurer::is_initialized());

        // Verify get() returns None after reset
        assert!(EntityRestructurer::get().is_none());
    }

    #[test]
    fn test_reset_method_exists() {
        // Simple test to verify that the reset method exists and can be called
        // This ensures the trigger functionality is available
        EntityRestructurer::reset();

        // After reset, should not be initialized
        assert!(!EntityRestructurer::is_initialized());
    }
}
