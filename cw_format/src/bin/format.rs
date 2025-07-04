use clap::Parser;
use cw_format::format_module;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

#[derive(Parser)]
#[command(name = "cw-format")]
#[command(about = "A formatter for Clausewitz files")]
#[command(version = "0.1.0")]
struct Args {
    /// Check if files are formatted
    #[arg(short, long)]
    check: bool,

    /// Write formatted files
    #[arg(short, long)]
    write: bool,

    /// List files that are different from formatted
    #[arg(short, long)]
    list_different: bool,

    /// Read from stdin
    #[arg(long)]
    stdin: bool,

    /// Path to use when reading from stdin
    #[arg(long)]
    stdin_filepath: Option<String>,

    /// Files to format
    #[arg(value_name = "FILE")]
    files: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.stdin {
        handle_stdin(&args)?;
    } else if !args.files.is_empty() {
        handle_files(&args)?;
    } else {
        eprintln!("Error: No files specified and --stdin not used");
        std::process::exit(1);
    }

    Ok(())
}

fn handle_stdin(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let formatted = format_module(input.clone());

    if args.check {
        if input != formatted {
            if let Some(filepath) = &args.stdin_filepath {
                eprintln!("File {} is not formatted", filepath);
            } else {
                eprintln!("stdin is not formatted");
            }
            std::process::exit(1);
        }
    } else if args.list_different {
        if input != formatted {
            if let Some(filepath) = &args.stdin_filepath {
                println!("{}", filepath);
            } else {
                println!("stdin");
            }
        }
    } else {
        // Default behavior: output formatted content
        print!("{}", formatted);
    }

    Ok(())
}

fn handle_files(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let mut has_unformatted = false;
    let mut files_to_list = Vec::new();

    for file_path in &args.files {
        let path = Path::new(file_path);

        if !path.exists() {
            eprintln!("Error: File {} does not exist", file_path);
            continue;
        }

        let content = fs::read_to_string(path)?;
        let formatted = format_module(content.clone());

        let is_formatted = content == formatted;

        if args.check {
            if !is_formatted {
                eprintln!("File {} is not formatted", file_path);
                has_unformatted = true;
            }
        } else if args.list_different {
            if !is_formatted {
                files_to_list.push(file_path.clone());
            }
        } else if args.write {
            if !is_formatted {
                fs::write(path, formatted)?;
                println!("Formatted {}", file_path);
            }
        } else {
            // Default behavior: output formatted content
            print!("{}", formatted);
        }
    }

    if args.check && has_unformatted {
        std::process::exit(1);
    }

    if args.list_different {
        for file in files_to_list {
            println!("{}", file);
        }
    }

    Ok(())
}
