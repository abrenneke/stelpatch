use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tower_lsp::Client;
use url::Url;

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
    let provider = provider::DiagnosticsProvider::new(documents.clone(), true);
    let diagnostics = provider.generate_diagnostics(uri);

    // Publish diagnostics to the client
    client
        .publish_diagnostics(Url::parse(uri).unwrap(), diagnostics, None)
        .await;
}
