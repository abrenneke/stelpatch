use std::{sync::Arc, time::Instant};

use clap::{command, Parser};
use lasso::ThreadedRodeo;
use stelpatch::playset::{base_game::BaseGame, game_mod::LoadMode};

#[derive(Parser)]
#[command()]
struct Cli {
    #[arg(short, long)]
    parallel: bool,

    #[arg(short, long)]
    samples: Option<usize>,
}

fn main() {
    let params = Cli::parse();

    let interner = Arc::new(ThreadedRodeo::default());
    let samples = params.samples.unwrap_or(1);

    for _ in 0..samples {
        let start_time = Instant::now();
        BaseGame::load_as_mod_definition(
            None,
            if params.parallel {
                LoadMode::Parallel
            } else {
                LoadMode::Serial
            },
            interner.clone(),
        )
        .unwrap();
        let duration = start_time.elapsed();
        println!("Parsed base game in {:?}", duration);
    }

    std::process::exit(0);
}
