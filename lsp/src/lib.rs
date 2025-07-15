use cw_model::GameMod;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::Client;

pub mod handlers;
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

    pub async fn get_cached_mod(&self, mod_path: &PathBuf) -> Option<GameMod> {
        let cache = self.mod_cache.read().await;
        cache.get(mod_path).cloned()
    }

    pub async fn cache_mod(&self, mod_path: PathBuf, game_mod: GameMod) {
        let mut cache = self.mod_cache.write().await;
        cache.insert(mod_path, game_mod);
    }

    pub async fn merge_mod_data(&self, game_mod: &GameMod) {
        ModDataCache::merge_mod_data(game_mod);
    }
}
