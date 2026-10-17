use super::args::Args;
use super::trace;
use super::worker::{PARALLEL_ITEMS_MIN_BYTES, ProcessResult, Setup, format_file};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::scope;

/// A file to format: its canonical path, its size in bytes for the largest-first queue (0
/// when it could not be read, which puts the file last), and the index of its [`Setup`]
/// (`project::Project::config_of`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub path: PathBuf,
    pub len: u64,
    pub config: usize,
}

impl SourceFile {
    /// A file named by a canonical path whose size is not known yet: one `metadata` call.
    pub fn at(path: PathBuf) -> Self {
        let len = fs::metadata(&path).map_or(0, |m| m.len());
        SourceFile {
            path,
            len,
            config: 0,
        }
    }
}

/// Discover all Rust files in a directory recursively, as canonical paths.
///
/// Entries found in the walk are skipped when they are hidden (the name starts with `.`,
/// such as `.git`), and so are directories tagged as caches by a `CACHEDIR.TAG` file (the
/// Cache Directory Tagging Specification, <https://bford.info/cachedir/>), which Cargo
/// writes in every `target` directory. `dir` itself is walked whatever its name and tag:
/// a path the user names is always used.
///
/// Every file reached is the one `Path::canonicalize` would name, but the walk resolves
/// paths once per root and once per symlink, not once per file: a directory entry that is
/// not a symlink, read from a canonical directory, is canonical as `directory/name` (it adds
/// no `.`, `..` or symlink component). Entry types come from the directory listing
/// (`DirEntry::file_type`, `d_type` on Linux) without a `stat`, and file sizes from one
/// `stat` relative to the open directory. A symlink, an entry of unknown type, and every
/// entry under a directory whose canonical path could not be taken are resolved as before:
/// `is_dir` following links, then `canonicalize`.
///
/// Each directory's entries are visited in name order, so for a tree without symlinked
/// directories the files come out in `Path` order and the caller's sort finds them sorted.
/// A symlink to a directory that is already being walked (a cycle) is not followed again:
/// every file under it is reached through the walk in progress. The previous walk followed
/// such a link until the kernel refused the path (40 links deep), reaching no other file.
pub fn discover_rust_files(dir: &Path) -> io::Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    match dir.canonicalize() {
        Ok(root) => {
            let listing = fs::read_dir(&root)?;
            walk_canonical(&root, listing, true, &mut Vec::new(), &mut files)?;
        }
        Err(_) => walk_resolving(dir, true, &mut files)?,
    }
    Ok(files)
}

/// The signature a `CACHEDIR.TAG` file starts with.
const CACHEDIR_SIGNATURE: &[u8] = b"Signature: 8a477f597d28d172789f06886806bc55";

/// Whether an entry named `name` is hidden from the walk.
fn hidden(name: &OsStr) -> bool {
    name.as_encoded_bytes().starts_with(b".")
}

/// Whether the directory `dir`, whose entries include `names`, is a cache directory: it has
/// a `CACHEDIR.TAG` file that starts with the specification's signature.
fn tagged_as_cache<'a>(dir: &Path, mut names: impl Iterator<Item = &'a OsStr>) -> bool {
    use std::io::Read;
    names.any(|name| name == "CACHEDIR.TAG")
        && fs::File::open(dir.join("CACHEDIR.TAG")).is_ok_and(|file| {
            let mut start = Vec::with_capacity(CACHEDIR_SIGNATURE.len());
            let read = file
                .take(CACHEDIR_SIGNATURE.len() as u64)
                .read_to_end(&mut start);
            read.is_ok() && start == CACHEDIR_SIGNATURE
        })
}

/// Walks `dir`, a canonical directory listed by `listing`, unless it is a cache directory
/// that is not the `root` of the walk; `stack` holds the canonical directories being
/// walked, to recognise a symlink back into one of them.
fn walk_canonical(
    dir: &Path,
    listing: fs::ReadDir,
    root: bool,
    stack: &mut Vec<PathBuf>,
    files: &mut Vec<SourceFile>,
) -> io::Result<()> {
    let mut entries = Vec::new();
    for entry in listing {
        let entry = entry?;
        entries.push((entry.file_name(), entry));
    }
    if !root && tagged_as_cache(dir, entries.iter().map(|(name, _)| name.as_os_str())) {
        return Ok(());
    }
    entries.retain(|(name, _)| !hidden(name));
    // Names are unique within a directory.
    entries.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    stack.push(dir.to_path_buf());
    for (name, entry) in entries {
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => {
                let path = dir.join(&name);
                match fs::read_dir(&path) {
                    Ok(listing) => walk_canonical(&path, listing, false, stack, files)?,
                    // As before: `is_dir` (false for a path the kernel refuses), then the
                    // error of listing it.
                    Err(_) => resolve_entry(&path, None, files)?,
                }
            }
            Ok(kind) if !kind.is_symlink() => {
                if Path::new(&name).extension() == Some(OsStr::new("rs")) {
                    let len = entry.metadata().map_or(0, |m| m.len());
                    files.push(SourceFile {
                        path: dir.join(&name),
                        len,
                        config: 0,
                    });
                }
            }
            // A symlink, or a type the listing did not give: resolve it.
            _ => resolve_entry(&entry.path(), Some(&mut *stack), files)?,
        }
    }
    stack.pop();
    Ok(())
}

/// Walks `dir`, a path that may not be canonical, resolving every entry: the walk before
/// this change, kept for a root or directory whose canonical path cannot be taken. The
/// entries are skipped as in [`walk_canonical`].
fn walk_resolving(dir: &Path, root: bool, files: &mut Vec<SourceFile>) -> io::Result<()> {
    let entries = fs::read_dir(dir)?.collect::<io::Result<Vec<_>>>()?;
    let names: Vec<_> = entries.iter().map(fs::DirEntry::file_name).collect();
    if !root && tagged_as_cache(dir, names.iter().map(|n| n.as_os_str())) {
        return Ok(());
    }
    for (entry, name) in entries.iter().zip(&names) {
        if !hidden(name) {
            resolve_entry(&entry.path(), None, files)?;
        }
    }
    Ok(())
}

/// The rule for one entry at `path`, following symlinks: a directory is walked, a `.rs`
/// file is added under its canonical path. Inside a canonical walk (`stack` given), a
/// directory is walked under its canonical path, unless that directory is already being
/// walked.
fn resolve_entry(
    path: &Path,
    stack: Option<&mut Vec<PathBuf>>,
    files: &mut Vec<SourceFile>,
) -> io::Result<()> {
    if path.is_dir() {
        match (stack, path.canonicalize()) {
            (Some(stack), Ok(target)) => {
                if !stack.contains(&target) {
                    let listing = fs::read_dir(&target)?;
                    walk_canonical(&target, listing, false, stack, files)?;
                }
            }
            _ => walk_resolving(path, false, files)?,
        }
    } else if path.extension() == Some(OsStr::new("rs")) {
        files.push(SourceFile::at(path.canonicalize()?));
    }
    Ok(())
}

/// Formats all files on `args.thread_count()` threads, each with its configuration's
/// [`Setup`]: its cache for files already known to be formatted, and its unit cache for the
/// unchanged items of the others.
///
/// The files are sorted largest first and handed out from a shared queue, so the large
/// files start first and no thread is left with a long tail while the others idle. A
/// file of at least `PARALLEL_ITEMS_MIN_BYTES` has its top-level items formatted on all
/// the threads (`format_source_with_config_parallel`); the file-level workers and the
/// item-level workers share the cores, and the scheduler sorts it out.
pub fn format_all(files: &[SourceFile], args: &Args, setups: &[Setup]) -> Vec<ProcessResult> {
    let num_threads = args.thread_count();

    if args.verbose {
        eprintln!("Found {} Rust file(s)", files.len());
        eprintln!("Using {} threads", num_threads);
        eprintln!();
    }

    // Largest first. A file whose size is unknown goes last. The sizes were taken when the
    // files were found.
    let mut order: Vec<(u64, usize)> = files.iter().enumerate().map(|(i, f)| (f.len, i)).collect();
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
    trace::mark("spawn");
    let done: Vec<Vec<(usize, ProcessResult)>> = scope(|s| {
        let handles: Vec<_> = (0..num_threads.min(files.len().max(1)))
            .map(|w| {
                let next = &next;
                let order = &order;
                s.spawn(move || {
                    let traced = trace::enabled();
                    let w_start = trace::now();
                    let mut done = Vec::new();
                    loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&(size, i)) = order.get(k) else {
                            break;
                        };
                        let start = trace::now();
                        done.push((
                            i,
                            format_file(
                                &files[i].path,
                                args,
                                files[i].config,
                                &setups[files[i].config],
                                item_threads(size),
                            ),
                        ));
                        if traced {
                            let (read, hashed, formatted, split) = trace::take_stamps();
                            trace::file(&trace::FileSpan {
                                worker: w,
                                bytes: size,
                                split,
                                start,
                                read,
                                hashed,
                                formatted,
                                end: trace::now(),
                            });
                        }
                    }
                    trace::worker(w, w_start, trace::now());
                    done
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_default())
            .collect()
    });
    trace::mark("joined");
    for (i, result) in done.into_iter().flatten() {
        results[i] = Some(result);
    }
    results.into_iter().flatten().collect()
}
