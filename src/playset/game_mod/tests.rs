#[cfg(test)]
mod tests {
    use const_format::concatcp;

    use crate::playset::base_game::BaseGame;

    use super::super::*;

    const MODS_BASE_DIR: &str = "C:\\Users\\Andy\\Documents\\Paradox Interactive\\Stellaris\\mod";

    const UNIVERSAL_RESOURCE_PATCH_MOD_FILE: &str = concatcp!(MODS_BASE_DIR, "/ugc_1688887083.mod");
    const ETHOS_UNIQUE_TECHS_BUILDINGS_MOD_FILE: &str =
        concatcp!(MODS_BASE_DIR, "/ugc_804732593.mod");
    const UNOFFICIAL_PATCH_MOD_FILE: &str = concatcp!(MODS_BASE_DIR, "/ugc_1995601384.mod");

    #[test]
    fn test_game_mod_load() {
        // Create a ModDefinition that matches the expected_output
        let mod_definition =
            ModDefinition::load_from_file(ETHOS_UNIQUE_TECHS_BUILDINGS_MOD_FILE).unwrap();

        let game_mod = GameMod::load_parallel(mod_definition).unwrap();
        assert!(game_mod.modules.len() > 0);

        assert!(game_mod.modules[0].type_path.len() > 0);
        dbg!(game_mod.modules.len());
    }

    #[test]
    fn test_game_mod_load_2() {
        // Create a ModDefinition that matches the expected_output
        let mod_definition =
            ModDefinition::load_from_file(UNIVERSAL_RESOURCE_PATCH_MOD_FILE).unwrap();

        let game_mod = GameMod::load_parallel(mod_definition).unwrap();
        assert!(game_mod.modules.len() > 0);

        assert!(game_mod.modules[0].type_path.len() > 0);
        dbg!(game_mod.modules.len());
    }

    #[test]
    fn mod_overrides_base() {
        let definition = ModDefinition::load_from_file(UNOFFICIAL_PATCH_MOD_FILE).unwrap();

        let base_game = BaseGame::load_as_mod_definition().unwrap();
        let game_mod = GameMod::load_parallel(definition).unwrap();

        let overrides = game_mod.get_overridden_modules(&base_game);

        assert!(overrides.len() > 0);

        let overridden_modules: Vec<String> = overrides.iter().map(|m| m.path()).collect();
        dbg!(overridden_modules);
    }

    #[test]
    fn mod_patch_of_base() {
        let definition = ModDefinition::load_from_file(UNOFFICIAL_PATCH_MOD_FILE).unwrap();

        let base_game = BaseGame::load_as_mod_definition().unwrap();
        let game_mod = GameMod::load_parallel(definition).unwrap();

        let overrides = game_mod.get_overridden_modules(&base_game);

        let first_override = overrides.first().unwrap();
        let patch = first_override.diff(&base_game.get_by_path(&first_override.path()).unwrap());

        dbg!(first_override.path());
        println!("{}", patch);
    }
}
