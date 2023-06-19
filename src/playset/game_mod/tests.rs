#[cfg(test)]
mod tests {
    use colored_diff::PrettyDifference;
    use lazy_static::lazy_static;

    use crate::playset::base_game::BaseGame;
    use crate::playset::diff::Diffable;
    use crate::playset::diff::EntityMergeMode;

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

        let interner = ThreadedRodeo::default();
        let game_mod = GameMod::load(mod_definition, LoadMode::Parallel, &interner).unwrap();
        assert!(game_mod.namespaces.len() > 0);
    }

    #[test]
    fn test_game_mod_load_2() {
        // Create a ModDefinition that matches the expected_output
        let mod_definition =
            ModDefinition::load_from_file(UNIVERSAL_RESOURCE_PATCH_MOD_FILE.as_path()).unwrap();

        let interner = ThreadedRodeo::default();
        let game_mod = GameMod::load(mod_definition, LoadMode::Parallel, &interner).unwrap();
        assert!(game_mod.namespaces.len() > 0);
    }

    #[test]
    fn mod_overrides_entities() {
        let definition =
            ModDefinition::load_from_file(UNIVERSAL_RESOURCE_PATCH_MOD_FILE.as_path()).unwrap();

        let interner = ThreadedRodeo::default();
        let _game_mod = GameMod::load(definition, LoadMode::Parallel, &interner).unwrap();
    }

    // #[test]
    // fn list_changes_by_namespace_in_mod_1() {
    //     let definition =
    //         ModDefinition::load_from_file(UNIVERSAL_RESOURCE_PATCH_MOD_FILE.as_path()).unwrap();

    //     let interner = ThreadedRodeo::default();
    //     let base_mod =
    //         BaseGame::load_as_mod_definition(None, LoadMode::Parallel, &interner).unwrap();
    //     let game_mod = GameMod::load(definition, LoadMode::Parallel, &interner).unwrap();

    //     let diff = base_mod.diff_to(&game_mod, EntityMergeMode::Unknown, &interner);

    //     for (namespace_name, namespace) in diff.namespaces {
    //         match &namespace.properties.kv {
    //             HashMapDiff::Modified(entities) => {
    //                 if entities.len() > 0 {
    //                     println!("{}", interner.resolve(&namespace_name).bold());
    //                     for (changed_entity_name, _entity_diff) in entities {
    //                         println!("  {}", interner.resolve(&changed_entity_name));
    //                     }
    //                     println!("");
    //                 }
    //             }
    //             HashMapDiff::Unchanged => {}
    //         }
    //     }
    // }

    // #[test]
    // fn list_changes_by_namespace_in_mod_2() {
    //     let definition =
    //         ModDefinition::load_from_file(UNOFFICIAL_PATCH_MOD_FILE.as_path()).unwrap();

    //     let interner = ThreadedRodeo::default();
    //     let base_mod =
    //         BaseGame::load_as_mod_definition(None, LoadMode::Parallel, &interner).unwrap();
    //     let game_mod = GameMod::load(definition, LoadMode::Parallel, &interner).unwrap();

    //     let diff = base_mod.diff_to(&game_mod, EntityMergeMode::LIOS, &interner);

    //     let diff_str = diff.short_changes_string(&interner);
    //     // print!("{}", diff_str);
    // }

    fn assert_eq_pretty(expected: &str, actual: &str) {
        if expected != actual {
            println!("{}", PrettyDifference { expected, actual });
            panic!("Expected and actual values do not match");
        }
    }

    #[test]
    fn change_one_define_only_diff() {
        let interner = ThreadedRodeo::default();
        let base_mod =
            BaseGame::load_as_mod_definition(None, LoadMode::Parallel, &interner).unwrap();

        let game_mod = GameMod::with_module(
            Module::parse(
                r#"NGameplay = {
                    BASE_SURVEY_TIME = 30.0
                }"#,
                "common/defines",
                "test",
                &interner,
            )
            .unwrap(),
            &interner,
        );

        let diff = base_mod.diff_to(&game_mod, EntityMergeMode::Unknown, &interner);

        let diff_str = diff.short_changes_string(&interner);

        // print!("{}", diff_str);
        assert_eq_pretty(
            diff_str.trim(),
            r#"
common/defines
  NGameplay/BASE_SURVEY_TIME: 20.0 -> 30.0
"#
            .trim(),
        );
    }

    #[test]
    fn flat_diff_1() {
        let definition = ModDefinition::load_from_file(EXPLORATION_TWEAKS.as_path()).unwrap();

        let interner = ThreadedRodeo::default();
        let base_mod =
            BaseGame::load_as_mod_definition(None, LoadMode::Parallel, &interner).unwrap();
        let game_mod = GameMod::load(definition, LoadMode::Parallel, &interner).unwrap();

        let diff = base_mod.diff_to(&game_mod, EntityMergeMode::Unknown, &interner);

        // print!("{}", &diff.to_string_with_interner(&interner));

        let diff_str = diff.short_changes_string(&interner);
        // print!("{}", diff_str);

        assert_eq_pretty(
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
            .trim(),
        );
    }

    #[test]
    fn flat_diff_economic_categories_simple() {
        let interner = ThreadedRodeo::default();

        let base = Module::parse(
            r#"planet_structures = {
                foo = bar
                generate_mult_modifiers = {
                    cost
                    upkeep
                }
            }"#,
            "common/economic_categories",
            "00_economic_categories",
            &interner,
        )
        .unwrap()
        .into_mod(&interner);

        let game_mod = Module::parse(
            r#"
        planet_structures = {
            foo = bar
            generate_mult_modifiers = {
                cost
                upkeep
                produces
            }
        }"#,
            "common/economic_categories",
            "Legw_categories",
            &interner,
        )
        .unwrap()
        .into_mod(&interner);

        let diff = base.diff_to(&game_mod, EntityMergeMode::Unknown, &interner);

        let diff_str = diff.short_changes_string(&interner);

        assert_eq!(
            diff_str.trim(),
            r#"common/economic_categories
  +planet_structures/generate_mult_modifiers/2: produces"#
                .trim()
        );
    }

    #[test]
    fn flat_diff_economic_categories() {
        let strs = ThreadedRodeo::default();
        let game_mod = Module::parse(
            r#"
        planet_buildings = {
            parent = planet_structures
            modifier_category = planet
            generate_mult_modifiers = {
                cost
                upkeep
                produces
            }
        }"#,
            "common/economic_categories",
            "Legw_categories",
            &strs,
        )
        .unwrap()
        .into_mod(&strs);

        let diff_str = BaseGame::load_as_mod_definition(None, LoadMode::Parallel, &strs)
            .unwrap()
            .diff_to(&game_mod, EntityMergeMode::Unknown, &strs)
            .short_changes_string(&strs);

        assert_eq!(
            diff_str.trim(),
            r#"common/economic_categories
  +planet_buildings/generate_mult_modifiers/2: produces"#
                .trim()
        );
    }
}
