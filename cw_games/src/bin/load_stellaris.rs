use cw_games::stellaris::BaseGame;
use cw_model::LoadMode;

pub fn main() {
    let loaded_mod = BaseGame::load_global_as_mod_definition(LoadMode::Serial);

    println!("Loaded mod: {:?}", loaded_mod.namespaces.len());
}
