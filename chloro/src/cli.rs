//! chloro: A minimal Rust code formatter
//!
//! Command-line interface for formatting Rust source files.

#![allow(clippy::multiple_crate_versions)]

// The binary's global allocator: glibc's `malloc` unless an `alloc-*` feature is enabled
// (all off by default; measured in EXP-014). The first enabled of mimalloc, jemalloc and
// snmalloc wins, so `--all-features` builds.
#[cfg(feature = "alloc-mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[cfg(all(feature = "alloc-jemalloc", not(feature = "alloc-mimalloc")))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
#[cfg(all(
    feature = "alloc-snmalloc",
    not(any(feature = "alloc-mimalloc", feature = "alloc-jemalloc"))
))]
#[global_allocator]
static GLOBAL: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

/// Command-line interface for formatting Rust source files.
#[cfg(feature = "cli")]
pub mod cli {
    pub mod args;
    pub mod cache;
    #[cfg(unix)]
    pub mod client;
    pub mod logs;
    pub mod orchestrate;
    #[cfg(unix)]
    pub mod protocol;
    pub mod report;
    #[cfg(unix)]
    pub mod server;
    #[cfg_attr(not(unix), allow(dead_code))]
    pub mod stat;
    pub mod units;
    pub mod worker;

    use crate::{ewriteln, vlog};
    use args::{Args, print_usage};
    use cache::Cache;
    use orchestrate::{SourceFile, discover_rust_files, format_all};
    use report::{AggregatedResults, aggregate_results, finish};
    use std::io::{self, Read, Write};
    use std::path::{Path, PathBuf};
    use units::Units;
    use worker::{Caches, PARALLEL_ITEMS_MIN_BYTES, ProcessResult, STDIN_NAME, format_stdin};

    /// Open the fixed-point cache named by the arguments, or `None` when it is disabled by
    /// `--no-cache`, when no cache directory resolves, or when the running executable cannot
    /// be hashed.
    fn open_cache(args: &Args, config: &chloro_core::Config) -> Option<Cache> {
        if args.no_cache {
            vlog!(args, { "Cache: disabled (--no-cache)" });
            return None;
        }
        let Some(dir) = cache::resolve_dir(&args.cache_dir) else {
            vlog!(args, {
                "Cache: disabled (no cache directory; set CHLORO_CACHE_DIR or --cache-dir)"
            });
            return None;
        };
        let cache = Cache::open(&dir, config);
        match &cache {
            Some(cache) => {
                vlog!(args, {"Cache: {} ({} entries)", cache.path().display(), cache.len()})
            }
            None => vlog!(args, {
                "Cache: disabled (cannot read the running executable)"
            }),
        }
        cache
    }

    /// A combination of options no run accepts, as the message to print.
    pub fn invalid_combination(args: &Args) -> Option<&'static str> {
        (args.stdin && args.write).then_some("--write cannot be used with --stdin")
    }

    /// The canonical files named by `args.sources`, each once, sorted; relative sources are
    /// resolved against `base` (the process's directory when `None`). `Ok(Err(code))` when a
    /// source does not exist (after printing the error to `err`).
    ///
    /// # Errors
    ///
    /// The first error canonicalising a file or listing a directory.
    pub fn collect_files(
        args: &Args,
        base: Option<&Path>,
        err: &mut dyn Write,
    ) -> io::Result<Result<Vec<SourceFile>, i32>> {
        let mut all_files = Vec::new();

        for source in &args.sources {
            let source_path = match base {
                Some(base) => base.join(source),
                None => PathBuf::from(source),
            };
            if !source_path.exists() {
                ewriteln!(err, "Error: Source path does not exist: {}", source);
                return Ok(Err(1));
            }

            if source_path.is_file() {
                all_files.push(SourceFile::at(source_path.canonicalize()?));
            } else {
                all_files.extend(discover_rust_files(&source_path)?);
            }
        }
        // A file reachable twice (named twice, through a symlink, or both named and
        // discovered) is processed once: every entry is canonical, so equal files compare
        // equal. Two workers writing one path could otherwise read a truncated file. The one
        // sort of the list: discovery yields each tree in this order already.
        all_files.sort_by(|a, b| a.path.cmp(&b.path));
        all_files.dedup_by(|a, b| a.path == b.path);
        Ok(Ok(all_files))
    }

    /// Threads for formatting the items of one source of `len` bytes.
    pub fn item_threads(args: &Args, len: usize) -> usize {
        if len >= PARALLEL_ITEMS_MIN_BYTES {
            args.thread_count()
        } else {
            1
        }
    }

    /// The `--stdin` source formatted and reported as one file.
    pub fn run_stdin(
        input: io::Result<Vec<u8>>,
        args: &Args,
        caches: Caches<'_>,
        out: &mut dyn Write,
    ) -> AggregatedResults {
        let result = match input {
            Ok(bytes) => {
                let threads = item_threads(args, bytes.len());
                format_stdin(bytes, args, caches, threads, out)
            }
            Err(e) => ProcessResult::Error(format!("Failed to read {STDIN_NAME}: {e}")),
        };
        aggregate_results(vec![result])
    }

    /// Entry point for the chloro CLI
    ///
    /// Formats the Rust sources named on the command line (or standard input with
    /// `--stdin`); `chloro serve` runs the resident server (Unix).
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if:
    /// - command-line argument parsing fails,
    /// - the source directory cannot be read,
    /// - files cannot be parsed,
    /// - or writing files fails.
    ///
    /// The process will also exit with a non-zero status if formatting fails, or in check
    /// mode if a file is not formatted.
    pub fn main() -> io::Result<()> {
        let args: Args = facet_args::from_std_args()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{e}")))?;

        if args.help {
            print_usage();
            std::process::exit(0);
        }

        #[cfg(unix)]
        if server::is_serve_invocation(&args) {
            let code = server::main(&args)?;
            std::process::exit(code);
        }

        let stdin = args.stdin.then(|| {
            let mut bytes = Vec::new();
            io::stdin().read_to_end(&mut bytes).map(|_| bytes)
        });

        #[cfg(unix)]
        if args.via_server_requested()
            && let Some(code) = client::forward(&args, stdin.as_ref())
        {
            if code != 0 {
                std::process::exit(code);
            }
            return Ok(());
        }

        let code = run(&args, stdin)?;
        if code != 0 {
            std::process::exit(code);
        }
        Ok(())
    }

    /// One run of the CLI in this process: the exit code.
    fn run(args: &Args, stdin: Option<io::Result<Vec<u8>>>) -> io::Result<i32> {
        let mut err = io::stderr();
        let config = match args.formatting_config() {
            Ok(config) => config,
            Err(e) => {
                ewriteln!(err, "Error: invalid --config: {e}");
                return Ok(1);
            }
        };
        if let Some(message) = invalid_combination(args) {
            ewriteln!(err, "Error: {message}");
            return Ok(1);
        }

        if args.verbose {
            if args.stdin {
                eprintln!("Sources: {STDIN_NAME}");
            } else {
                eprintln!("Sources: {}", args.sources.join(", "));
            }
            eprintln!(
                "Mode: {}",
                if args.check {
                    "check"
                } else if args.write {
                    "write"
                } else {
                    "print"
                }
            );
            eprintln!();
        }

        // Opened before the files are discovered: the executable is hashed as soon after
        // start-up as possible (`cache::tool_key`).
        let cache = open_cache(args, &config);

        // The unit cache lives next to the fixed-point cache and is read on the first file
        // the fixed-point cache does not answer.
        let open_units = || match &cache {
            Some(cache) if args.unit_cache => Some(Units::new(cache)),
            Some(_) => {
                vlog!(args, { "Unit cache: off (enable with --unit-cache)" });
                None
            }
            None => None,
        };

        let (agg, units) = if let Some(input) = stdin {
            let units = open_units();
            let caches = Caches {
                fixed: cache.as_ref(),
                units: units.as_ref(),
                stats: None,
            };
            (run_stdin(input, args, caches, &mut io::stdout()), units)
        } else {
            let all_files = match collect_files(args, None, &mut err)? {
                Ok(files) => files,
                Err(code) => return Ok(code),
            };

            if all_files.is_empty() {
                if args.verbose {
                    eprintln!("No Rust files found.");
                }
                return Ok(0);
            }

            if args.verbose {
                eprintln!("Found {} Rust file(s)", all_files.len());
                eprintln!("Processing with {} threads", args.thread_count());
                eprintln!();
            }

            let units = open_units();
            let caches = Caches {
                fixed: cache.as_ref(),
                units: units.as_ref(),
                stats: None,
            };
            // Format files in parallel
            let results = format_all(&all_files, args, caches);
            (aggregate_results(results), units)
        };

        if let (Some(units), Some(cache)) = (units, &cache) {
            if let Some(stats) = units.stats() {
                vlog!(args, {
                    "Unit cache: {} ({} entries read, {} bytes); units looked up: {}, found: {}, stored: {}; {} entries, {} record bytes held",
                    units.path().display(),
                    stats.entries_read,
                    stats.file_len,
                    stats.lookups,
                    stats.hits,
                    stats.stores,
                    stats.entries,
                    stats.record_bytes
                });
            }
            let path = units.path().to_path_buf();
            match units.save(cache) {
                Ok(true) => vlog!(args, {"Unit cache written: {}", path.display()}),
                Ok(false) => {}
                Err(e) => vlog!(args, {"Unit cache not written: {}: {e}", path.display()}),
            }
        }

        if let Some(cache) = cache {
            let path = cache.path().to_path_buf();
            match cache.save(agg.fixed_points.iter().copied()) {
                Ok(()) => vlog!(args, {"Cache written: {}", path.display()}),
                Err(e) => vlog!(args, {"Cache not written: {}: {e}", path.display()}),
            }
        }

        // Print the summary; exit with an error if in check mode and files need formatting
        Ok(finish(&agg, args, &mut err))
    }
}

/// Hint replacement CLI for when the cli module is used without building the cli feature.
#[cfg(not(feature = "cli"))]
pub mod cli {
    /// Provide a hint to the user that they did not build this crate with the cli feature.
    pub fn main() {
        eprintln!("Please build with the cli feature to run the CLI");
        eprintln!("Example: cargo install chloro --features cli");
        std::process::exit(1);
    }
}

pub use cli::main;
