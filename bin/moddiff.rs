use anyhow::anyhow;
use clap::*;
use std::path::PathBuf;
use stelpatch::playset::{
    base_game::BaseGame,
    diff::{Diffable, EntityMergeMode},
    game_mod::GameMod,
    loader::stellaris_documents_dir,
    mod_definition::ModDefinition,
};

#[derive(Parser)]
#[command(about, version, author = "Snea")]
struct Cli {
    #[command(flatten)]
    mod_input: ModInput,

    /// The path to the Stellaris game folder
    #[clap(short = 's', long)]
    stellaris_path: Option<PathBuf>,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
struct ModInput {
    /// The workshop mod ID to process
    #[clap(short, long)]
    workshop: Option<u64>,

    /// The path to the mod folder to process
    #[clap(short, long)]
    path: Option<PathBuf>,
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
    let stellaris_path = cli.stellaris_path;

    let base_game = BaseGame::load_as_mod_definition(stellaris_path)
        .map_err(|e| anyhow!("Could not load base Stellaris game: {}", e))?;

    let mod_def_path = match mod_path {
        Some(mod_path) => mod_path,
        None => {
            let mut path = stellaris_documents_dir(None).map_err(|e| {
                anyhow!("Could not find Stellaris directory in My Documents: {}", e)
            })?;

            path.push("mod");
            path.push(format!("ugc_{}.mod", workshop_id.unwrap()));

            path
        }
    };

    let mod_definition = ModDefinition::load_from_file(&mod_def_path.to_string_lossy().to_string())
        .map_err(|e| anyhow!("Could not load mod definition: {}", e))?;

    let game_mod =
        GameMod::load_parallel(mod_definition).map_err(|e| anyhow!("Could not load mod: {}", e))?;

    let diff = base_game.diff_to(&game_mod, EntityMergeMode::Unknown);

    let diff_str = diff.short_changes_string();

    println!("{}", diff_str);

    Ok(())
}
