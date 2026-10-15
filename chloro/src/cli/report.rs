//! Aggregation and reporting for `chloro` CLI.
use super::args::Args;
use super::cache::Digest;
use super::worker::ProcessResult;
use std::io::Write;

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

/// Writes a line to `err` as `eprintln!` writes to standard error: a failed write panics.
#[macro_export]
macro_rules! ewriteln {
    ($err:expr) => {
        if let Err(e) = writeln!($err) {
            panic!("failed printing to stderr: {e}");
        }
    };
    ($err:expr, $($arg:tt)*) => {
        if let Err(e) = writeln!($err, $($arg)*) {
            panic!("failed printing to stderr: {e}");
        }
    };
}

/// Write the summary report to `err` (standard error for the CLI, the reply for a server)
pub fn print_summary(agg: &AggregatedResults, args: &Args, err: &mut dyn Write) {
    if !args.verbose && agg.errors.is_empty() {
        return;
    }

    ewriteln!(err);
    ewriteln!(err, "=== Formatting Summary ===");
    ewriteln!(err, "Files processed: {}", agg.files_processed);
    ewriteln!(err, "Files changed: {}", agg.files_changed);
    ewriteln!(err, "Files unchanged: {}", agg.files_unchanged);
    if args.verbose {
        ewriteln!(err, "Cache hits: {}", agg.cache_hits);
    }

    if !agg.errors.is_empty() {
        ewriteln!(err);
        ewriteln!(err, "Errors: {}", agg.errors.len());
        if args.verbose {
            for error in &agg.errors {
                ewriteln!(err, "  - {}", error);
            }
        } else {
            ewriteln!(err, "Run with --verbose to see details");
        }
    }
}

/// The end of a run: the summary, the check-mode error, and the exit code.
pub fn finish(agg: &AggregatedResults, args: &Args, err: &mut dyn Write) -> i32 {
    print_summary(agg, args, err);

    // Exit with error if in check mode and files need formatting
    if args.check && agg.files_changed > 0 {
        ewriteln!(err);
        ewriteln!(err, "Error: {} file(s) need formatting", agg.files_changed);
        return 1;
    }

    if !agg.errors.is_empty() {
        return 1;
    }
    0
}
