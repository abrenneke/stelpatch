use glob::glob;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::cw_model::Module;

fn create_dir_if_not_exists(path: &PathBuf) {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).expect("Failed to create directories");
        }
    }
}

#[test]
fn snapshot_tests() {
    let search_dir = "D:\\SteamLibrary\\steamapps\\common\\Stellaris\\common";
    let mut snapshot_tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    snapshot_tests_dir.push("snapshot_tests");

    let ignore = vec!["HOW_TO_MAKE_NEW_SHIPS.txt"];

    for entry in glob(&format!("{}/**/*.txt", search_dir)).expect("Failed to read glob pattern") {
        let path = entry.expect("Failed to process entry");
        let original_path = path.clone();

        // Ignore any file in ignores
        if ignore.contains(&path.file_name().unwrap().to_str().unwrap()) {
            continue;
        }

        let mut original_file = File::open(&original_path).expect("Failed to open file");
        let mut contents = String::new();
        original_file
            .read_to_string(&mut contents)
            .expect("Failed to read file");

        let parse_result = Module::parse(contents, "test", "test");

        if parse_result.is_err() {
            println!("{} - parse FAILED", original_path.display());
            continue;
        }

        let str_result = parse_result.unwrap().to_string();

        let relative_path = original_path.strip_prefix(search_dir).unwrap();
        let mut actual_path = snapshot_tests_dir.clone();
        actual_path.push(relative_path);
        actual_path.set_extension("actual.stellaris");

        create_dir_if_not_exists(&actual_path);

        let mut actual_file = File::create(&actual_path)
            .expect(format!("Failed to create .actual file at {}", actual_path.display()).as_str());
        actual_file
            .write_all(str_result.as_bytes())
            .expect("Failed to write .actual file");

        let mut original_dest_path = actual_path.clone();
        original_dest_path.set_extension("original.stellaris");
        fs::copy(&original_path, &original_dest_path).expect("Failed to copy .original file");

        let mut expected_path = actual_path.clone();
        expected_path.set_extension("expected.stellaris");

        if expected_path.exists() {
            let mut expected_file =
                File::open(&expected_path).expect("Failed to open .expected file");
            let mut expected_contents = String::new();
            expected_file
                .read_to_string(&mut expected_contents)
                .expect("Failed to read .expected file");

            if str_result != expected_contents {
                println!("{} - snapshot FAILED", relative_path.display());
                continue;
            } else {
                println!("{} - snapshot OK", relative_path.display());
            }
        } else {
            fs::copy(&actual_path, &expected_path)
                .expect("Failed to create .expected file from .actual");

            println!("{} - snapshot written", relative_path.display());
        }
    }
}
