use std::collections::HashSet;
use std::sync::Arc;
use std::{collections::HashMap, sync::OnceLock};

use cw_games::stellaris::BaseGame;
use cw_model::{Entity, LoadMode, Value};

/// Cache for actual game data keys from namespaces (e.g., "energy", "minerals" from resources namespace)
pub struct GameDataCache {
    /// Maps namespace -> set of keys defined in that namespace
    namespaces: HashMap<String, Namespace>,
    scripted_variables: HashMap<String, Value>,
}

pub struct Namespace {
    pub entities: HashMap<String, Entity>,
    pub entity_keys: Vec<String>,
    pub entity_keys_set: Arc<HashSet<String>>,
    pub scripted_variables: HashMap<String, Value>,
}

impl Namespace {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            entity_keys: Vec::new(),
            entity_keys_set: Arc::new(HashSet::new()),
            scripted_variables: HashMap::new(),
        }
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
            let base_game = BaseGame::load_global_as_mod_definition(LoadMode::Parallel);

            eprintln!(
                "Building namespace keys cache from {} namespaces",
                base_game.namespaces.len()
            );

            let mut global_scripted_variables = HashMap::new();

            // Extract keys from each namespace
            let mut namespaces = HashMap::new();
            for (namespace_name, namespace) in &base_game.namespaces {
                let mut namespace_data = Namespace::new();

                let properties = &namespace.properties.kv;

                for (key, value) in properties {
                    if namespace_name == "common/scripted_variables" {
                        global_scripted_variables
                            .insert(key.to_string(), value.0.first().unwrap().value.clone());
                    } else if key.starts_with("@") {
                        namespace_data
                            .scripted_variables
                            .insert(key.to_string(), value.0.first().unwrap().value.clone());
                    } else {
                        if let Some(entity) = value.0.first().unwrap().value.as_entity() {
                            namespace_data
                                .entities
                                .insert(key.to_string(), entity.clone());
                        }
                    }
                }

                // Pre-compute entity keys for fast access
                namespace_data.entity_keys = namespace_data.entities.keys().cloned().collect();
                namespace_data.entity_keys_set =
                    Arc::new(namespace_data.entities.keys().cloned().collect());

                namespaces.insert(namespace_name.clone(), namespace_data);
            }

            eprintln!(
                "Built game data cache with {} namespaces and {} scripted variables",
                namespaces.len(),
                global_scripted_variables.len()
            );

            GameDataCache {
                namespaces,
                scripted_variables: global_scripted_variables,
            }
        })
    }

    /// Get all keys defined in a namespace
    pub fn get_namespace_entity_keys(&self, namespace: &str) -> Option<&Vec<String>> {
        if let Some(namespace) = self.namespaces.get(namespace) {
            Some(&namespace.entity_keys)
        } else {
            None
        }
    }

    /// Get all keys defined in a namespace as a HashSet (for LiteralSet)
    pub fn get_namespace_entity_keys_set(&self, namespace: &str) -> Option<Arc<HashSet<String>>> {
        if let Some(namespace) = self.namespaces.get(namespace) {
            Some(namespace.entity_keys_set.clone())
        } else {
            None
        }
    }

    /// Get all namespaces
    pub fn get_namespaces(&self) -> &HashMap<String, Namespace> {
        &self.namespaces
    }

    /// Check if the game data cache is initialized
    pub fn is_initialized() -> bool {
        GAME_DATA_CACHE.get().is_some()
    }
}
