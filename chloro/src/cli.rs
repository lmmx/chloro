//! chloro: A minimal Rust code formatter
//!
//! Command-line interface for formatting Rust source files.

#![allow(clippy::multiple_crate_versions)]

/// Command-line interface for formatting Rust source files.
#[cfg(feature = "cli")]
pub mod cli {
    pub mod args;
    pub mod logs;
    pub mod orchestrate;
    pub mod report;
    pub mod worker;

    use args::{print_usage, Args};
    use orchestrate::{discover_rust_files, format_all};
    use report::{aggregate_results, print_summary};
    use std::io;
    use std::path::Path;

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

        let source_path = Path::new(&args.source);
        if !source_path.exists() {
            eprintln!("Error: Source path does not exist: {}", args.source);
            std::process::exit(1);
        }

        if args.verbose {
            eprintln!("Source: {}", source_path.display());
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

        // Discover files
        let files = if source_path.is_file() {
            vec![source_path.to_path_buf()]
        } else {
            discover_rust_files(source_path)?
        };

        if files.is_empty() {
            if args.verbose {
                eprintln!("No Rust files found.");
            }
            return Ok(());
        }

        // Format files in parallel
        let results = format_all(&files, &args);
        let agg = aggregate_results(results);

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
