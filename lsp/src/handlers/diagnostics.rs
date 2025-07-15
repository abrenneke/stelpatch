use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tower_lsp::Client;

pub mod diagnostic;
pub mod provider;
pub mod structural;
pub mod type_validation;
pub mod util;
pub mod value;

/// Generate diagnostics for a document (convenience function)
pub async fn generate_diagnostics(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    uri: &str,
) {
    let provider = provider::DiagnosticsProvider::new(documents.clone());
    let client_provider = provider::ClientDiagnosticsProvider::new(client, provider);
    client_provider.generate_diagnostics(uri).await;
}
