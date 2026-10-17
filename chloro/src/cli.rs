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
    pub mod logs;
    pub mod orchestrate;
    pub mod project;
    pub mod report;
    pub mod stdin;
    pub mod trace;
    pub mod units;
    pub mod worker;

    use crate::vlog;
    use args::{Args, print_usage};
    use cache::{Cache, Tool};
    use orchestrate::{SourceFile, discover_rust_files, format_all};
    use project::Project;
    use report::{aggregate_results, print_summary};
    use std::io;
    use std::path::Path;
    use units::Units;
    use worker::Setup;

    /// The executable's identity for the fixed-point caches named by the arguments, or `None`
    /// when they are disabled by `--no-cache`, when no cache directory resolves, or when the
    /// running executable cannot be hashed.
    fn identify_tool(args: &Args) -> Option<Tool> {
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
        let tool = Tool::identify(&dir);
        if tool.is_none() {
            vlog!(args, {
                "Cache: disabled (cannot read the running executable)"
            });
        }
        tool
    }

    /// The setup of `config`: its fixed-point cache when `tool` is given, and its unit cache
    /// next to that with `--unit-cache`.
    fn setup(args: &Args, tool: Option<&Tool>, config: &chloro_core::Config) -> Setup {
        let cache = tool.map(|tool| Cache::open_for(tool, config));
        if let Some(cache) = &cache {
            vlog!(args, {"Cache: {} ({} entries)", cache.path().display(), cache.len()});
        }
        // The unit cache lives next to the fixed-point cache and is read on the first file
        // the fixed-point cache does not answer.
        let units = cache.as_ref().filter(|_| args.unit_cache).map(Units::new);
        Setup {
            config: config.clone(),
            cache,
            units,
        }
    }

    /// Write `setup`'s caches: its unit cache, then its fixed-point cache with `fixed_points`
    /// recorded.
    fn save_caches(args: &Args, setup: Setup, fixed_points: Vec<cache::Digest>) {
        let Some(cache) = setup.cache else {
            return;
        };
        if let Some(units) = setup.units {
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
            match units.save(&cache) {
                Ok(true) => vlog!(args, {"Unit cache written: {}", path.display()}),
                Ok(false) => {}
                Err(e) => vlog!(args, {"Unit cache not written: {}: {e}", path.display()}),
            }
        }
        let path = cache.path().to_path_buf();
        match cache.save(fixed_points) {
            Ok(()) => vlog!(args, {"Cache written: {}", path.display()}),
            Err(e) => vlog!(args, {"Cache not written: {}: {e}", path.display()}),
        }
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
        trace::init();
        let args: Args = facet_args::from_std_args()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("{e}")))?;

        if args.help {
            print_usage();
            std::process::exit(0);
        }

        if args.version {
            // `build.rs` sets the version (`-dev` unless the source is the tagged release) and
            // its details; `just reinstall` records the checkout's build ID (`just build-id`)
            let mut details: Vec<&str> = env!("CHLORO_VERSION_DETAILS")
                .split(", ")
                .filter(|d| !d.is_empty())
                .collect();
            let build = option_env!("CHLORO_BUILD_ID").map(|id| format!("build {id}"));
            details.extend(build.as_deref());
            match details.is_empty() {
                true => println!("chloro {}", env!("CHLORO_VERSION")),
                false => println!("chloro {} ({})", env!("CHLORO_VERSION"), details.join(", ")),
            }
            std::process::exit(0);
        }

        if let Err(e) = args.formatting_config() {
            eprintln!("Error: invalid --config: {e}");
            std::process::exit(1);
        }
        let mut project = match Project::new(&args) {
            Ok(project) => project,
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        };

        if args.stdin || !args.stdin_filepath.is_empty() {
            std::process::exit(stdin::format_stdin(&args, &mut project));
        }

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

        // Identified before the files are discovered: the executable is hashed as soon after
        // start-up as possible (`cache::tool_key`).
        trace::mark("args");
        let tool = identify_tool(&args);
        trace::mark("cache");

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
        trace::mark("walk");
        all_files.sort_by(|a, b| a.path.cmp(&b.path));
        all_files.dedup_by(|a, b| a.path == b.path);
        trace::mark("sort");

        if all_files.is_empty() {
            if args.verbose {
                eprintln!("No Rust files found.");
            }
            return Ok(());
        }

        // Each file's configuration, before any file is formatted: a configuration file
        // that rustfmt would refuse stops the run, with rustfmt's message.
        for file in &mut all_files {
            file.config = match project.config_of(&file.path) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            };
        }
        trace::mark("config");

        if args.verbose {
            eprintln!("Found {} Rust file(s)", all_files.len());
            eprintln!("Processing with {} threads", args.thread_count());
            eprintln!();
        }

        let setups: Vec<Setup> = project
            .configs()
            .iter()
            .map(|config| setup(&args, tool.as_ref(), config))
            .collect();
        if tool.is_some() && !args.unit_cache {
            vlog!(args, { "Unit cache: off (enable with --unit-cache)" });
        }

        // Format files in parallel
        let results = format_all(&all_files, &args, &setups);
        trace::mark("formatted");
        let mut agg = aggregate_results(results, setups.len());

        for (setup, fixed_points) in setups
            .into_iter()
            .zip(std::mem::take(&mut agg.fixed_points))
        {
            save_caches(&args, setup, fixed_points);
        }

        trace::mark("saved");
        // As rustfmt reports a file that does not parse: its error, and exit status 1
        for diagnostic in &agg.parse_errors {
            eprint!("{diagnostic}");
        }
        // Print summary
        print_summary(&agg, &args);
        trace::finish();

        // Exit with error if in check mode and files need formatting
        if args.check && agg.files_changed > 0 {
            eprintln!();
            eprintln!("Error: {} file(s) need formatting", agg.files_changed);
            std::process::exit(1);
        }

        if !agg.errors.is_empty() || !agg.parse_errors.is_empty() {
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
