use std::collections::HashMap;
use std::env;

use cw_games::stellaris::BaseGame;
use cw_model::{LoadMode, TypeGenerator, TypeInferenceEngine};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <namespace>", args[0]);
        eprintln!("       {} --all", args[0]);
        eprintln!("Example: {} common/buildings", args[0]);
        eprintln!("         {} --all", args[0]);
        std::process::exit(1);
    }

    let target_arg = &args[1];
    let process_all = target_arg == "--all";

    let game_mod = BaseGame::load_global_as_mod_definition(LoadMode::Parallel);

    if process_all {
        // Process all namespaces
        let mut results = HashMap::new();

        for (namespace_name, namespace) in &game_mod.namespaces {
            // Initialize type inference engine for this namespace
            let mut type_engine = TypeInferenceEngine::new();

            // Process all modules in the namespace
            let modules: Vec<_> = namespace.modules.values().collect();
            type_engine.process_modules(&modules);

            // Get the inferred types for this namespace
            let registry = type_engine.registry();

            // Get the merged entity type for this namespace
            if let Some(entity_type) = registry.get_type(namespace_name, "entity") {
                let generator = TypeGenerator::new(registry);
                let entity_schema = generator.json_schema_type_definition(entity_type);
                results.insert(namespace_name.clone(), entity_schema);
            }
        }

        // Output all results as a JSON object
        println!("{}", serde_json::to_string_pretty(&results).unwrap());
    } else {
        // Process single namespace (existing logic)
        let target_namespace = target_arg;

        // Check if the namespace exists
        let namespace = match game_mod.get_namespace(target_namespace) {
            Some(ns) => ns,
            None => {
                eprintln!("Namespace '{}' not found.", target_namespace);
                eprintln!("Available namespaces:");
                for namespace_name in game_mod.namespaces.keys() {
                    eprintln!("  {}", namespace_name);
                }
                std::process::exit(1);
            }
        };

        // Initialize type inference engine
        let mut type_engine = TypeInferenceEngine::new();

        // Process all modules in the namespace
        let modules: Vec<_> = namespace.modules.values().collect();
        type_engine.process_modules(&modules);

        // Get the inferred types for this namespace
        let registry = type_engine.registry();

        // Get the merged entity type for this namespace
        if let Some(entity_type) = registry.get_type(target_namespace, "entity") {
            let generator = TypeGenerator::new(registry);
            let entity_schema = generator.json_schema_type_definition(entity_type);
            println!("{}", serde_json::to_string_pretty(&entity_schema).unwrap());
        } else {
            println!("{{}}");
        }
    }
}
