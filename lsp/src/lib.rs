use cw_model::GameMod;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tower_lsp::Client;

pub mod base_game;
pub mod handlers;
pub mod interner;
pub mod semantic_token_collector;

use handlers::cache::game_data::ModDataCache;
use handlers::document_cache::DocumentCache;

pub struct CwLspServer {
    client: Client,
    documents: Arc<RwLock<HashMap<String, String>>>,
    document_cache: DocumentCache,
    mod_cache: Arc<RwLock<HashMap<PathBuf, GameMod>>>,
}

impl CwLspServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            document_cache: DocumentCache::new(),
            mod_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_cached_mod(&self, mod_path: &PathBuf) -> Option<GameMod> {
        let cache = self.mod_cache.read().unwrap();
        cache.get(mod_path).cloned()
    }

    pub fn cache_mod(&self, mod_path: PathBuf, game_mod: GameMod) {
        let mut cache = self.mod_cache.write().unwrap();
        cache.insert(mod_path, game_mod);
    }

    pub fn merge_mod_data(&self, game_mod: &GameMod) {
        ModDataCache::merge_mod_data(game_mod);
    }
}
