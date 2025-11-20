use super::args::Args;
use super::worker::{format_file, ProcessResult};
use std::path::PathBuf;
use std::thread::{available_parallelism, scope};

/// Discover all Rust files in a directory recursively
pub fn discover_rust_files(dir: &std::path::Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    discover_recursive(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn discover_recursive(dir: &std::path::Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            discover_recursive(&path, files)?;
        } else if path.extension() == Some(std::ffi::OsStr::new("rs")) {
            files.push(path.canonicalize()?);
        }
    }
    Ok(())
}

/// Format all files in parallel
pub fn format_all(files: &[PathBuf], args: &Args) -> Vec<ProcessResult> {
    let num_threads = available_parallelism().map_or(1, |n| n.get());

    if args.verbose {
        eprintln!("Found {} Rust file(s)", files.len());
        eprintln!("Using {} threads", num_threads);
        eprintln!();
    }

    let oversubscribe = 4;
    let total_chunks = num_threads * oversubscribe;
    let chunk_size = files.len().div_ceil(total_chunks);

    scope(|s| {
        let handles: Vec<_> = files
            .chunks(chunk_size)
            .map(|chunk| {
                s.spawn(|| {
                    chunk
                        .iter()
                        .map(|file| format_file(file, args))
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        // Flatten results from all threads
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    })
}
