use super::args::Args;
use super::cache::Cache;
use super::units::Units;
use super::worker::{PARALLEL_ITEMS_MIN_BYTES, ProcessResult, format_file};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::scope;

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

/// Formats all files on `args.thread_count()` threads, consulting `cache` for files already
/// known to be formatted and `units` for the unchanged items of the others.
///
/// The files are sorted largest first and handed out from a shared queue, so the large
/// files start first and no thread is left with a long tail while the others idle. A
/// file of at least `PARALLEL_ITEMS_MIN_BYTES` has its top-level items formatted on all
/// the threads (`format_source_with_config_parallel`); the file-level workers and the
/// item-level workers share the cores, and the scheduler sorts it out.
pub fn format_all(
    files: &[PathBuf],
    args: &Args,
    cache: Option<&Cache>,
    units: Option<&Units>,
) -> Vec<ProcessResult> {
    let num_threads = args.thread_count();

    if args.verbose {
        eprintln!("Found {} Rust file(s)", files.len());
        eprintln!("Using {} threads", num_threads);
        eprintln!();
    }

    // Largest first. A file whose size is unknown goes last.
    let mut order: Vec<(u64, usize)> = files
        .iter()
        .enumerate()
        .map(|(i, f)| (std::fs::metadata(f).map_or(0, |m| m.len()), i))
        .collect();
    order.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let next = AtomicUsize::new(0);
    let item_threads = |size: u64| {
        if size >= PARALLEL_ITEMS_MIN_BYTES as u64 {
            num_threads
        } else {
            1
        }
    };

    let mut results: Vec<Option<ProcessResult>> = (0..files.len()).map(|_| None).collect();
    let done: Vec<Vec<(usize, ProcessResult)>> = scope(|s| {
        let handles: Vec<_> = (0..num_threads.min(files.len().max(1)))
            .map(|_| {
                s.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&(size, i)) = order.get(k) else {
                            break;
                        };
                        done.push((
                            i,
                            format_file(&files[i], args, cache, units, item_threads(size)),
                        ));
                    }
                    done
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_default())
            .collect()
    });
    for (i, result) in done.into_iter().flatten() {
        results[i] = Some(result);
    }
    results.into_iter().flatten().collect()
}
