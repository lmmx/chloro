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

    /// Read the source from standard input; print it formatted (or, with `--check`, exit 1
    /// if it is not formatted)
    #[facet(named, long, default)]
    pub stdin: bool,

    /// Send the request to a running `chloro serve` (Unix); run here if none answers
    #[facet(named, long, default)]
    pub via_server: bool,

    /// Socket of `chloro serve` (default: `chloro.sock` in the cache directory)
    #[facet(named, long, default)]
    pub socket: String,

    /// `chloro serve`: exit after this many seconds without a request (0: never)
    #[facet(named, long, default = 1800)]
    pub idle_timeout: u64,

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
    println!("      --stdin        Read the source from standard input and print it formatted");
    println!("                     (with --check: exit 1 if it is not formatted)");
    println!("      --via-server   Send the run to a running `chloro serve` (Unix; also");
    println!("                     CHLORO_VIA_SERVER=1); run here if none answers");
    println!("      --socket PATH  Socket of `chloro serve` (default: <cache dir>/chloro.sock, or");
    println!("                     $XDG_RUNTIME_DIR/chloro-<hash>.sock if that path is too long)");
    println!("  -h, --help         Show this help message");
    println!();
    println!("Server (Unix):");
    println!("  chloro serve [--socket PATH] [--idle-timeout SECS] [--cache-dir D] [--threads N]");
    println!();
    println!("  --threads N: threads for a request of several files (default: one per core)");
    println!("  and for the items of a large file in a request of one source (default: 1;");
    println!("  item threads started on every request grow the server's memory).");
    println!();
    println!("  Keeps the caches of one cache directory in memory and answers requests on a");
    println!("  Unix-domain socket (one server per cache directory). It exits after");
    println!("  --idle-timeout seconds without a request (default 1800; 0: never), when the");
    println!("  chloro executable on disk changes, or on a `stop` request. `chloro --check");
    println!("  --via-server PATHS` (also --write, --stdin) sends the run to it and prints its");
    println!("  reply; the output and exit code are those of the same run without the server.");
    println!("  -v, --help, and print mode on paths always run in the client.");
    println!();
    println!("  Protocol: a message is a sequence of frames `NAME LEN\\n` + LEN bytes, ended");
    println!("  by `end 0\\n`. A request starts with `chloro 1`, then `exe` (the client's");
    println!("  executable identity), `cwd`, `cache-dir`, `config`, `threads`, `flag`");
    println!("  (`no-cache` or `unit-cache`, repeated), then sources: `check PATH`...,");
    println!("  `write PATH`..., `stdin BYTES` or `stdin-check BYTES`; or alone `ping`,");
    println!("  `status` or `stop`. The reply: `status` (`ok`, or `fallback` with a `reason`:");
    println!("  the client runs the request itself), `exit`, `stdout`, `stderr`, `end`.");
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
    println!("  # Format standard input");
    println!("  chloro --stdin < src/lib.rs");
    println!();
    println!("  # Keep a server running and check through it");
    println!("  chloro serve &");
    println!("  chloro --check --via-server src/lib.rs");
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

/// Environment variable that sends runs to a running `chloro serve`, as `--via-server`
/// does, when set to `1`.
pub const VIA_SERVER_ENV: &str = "CHLORO_VIA_SERVER";

impl Args {
    /// Whether `--via-server` or `CHLORO_VIA_SERVER=1` asks for the server.
    pub fn via_server_requested(&self) -> bool {
        self.via_server || std::env::var_os(VIA_SERVER_ENV).is_some_and(|v| v == "1")
    }

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
