//! chloro: A minimal Rust code formatter
//!
//! Command-line interface for formatting Rust source files.

#![allow(clippy::multiple_crate_versions)]

/// Command-line interface for formatting Rust source files.
#[cfg(feature = "cli")]
pub mod cli {
    pub mod args;
    pub mod cache;
    pub mod logs;
    pub mod orchestrate;
    pub mod report;
    pub mod units;
    pub mod worker;

    use crate::vlog;
    use args::{Args, print_usage};
    use cache::Cache;
    use orchestrate::{SourceFile, discover_rust_files, format_all};
    use report::{aggregate_results, print_summary};
    use std::io;
    use std::path::Path;
    use units::Units;

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

    /// Entry point for the chloro CLI
    ///
    /// Migrates Rust documentation to markdown files.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if:
    /// - command-line argument parsing fails,
    /// - the source directory cannot be read,
    /// - files cannot be parsed,
    /// - or writing files fails.
    ///
    /// The process will also exit with a non-zero status if migration fails.
    pub fn main() -> io::Result<()> {
        let args: Args = facet_args::from_std_args()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{e}")))?;

        if args.help {
            print_usage();
            std::process::exit(0);
        }

        let config = match args.formatting_config() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Error: invalid --config: {e}");
                std::process::exit(1);
            }
        };

        if args.verbose {
            eprintln!("Sources: {}", args.sources.join(", "));
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
        let cache = open_cache(&args, &config);

        // Collect all files from all source paths
        let mut all_files = Vec::new();

        for source in &args.sources {
            let source_path = Path::new(source);
            if !source_path.exists() {
                eprintln!("Error: Source path does not exist: {}", source);
                std::process::exit(1);
            }

            if source_path.is_file() {
                all_files.push(SourceFile::at(source_path.canonicalize()?));
            } else {
                all_files.extend(discover_rust_files(source_path)?);
            }
        }
        // A file reachable twice (named twice, through a symlink, or both named and
        // discovered) is processed once: every entry is canonical, so equal files compare
        // equal. Two workers writing one path could otherwise read a truncated file. The one
        // sort of the list: discovery yields each tree in this order already.
        all_files.sort_by(|a, b| a.path.cmp(&b.path));
        all_files.dedup_by(|a, b| a.path == b.path);

        if all_files.is_empty() {
            if args.verbose {
                eprintln!("No Rust files found.");
            }
            return Ok(());
        }

        if args.verbose {
            eprintln!("Found {} Rust file(s)", all_files.len());
            eprintln!("Processing with {} threads", args.thread_count());
            eprintln!();
        }

        // The unit cache lives next to the fixed-point cache and is read on the first file
        // the fixed-point cache does not answer.
        let units = match &cache {
            Some(cache) if args.unit_cache => Some(Units::new(cache)),
            Some(_) => {
                vlog!(args, { "Unit cache: off (enable with --unit-cache)" });
                None
            }
            None => None,
        };

        // Format files in parallel
        let results = format_all(&all_files, &args, cache.as_ref(), units.as_ref());
        let agg = aggregate_results(results);

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

        // Print summary
        print_summary(&agg, &args);

        // Exit with error if in check mode and files need formatting
        if args.check && agg.files_changed > 0 {
            eprintln!();
            eprintln!("Error: {} file(s) need formatting", agg.files_changed);
            std::process::exit(1);
        }

        if !agg.errors.is_empty() {
            std::process::exit(1);
        }

        Ok(())
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
