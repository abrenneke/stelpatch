use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::base_game::BaseGame;
use crate::interner::get_interner;
use cw_model::{Entity, SpurMap};
use lasso::Spur;

use crate::handlers::cache::{GameDataCache, Namespace};

/// Integrates modifiers from Stellaris logs into the GameDataCache
pub fn integrate_modifiers_into_cache(
    game_data_cache: &mut GameDataCache,
) -> Result<(), anyhow::Error> {
    let interner = get_interner();
    // Load modifiers from Stellaris logs
    let modifiers = BaseGame::load_modifiers(get_interner())?;

    // Create artificial entities for each modifier
    let mut modifier_entities = SpurMap::new();
    let mut entity_keys = Vec::new();

    for modifier in modifiers {
        // Create an empty entity for the modifier
        let entity = Entity::new();

        modifier_entities.insert(modifier.name.clone(), Arc::new(entity));
        entity_keys.push(modifier.name);
    }

    // Create entity keys set
    let entity_keys_set = Arc::new(entity_keys.iter().cloned().collect::<HashSet<Spur>>());

    // Create the modifiers namespace
    let modifiers_namespace = Namespace {
        entities: modifier_entities,
        entity_keys,
        entity_keys_set,
        scripted_variables: SpurMap::new(),
        modules: HashMap::new(),
        values: Vec::new(),
    };

    // Insert the modifiers namespace into the game data cache
    game_data_cache.namespaces.insert(
        interner.get_or_intern("game/modifiers"),
        modifiers_namespace,
    );

    Ok(())
}
