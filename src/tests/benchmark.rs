use futures::StreamExt;
use glob::glob;
use std::fs::File;
use std::io::Read;
use std::panic;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;
use walkdir::WalkDir;

use crate::cw_model::Module;

#[test]
fn benchmark_serial() {
    let search_dir = "D:\\SteamLibrary\\steamapps\\common\\Stellaris\\common";
    let ignore = vec!["HOW_TO_MAKE_NEW_SHIPS.txt"];

    let start_time = Instant::now();

    let mut file_count = 0;
    let mut successful_parse_count = 0;

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

        let result = panic::catch_unwind(|| Module::parse(contents, "test", "test"));
        match result {
            Ok(parse_result) => {
                if parse_result.is_ok() {
                    successful_parse_count += 1;
                }
            }
            Err(_) => {
                println!("{} - parse FAILED", original_path.display());
                panic!();
            }
        }
    }

    let duration = start_time.elapsed();

    println!(
        "Processed {} files, successfully parsed {} files",
        file_count, successful_parse_count
    );
    println!("Total time taken: {:?}", duration);
}

async fn read_file_async(path: PathBuf, tx: UnboundedSender<()>) {
    if path.extension().unwrap() != "txt" {
        return;
    }

    let mut original_file = File::open(&path).expect("Failed to open file");
    let mut contents = String::new();
    original_file
        .read_to_string(&mut contents)
        .expect(format!("Failed to read file {}", path.display()).as_str());

    let parse_result = Module::parse(contents, "test", "test");

    tx.send(()).unwrap();
}

#[test]
fn parallel_benchmark() {
    let search_dir = "D:\\SteamLibrary\\steamapps\\common\\Stellaris\\common";
    let ignore = vec!["HOW_TO_MAKE_NEW_SHIPS.txt"];

    let start_time = Instant::now();

    let mut file_count = 0;
    let mut successful_parse_count = 0;

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut rx_stream: UnboundedReceiverStream<()>;
    let mut read_tasks: Vec<JoinHandle<()>> = Vec::new();

    {
        let (tx, rx) = mpsc::unbounded_channel();
        rx_stream = UnboundedReceiverStream::new(rx);

        for entry in WalkDir::new(search_dir).into_iter().filter_map(|e| e.ok()) {
            let original_path = entry.clone().into_path();
            let file_name = entry.file_name().to_str().unwrap();

            if entry.file_type().is_dir() || ignore.contains(&file_name) {
                continue;
            }

            let tx_clone = tx.clone();
            let task = runtime.spawn(async move {
                read_file_async(original_path, tx_clone).await;
            });

            read_tasks.push(task);

            file_count += 1;
        }
    }

    runtime.block_on(async {
        tokio::try_join!(
            async {
                while let Some(()) = rx_stream.next().await {
                    successful_parse_count += 1;
                }
                Ok::<(), ()>(())
            },
            async {
                for task in read_tasks {
                    task.await.unwrap();
                }
                Ok(())
            }
        )
        .unwrap()
    });

    let duration = start_time.elapsed();

    println!(
        "Processed {} files, successfully parsed {} files",
        file_count, successful_parse_count
    );
    println!("Total time taken: {:?}", duration);
}
