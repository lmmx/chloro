use facet::Facet;

#[derive(Facet)]
pub struct Args {
    /// Path(s) to source file(s) or directory to format
    #[facet(positional, default = vec![".".to_string()])]
    pub sources: Vec<String>,

    /// Check mode: exit with error if files need formatting
    #[facet(named, short = 'c', long, default)]
    pub check: bool,

    /// Write formatted output back to files (default: print to stdout)
    #[facet(named, short = 'w', long, default)]
    pub write: bool,

    /// Show verbose output
    #[facet(named, short = 'v', long, default)]
    pub verbose: bool,

    /// rustfmt options as comma-separated `key=value` pairs, e.g. `max_width=80,tab_spaces=2`
    #[facet(named, long, default)]
    pub config: String,

    /// Do not read or write the fixed-point cache
    #[facet(named, long, default)]
    pub no_cache: bool,

    /// Also keep the formatted output of each top-level item, so that a changed file
    /// formats only the items not seen before (off by default; `--no-cache` overrides it)
    #[facet(named, long, default)]
    pub unit_cache: bool,

    /// Directory of the fixed-point cache (default: `CHLORO_CACHE_DIR`, else the user cache
    /// directory)
    #[facet(named, long, default)]
    pub cache_dir: String,

    /// Threads to use (default: one per core)
    #[facet(named, long, default)]
    pub threads: usize,

    /// Show this help message
    #[facet(named, short = 'h', long, default)]
    pub help: bool,
}

pub fn print_usage() {
    println!("Usage: chloro [OPTIONS] [SOURCES]...");
    println!();
    println!("A Rust code formatter reproducing `rustfmt --edition 2024`.");
    println!();
    println!("Arguments:");
    println!("  [SOURCES]...       Path(s) to file(s) or directory to format (default: 'src')");
    println!();
    println!("Options:");
    println!("  -c, --check        Check if files need formatting (exit 1 if so)");
    println!("  -w, --write        Write formatted output back to files");
    println!("  -v, --verbose      Show verbose output");
    println!("      --config OPTS  rustfmt options as comma-separated key=value pairs");
    println!("      --no-cache     Format every file; do not read or write the caches");
    println!("      --unit-cache   Also cache each formatted top-level item, so that a changed");
    println!("                     file formats only the items not seen before (off by default)");
    println!("      --cache-dir D  Cache directory (default: $CHLORO_CACHE_DIR, else the");
    println!("                     user cache directory, e.g. ~/.cache/chloro)");
    println!("      --threads N    Threads to use (default: one per core)");
    println!("  -h, --help         Show this help message");
    println!();
    println!("Examples:");
    println!("  # Format a single file and print to stdout");
    println!("  chloro src/lib.rs");
    println!();
    println!("  # Format multiple files");
    println!("  chloro src/*.rs");
    println!();
    println!("  # Check if files need formatting");
    println!("  chloro --check src/");
    println!();
    println!("  # Format files in-place");
    println!("  chloro --write src/");
    println!();
    println!("  # Format with rustfmt options");
    println!("  chloro --config max_width=80,tab_spaces=2 src/lib.rs");
    println!();
    println!("Cache:");
    println!("  chloro records the content hash of every file it finds already formatted,");
    println!("  keyed by the chloro binary and the --config options, and skips formatting");
    println!("  files whose content it has seen formatted before. The output and exit code");
    println!("  are the same with and without the cache.");
    println!();
    println!("  With --unit-cache, chloro also keeps the formatted output of each top-level");
    println!("  item (or group of `use`/`mod`/`extern crate` items) under the same key and");
    println!("  the item's text, and formats only the items of a changed file it has not");
    println!("  seen. The output and exit code are the same. It is off by default: a run that");
    println!("  finds nothing in it (a cold CI run) costs 10-15% more with it on.");
}

impl Args {
    /// `--threads`, or one thread per core.
    pub fn thread_count(&self) -> usize {
        if self.threads > 0 {
            self.threads
        } else {
            std::thread::available_parallelism().map_or(1, |n| n.get())
        }
    }

    /// The formatting configuration given by `--config`.
    ///
    /// # Errors
    ///
    /// Returns a message naming the first option that is malformed, unknown, or has an
    /// invalid value.
    pub fn formatting_config(&self) -> Result<chloro_core::Config, String> {
        let mut config = chloro_core::Config::default();
        for option in self.config.split(',').filter(|o| !o.trim().is_empty()) {
            let (key, value) = option
                .split_once('=')
                .ok_or_else(|| format!("expected key=value, found `{option}`"))?;
            config.set(key.trim(), value.trim())?;
        }
        Ok(config)
    }
}
