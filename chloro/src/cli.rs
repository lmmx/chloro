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
    pub mod worker;

    use crate::vlog;
    use args::{Args, print_usage};
    use cache::Cache;
    use orchestrate::{discover_rust_files, format_all};
    use report::{aggregate_results, print_summary};
    use std::io;
    use std::path::Path;

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

        // Collect all files from all source paths
        let mut all_files = Vec::new();

        for source in &args.sources {
            let source_path = Path::new(source);
            if !source_path.exists() {
                eprintln!("Error: Source path does not exist: {}", source);
                std::process::exit(1);
            }

            if source_path.is_file() {
                all_files.push(source_path.to_path_buf());
            } else {
                all_files.extend(discover_rust_files(source_path)?);
            }
        }

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

        let cache = open_cache(&args, &config);

        // Format files in parallel
        let results = format_all(&all_files, &args, cache.as_ref());
        let agg = aggregate_results(results);

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
