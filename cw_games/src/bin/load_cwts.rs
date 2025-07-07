use cw_model::types::CwtAnalyzer;
use cw_parser::CwtModuleCell;

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

    let parse_duration = start_time.elapsed();

    println!("\n=== PARSING SUMMARY ===");
    println!("Total files processed: {}", cwt_files.len());
    println!("Successfully parsed: {}", modules.len());
    println!("Parse errors: {}", parse_errors.len());
    println!("Parse time: {:.2?}", parse_duration);

    if !modules.is_empty() {
        let avg_time = parse_duration / modules.len() as u32;
        println!("Average parse time per file: {:.2?}", avg_time);
    }

    // Print statistics about the parsed modules
    if !modules.is_empty() {
        println!("\n=== MODULE STATISTICS ===");
        let mut total_items = 0;
        let mut total_rules = 0;

        for (file_path, module) in &modules {
            let module = module.borrow_dependent().as_ref().unwrap();

            let items = module.items.len();
            let rules = module.rules().count();

            total_items += items;
            total_rules += rules;

            println!(
                "  {}: {} items ({} rules)",
                file_path.file_name().unwrap_or_default().to_string_lossy(),
                items,
                rules,
            );
        }

        println!("Total items: {}", total_items);
        println!("Total rules: {}", total_rules);
    }

    // Convert the parsed modules using CwtConverter
    if !modules.is_empty() {
        println!("\n=== CONVERTING TO INFERRED TYPES ===");
        let convert_start = Instant::now();

        let mut converter = CwtAnalyzer::new();
        let mut conversion_errors = Vec::new();

        for (file_path, module) in &modules {
            print!(
                "Converting {}... ",
                file_path.file_name().unwrap_or_default().to_string_lossy()
            );

            let module = module.borrow_dependent().as_ref().unwrap();
            let file_convert_start = Instant::now();

            match converter.convert_module(module) {
                Ok(()) => {
                    let convert_duration = file_convert_start.elapsed();
                    println!("✓ ({:.2?})", convert_duration);
                }
                Err(errors) => {
                    let convert_duration = file_convert_start.elapsed();
                    println!("✗ ({:.2?}) - {} errors", convert_duration, errors.len());
                    conversion_errors.extend(errors.into_iter().map(|e| (file_path.clone(), e)));
                }
            }
        }

        let convert_duration = convert_start.elapsed();

        println!("\n=== CONVERSION SUMMARY ===");
        println!("Conversion time: {:.2?}", convert_duration);
        println!("Conversion errors: {}", conversion_errors.len());

        // Print converted type statistics
        println!("\n=== CONVERTED TYPE STATISTICS ===");
        println!("Types defined: {}", converter.types.len());
        println!("Enums defined: {}", converter.enums.len());
        println!("Value sets defined: {}", converter.value_sets.len());
        println!("Aliases defined: {}", converter.aliases.len());
        println!("Single aliases defined: {}", converter.single_aliases.len());

        // Print detailed type information
        if !converter.types.is_empty() {
            println!("\n=== TYPE DEFINITIONS ===");
            for (name, type_def) in &converter.types {
                println!("  Type: {}", name);
                if let Some(path) = &type_def.path {
                    println!("    Path: {}", path);
                }
                if let Some(name_field) = &type_def.name_field {
                    println!("    Name field: {}", name_field);
                }
                if !type_def.subtypes.is_empty() {
                    println!("    Subtypes: {}", type_def.subtypes.len());
                    for (subtype_name, _) in &type_def.subtypes {
                        println!("      - {}", subtype_name);
                    }
                }
                if !type_def.localisation.is_empty() {
                    println!(
                        "    Localisation requirements: {}",
                        type_def.localisation.len()
                    );
                }
                if !type_def.modifiers.is_empty() {
                    println!("    Modifier generation: {}", type_def.modifiers.len());
                }
                println!("    Rules: {:?}", type_def.rules);
                println!();
            }
        }

        // Print detailed enum information
        if !converter.enums.is_empty() {
            println!("\n=== ENUM DEFINITIONS ===");
            for (name, enum_def) in &converter.enums {
                println!("  Enum: {}", name);
                if !enum_def.values.is_empty() {
                    println!("    Values: {:?}", enum_def.values);
                }
                if let Some(complex) = &enum_def.complex {
                    println!("    Complex enum:");
                    println!("      Path: {}", complex.path);
                    println!("      Start from root: {}", complex.start_from_root);
                    println!("      Name structure: {:?}", complex.name_structure);
                }
                println!();
            }
        }

        // Print value sets
        if !converter.value_sets.is_empty() {
            println!("\n=== VALUE SETS ===");
            for (name, values) in &converter.value_sets {
                println!("  Value set: {}", name);
                println!("    Values: {:?}", values);
                println!();
            }
        }

        // Print aliases
        if !converter.aliases.is_empty() {
            println!("\n=== ALIASES ===");
            for (name, alias) in &converter.aliases {
                println!("  Alias: {}", name);
                println!("    Category: {}", alias.category);
                println!("    Name: {}", alias.name);
                println!("    Rules: {:?}", alias.rules);
                println!();
            }
        }

        // Print single aliases
        if !converter.single_aliases.is_empty() {
            println!("\n=== SINGLE ALIASES ===");
            for (name, alias_type) in &converter.single_aliases {
                println!("  Single alias: {}", name);
                println!("    Type: {:?}", alias_type);
                println!();
            }
        }

        // Print conversion errors if any
        if !conversion_errors.is_empty() {
            println!("\n=== CONVERSION ERRORS ===");
            for (file_path, error) in &conversion_errors {
                println!("  {}: {}", file_path.display(), error);
            }
        }
    }

    // Print parse errors if any
    if !parse_errors.is_empty() {
        println!("\n=== PARSE ERRORS ===");
        for (file_path, error) in &parse_errors {
            println!("  {}: {}", file_path.display(), error);
        }
    }

    let total_duration = start_time.elapsed();
    println!("\n=== TOTAL SUMMARY ===");
    println!("Total time: {:.2?}", total_duration);

    Ok(())
}
