use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Instant;

use crate::base_game::BaseGame;
use crate::interner::get_interner;
use cw_model::Module;
use cw_model::SpurMap;
use cw_model::{Entity, GameMod, LoadMode, Value};
use lasso::Spur;

use crate::handlers::cache::EntityRestructurer;
use crate::handlers::cache::FullAnalysis;
use crate::handlers::cache::TypeCache;

/// Cache for actual game data keys from namespaces (e.g., "energy", "minerals" from resources namespace)
pub struct GameDataCache {
    /// Maps namespace -> set of keys defined in that namespace
    pub namespaces: SpurMap<Namespace>,
    pub scripted_variables: SpurMap<Value>,
}

#[derive(Clone)]
pub struct Namespace {
    pub entities: SpurMap<Entity>,
    pub values: Vec<Spur>,
    pub entity_keys: Vec<Spur>,
    pub entity_keys_set: Arc<HashSet<Spur>>,
    pub scripted_variables: SpurMap<Value>,

    /// Individual modules in this namespace (for restructuring)
    pub modules: HashMap<String, Arc<Module>>,
}

impl Namespace {
    pub fn new() -> Self {
        Self {
            entities: SpurMap::new(),
            values: Vec::new(),
            entity_keys: Vec::new(),
            entity_keys_set: Arc::new(HashSet::new()),
            scripted_variables: SpurMap::new(),
            modules: HashMap::new(),
        }
    }

    /// Update the entity keys and set after modifications
    pub fn update_keys(&mut self) {
        self.entity_keys = self.entities.keys().collect();
        self.entity_keys_set = Arc::new(self.entities.keys().collect());
    }
}

static GAME_DATA_CACHE: OnceLock<GameDataCache> = OnceLock::new();

impl GameDataCache {
    /// Initialize the game data cache by loading Stellaris base game data
    pub fn initialize_in_background() {
        // This runs in a background task since it can take time
        std::thread::spawn(|| {
            let _ = Self::get_or_init_blocking();
        });
    }

    pub fn get() -> Option<&'static GameDataCache> {
        GAME_DATA_CACHE.get()
    }

    /// Get or initialize the global game data cache (blocking version)
    fn get_or_init_blocking() -> &'static GameDataCache {
        GAME_DATA_CACHE.get_or_init(|| {
            eprintln!("Initializing game data cache");

            // Load base game data
            let base_game =
                BaseGame::load_global_as_mod_definition(LoadMode::Parallel, get_interner());

            let interner = get_interner();

            eprintln!(
                "Building namespace keys cache from {} namespaces",
                base_game.namespaces.len()
            );

            let mut global_scripted_variables: SpurMap<Value> = SpurMap::new();

            // Extract keys from each namespace
            let mut namespaces: SpurMap<Namespace> = SpurMap::new();
            for (namespace_name, namespace) in &base_game.namespaces {
                let mut namespace_data = Namespace::new();

                // Store individual modules for restructuring
                namespace_data.modules = namespace.modules.clone();

                let properties = &namespace.properties.kv;

                for (key, value) in properties {
                    let key_str = get_interner().resolve(&key);

                    if namespace_name == "common/scripted_variables" {
                        global_scripted_variables
                            .insert(key, value.0.first().unwrap().value.clone());
                    } else if key_str.starts_with("@") {
                        namespace_data
                            .scripted_variables
                            .insert(key, value.0.first().unwrap().value.clone());
                    } else {
                        // Handle multiple entities with the same key (like multiple random_list entries)
                        for (index, property_info) in value.0.iter().enumerate() {
                            if let Some(entity) = property_info.value.as_entity() {
                                let entity_key = if index == 0 {
                                    key_str.to_string()
                                } else {
                                    format!("{}_{}", key_str, index + 1)
                                };

                                namespace_data
                                    .entities
                                    .insert(interner.get_or_intern(entity_key), entity.clone());
                            }
                        }
                    }
                }

                for value in &namespace.values {
                    if let Value::String(string) = value {
                        namespace_data.values.push(string.clone());
                    }
                }

                let namespace_name =
                    TypeCache::get_actual_namespace(interner.get_or_intern(namespace_name));

                if let Some(existing) = namespaces.get_mut(&namespace_name) {
                    existing.entities.extend(namespace_data.entities);
                    existing.values.extend(namespace_data.values);
                    existing
                        .scripted_variables
                        .extend(namespace_data.scripted_variables);
                    existing.modules.extend(namespace_data.modules);
                } else {
                    namespaces.insert(namespace_name, namespace_data);
                }
            }

            for namespace in namespaces.values_mut() {
                namespace.update_keys();
            }

            eprintln!(
                "Built game data cache with {} namespaces and {} scripted variables",
                namespaces.len(),
                global_scripted_variables.len()
            );

            let mut cache = GameDataCache {
                namespaces,
                scripted_variables: global_scripted_variables,
            };

            // Load modifiers and integrate them into the cache
            let start = Instant::now();

            if let Err(e) = crate::handlers::modifiers::integrate_modifiers_into_cache(&mut cache) {
                eprintln!("Warning: Failed to load modifiers: {}", e);
            } else {
                eprintln!("Loaded modifiers into cache in {:?}", start.elapsed());
            }

            cache
        })
    }

    /// Get all keys defined in a namespace
    pub fn get_namespace_entity_keys(&self, namespace: Spur) -> Option<&Vec<Spur>> {
        if let Some(namespace) = self.namespaces.get(&namespace) {
            Some(&namespace.entity_keys)
        } else {
            None
        }
    }

    /// Get all keys defined in a namespace as a HashSet (for LiteralSet)
    pub fn get_namespace_entity_keys_set(&self, namespace: Spur) -> Option<Arc<HashSet<Spur>>> {
        if let Some(namespace) = self.namespaces.get(&namespace) {
            Some(namespace.entity_keys_set.clone())
        } else {
            None
        }
    }

    /// Get all namespaces
    pub fn get_namespaces(&self) -> &SpurMap<Namespace> {
        &self.namespaces
    }

    /// Check if the game data cache is initialized
    pub fn is_initialized() -> bool {
        GAME_DATA_CACHE.get().is_some()
    }
}

/// Global cache for mod data that can be modified at runtime
pub struct ModDataCache {
    /// Maps namespace -> set of keys defined in that namespace
    pub namespaces: SpurMap<Namespace>,
    pub scripted_variables: SpurMap<Value>,
}

static MOD_DATA_CACHE: OnceLock<RwLock<ModDataCache>> = OnceLock::new();

impl ModDataCache {
    /// Get the global mod data cache
    pub fn get() -> &'static RwLock<ModDataCache> {
        MOD_DATA_CACHE.get_or_init(|| {
            RwLock::new(ModDataCache {
                namespaces: SpurMap::new(),
                scripted_variables: SpurMap::new(),
            })
        })
    }

    /// Merge mod data into the cache and trigger restructuring
    pub fn merge_mod_data(game_mod: &GameMod) {
        let cache_lock = Self::get();
        let mut cache = cache_lock.write().unwrap();

        eprintln!("Merging mod data: {}", game_mod.definition.name);
        let interner = get_interner();

        let mut added_entities = 0;
        let mut added_variables = 0;

        // Process each namespace in the mod
        for (namespace_name, namespace) in &game_mod.namespaces {
            let properties = &namespace.properties.kv;

            for (key, value) in properties {
                let key_str = get_interner().resolve(&key);

                if namespace_name == "common/scripted_variables" {
                    cache
                        .scripted_variables
                        .insert(key, value.0.first().unwrap().value.clone());
                    added_variables += 1;
                } else if key_str.starts_with("@") {
                    let namespace_data = cache
                        .namespaces
                        .entry(interner.get_or_intern(namespace_name))
                        .or_insert_with(Namespace::new);
                    namespace_data
                        .scripted_variables
                        .insert(key, value.0.first().unwrap().value.clone());
                    added_variables += 1;
                } else {
                    if let Some(entity) = value.0.first().unwrap().value.as_entity() {
                        let namespace_data = cache
                            .namespaces
                            .entry(interner.get_or_intern(namespace_name))
                            .or_insert_with(Namespace::new);
                        namespace_data
                            .entities
                            .insert(interner.get_or_intern(key_str), entity.clone());
                        added_entities += 1;
                    }
                }
            }
        }

        // Update keys for all modified namespaces
        for namespace_data in cache.namespaces.values_mut() {
            namespace_data.update_keys();
        }

        eprintln!(
            "Merged mod '{}': {} entities, {} variables across {} namespaces",
            game_mod.definition.name,
            added_entities,
            added_variables,
            game_mod.namespaces.len()
        );

        // Drop the lock before triggering restructuring
        drop(cache);

        // Trigger entity restructuring to include the new mod data
        Self::trigger_restructuring();
    }

    /// Trigger entity restructuring and full analysis to include mod data
    fn trigger_restructuring() {
        // Reset the EntityRestructurer so it will reload with the new mod data
        // This is a simple approach - in a more sophisticated system we might
        // incrementally update the restructurer
        eprintln!("Triggering entity restructuring and full analysis to include mod data");

        // Reset the EntityRestructurer cache to force re-initialization
        // with the new mod data included
        EntityRestructurer::reset();
        FullAnalysis::reset();

        // The next time EntityRestructurer and FullAnalysis methods are called, they will
        // automatically reload and include the new mod data
    }

    /// Get all keys defined in a namespace from mod data only
    pub fn get_namespace_entity_keys(namespace: Spur) -> Option<Vec<Spur>> {
        let cache = Self::get().read().unwrap();
        if let Some(mod_namespace) = cache.namespaces.get(&namespace) {
            Some(mod_namespace.entity_keys.clone())
        } else {
            None
        }
    }

    /// Get all keys defined in a namespace as a HashSet from mod data only
    pub fn get_namespace_entity_keys_set(namespace: Spur) -> Option<Arc<HashSet<Spur>>> {
        let cache = Self::get().read().unwrap();
        if let Some(mod_namespace) = cache.namespaces.get(&namespace) {
            Some(mod_namespace.entity_keys_set.clone())
        } else {
            None
        }
    }

    /// Get a specific entity from mod data only
    pub fn get_entity(namespace: Spur, entity_name: Spur) -> Option<Entity> {
        let cache = Self::get().read().unwrap();
        if let Some(mod_namespace) = cache.namespaces.get(&namespace) {
            mod_namespace.entities.get(&entity_name).cloned()
        } else {
            None
        }
    }

    /// Get all namespaces from mod data
    pub fn get_namespaces() -> SpurMap<Namespace> {
        let cache = Self::get().read().unwrap();
        cache.namespaces.clone()
    }

    /// Get scripted variables from mod data
    pub fn get_scripted_variables() -> SpurMap<Value> {
        let cache = Self::get().read().unwrap();
        cache.scripted_variables.clone()
    }

    /// Get namespace scripted variables from mod data
    pub fn get_namespace_scripted_variables(namespace: Spur) -> Option<SpurMap<Value>> {
        let cache = Self::get().read().unwrap();
        if let Some(mod_namespace) = cache.namespaces.get(&namespace) {
            Some(mod_namespace.scripted_variables.clone())
        } else {
            None
        }
    }
}
