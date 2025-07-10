use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::Client;

mod diagnostic;
mod key;
mod provider;
mod structural;
mod type_validation;
mod util;
mod value;

/// Generate diagnostics for a document (convenience function)
pub async fn generate_diagnostics(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    uri: &str,
) {
    let provider = provider::DiagnosticsProvider::new(client, documents.clone());
    provider.generate_diagnostics(uri).await;
}
