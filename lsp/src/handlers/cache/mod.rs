// Sub-modules
mod api;
mod collector;
mod core;
mod entity_restructurer;
mod formatter;
mod full_analysis;
mod game_data;
mod resolver;
mod types;

// Re-exports for public API
pub use api::{get_entity_property_type, get_namespace_entity_type};
pub use collector::*;
pub use core::*;
pub use entity_restructurer::*;
pub use full_analysis::*;
pub use game_data::*;
