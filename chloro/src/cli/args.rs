use facet::Facet;

#[derive(Facet)]
pub struct Args {
    /// Files or directories to format (default: the current directory)
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

    /// The configuration file for every file, or a directory holding one (default: each
    /// file's nearest `rustfmt.toml` or `.rustfmt.toml`)
    #[facet(named, long, default)]
    pub config_path: String,

    /// Read no configuration file and no `Cargo.toml`: only `--config` applies
    #[facet(named, long, default)]
    pub isolated: bool,

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

    /// Format standard input and write the result to standard output
    #[facet(named, long, default)]
    pub stdin: bool,

    /// With `--stdin`: the path the text belongs to, for its configuration and its messages
    #[facet(named, long, default)]
    pub stdin_filepath: String,

    /// Show this help message
    #[facet(named, short = 'h', long, default)]
    pub help: bool,

    /// Print the version (and the build ID `just reinstall` records) and exit
    #[facet(named, short = 'V', long, default)]
    pub version: bool,
}

pub fn print_usage() {
    println!("Usage: chloro [OPTIONS] [SOURCES]...");
    println!();
    println!("A Rust code formatter reproducing `rustfmt --edition 2024`.");
    println!();
    println!("Arguments:");
    println!("  [SOURCES]...       Files or directories to format (default: '.', the current");
    println!("                     directory). Directories are walked for .rs files, skipping");
    println!("                     hidden entries and cache directories such as target/");
    println!();
    println!("Options:");
    println!("  -c, --check        Check if files need formatting (exit 1 if so)");
    println!("  -w, --write        Write formatted output back to files");
    println!("  -v, --verbose      Show verbose output");
    println!("      --config OPTS  rustfmt options as comma-separated key=value pairs, over");
    println!("                     the configuration file's");
    println!("      --config-path PATH");
    println!("                     The rustfmt.toml for every file, or a directory holding one");
    println!("      --isolated     Read no rustfmt.toml and no Cargo.toml edition");
    println!("      --stdin        Format standard input to standard output");
    println!("      --stdin-filepath PATH");
    println!("                     With --stdin: the file the text belongs to, whose");
    println!("                     configuration applies (default: the current directory's)");
    println!("      --no-cache     Format every file; do not read or write the caches");
    println!("      --unit-cache   Also cache each formatted top-level item, so that a changed");
    println!("                     file formats only the items not seen before (off by default)");
    println!("      --cache-dir D  Cache directory (default: $CHLORO_CACHE_DIR, else the");
    println!("                     user cache directory, e.g. ~/.cache/chloro)");
    println!("      --threads N    Threads to use (default: one per core)");
    println!("  -V, --version      Print the version and exit");
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
    println!("  # Format an editor's buffer, under the configuration of its file");
    println!("  chloro --stdin --stdin-filepath src/lib.rs < buffer.rs");
    println!();
    println!("  # Format with rustfmt options");
    println!("  chloro --config max_width=80,tab_spaces=2 src/lib.rs");
    println!();
    println!("Configuration:");
    println!("  As rustfmt, chloro reads each file's nearest .rustfmt.toml or rustfmt.toml");
    println!("  (else the one in the home or the user configuration directory), and, as cargo");
    println!("  fmt, takes the edition from the nearest Cargo.toml's [package]. --config");
    println!("  overrides both.");
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
        for (key, value) in self.config_pairs()? {
            config.set(&key, &value)?;
        }
        Ok(config)
    }

    /// The `key=value` pairs of `--config`, in order.
    ///
    /// # Errors
    ///
    /// Returns a message naming the first pair without `=`.
    pub fn config_pairs(&self) -> Result<Vec<(String, String)>, String> {
        self.config
            .split(',')
            .filter(|o| !o.trim().is_empty())
            .map(|option| {
                let (key, value) = option
                    .split_once('=')
                    .ok_or_else(|| format!("expected key=value, found `{option}`"))?;
                Ok((key.trim().to_owned(), value.trim().to_owned()))
            })
            .collect()
    }
}
