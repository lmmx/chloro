//! Aggregation and reporting for `chloro` CLI.
use super::args::Args;
use super::cache::Digest;
use super::worker::ProcessResult;
use chloro_core::ParseError;

/// Aggregated results of a CLI run.
pub struct AggregatedResults {
    pub files_processed: usize,
    pub files_changed: usize,
    pub files_unchanged: usize,
    /// The errors of the files that do not parse, as rustfmt prints them, in file order.
    pub parse_errors: Vec<String>,
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
            parse_errors: Vec::new(),
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
            ProcessResult::ParseError { diagnostic, .. } => {
                agg.files_processed += 1;
                agg.parse_errors.push(diagnostic);
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
    if !agg.parse_errors.is_empty() {
        eprintln!("Files that do not parse: {}", agg.parse_errors.len());
    }
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

/// A parse error of the file at `path`, whose text is `source`, laid out as rustfmt prints
/// one (rustc's diagnostics): the message, the position, and the line with the error marked,
/// followed by a blank line.
///
/// ```text
/// error: expected expression, found `;`
///  --> src/main.rs:2:13
///   |
/// 2 |     let x = ;
///   |             ^
///
/// ```
///
/// As in rustc, a tab is shown as four spaces. The message is rust-analyzer's or chloro's
/// ([`ParseError`]), not rustc's.
pub fn render_parse_error(path: &str, source: &str, error: &ParseError) -> String {
    let number = error.line().to_string();
    let pad = " ".repeat(number.len());
    let line = source.lines().nth(error.line() - 1).unwrap_or("");
    let line = line.strip_prefix('\u{feff}').unwrap_or(line);
    let indent: usize = line
        .chars()
        .take(error.column() - 1)
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum();
    let shown = line.replace('\t', "    ");
    let sep = if shown.is_empty() { "" } else { " " };
    format!(
        "error: {message}\n{pad}--> {path}:{number}:{column}\n{pad} |\n{number} |{sep}{shown}\n{pad} | {spaces}{marks}\n\n",
        message = error.message(),
        column = error.column(),
        spaces = " ".repeat(indent),
        marks = "^".repeat(error.width()),
    )
}
