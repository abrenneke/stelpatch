use cw_parser::CwtModuleCell;
use cw_parser::cwt::CwtModule;
use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <directory_path>", args[0]);
        eprintln!("Example: {} /path/to/cwt/files", args[0]);
        std::process::exit(1);
    }

    let directory_path = &args[1];
    let dir_path = Path::new(directory_path);

    if !dir_path.exists() {
        eprintln!("Error: Directory '{}' does not exist", directory_path);
        std::process::exit(1);
    }

    if !dir_path.is_dir() {
        eprintln!("Error: '{}' is not a directory", directory_path);
        std::process::exit(1);
    }

    println!("Loading CWT files from: {}", directory_path);
    let start_time = Instant::now();

    let mut cwt_files = Vec::new();
    let mut modules = Vec::new();
    let mut parse_errors = Vec::new();

    // Find all .cwt files in the directory
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "cwt") {
            cwt_files.push(path);
        }
    }

    println!("Found {} CWT files", cwt_files.len());

    // Parse each CWT file
    for cwt_file in &cwt_files {
        print!("Parsing {}... ", cwt_file.display());

        let file_start = Instant::now();
        let content = fs::read_to_string(cwt_file)?;

        let module = CwtModuleCell::from_input(content);

        let mut is_ok = false;
        match module.borrow_dependent().as_ref() {
            Ok(_) => {
                let parse_duration = file_start.elapsed();
                println!("✓ ({:.2?})", parse_duration);
                is_ok = true;
            }
            Err(error) => {
                let parse_duration = file_start.elapsed();
                println!("✗ ({:.2?}) - Error: {}", parse_duration, error);
                parse_errors.push((cwt_file.clone(), error.clone()));
            }
        }

        if is_ok {
            modules.push((cwt_file.clone(), module));
        }
    }

    let total_duration = start_time.elapsed();

    println!("\n=== SUMMARY ===");
    println!("Total files processed: {}", cwt_files.len());
    println!("Successfully parsed: {}", modules.len());
    println!("Parse errors: {}", parse_errors.len());
    println!("Total time: {:.2?}", total_duration);

    if !modules.is_empty() {
        let avg_time = total_duration / modules.len() as u32;
        println!("Average time per file: {:.2?}", avg_time);
    }

    // Print statistics about the parsed modules
    if !modules.is_empty() {
        println!("\n=== MODULE STATISTICS ===");
        let mut total_items = 0;
        let mut total_rules = 0;
        let mut total_blocks = 0;

        for (file_path, module) in &modules {
            let module = module.borrow_dependent().as_ref().unwrap();

            let items = module.items.len();
            let rules = module.rules().count();
            let blocks = module.blocks().count();

            total_items += items;
            total_rules += rules;
            total_blocks += blocks;

            println!(
                "  {}: {} items ({} rules, {} blocks)",
                file_path.file_name().unwrap_or_default().to_string_lossy(),
                items,
                rules,
                blocks
            );
        }

        println!("Total items: {}", total_items);
        println!("Total rules: {}", total_rules);
        println!("Total blocks: {}", total_blocks);
    }

    // Print parse errors if any
    if !parse_errors.is_empty() {
        println!("\n=== PARSE ERRORS ===");
        for (file_path, error) in &parse_errors {
            println!("  {}: {}", file_path.display(), error);
        }
    }

    Ok(())
}
