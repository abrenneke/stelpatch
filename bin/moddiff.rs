use anyhow::anyhow;
use clap::*;
use colored::Colorize;
use lasso::ThreadedRodeo;
use std::path::PathBuf;
use stelpatch::playset::{
    base_game::BaseGame,
    diff::{Diffable, EntityMergeMode},
    game_mod::{GameMod, LoadMode},
    loader::stellaris_documents_dir,
    mod_definition::{ModDefinition, ModDefinitionList},
};

#[derive(Parser)]
#[command(about, version, author = "Snea")]
struct Cli {
    #[command(flatten)]
    mod_input: ModInput,

    /// The path to the Stellaris game folder
    #[arg(short = 's', long)]
    stellaris_path: Option<PathBuf>,

    #[arg(long)]
    mods_path: Option<PathBuf>,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
struct ModInput {
    /// The workshop mod ID to process
    #[arg(short, long)]
    workshop: Option<u64>,

    /// The path to the mod folder to process
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// The name, or a part of the name, of the mod to process
    #[arg(short = 'm', long = "mod")]
    mod_name: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = moddiff(cli) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn moddiff(cli: Cli) -> Result<(), anyhow::Error> {
    let workshop_id = cli.mod_input.workshop;
    let mod_path = cli.mod_input.path;
    let stellaris_path = cli.stellaris_path.as_ref().map(|p| p.as_path());
    let mod_name_search = cli.mod_input.mod_name.as_deref();
    let mods_path = cli.mods_path.as_ref().map(|p| p.as_path());

    let interner = ThreadedRodeo::default();
    let (base_game_res, game_mod_res) = rayon::join(
        || {
            BaseGame::load_as_mod_definition(stellaris_path, LoadMode::Parallel, &interner)
                .map_err(|e| anyhow!("Could not load base Stellaris game: {e}"))
        },
        || -> Result<GameMod, anyhow::Error> {
            let mod_definition = match mod_name_search {
                Some(mod_name_search) => {
                    let all_mods = ModDefinitionList::load_from_my_documents(mods_path)?;
                    let mod_match = all_mods.search_first(mod_name_search).map_err(|e| {
                        anyhow!("Could not find mod matching '{mod_name_search}': {e}")
                    })?;
                    mod_match.to_owned()
                }
                None => {
                    let mod_def_path = match mod_path {
                        Some(mod_path) => mod_path,
                        None => {
                            let mut path = stellaris_documents_dir(None)?;

                            path.push("mod");
                            path.push(format!("ugc_{}.mod", workshop_id.unwrap()));

                            path
                        }
                    };

                    ModDefinition::load_from_file(&mod_def_path)
                        .map_err(|e| anyhow!("Could not load mod definition: {}", e))?
                }
            };

            let game_mod = GameMod::load(mod_definition, LoadMode::Parallel, &interner)
                .map_err(|e| anyhow!("Could not load mod: {}", e))?;

            Ok(game_mod)
        },
    );
    let base_game = base_game_res?;
    let game_mod = game_mod_res?;

    let diff = base_game.diff_to(&game_mod, EntityMergeMode::Unknown, &interner);

    println!("Showing changes in {}", &game_mod.definition.name.bold());
    let diff_str = diff.short_changes_string(&interner);

    println!("{}", diff_str);

    Ok(())
}
