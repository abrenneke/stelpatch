#[cfg(test)]
mod tests {
    use lazy_static::lazy_static;

    use crate::cw_model::ToStringWithInterner;
    use crate::playset::base_game::BaseGame;
    use crate::playset::diff::Diffable;
    use crate::playset::diff::EntityMergeMode;

    use crate::playset::diff::HashMapDiff;

    use super::super::*;

    lazy_static! {
        pub static ref MODS_BASE_DIR: &'static str =
            "C:\\Users\\Andy\\Documents\\Paradox Interactive\\Stellaris\\mod";
        pub static ref UNIVERSAL_RESOURCE_PATCH_MOD_FILE: PathBuf =
            vec![&MODS_BASE_DIR, "ugc_1688887083.mod"]
                .into_iter()
                .collect();
        pub static ref ETHOS_UNIQUE_TECHS_BUILDINGS_MOD_FILE: PathBuf =
            vec![&MODS_BASE_DIR, "ugc_804732593.mod"]
                .into_iter()
                .collect();
        pub static ref UNOFFICIAL_PATCH_MOD_FILE: PathBuf =
            vec![&MODS_BASE_DIR, "ugc_1995601384.mod"]
                .into_iter()
                .collect();
        pub static ref EXPLORATION_TWEAKS: PathBuf = vec![&MODS_BASE_DIR, "ugc_2802824108.mod"]
            .into_iter()
            .collect();
    }

    #[test]
    fn test_game_mod_load() {
        // Create a ModDefinition that matches the expected_output
        let mod_definition =
            ModDefinition::load_from_file(ETHOS_UNIQUE_TECHS_BUILDINGS_MOD_FILE.as_path()).unwrap();

        let interner = Arc::new(ThreadedRodeo::default());
        let game_mod = GameMod::load(mod_definition, LoadMode::Parallel, interner.clone()).unwrap();
        assert!(game_mod.modules.len() > 0);

        assert!(interner.resolve(&game_mod.modules[0].namespace).len() > 0);
        game_mod.print_contents(&interner.clone());
    }

    #[test]
    fn test_game_mod_load_2() {
        // Create a ModDefinition that matches the expected_output
        let mod_definition =
            ModDefinition::load_from_file(UNIVERSAL_RESOURCE_PATCH_MOD_FILE.as_path()).unwrap();

        let interner = Arc::new(ThreadedRodeo::default());
        let game_mod = GameMod::load(mod_definition, LoadMode::Parallel, interner.clone()).unwrap();
        assert!(game_mod.modules.len() > 0);

        assert!(interner.resolve(&game_mod.modules[0].namespace).len() > 0);
        dbg!(game_mod.modules.len());
        game_mod.print_contents(&interner);
    }

    // #[test]
    // fn mod_overrides_base() {
    //     let definition =
    //         ModDefinition::load_from_file(UNOFFICIAL_PATCH_MOD_FILE.as_path()).unwrap();

    //     let game_mod = GameMod::load(definition, LoadMode::Parallel, interner).unwrap();

    //     let overrides = game_mod.get_overridden_modules_by_namespace(
    //         &BaseGame::load_as_mod_definition(None, LoadMode::Parallel, interner).unwrap(),
    //     );

    //     println!("{}", "Overridden Modules by Namespace:".bold());

    //     for namespace in overrides.values() {
    //         println!("{}", namespace.namespace.bold());
    //         for module in namespace.modules.values() {
    //             println!("  {}", module.filename);
    //         }
    //     }
    // }

    #[test]
    fn mod_overrides_entities() {
        let definition =
            ModDefinition::load_from_file(UNIVERSAL_RESOURCE_PATCH_MOD_FILE.as_path()).unwrap();

        let interner = Arc::new(ThreadedRodeo::default());
        // let _base_game = BASE_MOD.unwrap();
        let _game_mod = GameMod::load(definition, LoadMode::Parallel, interner).unwrap();

        // let overrides = game_mod.get_overridden_entities(&base_game);

        // println!("{}", "Overridden Entities:".bold());

        // for (namespace, entity_name, _) in overrides {
        //     println!("  {} - {}", namespace, entity_name);
        // }
    }

    #[test]
    fn mod_patch_of_base() {
        let definition =
            ModDefinition::load_from_file(UNOFFICIAL_PATCH_MOD_FILE.as_path()).unwrap();

        let interner = Arc::new(ThreadedRodeo::default());
        let game_mod = GameMod::load(definition, LoadMode::Parallel, interner.clone()).unwrap();

        let base_mod =
            BaseGame::load_as_mod_definition(None, LoadMode::Parallel, interner.clone()).unwrap();
        let overrides = game_mod.get_overridden_modules(&base_mod, &interner);

        let first_override = overrides.first().unwrap();
        let patch = first_override.diff_to(
            base_mod
                .get_by_path(&interner.get_or_intern(&first_override.path(&interner)))
                .unwrap(),
            EntityMergeMode::LIOS,
            &interner,
        );

        dbg!(first_override.path(&interner));
        println!("{}", patch.to_string_with_interner(&interner));
    }

    #[test]
    fn mod_patch_of_base_2() {
        let definition =
            ModDefinition::load_from_file(UNOFFICIAL_PATCH_MOD_FILE.as_path()).unwrap();

        let interner = Arc::new(ThreadedRodeo::default());
        let base_mod =
            BaseGame::load_as_mod_definition(None, LoadMode::Parallel, interner.clone()).unwrap();
        let game_mod = GameMod::load(definition, LoadMode::Parallel, interner.clone()).unwrap();

        let overrides = game_mod.get_overridden_modules(&base_mod, &interner);

        let first_override = overrides.first().unwrap();
        let patch = first_override.diff_to(
            base_mod
                .get_by_path(&interner.get_or_intern(&first_override.path(&interner)))
                .unwrap(),
            EntityMergeMode::LIOS,
            &interner,
        );

        dbg!(first_override.path(&interner));
        println!("{}", patch.to_string_with_interner(&interner));
    }

    #[test]
    fn list_changes_by_namespace_in_mod_1() {
        let definition =
            ModDefinition::load_from_file(UNIVERSAL_RESOURCE_PATCH_MOD_FILE.as_path()).unwrap();

        let interner = Arc::new(ThreadedRodeo::default());
        let base_mod =
            BaseGame::load_as_mod_definition(None, LoadMode::Parallel, interner.clone()).unwrap();
        let game_mod = GameMod::load(definition, LoadMode::Parallel, interner.clone()).unwrap();

        let diff = base_mod.diff_to(&game_mod, EntityMergeMode::Unknown, &interner);

        for (namespace_name, namespace) in diff.namespaces {
            match &namespace.properties.kv {
                HashMapDiff::Modified(entities) => {
                    if entities.len() > 0 {
                        println!("{}", interner.resolve(&namespace_name).bold());
                        for (changed_entity_name, _entity_diff) in entities {
                            println!("  {}", interner.resolve(&changed_entity_name));
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
        let definition =
            ModDefinition::load_from_file(UNOFFICIAL_PATCH_MOD_FILE.as_path()).unwrap();

        let interner = Arc::new(ThreadedRodeo::default());
        let base_mod =
            BaseGame::load_as_mod_definition(None, LoadMode::Parallel, interner.clone()).unwrap();
        let game_mod = GameMod::load(definition, LoadMode::Parallel, interner.clone()).unwrap();

        let diff = base_mod.diff_to(&game_mod, EntityMergeMode::LIOS, &interner);

        let diff_str = diff.short_changes_string(&interner);
        print!("{}", diff_str);
    }

    #[test]
    fn flat_diff() {
        let definition = ModDefinition::load_from_file(EXPLORATION_TWEAKS.as_path()).unwrap();

        let interner = Arc::new(ThreadedRodeo::default());
        let base_mod =
            BaseGame::load_as_mod_definition(None, LoadMode::Parallel, interner.clone()).unwrap();
        let game_mod = GameMod::load(definition, LoadMode::Parallel, interner.clone()).unwrap();

        let diff = base_mod.diff_to(&game_mod, EntityMergeMode::Unknown, &interner);

        let diff_str = diff.short_changes_string(&interner);
        print!("{}", diff_str);

        assert_eq!(
            diff_str.trim(),
            r#"
common/defines
  NGameplay/BASE_SURVEY_TIME: 20.0 -> 30.0
  NGameplay/CONSTRUCTION_SHIP_WORK_SPEED_MULT: 1 -> 0.5
  NGameplay/PLANET_HYPERLANE_RANGE: 2 -> 1
  NShip/HYPERDRIVE_INTERSTELLAR_TRAVEL_SPEED: 1.0 -> 0.12
common/on_actions
  +on_game_start/events: gee_game_start.1
common/technology
  +GT_ftl_speed_1
  +GT_ftl_speed_2
  +GT_ftl_speed_3
  +GT_ftl_speed_4
  +GT_science_ship_survey_speed
  +GT_ship_emergency_ftl_min_days_add
  +GT_ship_interstellar_speed_mult
  +GT_shipsize_mining_station_build_speed_mult
  +GT_shipsize_research_station_build_speed_mult
  +gee_science_ship_survey_speed_2
  +gee_science_ship_survey_speed_3
"#
            .trim()
        );
    }
}
