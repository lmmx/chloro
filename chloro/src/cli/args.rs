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
}

impl Args {
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
