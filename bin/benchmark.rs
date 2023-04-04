use std::{fs::File, io::Read, path::PathBuf, time::Instant};

use rayon::prelude::*;
use stelpatch::cw_model::Module;
use walkdir::WalkDir;

fn main() {
    let search_dir = "D:\\SteamLibrary\\steamapps\\common\\Stellaris\\common";
    let ignore = vec!["HOW_TO_MAKE_NEW_SHIPS.txt", "99_README"];

    let start_time = Instant::now();

    let mut successful_parse_count = 0;

    let mut files = Vec::new();

    for entry in WalkDir::new(search_dir).into_iter().filter_map(|e| {
        if let Ok(e) = e {
            if e.file_type().is_file() {
                return Some(e);
            }
        }
        None
    }) {
        let path = entry.clone().into_path();
        // let file_name = entry.file_name().to_str().unwrap();

        // Ignore any file in ignores
        let mut ignore_file = false;
        if ignore.contains(&path.file_name().unwrap().to_str().unwrap()) {
            ignore_file = true;
        }

        for ignore in ignore.iter() {
            if path.to_str().unwrap().contains(ignore) {
                ignore_file = true;
            }
        }

        if ignore_file {
            continue;
        }

        files.push(path.to_string_lossy().to_string());
    }

    // dbg!(files);

    let results: Vec<Option<Module>> = files
        .par_iter()
        .map(|path| {
            let path = PathBuf::from(path);
            match path.extension() {
                Some(extension) => {
                    if extension != "txt" {
                        return None;
                    }
                }
                None => {
                    return None;
                }
            }

            let mut original_file = File::open(&path).ok()?;
            let mut contents = String::new();
            original_file.read_to_string(&mut contents).ok()?;

            let parse_result = Module::parse(contents, "test", "test").ok()?;
            Some(parse_result)
        })
        .collect();

    for result in results {
        match result {
            Some(_) => {
                successful_parse_count += 1;
            }
            None => {}
        }
    }

    let duration = start_time.elapsed();

    println!(
        "Successfully successfully parsed {}/{} files",
        successful_parse_count,
        files.len()
    );
    println!("Total time taken: {:?}", duration);
}
