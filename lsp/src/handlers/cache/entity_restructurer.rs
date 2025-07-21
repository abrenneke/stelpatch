use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use cw_model::{
    Entity, LowerCaseHashMap, Operator, PropertyInfo, PropertyInfoList, SkipRootKey,
    TypeDefinition, TypeKeyFilter, Value, entity_from_ast,
};
use cw_parser::AstEntity;

use crate::handlers::cache::{GameDataCache, ModDataCache, Namespace, TypeCache};

/// Special property key used to store the original structural key
/// This is needed for subtype determination when entities are restructured
pub const ORIGINAL_KEY_PROPERTY: &str = "_original_key";

/// Post-processor that restructures entities according to type definitions
///
/// Handles two main restructuring scenarios:
/// 1. `skip_root_key`: Skip a container key and use nested keys as entity names
/// 2. `name_field`: Use a field within an entity as the key instead of structural key
///
/// These can be used independently or together.
///
/// When restructuring occurs, the original structural key is preserved in a special
/// property `_original_key` to enable subtype determination.
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
/// // entities["namespace"]["entity_a"] = { prop = value, _original_key = "entity_a" }
/// // entities["namespace"]["entity_b"] = { prop = value, _original_key = "entity_b" }
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
/// // entities["namespace"]["actual_entity_name"] = { name = "actual_entity_name", prop = value, _original_key = "some_key" }
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
/// // entities["interface"]["GFX_my_sprite"] = { name = "GFX_my_sprite", textureFile = "...", _original_key = "spriteType" }
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
    pub entities: LowerCaseHashMap<LowerCaseHashMap<Entity>>,
    /// Track which namespaces were restructured
    pub restructured_namespaces: LowerCaseHashMap<RestructureInfo>,
}

/// Information about how a namespace was restructured
#[derive(Clone, Debug)]
pub struct RestructureInfo {
    pub skip_root_key: Option<String>,
    pub name_field: Option<String>,
    pub original_entity_count: usize,
    pub restructured_entity_count: usize,
}

static RESTRUCTURED_ENTITIES: RwLock<Option<Arc<RestructuredEntities>>> = RwLock::new(None);

impl EntityRestructurer {
    /// Create a new EntityRestructurer
    pub fn new(game_data: &'static GameDataCache, type_cache: &'static TypeCache) -> Self {
        Self {
            game_data,
            type_cache,
        }
    }

    /// Add the original structural key to an entity as a special property
    /// This preserves the key information needed for subtype determination
    fn add_original_key_to_entity(&self, mut entity: Entity, original_key: &str) -> Entity {
        entity
            .properties
            .kv
            .entry(ORIGINAL_KEY_PROPERTY.to_string())
            .or_insert_with(PropertyInfoList::new)
            .0
            .push(PropertyInfo {
                operator: Operator::Equals,
                value: Value::String(original_key.to_string()),
            });

        entity
    }

    /// Extract the original key from an entity (if it was stored during restructuring)
    pub fn get_original_key_from_entity(entity: &Entity) -> Option<String> {
        if let Some(property_list) = entity.properties.kv.get(ORIGINAL_KEY_PROPERTY) {
            if let Some(first_property) = property_list.0.first() {
                return Some(first_property.value.to_string());
            }
        }
        None
    }

    /// Get the restructured entities result
    pub fn get() -> Option<Arc<RestructuredEntities>> {
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
            entities: LowerCaseHashMap::new(),
            restructured_namespaces: LowerCaseHashMap::new(),
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
            *cache = Some(Arc::new(restructured));
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
                let (entities, info) =
                    self.process_namespace(&namespace, &type_defs, namespace_data);

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
    fn get_types_needing_restructure(&self) -> LowerCaseHashMap<Vec<TypeDefinition>> {
        let mut result: LowerCaseHashMap<Vec<TypeDefinition>> = LowerCaseHashMap::new();

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

                    result
                        .entry(namespace)
                        .or_insert_with(Vec::new)
                        .push(type_def.clone());
                }
            }
        }

        result
    }

    /// Helper method to insert into a HashMap with duplicate detection using efficient counters
    fn insert_with_duplicate_warning(
        &self,
        map: &mut LowerCaseHashMap<Entity>,
        key_counters: &mut LowerCaseHashMap<u32>,
        key: String,
        entity: Entity,
        _context: &str,
        _namespace: &str,
    ) {
        if let Some(_existing) = map.get(&key) {
            // Key collision detected - generate unique key for the new entity
            let counter = key_counters.entry(key.clone()).or_insert(1);
            *counter += 1;
            let unique_key = format!("{}_{}", key, counter);

            map.insert(unique_key, entity);
            return;
        }

        map.insert(key, entity);
    }

    /// Helper method to extend a HashMap with duplicate detection
    fn extend_with_duplicate_warnings(
        &self,
        target: &mut LowerCaseHashMap<Entity>,
        key_counters: &mut LowerCaseHashMap<u32>,
        source: LowerCaseHashMap<Entity>,
        context: &str,
        namespace: &str,
    ) {
        for (key, entity) in source {
            self.insert_with_duplicate_warning(
                target,
                key_counters,
                key,
                entity,
                context,
                namespace,
            );
        }
    }

    /// Process a single namespace according to its type definition
    fn process_namespace(
        &self,
        namespace: &str,
        type_defs: &Vec<TypeDefinition>,
        namespace_data: &Namespace,
    ) -> (LowerCaseHashMap<Entity>, RestructureInfo) {
        let mut restructured_entities = LowerCaseHashMap::new();
        let mut key_counters: LowerCaseHashMap<u32> = LowerCaseHashMap::new();
        let mut original_count = 0;

        // Process each module in the namespace individually to avoid key overwrites
        for (_module_name, module) in &namespace_data.modules {
            // Process each property in the module
            for (key, property_list) in &module.properties.kv {
                // Process all properties, not just the first one, to handle duplicate keys
                for property_info in &property_list.0 {
                    if let Value::Entity(entity) = &property_info.value {
                        original_count += 1;

                        // Check if any type definition wants to skip this root key
                        let skip_root_type_defs: Vec<&TypeDefinition> = type_defs
                            .iter()
                            .filter(|type_def| {
                                self.should_skip_root_key(key, &type_def.skip_root_key)
                            })
                            .collect();

                        if !skip_root_type_defs.is_empty() {
                            // Skip this root key and process nested entities using all applicable type definitions
                            let extracted_entities = self
                                .extract_entities_from_container_with_multiple_types(
                                    entity,
                                    &skip_root_type_defs,
                                    namespace,
                                );
                            self.extend_with_duplicate_warnings(
                                &mut restructured_entities,
                                &mut key_counters,
                                extracted_entities,
                                &format!("skip_root_key from container '{}'", key),
                                namespace,
                            );
                        } else {
                            // Get type definitions that are applicable for this specific entity
                            let applicable_type_defs =
                                self.get_applicable_type_defs(key, type_defs);

                            // if applicable_type_defs.is_empty() {
                            //     eprintln!(
                            //         "WARN: No applicable type definitions found for entity: {}",
                            //         key
                            //     );
                            // }

                            // Check if any applicable type definition has a name_field
                            if let Some(name_field_type_def) = applicable_type_defs
                                .iter()
                                .find(|type_def| type_def.name_field.is_some())
                            {
                                // Use name field to determine entity key, but don't skip root
                                if let Some(entity_name) = Self::extract_name_from_entity(
                                    entity,
                                    &name_field_type_def.name_field,
                                ) {
                                    // Add original key to entity to preserve subtype information
                                    let entity_with_original_key =
                                        self.add_original_key_to_entity(entity.clone(), key);
                                    self.insert_with_duplicate_warning(
                                        &mut restructured_entities,
                                        &mut key_counters,
                                        entity_name,
                                        entity_with_original_key,
                                        &format!("name_field from key '{}'", key),
                                        namespace,
                                    );
                                } else {
                                    // Fallback to original key if name field not found
                                    self.insert_with_duplicate_warning(
                                        &mut restructured_entities,
                                        &mut key_counters,
                                        key.clone(),
                                        entity.clone(),
                                        &format!("name_field fallback from key '{}'", key),
                                        namespace,
                                    );
                                }
                            } else {
                                // Standard processing - use the key as-is
                                self.insert_with_duplicate_warning(
                                    &mut restructured_entities,
                                    &mut key_counters,
                                    key.clone(),
                                    entity.clone(),
                                    &format!("standard processing from key '{}'", key),
                                    namespace,
                                );
                            }
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
            Some(SkipRootKey::Specific(skip_key)) => key.to_lowercase() == skip_key.to_lowercase(),
            Some(SkipRootKey::Any) => true,
            Some(SkipRootKey::Except(exceptions)) => !exceptions
                .iter()
                .any(|exception| exception.to_lowercase() == key.to_lowercase()),
            Some(SkipRootKey::Multiple(keys)) => {
                keys.iter().any(|k| k.to_lowercase() == key.to_lowercase())
            }
            None => false,
        }
    }

    /// Check if a key passes the type_key_filter
    fn passes_type_key_filter(&self, key: &str, type_defs: &Vec<TypeDefinition>) -> bool {
        // If no type_key_filter is defined, allow all keys
        let type_key_filters: Vec<&TypeKeyFilter> = type_defs
            .iter()
            .filter_map(|type_def| type_def.rule_options.type_key_filter.as_ref())
            .collect();

        if type_key_filters.is_empty() {
            return true;
        }

        // Check if the key matches any of the type_key_filters
        for filter in &type_key_filters {
            if self.matches_type_key_filter(key, filter) {
                return true;
            }
        }

        false
    }

    /// Check if a key matches a specific type_key_filter
    fn matches_type_key_filter(&self, key: &str, filter: &TypeKeyFilter) -> bool {
        match filter {
            TypeKeyFilter::Specific(required_key) => {
                key.to_lowercase() == required_key.to_lowercase()
            }
            TypeKeyFilter::OneOf(required_keys) => required_keys
                .iter()
                .any(|k| k.to_lowercase() == key.to_lowercase()),
            TypeKeyFilter::Not(excluded_key) => key.to_lowercase() != excluded_key.to_lowercase(),
        }
    }

    /// Get the type definitions that are applicable for a given entity based on type_key_filter
    fn get_applicable_type_defs<'a>(
        &self,
        key: &str,
        type_defs: &'a Vec<TypeDefinition>,
    ) -> Vec<&'a TypeDefinition> {
        type_defs
            .iter()
            .filter(|type_def| {
                // If no type_key_filter, this type definition applies to all entities
                if let Some(filter) = &type_def.rule_options.type_key_filter {
                    // Check if ANY key within the entity matches the type_key_filter
                    self.matches_type_key_filter(key, filter)
                } else {
                    true
                }
            })
            .collect()
    }

    /// Extract entities from a container entity using multiple type definitions (when skipping root key)
    fn extract_entities_from_container_with_multiple_types(
        &self,
        container_entity: &Entity,
        type_defs: &Vec<&TypeDefinition>,
        namespace: &str,
    ) -> LowerCaseHashMap<Entity> {
        let mut result = LowerCaseHashMap::new();
        let mut key_counters: LowerCaseHashMap<u32> = LowerCaseHashMap::new();

        // Look for child entities in the container
        for (child_key, child_property_list) in &container_entity.properties.kv {
            for property_info in &child_property_list.0 {
                if let Value::Entity(child_entity) = &property_info.value {
                    // Find the first type definition that matches this child entity
                    if let Some(matching_type_def) = type_defs.iter().find(|type_def| {
                        self.passes_type_key_filter(child_key, &vec![(**type_def).clone()])
                    }) {
                        // Determine the entity name based on whether we have a name field
                        let entity_name = if matching_type_def.name_field.is_some() {
                            // Use name field if available, fallback to structural key
                            Self::extract_name_from_entity(
                                child_entity,
                                &matching_type_def.name_field,
                            )
                            .unwrap_or_else(|| child_key.clone())
                        } else {
                            // No name field, use the structural key
                            child_key.clone()
                        };

                        // Add original key to entity to preserve subtype information
                        let entity_with_original_key =
                            self.add_original_key_to_entity(child_entity.clone(), child_key);
                        self.insert_with_duplicate_warning(
                            &mut result,
                            &mut key_counters,
                            entity_name,
                            entity_with_original_key,
                            &format!("extracted from container child '{}'", child_key),
                            namespace,
                        );
                    }
                    // If no type definition matches, skip this entity (it will be handled by normal processing)
                }
            }
        }

        result
    }

    /// Extract the name field value from an entity
    fn extract_name_from_entity(entity: &Entity, name_field: &Option<String>) -> Option<String> {
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
        let namespace = TypeCache::get_actual_namespace(namespace);

        // Check restructured entities first
        if let Some(restructured) = Self::get() {
            if let Some(entities) = restructured.entities.get(namespace) {
                if let Some(entity) = entities.get(entity_name) {
                    return Some(entity.clone());
                }
            }
        }

        // Check mod data
        if let Some(entity) = ModDataCache::get_entity(namespace, entity_name) {
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
    pub fn get_namespace_entities_map(namespace: &str) -> Option<LowerCaseHashMap<Entity>> {
        let namespace = TypeCache::get_actual_namespace(namespace);
        Self::get()?.entities.get(namespace).cloned()
    }

    /// Check if a namespace was restructured
    pub fn was_restructured(namespace: &str) -> bool {
        let namespace = TypeCache::get_actual_namespace(namespace);
        Self::get()
            .map(|r| r.restructured_namespaces.contains_key(namespace))
            .unwrap_or(false)
    }

    /// Get restructure info for a namespace
    pub fn get_restructure_info(namespace: &str) -> Option<RestructureInfo> {
        let namespace = TypeCache::get_actual_namespace(namespace);
        Self::get()?.restructured_namespaces.get(namespace).cloned()
    }

    /// Get entity keys for a namespace, using restructured keys if available
    pub fn get_namespace_entity_keys(namespace: &str) -> Vec<String> {
        let mut all_keys = HashSet::new();
        let namespace = TypeCache::get_actual_namespace(namespace);

        // Add restructured entity keys if available
        if let Some(restructured) = Self::get() {
            if let Some(entities) = restructured.entities.get(namespace) {
                all_keys.extend(entities.keys().cloned());
            }
        }

        // Add mod entity keys
        if let Some(mod_keys) = ModDataCache::get_namespace_entity_keys(namespace) {
            all_keys.extend(mod_keys.iter().cloned());
        }

        // Add original entity keys from GameDataCache
        if let Some(game_data) = GameDataCache::get() {
            if let Some(original_keys) = game_data.get_namespace_entity_keys(namespace) {
                all_keys.extend(original_keys.iter().cloned());
            }
        }

        all_keys.into_iter().collect()
    }

    /// Get entities for a namespace as a vector of (key, entity) tuples
    pub fn get_namespace_entities(namespace: &str) -> Option<Vec<(String, Entity)>> {
        let mut all_entities = HashMap::new();
        let namespace = TypeCache::get_actual_namespace(namespace);

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
        let namespace = TypeCache::get_actual_namespace(namespace);

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
        let namespace = TypeCache::get_actual_namespace(namespace);

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
    pub fn get_all_namespace_entities(namespace: &str) -> Option<LowerCaseHashMap<Entity>> {
        let namespace = TypeCache::get_actual_namespace(namespace);

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

    pub fn get_namespace_values(namespace: &str) -> Option<Vec<String>> {
        let namespace = TypeCache::get_actual_namespace(namespace);

        if let Some(game_data) = GameDataCache::get() {
            if let Some(namespace_data) = game_data.get_namespaces().get(namespace) {
                Some(namespace_data.values.clone())
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
    pub fn get_namespace_scripted_variables(namespace: &str) -> Option<LowerCaseHashMap<Value>> {
        let mut all_variables = LowerCaseHashMap::new();
        let namespace = TypeCache::get_actual_namespace(namespace);
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
        let namespace = TypeCache::get_actual_namespace(namespace);
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
    pub fn get_all_entities_map(namespace: &str) -> Option<LowerCaseHashMap<Entity>> {
        let mut all_entities = LowerCaseHashMap::new();
        let namespace = TypeCache::get_actual_namespace(namespace);
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
        let mod_namespaces = ModDataCache::get_namespaces();
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

    /// Convert an AstEntity to Entity and apply restructuring logic for subtype narrowing
    /// Returns (effective_entity_key, restructured_entity) that can be used for correct subtype determination
    pub fn get_effective_entity_for_subtype_narrowing(
        namespace: &str,
        container_key: &str,
        entity_key: &str,
        ast_entity: &AstEntity,
    ) -> (String, Entity) {
        let container_entity = entity_from_ast(ast_entity);
        let namespace = TypeCache::get_actual_namespace(namespace);

        // Get type definitions for this namespace to check for skip_root_key and name_field
        if let Some(type_cache) = TypeCache::get() {
            let mut applicable_type_defs = Vec::new();

            for (_type_name, type_def) in type_cache.get_cwt_analyzer().get_types() {
                if let Some(path) = &type_def.path {
                    let type_namespace = if let Some(stripped) = path.strip_prefix("game/") {
                        stripped.to_string()
                    } else {
                        path.clone()
                    };

                    if type_namespace == namespace
                        && (type_def.skip_root_key.is_some() || type_def.name_field.is_some())
                    {
                        applicable_type_defs.push(type_def);
                    }
                }
            }

            // Check if this container_key matches any skip_root_key
            if let Some(matching_type_def) =
                applicable_type_defs
                    .iter()
                    .find(|type_def| match &type_def.skip_root_key {
                        Some(SkipRootKey::Specific(skip_key)) => {
                            container_key.to_lowercase() == skip_key.to_lowercase()
                        }
                        Some(SkipRootKey::Any) => true,
                        Some(SkipRootKey::Except(exceptions)) => {
                            !exceptions.iter().any(|exception| {
                                exception.to_lowercase() == container_key.to_lowercase()
                            })
                        }
                        Some(SkipRootKey::Multiple(keys)) => keys
                            .iter()
                            .any(|k| k.to_lowercase() == container_key.to_lowercase()),
                        None => false,
                    })
            {
                // This is a skipped container scenario
                if entity_key == container_key {
                    // We're being asked to process the container itself, but we should extract nested entities
                    // Return the first valid nested entity
                    for (child_key, child_property_list) in &container_entity.properties.kv {
                        for property_info in &child_property_list.0 {
                            if let Value::Entity(mut child_entity) = property_info.value.clone() {
                                // Determine the effective key based on name field
                                let effective_key =
                                    if let Some(name_field) = &matching_type_def.name_field {
                                        Self::extract_name_from_entity(
                                            &child_entity,
                                            &Some(name_field.clone()),
                                        )
                                        .unwrap_or_else(|| child_key.clone())
                                    } else {
                                        child_key.clone()
                                    };

                                // Add original key to entity for subtype determination
                                Self::add_original_key_to_entity_static(
                                    &mut child_entity,
                                    child_key,
                                );

                                return (effective_key, child_entity);
                            }
                        }
                    }
                } else {
                    // We're being asked to extract a specific nested entity
                    if let Some(property_list) = container_entity.properties.kv.get(entity_key) {
                        if let Some(property_info) = property_list.0.first() {
                            if let Value::Entity(mut nested_entity) = property_info.value.clone() {
                                // Determine the effective key based on name field
                                let effective_key =
                                    if let Some(name_field) = &matching_type_def.name_field {
                                        Self::extract_name_from_entity(
                                            &nested_entity,
                                            &Some(name_field.clone()),
                                        )
                                        .unwrap_or_else(|| entity_key.to_string())
                                    } else {
                                        entity_key.to_string()
                                    };

                                // Add original key to entity for subtype determination
                                Self::add_original_key_to_entity_static(
                                    &mut nested_entity,
                                    entity_key,
                                );

                                return (effective_key, nested_entity);
                            }
                        }
                    }
                }

                // Fallback if we can't extract any nested entity
                let mut fallback_entity = container_entity;
                Self::add_original_key_to_entity_static(&mut fallback_entity, entity_key);
                return (entity_key.to_string(), fallback_entity);
            } else {
                // Check for name_field scenarios (without skip_root_key)
                if let Some(name_field_type_def) = applicable_type_defs.iter().find(|type_def| {
                    type_def.name_field.is_some() && type_def.skip_root_key.is_none()
                }) {
                    // Name field scenario - use name field for key, add original key to entity
                    let mut entity = container_entity;
                    let effective_key =
                        Self::extract_name_from_entity(&entity, &name_field_type_def.name_field)
                            .unwrap_or_else(|| entity_key.to_string());

                    // Add original key to entity for subtype determination
                    Self::add_original_key_to_entity_static(&mut entity, entity_key);

                    return (effective_key, entity);
                }
            }
        }

        // No restructuring applies, but still add original key for subtype determination
        let mut entity = container_entity;
        Self::add_original_key_to_entity_static(&mut entity, entity_key);
        (entity_key.to_string(), entity)
    }

    /// Static version of add_original_key_to_entity for use in static contexts
    fn add_original_key_to_entity_static(entity: &mut Entity, original_key: &str) {
        entity
            .properties
            .kv
            .entry(ORIGINAL_KEY_PROPERTY.to_string())
            .or_insert_with(PropertyInfoList::new)
            .0
            .push(PropertyInfo {
                operator: Operator::Equals,
                value: Value::String(original_key.to_string()),
            });
    }

    /// Helper to get restructuring info for a namespace without needing the full cache
    pub fn get_namespace_restructure_info(namespace: &str) -> Option<RestructureInfo> {
        if !TypeCache::is_initialized() || !GameDataCache::is_initialized() {
            return None;
        }

        let type_cache = TypeCache::get()?;

        // Get type definitions that need restructuring for this namespace
        for (_type_name, type_def) in type_cache.get_cwt_analyzer().get_types() {
            if type_def.skip_root_key.is_some() || type_def.name_field.is_some() {
                if let Some(path) = &type_def.path {
                    let ns = if let Some(stripped) = path.strip_prefix("game/") {
                        stripped.to_string()
                    } else {
                        path.clone()
                    };

                    if ns == namespace {
                        return Some(RestructureInfo {
                            skip_root_key: type_def.skip_root_key.as_ref().and_then(|s| match s {
                                SkipRootKey::Specific(key) => Some(key.clone()),
                                _ => None,
                            }),
                            name_field: type_def.name_field.clone(),
                            original_entity_count: 0, // Not relevant for this use case
                            restructured_entity_count: 0, // Not relevant for this use case
                        });
                    }
                }
            }
        }

        None
    }
}
