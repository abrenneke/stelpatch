#[cfg(test)]
mod tests {
    use std::{fs::File, io::Read};

    use glob::glob;
    use lasso::ThreadedRodeo;

    use cw_parser::model::Module;

    #[test]
    fn test_vanilla_files() {
        let search_dir = "D:\\SteamLibrary\\steamapps\\common\\Stellaris\\common";
        let ignore = vec!["HOW_TO_MAKE_NEW_SHIPS.txt", "99_README"];

        for entry in glob(&format!("{}/**/*.txt", search_dir)).expect("Failed to read glob pattern")
        {
            let path = entry.expect("Failed to process entry");
            let original_path = path.clone();

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

            let mut original_file = File::open(&original_path).expect("Failed to open file");
            let mut contents = String::new();
            original_file
                .read_to_string(&mut contents)
                .expect("Failed to read file");
            let interner = &ThreadedRodeo::default();
            let _result = Module::parse(&contents, "test", "test", interner);

            // if let Err(e) = result {
            //     println!("{} - parse FAILED", original_path.display());

            //     match e {
            //         nom::Err::Error(e) => {
            //             let (input, last_tried) = e.errors.first().unwrap();
            //             let lines: String = input.lines().take(2).collect();
            //             println!("Failed at: {}", lines);
            //             println!("Last tried: {:?}", last_tried);

            //             print!("Context Chain: ");
            //             for (_, err) in e.errors {
            //                 match err {
            //                     nom::error::VerboseErrorKind::Context(context) => {
            //                         print!("/{}", context);
            //                     }
            //                     nom::error::VerboseErrorKind::Char(c) => {
            //                         print!("Char: {}", c);
            //                     }
            //                     _ => {}
            //                 }
            //             }
            //             println!("");
            //         }
            //         nom::Err::Failure(e) => {
            //             let (input, last_tried) = e.errors.first().unwrap();
            //             let lines: String = input.lines().take(2).collect();
            //             println!("Failed at: {}", lines);
            //             println!("Last tried: {:?}", last_tried);

            //             print!("Context Chain: ");
            //             for (_, err) in e.errors {
            //                 match err {
            //                     nom::error::VerboseErrorKind::Context(context) => {
            //                         print!("/{}", context);
            //                     }
            //                     nom::error::VerboseErrorKind::Char(c) => {
            //                         print!("Char: {}", c);
            //                     }
            //                     _ => {}
            //                 }
            //             }
            //             println!("");
            //         }
            //         nom::Err::Incomplete(_) => {
            //             println!("Incomplete");
            //         }
            //     }
            //     panic!("Failed to parse file");
            // }
        }
    }
}
