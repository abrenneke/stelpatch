// Sub-modules
mod api;
mod collector;
mod core;
mod entity_restructurer;
mod file_index;
mod formatter;
mod full_analysis;
pub mod game_data;
mod resolver;
mod resolver_modules;
pub mod types;

// Re-exports for public API
pub use api::{get_entity_property_type_from_ast, get_namespace_entity_type};
pub use collector::*;
pub use core::*;
pub use entity_restructurer::*;
pub use file_index::{
    FileIndex, add_mod_to_index, add_mods_to_index, file_exists, find_files_containing,
    find_files_with_extension, initialize_file_index,
};
pub use formatter::TypeFormatter;
pub use full_analysis::*;
pub use game_data::*;
pub use resolver_modules::*;
