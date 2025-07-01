use cw_games::stellaris::BaseGame;
use cw_model::LoadMode;

pub fn main() {
    let loaded_mod = BaseGame::load_global_as_mod_definition(LoadMode::Serial);

    let num_namespaces = loaded_mod.namespaces.len();
    let num_modules = loaded_mod
        .namespaces
        .values()
        .map(|ns| ns.modules.len())
        .sum::<usize>();

    println!(
        "Loaded {} namespaces with {} modules",
        num_namespaces, num_modules
    );
}
