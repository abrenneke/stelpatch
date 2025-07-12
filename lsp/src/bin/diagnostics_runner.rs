use std::env;
use std::fs;
use std::io;
use std::path::Path;

use cw_lsp::handlers::cache::FullAnalysis;
use cw_lsp::handlers::cache::{GameDataCache, TypeCache, get_namespace_entity_type};
use cw_lsp::handlers::diagnostics::type_validation::validate_entity_value;
use cw_lsp::handlers::utils::extract_namespace_from_uri;
use cw_parser::AstModule;

/// Recursively find all .txt files in a directory
fn find_txt_files(dir: &Path) -> io::Result<Vec<std::path::PathBuf>> {
    let mut txt_files = Vec::new();

    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                txt_files.extend(find_txt_files(&path)?);
            } else if path.is_file() && path.extension().map_or(false, |ext| ext == "txt") {
                txt_files.push(path);
            }
        }
    }

    Ok(txt_files)
}

/// Generate diagnostics for a single file
async fn generate_file_diagnostics(file_path: &Path, content: &str) -> usize {
    let mut diagnostic_count = 0;

    // Create a fake URI for the file
    let uri = format!("file://{}", file_path.display());

    // First, try to parse the content
    let mut module = AstModule::new();
    match module.parse_input(content) {
        Ok(()) => {
            // If parsing succeeds, do type checking
            let type_diagnostics = generate_type_diagnostics(&module, &uri, content).await;
            diagnostic_count += type_diagnostics;
        }
        Err(_error) => {
            // If parsing fails, count as one diagnostic
            diagnostic_count += 1;
        }
    }

    diagnostic_count
}

/// Generate type-checking diagnostics for a successfully parsed document
async fn generate_type_diagnostics(module: &AstModule<'_>, uri: &str, content: &str) -> usize {
    let mut diagnostic_count = 0;

    // Check if type cache is initialized
    if !TypeCache::is_initialized() {
        return 0;
    }

    if !GameDataCache::is_initialized() {
        return 0;
    }

    // Extract namespace from URI
    let namespace = match extract_namespace_from_uri(uri) {
        Some(ns) => ns,
        None => {
            return 0;
        }
    };

    // Get type information for this namespace
    let type_info = match get_namespace_entity_type(&namespace) {
        Some(info) => info,
        None => {
            return 0;
        }
    };

    let namespace_type = match &type_info.scoped_type {
        Some(t) => t.clone(),
        None => {
            return 0;
        }
    };

    // Validate each entity in the module
    for item in &module.items {
        if let cw_parser::AstEntityItem::Expression(expr) = item {
            // Top-level keys are entity names - they can be anything, so don't validate them
            // Instead, validate their VALUES against the namespace structure
            let entity_diagnostics =
                validate_entity_value(&expr.value, namespace_type.clone(), content, &namespace, 0);
            diagnostic_count += entity_diagnostics.len();
        }
    }

    diagnostic_count
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <directory>", args[0]);
        std::process::exit(1);
    }

    let dir_path = Path::new(&args[1]);
    if !dir_path.exists() {
        eprintln!("Error: Directory '{}' does not exist", args[1]);
        std::process::exit(1);
    }

    println!("Initializing caches...");

    // Initialize caches in background
    TypeCache::initialize_in_background();
    GameDataCache::initialize_in_background();

    // Wait for caches to be initialized
    let timeout = std::time::Duration::from_secs(30);
    let start = std::time::Instant::now();

    while !TypeCache::is_initialized() || !GameDataCache::is_initialized() {
        if start.elapsed() > timeout {
            eprintln!("Error: Timeout waiting for caches to initialize");
            std::process::exit(1);
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let full_analysis_start = std::time::Instant::now();

    println!("Loading full analysis...");
    let full_analysis = FullAnalysis::new(GameDataCache::get().unwrap(), TypeCache::get().unwrap());
    full_analysis.load();

    let full_analysis_duration = full_analysis_start.elapsed();
    println!("Full analysis loaded in {:?}", full_analysis_duration);

    Ok(())

    // println!("Caches initialized. Finding .txt files...");

    // // Find all .txt files
    // let txt_files = find_txt_files(dir_path)?;
    // println!("Found {} .txt files", txt_files.len());

    // let mut total_diagnostics = 0;
    // let mut processed_files = 0;

    // // Process each file
    // for file_path in txt_files {
    //     match fs::read_to_string(&file_path) {
    //         Ok(content) => {
    //             let diagnostics = generate_file_diagnostics(&file_path, &content).await;
    //             total_diagnostics += diagnostics;
    //             processed_files += 1;

    //             if diagnostics > 0 {
    //                 println!("{}: {} diagnostics", file_path.display(), diagnostics);
    //             }
    //         }
    //         Err(e) => {
    //             eprintln!("Error reading file {}: {}", file_path.display(), e);
    //         }
    //     }
    // }

    // println!("\nSummary:");
    // println!("Processed {} files", processed_files);
    // println!("Total diagnostics: {}", total_diagnostics);

    // Ok(())
}
