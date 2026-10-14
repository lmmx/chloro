use super::args::Args;
use super::cache::{Cache, Digest, content_digest};
use crate::vlog;
use chloro_core::format_source_with_config;
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

/// Process a single Rust source file
pub fn format_file(file_path: &Path, args: &Args, cache: Option<&Cache>) -> ProcessResult {
    vlog!(args, {"Processing: {}", file_path.display()});

    // Read the file
    let original = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            return ProcessResult::Error(format!("Failed to read {}: {}", file_path.display(), e));
        }
    };

    // A cache hit means the content was observed, under this binary and configuration, to
    // come out of the formatter unchanged: the file is already formatted.
    let digest = cache.map(|_| content_digest(original.as_bytes()));
    if let (Some(cache), Some(digest)) = (cache, digest)
        && cache.contains(&digest)
    {
        vlog!(args, { "  Cached: already formatted" });
        if !args.write && !args.check {
            // Print mode: the formatted output equals the input.
            if let Err(e) = io::stdout().write_all(original.as_bytes()) {
                return ProcessResult::Error(format!("Failed to write to stdout: {}", e));
            }
        }
        return ProcessResult::Formatted {
            path: file_path.display().to_string(),
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
    let formatted = format_source_with_config(&original, &config);

    let changed = original != formatted;
    let original_len = original.len();
    let formatted_len = formatted.len();

    // Unchanged content is a fixed point by observation. Content this run writes is recorded
    // only when a second pass leaves it unchanged, so every recorded digest was observed to
    // be a fixed point; the check costs one more format of each rewritten file.
    let mut fixed_point = if changed { None } else { digest };

    // Handle output based on mode
    if args.write {
        if changed {
            if let Err(e) = fs::write(file_path, &formatted) {
                return ProcessResult::Error(format!(
                    "Failed to write {}: {}",
                    file_path.display(),
                    e
                ));
            }
            vlog!(args, { "  Wrote formatted output" });
            if cache.is_some() && format_source_with_config(&formatted, &config) == formatted {
                fixed_point = Some(content_digest(formatted.as_bytes()));
            }
        } else {
            vlog!(args, { "  No changes needed" });
        }
    } else if !args.check {
        // Print to stdout (only if not in check mode)
        if let Err(e) = io::stdout().write_all(formatted.as_bytes()) {
            return ProcessResult::Error(format!("Failed to write to stdout: {}", e));
        }
    }

    ProcessResult::Formatted {
        path: file_path.display().to_string(),
        changed,
        original_len,
        formatted_len,
        cached: false,
        fixed_point,
    }
}
