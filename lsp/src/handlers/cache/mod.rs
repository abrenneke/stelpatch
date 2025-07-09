// Sub-modules
mod api;
mod core;
mod formatter;
mod resolver;
mod types;

// Re-exports for public API
pub use api::{get_entity_property_type, get_namespace_entity_type};
pub use core::{GameDataCache, TypeCache};
pub use resolver::{resolve_type, resolve_type_with_display_info};
