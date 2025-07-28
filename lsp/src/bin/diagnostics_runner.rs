use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clap::Parser;
use colored::Colorize;
use cw_lsp::base_game::game::detect_base_directory;
use cw_lsp::handlers::diagnostics::provider::DiagnosticsProvider;
use cw_lsp::handlers::initialization::CacheInitializer;
use cw_lsp::handlers::settings::Settings;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use tower_lsp::lsp_types::Diagnostic;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Diagnostics runner with integrated settings
#[derive(Debug, Parser)]
#[command(name = "diagnostics_runner")]
#[command(about = "Run diagnostics on Clausewitz script files")]
struct Args {
    /// Path to directory or file to process
    #[arg(help = "Directory or file to process")]
    path: String,

    /// Print all diagnostics to console
    #[arg(long, short, help = "Print all diagnostics to console")]
    print: bool,

    #[command(flatten)]
    settings: Settings,
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
    root_dir: &Path,
) -> (usize, Vec<Diagnostic>) {
    // Create a fake URI for the file
    let uri = format!("file://{}", file_path.display());

    // Use the DiagnosticsProvider to generate diagnostics with content directly
    let diagnostics = provider.generate_diagnostics_for_content(&uri, content, root_dir);
    let diagnostic_count = diagnostics.len();

    if print_diagnostics {
        (
            diagnostic_count,
            diagnostics.into_iter().map(|d| d.into()).collect(),
        )
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
    let args = Args::parse();

    // Start timing the entire operation
    let start_time = Instant::now();

    // Initialize global settings with the parsed settings
    Settings::init_global(args.settings.clone());

    let input_path = Path::new(&args.path);
    if !input_path.exists() {
        eprintln!(
            "{} {}",
            "Error:".red().bold(),
            format!("Path '{}' does not exist", args.path).bright_white()
        );
        std::process::exit(1);
    }

    // Use the unified initialization logic with timeout
    let timeout = Duration::from_secs(60);
    match CacheInitializer::initialize_with_timeout(timeout) {
        Ok(result) => {
            println!(
                "{} {}",
                "Full analysis loaded in".green().bold(),
                format!("{:?}", result.full_analysis_duration).bright_yellow()
            );
            println!("{}", "Caches initialized.".green().bold());
        }
        Err(err) => {
            eprintln!(
                "{} {}",
                "Error:".red().bold(),
                format!("Initialization failed: {}", err).bright_white()
            );
            std::process::exit(1);
        }
    }

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
        println!(
            "{} {}",
            "Finding .txt files in directory...".yellow().bold(),
            input_path.display().to_string().bright_white()
        );
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
    let all_diagnostics = if args.print {
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

    let root_dir = detect_base_directory(txt_files.first().unwrap());

    if root_dir.is_none() {
        eprintln!(
            "{} {}",
            "Error:".red().bold(),
            format!(
                "Could not detect base directory for file: {}",
                txt_files.first().unwrap().display()
            )
            .bright_white()
        );
        std::process::exit(1);
    }

    // Process each file in parallel with progress bar
    txt_files.par_iter().for_each(|file_path| {
        match fs::read_to_string(&file_path) {
            Ok(content) => {
                let (diagnostic_count, diagnostics) = generate_file_diagnostics(
                    &file_path,
                    &content,
                    &provider,
                    args.print,
                    root_dir.as_ref().unwrap(),
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

    // Display total execution time
    let total_duration = start_time.elapsed();
    println!(
        "{} {}",
        "Total time:".green().bold(),
        format!("{:?}", total_duration).bright_yellow()
    );

    // Print diagnostics if requested
    if args.print {
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
