use cw_games::stellaris::BaseGame;
use cw_model::{CaseInsensitiveInterner, LoadMode};
use std::time::Instant;

pub fn main() {
    println!("Starting to load Stellaris base game...");
    let start_time = Instant::now();

    let interner = CaseInsensitiveInterner::new();
    let loaded_mod =
        BaseGame::load_global_as_mod_definition(LoadMode::Parallel, &interner, None, false);

    let load_duration = start_time.elapsed();
    println!("Loading completed in: {:?}", load_duration);

    let num_namespaces = loaded_mod.namespaces.len();
    let num_modules = loaded_mod
        .namespaces
        .values()
        .map(|ns| ns.modules.len())
        .sum::<usize>();

    let num_properties = loaded_mod
        .namespaces
        .values()
        .map(|ns| ns.properties.kv.len())
        .sum::<usize>();

    println!(
        "Loaded {} namespaces with {} modules and {} properties",
        num_namespaces, num_modules, num_properties
    );
}
