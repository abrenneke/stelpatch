use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::Client;

pub mod handlers;
pub mod semantic_token_collector;

use handlers::document_cache::DocumentCache;

pub struct CwLspServer {
    client: Client,
    documents: Arc<RwLock<HashMap<String, String>>>,
    document_cache: DocumentCache,
}

impl CwLspServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            document_cache: DocumentCache::new(),
        }
    }
}
