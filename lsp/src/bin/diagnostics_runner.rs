use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use colored::Colorize;
use cw_lsp::handlers::cache::FullAnalysis;
use cw_lsp::handlers::cache::{EntityRestructurer, GameDataCache, TypeCache};
use cw_lsp::handlers::diagnostics::provider::DiagnosticsProvider;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use tower_lsp::lsp_types::Diagnostic;

/// Command line arguments
#[derive(Debug, Default)]
struct Args {
    path: String,
    print_diagnostics: bool,
}

fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Err("Usage: diagnostics_runner [--print] <directory|file>".to_string());
    }

    let mut parsed = Args::default();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--print" => {
                parsed.print_diagnostics = true;
                i += 1;
            }
            _ => {
                if parsed.path.is_empty() {
                    parsed.path = args[i].clone();
                    i += 1;
                } else {
                    return Err(format!("Unexpected argument: {}", args[i]));
                }
            }
        }
    }

    if parsed.path.is_empty() {
        return Err("Usage: diagnostics_runner [--print] <directory|file>".to_string());
    }

    Ok(parsed)
}

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

/// Generate diagnostics for a single file using DiagnosticsProvider
fn generate_file_diagnostics(
    file_path: &Path,
    content: &str,
    provider: &DiagnosticsProvider,
    print_diagnostics: bool,
) -> (usize, Vec<Diagnostic>) {
    // Create a fake URI for the file
    let uri = format!("file://{}", file_path.display());

    // Use the DiagnosticsProvider to generate diagnostics with content directly
    let diagnostics = provider.generate_diagnostics_for_content(&uri, content);
    let diagnostic_count = diagnostics.len();

    if print_diagnostics {
        (diagnostic_count, diagnostics)
    } else {
        (diagnostic_count, Vec::new())
    }
}

fn print_diagnostic(file_path: &Path, diagnostic: &Diagnostic, show_file_path: bool) {
    let (severity_text, severity_color) = match diagnostic.severity {
        Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR) => ("ERROR", "red"),
        Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING) => ("WARNING", "yellow"),
        Some(tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION) => ("INFO", "blue"),
        Some(tower_lsp::lsp_types::DiagnosticSeverity::HINT) => ("HINT", "cyan"),
        Some(_) => ("UNKNOWN", "white"),
        None => ("UNKNOWN", "white"),
    };

    let source = diagnostic
        .source
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("unknown");
    let line = diagnostic.range.start.line + 1; // Convert to 1-based line numbers
    let col = diagnostic.range.start.character + 1; // Convert to 1-based column numbers

    let colored_severity = match severity_color {
        "red" => severity_text.red().bold(),
        "yellow" => severity_text.yellow().bold(),
        "blue" => severity_text.blue().bold(),
        "cyan" => severity_text.cyan().bold(),
        _ => severity_text.white().bold(),
    };

    if show_file_path {
        println!(
            "{}:{}:{} [{}] [{}] {}",
            file_path.display().to_string().bright_white(),
            line.to_string().bright_white(),
            col.to_string().bright_white(),
            colored_severity,
            source.bright_black(),
            diagnostic.message
        );
    } else {
        println!(
            "{}:{} [{}] [{}] {}",
            line.to_string().bright_white(),
            col.to_string().bright_white(),
            colored_severity,
            source.bright_black(),
            diagnostic.message
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e.bright_white());
            std::process::exit(1);
        }
    };

    let input_path = Path::new(&args.path);
    if !input_path.exists() {
        eprintln!(
            "{} {}",
            "Error:".red().bold(),
            format!("Path '{}' does not exist", args.path).bright_white()
        );
        std::process::exit(1);
    }

    println!("{}", "Initializing caches...".blue().bold());

    // Initialize caches in background
    TypeCache::initialize_in_background();
    GameDataCache::initialize_in_background();

    // Wait for caches to be initialized
    let timeout = std::time::Duration::from_secs(60);
    let start = std::time::Instant::now();

    while !TypeCache::is_initialized() || !GameDataCache::is_initialized() {
        if start.elapsed() > timeout {
            eprintln!(
                "{} {}",
                "Error:".red().bold(),
                "Timeout waiting for caches to initialize".bright_white()
            );
            std::process::exit(1);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("{}", "Restructuring entities...".blue().bold());
    let entity_restructurer =
        EntityRestructurer::new(GameDataCache::get().unwrap(), TypeCache::get().unwrap());
    entity_restructurer.load();

    let full_analysis_start = std::time::Instant::now();

    println!("{}", "Loading full analysis...".blue().bold());
    let full_analysis = FullAnalysis::new(GameDataCache::get().unwrap(), TypeCache::get().unwrap());
    full_analysis.load();

    let full_analysis_duration = full_analysis_start.elapsed();
    println!(
        "{} {}",
        "Full analysis loaded in".green().bold(),
        format!("{:?}", full_analysis_duration).bright_yellow()
    );

    println!("{}", "Caches initialized.".green().bold());

    // Determine if input is a file or directory and get list of files to process
    let is_single_file = input_path.is_file();
    let txt_files = if is_single_file {
        println!(
            "{} {}",
            "Processing single file:".yellow().bold(),
            input_path.display().to_string().bright_white()
        );
        vec![input_path.to_path_buf()]
    } else if input_path.is_dir() {
        println!("{}", "Finding .txt files in directory...".yellow().bold());
        let files = find_txt_files(input_path)?;
        println!(
            "{} {}",
            "Found".green().bold(),
            format!("{} .txt files", files.len()).bright_white()
        );
        files
    } else {
        eprintln!(
            "{} {}",
            "Error:".red().bold(),
            format!("'{}' is neither a file nor a directory", args.path).bright_white()
        );
        std::process::exit(1);
    };

    let total_diagnostics = AtomicUsize::new(0);
    let processed_files = AtomicUsize::new(0);
    let all_diagnostics = if args.print_diagnostics {
        Some(Mutex::new(Vec::new()))
    } else {
        None
    };

    // Create progress bar
    let progress_bar = ProgressBar::new(txt_files.len() as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    println!("{}", "Processing files...".yellow().bold());

    // Create DiagnosticsProvider
    let documents = Arc::new(RwLock::new(HashMap::new()));
    let provider = DiagnosticsProvider::new(documents.clone(), false);

    // Process each file in parallel with progress bar
    txt_files.par_iter().for_each(|file_path| {
        match fs::read_to_string(&file_path) {
            Ok(content) => {
                let (diagnostic_count, diagnostics) = generate_file_diagnostics(
                    &file_path,
                    &content,
                    &provider,
                    args.print_diagnostics,
                );
                total_diagnostics.fetch_add(diagnostic_count, Ordering::Relaxed);
                processed_files.fetch_add(1, Ordering::Relaxed);

                if let Some(ref all_diags) = all_diagnostics {
                    let mut all_diags_lock = all_diags.lock().unwrap();
                    for diagnostic in diagnostics {
                        all_diags_lock.push((file_path.clone(), diagnostic));
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "{} {}",
                    "Error reading file:".red().bold(),
                    format!("{}: {}", file_path.display(), e).bright_white()
                );
            }
        }
        progress_bar.inc(1);
    });

    progress_bar.finish_with_message("Processing complete!");

    println!("\n{}", "Summary:".cyan().bold());
    println!(
        "{} {}",
        "Processed".green().bold(),
        format!("{} files", processed_files.load(Ordering::Relaxed)).bright_white()
    );
    println!(
        "{} {}",
        "Total diagnostics:".green().bold(),
        total_diagnostics
            .load(Ordering::Relaxed)
            .to_string()
            .bright_white()
    );

    // Print diagnostics if requested
    if args.print_diagnostics {
        if let Some(all_diags) = all_diagnostics {
            let all_diags_lock = all_diags.lock().unwrap();
            if !all_diags_lock.is_empty() {
                println!("\n{}", "Diagnostics:".cyan().bold());
                for (file_path, diagnostic) in all_diags_lock.iter() {
                    print_diagnostic(file_path, diagnostic, !is_single_file);
                }
            }
        }
    }

    Ok(())
}
