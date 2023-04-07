#[cfg(test)]
mod tests {
    use const_format::concatcp;

    use crate::playset::diff::Diff;
    use crate::playset::diff::Diffable;
    use crate::playset::diff::EntityMergeMode;
    use crate::playset::flat_diff::*;
    use crate::playset::to_string_one_line::*;
    use crate::playset::{base_game::BaseGame, diff::HashMapDiff};

    use super::super::*;

    const MODS_BASE_DIR: &str = "C:\\Users\\Andy\\Documents\\Paradox Interactive\\Stellaris\\mod";

    const UNIVERSAL_RESOURCE_PATCH_MOD_FILE: &str = concatcp!(MODS_BASE_DIR, "/ugc_1688887083.mod");
    const ETHOS_UNIQUE_TECHS_BUILDINGS_MOD_FILE: &str =
        concatcp!(MODS_BASE_DIR, "/ugc_804732593.mod");
    const UNOFFICIAL_PATCH_MOD_FILE: &str = concatcp!(MODS_BASE_DIR, "/ugc_1995601384.mod");
    const EXPLORATION_TWEAKS: &str = concatcp!(MODS_BASE_DIR, "/ugc_2802824108.mod");

    #[test]
    fn test_game_mod_load() {
        // Create a ModDefinition that matches the expected_output
        let mod_definition =
            ModDefinition::load_from_file(ETHOS_UNIQUE_TECHS_BUILDINGS_MOD_FILE).unwrap();

        let game_mod = GameMod::load_parallel(mod_definition).unwrap();
        assert!(game_mod.modules.len() > 0);

        assert!(game_mod.modules[0].namespace.len() > 0);
        game_mod.print_contents();
    }

    #[test]
    fn test_game_mod_load_2() {
        // Create a ModDefinition that matches the expected_output
        let mod_definition =
            ModDefinition::load_from_file(UNIVERSAL_RESOURCE_PATCH_MOD_FILE).unwrap();

        let game_mod = GameMod::load_parallel(mod_definition).unwrap();
        assert!(game_mod.modules.len() > 0);

        assert!(game_mod.modules[0].namespace.len() > 0);
        dbg!(game_mod.modules.len());
        game_mod.print_contents();
    }

    #[test]
    fn mod_overrides_base() {
        let definition = ModDefinition::load_from_file(UNOFFICIAL_PATCH_MOD_FILE).unwrap();

        let base_game = BaseGame::load_as_mod_definition().unwrap();
        let game_mod = GameMod::load_parallel(definition).unwrap();

        let overrides = game_mod.get_overridden_modules_by_namespace(&base_game);

        println!("{}", "Overridden Modules by Namespace:".bold());

        for namespace in overrides.values() {
            println!("{}", namespace.namespace.bold());
            for module in namespace.modules.values() {
                println!("  {}", module.filename);
            }
        }
    }

    #[test]
    fn mod_overrides_entities() {
        let definition = ModDefinition::load_from_file(UNIVERSAL_RESOURCE_PATCH_MOD_FILE).unwrap();

        let base_game = BaseGame::load_as_mod_definition().unwrap();
        let game_mod = GameMod::load_parallel(definition).unwrap();

        let overrides = game_mod.get_overridden_entities(&base_game);

        println!("{}", "Overridden Entities:".bold());

        for (namespace, entity_name, _) in overrides {
            println!("  {} - {}", namespace, entity_name);
        }
    }

    #[test]
    fn mod_patch_of_base() {
        let definition = ModDefinition::load_from_file(UNOFFICIAL_PATCH_MOD_FILE).unwrap();

        let base_game = BaseGame::load_as_mod_definition().unwrap();
        let game_mod = GameMod::load_parallel(definition).unwrap();

        let overrides = game_mod.get_overridden_modules(&base_game);

        let first_override = overrides.first().unwrap();
        let patch = first_override.diff_to(
            &base_game.get_by_path(&first_override.path()).unwrap(),
            EntityMergeMode::LIOS,
        );

        dbg!(first_override.path());
        println!("{}", patch);
    }

    #[test]
    fn mod_patch_of_base_2() {
        let definition = ModDefinition::load_from_file(UNOFFICIAL_PATCH_MOD_FILE).unwrap();

        let base_game = BaseGame::load_as_mod_definition().unwrap();
        let game_mod = GameMod::load_parallel(definition).unwrap();

        let overrides = game_mod.get_overridden_modules(&base_game);

        let first_override = overrides.first().unwrap();
        let patch = first_override.diff_to(
            &base_game.get_by_path(&first_override.path()).unwrap(),
            EntityMergeMode::LIOS,
        );

        dbg!(first_override.path());
        println!("{}", patch);
    }

    #[test]
    fn list_changes_by_namespace_in_mod_1() {
        let definition = ModDefinition::load_from_file(UNIVERSAL_RESOURCE_PATCH_MOD_FILE).unwrap();

        let base_game = BaseGame::load_as_mod_definition().unwrap();
        let game_mod = GameMod::load_parallel(definition).unwrap();

        let diff = base_game.diff_to(&game_mod, EntityMergeMode::Unknown);

        for (namespace_name, namespace) in diff.namespaces {
            match namespace.entities {
                HashMapDiff::Modified(entities) => {
                    if entities.len() > 0 {
                        println!("{}", namespace_name.bold());
                        for (changed_entity_name, _entity_diff) in entities {
                            println!("  {}", changed_entity_name);
                        }
                        println!("");
                    }
                }
                HashMapDiff::Unchanged => {}
            }
        }
    }

    #[test]
    fn list_changes_by_namespace_in_mod_2() {
        let definition = ModDefinition::load_from_file(UNOFFICIAL_PATCH_MOD_FILE).unwrap();

        let base_game = BaseGame::load_as_mod_definition().unwrap();
        let game_mod = GameMod::load_parallel(definition).unwrap();

        let diff = base_game.diff_to(&game_mod, EntityMergeMode::LIOS);

        for (namespace_name, namespace) in diff.namespaces {
            match namespace.entities {
                HashMapDiff::Modified(entities) => {
                    if entities.len() > 0 {
                        println!("{}", namespace_name.bold());
                        for (changed_entity_name, entity_diff) in entities {
                            match entity_diff {
                                Diff::Added(_) => {
                                    println!("  (Added) {}", changed_entity_name)
                                }
                                Diff::Removed(_) => {
                                    println!("  (Removed) {}", changed_entity_name)
                                }
                                Diff::Modified(diff) => {
                                    println!(
                                        "  {}: {}",
                                        changed_entity_name,
                                        diff.to_string_one_line()
                                    )
                                }
                                Diff::Unchanged => {}
                            }
                        }
                        println!("");
                    }
                }
                HashMapDiff::Unchanged => {}
            }
        }
    }

    #[test]
    fn flat_diff() {
        let definition = ModDefinition::load_from_file(EXPLORATION_TWEAKS).unwrap();

        let base_game = BaseGame::load_as_mod_definition().unwrap();
        let game_mod = GameMod::load_parallel(definition).unwrap();

        let diff = base_game.diff_to(&game_mod, EntityMergeMode::LIOS);

        for (namespace_name, namespace) in diff.namespaces {
            match namespace.entities {
                HashMapDiff::Modified(entities) => {
                    if entities.len() > 0 {
                        println!("{}", namespace_name.bold());
                        for (changed_entity_name, entity_diff) in entities {
                            match entity_diff {
                                Diff::Added(_) => {
                                    println!("  (Added) {}", changed_entity_name)
                                }
                                Diff::Removed(_) => {
                                    println!("  (Removed) {}", changed_entity_name)
                                }
                                Diff::Modified(diff) => {
                                    let flattened = diff.flatten_diff(&changed_entity_name);
                                    for flat_diff in flattened {
                                        println!("  {}", flat_diff.to_string_one_line())
                                    }
                                }
                                Diff::Unchanged => {}
                            }
                        }
                        println!("");
                    }
                }
                HashMapDiff::Unchanged => {}
            }
        }
    }
}
