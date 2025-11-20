//! Aggregation and reporting for `chloro` CLI.
use super::args::Args;
use super::worker::ProcessResult;

/// Aggregated results of a CLI run.
pub struct AggregatedResults {
    pub files_processed: usize,
    pub files_changed: usize,
    pub files_unchanged: usize,
    pub errors: Vec<String>,
}

impl Default for AggregatedResults {
    fn default() -> Self {
        Self::new()
    }
}

impl AggregatedResults {
    pub fn new() -> Self {
        Self {
            files_processed: 0,
            files_changed: 0,
            files_unchanged: 0,
            errors: Vec::new(),
        }
    }
}

/// Aggregate results from all files
pub fn aggregate_results(results: Vec<ProcessResult>) -> AggregatedResults {
    let mut agg = AggregatedResults::new();

    for result in results {
        match result {
            ProcessResult::Formatted { changed, .. } => {
                agg.files_processed += 1;
                if changed {
                    agg.files_changed += 1;
                } else {
                    agg.files_unchanged += 1;
                }
            }
            ProcessResult::Error(e) => {
                agg.errors.push(e);
            }
        }
    }

    agg
}

/// Print summary report
pub fn print_summary(agg: &AggregatedResults, args: &Args) {
    if !args.verbose && agg.errors.is_empty() {
        return;
    }

    eprintln!();
    eprintln!("=== Formatting Summary ===");
    eprintln!("Files processed: {}", agg.files_processed);
    eprintln!("Files changed: {}", agg.files_changed);
    eprintln!("Files unchanged: {}", agg.files_unchanged);

    if !agg.errors.is_empty() {
        eprintln!();
        eprintln!("Errors: {}", agg.errors.len());
        if args.verbose {
            for error in &agg.errors {
                eprintln!("  - {}", error);
            }
        } else {
            eprintln!("Run with --verbose to see details");
        }
    }
}
