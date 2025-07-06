use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::{Client, LspService, Server};

mod handlers;
mod semantic_token_collector;

use handlers::document_cache::DocumentCache;
use handlers::type_cache::TypeCache;

struct CwLspServer {
    client: Client,
    documents: Arc<RwLock<HashMap<String, String>>>,
    document_cache: DocumentCache,
}

impl CwLspServer {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            document_cache: DocumentCache::new(),
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize type cache with Stellaris data in the background
    TypeCache::initialize_in_background();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| CwLspServer::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
