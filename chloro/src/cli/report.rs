//! Aggregation and reporting for `chloro` CLI.
use super::args::Args;
use super::cache::Digest;
use super::worker::ProcessResult;

/// Aggregated results of a CLI run.
pub struct AggregatedResults {
    pub files_processed: usize,
    pub files_changed: usize,
    pub files_unchanged: usize,
    /// Files reported as formatted from the cache without running the formatter.
    pub cache_hits: usize,
    /// Content digests confirmed as fixed points in this run, to record in the cache.
    pub fixed_points: Vec<Digest>,
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
            cache_hits: 0,
            fixed_points: Vec::new(),
            errors: Vec::new(),
        }
    }
}

/// Aggregate results from all files
pub fn aggregate_results(results: Vec<ProcessResult>) -> AggregatedResults {
    let mut agg = AggregatedResults::new();

    for result in results {
        match result {
            ProcessResult::Formatted {
                changed,
                cached,
                fixed_point,
                ..
            } => {
                agg.files_processed += 1;
                if changed {
                    agg.files_changed += 1;
                } else {
                    agg.files_unchanged += 1;
                }
                if cached {
                    agg.cache_hits += 1;
                }
                agg.fixed_points.extend(fixed_point);
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
    if args.verbose {
        eprintln!("Cache hits: {}", agg.cache_hits);
    }

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
