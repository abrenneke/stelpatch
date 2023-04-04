#[cfg(test)]
mod tests {
    use glob::glob;
    use rayon::prelude::*;
    use std::ffi::OsStr;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;
    use std::time::Instant;
    use walkdir::WalkDir;

    use crate::cw_model::Module;

    #[test]
    fn benchmark_serial() {
        let search_dir = "D:\\SteamLibrary\\steamapps\\common\\Stellaris\\common";
        let ignore = vec!["HOW_TO_MAKE_NEW_SHIPS.txt", "99_README"];

        let start_time = Instant::now();

        let mut file_count = 0;
        let mut successful_parse_count = 0;

        for entry in glob(&format!("{}/**/*.txt", search_dir)).expect("Failed to read glob pattern")
        {
            let path = entry.expect("Failed to process entry");
            let original_path = path.clone();

            // Ignore any file in ignores
            if ignore.contains(&path.file_name().unwrap().to_str().unwrap()) {
                continue;
            }

            file_count += 1;

            let mut original_file = File::open(&original_path).expect("Failed to open file");
            let mut contents = String::new();
            original_file
                .read_to_string(&mut contents)
                .expect("Failed to read file");

            let result = Module::parse(contents, "test", "test");
            match result {
                Ok(_) => {
                    successful_parse_count += 1;
                }
                Err(_) => {
                    println!("{} - parse FAILED", original_path.display());
                }
            }
        }

        let duration = start_time.elapsed();

        println!(
            "Processed {} files, successfully parsed {} files, failed to parse {} files",
            file_count,
            successful_parse_count,
            file_count - successful_parse_count
        );
        println!("Total time taken: {:?}", duration);
    }

    #[test]
    fn parallel_benchmark() {
        let search_dir = "D:\\SteamLibrary\\steamapps\\common\\Stellaris\\common";
        let ignore = vec!["HOW_TO_MAKE_NEW_SHIPS.txt", "99_README"];

        let start_time = Instant::now();

        let mut successful_parse_count = 0;

        let mut files = Vec::new();

        for entry in WalkDir::new(search_dir).into_iter().filter_map(|e| {
            if let Ok(e) = e {
                if e.file_type().is_file()
                    && e.path().extension().unwrap_or(OsStr::new("")).to_str() == Some("txt")
                {
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

        for (result, path) in results.iter().zip(files.iter()) {
            match result {
                Some(_) => {
                    successful_parse_count += 1;
                }
                None => {
                    println!("{} - parse FAILED", path);
                }
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
}
