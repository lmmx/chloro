use super::args::Args;
use super::cache::{Cache, Digest, content_digest};
use super::stat::{StatTable, probe};
use super::units::Units;
use crate::vlog;
use chloro_core::{
    format_source_with_config, format_source_with_config_parallel, format_source_with_unit_cache,
};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Result of processing a single file
#[derive(Debug)]
pub enum ProcessResult {
    /// File was formatted successfully
    Formatted {
        path: String,
        changed: bool,
        original_len: usize,
        formatted_len: usize,
        /// The file was reported from the cache without running the formatter.
        cached: bool,
        /// Digest of content observed to be a fixed point of the formatter in this run (the
        /// content on disk after this run), for the cache to record.
        fixed_point: Option<Digest>,
    },
    /// File formatting failed
    Error(String),
}

/// The caches a run consults: the fixed-point cache, the unit cache, and (in a resident
/// server only) the stamps of the files read before.
#[derive(Clone, Copy, Default)]
pub struct Caches<'a> {
    pub fixed: Option<&'a Cache>,
    pub units: Option<&'a Units>,
    pub stats: Option<&'a StatTable>,
}

/// Files of at least this many bytes have their top-level items formatted on `threads`
/// threads. In process the split pays from about 8 KiB, but the CLI already runs one
/// file per core: splitting every medium file oversubscribes the cores and costs more
/// than it saves on a crate, while splitting only the largest files shortens the wall
/// time of crates whose time is one file's (measured in
/// `docs/journal/2026-10-14-autoresearch/experiments/EXP-001-parallel-items.md`).
pub const PARALLEL_ITEMS_MIN_BYTES: usize = 128 * 1024;

/// Name under which `--stdin` reports its source.
pub const STDIN_NAME: &str = "<stdin>";

/// Process a single Rust source file: a cache hit is reported without formatting; otherwise
/// the file is formatted, its items on up to `threads` threads when it is large enough, and
/// with the unit cache when there is one (only the items it does not hold are formatted).
pub fn format_file(
    file_path: &Path,
    args: &Args,
    caches: Caches<'_>,
    threads: usize,
) -> ProcessResult {
    vlog!(args, {"Processing: {}", file_path.display()});

    // A resident server answers a file whose stamp is the one it last read, if that content
    // is a known fixed point, without reading it (`stat.rs`). Not in print mode, which
    // prints the content.
    let stamp = match (caches.stats, caches.fixed) {
        (Some(_), Some(_)) if args.check || args.write => probe(file_path),
        _ => None,
    };
    if let (Some(stats), Some(cache), Some(stamp)) = (caches.stats, caches.fixed, &stamp)
        && let Some(digest) = stats.digest(file_path, stamp)
        && cache.contains(&digest)
    {
        let len = usize::try_from(stamp.stamp.len).unwrap_or(usize::MAX);
        return ProcessResult::Formatted {
            path: file_path.display().to_string(),
            changed: false,
            original_len: len,
            formatted_len: len,
            cached: true,
            fixed_point: Some(digest),
        };
    }

    // Read the file
    let original = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            return ProcessResult::Error(format!("Failed to read {}: {}", file_path.display(), e));
        }
    };
    let digest = caches.fixed.map(|_| content_digest(original.as_bytes()));
    if let (Some(stats), Some(stamp), Some(digest)) = (caches.stats, stamp, digest) {
        stats.record(file_path, stamp, digest);
    }

    let source = Source::File(file_path);
    format_content(
        &source,
        original,
        digest,
        args,
        caches,
        threads,
        &mut io::stdout(),
    )
}

/// Where the content came from, and where print and write mode put the output.
enum Source<'a> {
    File(&'a Path),
    Stdin,
}

impl Source<'_> {
    fn name(&self) -> String {
        match self {
            Source::File(path) => path.display().to_string(),
            Source::Stdin => STDIN_NAME.to_string(),
        }
    }
}

/// Format the source read by `--stdin`, printing the result to `out` unless in check mode.
/// `bytes` that are not UTF-8 are reported as a file that cannot be read.
pub fn format_stdin(
    bytes: Vec<u8>,
    args: &Args,
    caches: Caches<'_>,
    threads: usize,
    out: &mut dyn Write,
) -> ProcessResult {
    vlog!(args, {"Processing: {}", STDIN_NAME});
    let original = match String::from_utf8(bytes) {
        Ok(content) => content,
        Err(_) => {
            return ProcessResult::Error(format!(
                "Failed to read {STDIN_NAME}: stream did not contain valid UTF-8"
            ));
        }
    };
    let digest = caches.fixed.map(|_| content_digest(original.as_bytes()));
    format_content(&Source::Stdin, original, digest, args, caches, threads, out)
}

/// The fixed-point cache check, the format, and the output of one source whose content is
/// `original` (with its `digest` when there is a fixed-point cache).
fn format_content(
    source: &Source<'_>,
    original: String,
    digest: Option<Digest>,
    args: &Args,
    caches: Caches<'_>,
    threads: usize,
    out: &mut dyn Write,
) -> ProcessResult {
    // A cache hit means the content was observed, under this binary and configuration, to
    // come out of the formatter unchanged: the file is already formatted.
    if let (Some(cache), Some(digest)) = (caches.fixed, digest)
        && cache.contains(&digest)
    {
        vlog!(args, { "  Cached: already formatted" });
        if !args.write && !args.check {
            // Print mode: the formatted output equals the input.
            if let Err(e) = out.write_all(original.as_bytes()) {
                return ProcessResult::Error(format!("Failed to write to stdout: {}", e));
            }
        }
        return ProcessResult::Formatted {
            path: source.name(),
            changed: false,
            original_len: original.len(),
            formatted_len: original.len(),
            cached: true,
            fixed_point: Some(digest),
        };
    }

    // Format it. `--config` was validated before any file was processed.
    let config = match args.formatting_config() {
        Ok(config) => config,
        Err(e) => return ProcessResult::Error(format!("Invalid --config: {e}")),
    };
    let formatted = if let Some(units) = caches.units {
        format_source_with_unit_cache(&original, &config, threads, units.store())
    } else if threads > 1 && original.len() >= PARALLEL_ITEMS_MIN_BYTES {
        format_source_with_config_parallel(&original, &config, threads)
    } else {
        format_source_with_config(&original, &config)
    };

    let changed = original != formatted;
    let original_len = original.len();
    let formatted_len = formatted.len();

    // Unchanged content is a fixed point by observation. Content this run writes is recorded
    // only when a second pass leaves it unchanged, so every recorded digest was observed to
    // be a fixed point; the check costs one more format of each rewritten file.
    let mut fixed_point = if changed { None } else { digest };

    // Handle output based on mode
    if args.write {
        if let (true, Source::File(file_path)) = (changed, source) {
            if let Err(e) = fs::write(file_path, &formatted) {
                return ProcessResult::Error(format!(
                    "Failed to write {}: {}",
                    file_path.display(),
                    e
                ));
            }
            vlog!(args, { "  Wrote formatted output" });
            if let Some(stats) = caches.stats {
                stats.forget(file_path);
            }
            if caches.fixed.is_some() && format_source_with_config(&formatted, &config) == formatted
            {
                fixed_point = Some(content_digest(formatted.as_bytes()));
            }
        } else {
            vlog!(args, { "  No changes needed" });
        }
    } else if !args.check {
        // Print to stdout (only if not in check mode)
        if let Err(e) = out.write_all(formatted.as_bytes()) {
            return ProcessResult::Error(format!("Failed to write to stdout: {}", e));
        }
    }

    ProcessResult::Formatted {
        path: source.name(),
        changed,
        original_len,
        formatted_len,
        cached: false,
        fixed_point,
    }
}
