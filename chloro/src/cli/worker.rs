use super::args::Args;
use crate::vlog;
use chloro_core::{format_source_with_config, format_source_with_config_parallel};
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
    },
    /// File formatting failed
    Error(String),
}

/// Files of at least this many bytes have their top-level items formatted on `threads`
/// threads; below it the thread start-up costs more than the split saves (measured in
/// `docs/journal/2026-10-14-autoresearch/experiments/EXP-001-parallel-items.md`).
pub const PARALLEL_ITEMS_MIN_BYTES: usize = 32 * 1024;

/// Process a single Rust source file, formatting its items on up to `threads` threads
/// when it is large enough.
pub fn format_file(file_path: &Path, args: &Args, threads: usize) -> ProcessResult {
    vlog!(args, {"Processing: {}", file_path.display()});

    // Read the file
    let original = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            return ProcessResult::Error(format!("Failed to read {}: {}", file_path.display(), e));
        }
    };

    // Format it. `--config` was validated before any file was processed.
    let config = match args.formatting_config() {
        Ok(config) => config,
        Err(e) => return ProcessResult::Error(format!("Invalid --config: {e}")),
    };
    let formatted = if threads > 1 && original.len() >= PARALLEL_ITEMS_MIN_BYTES {
        format_source_with_config_parallel(&original, &config, threads)
    } else {
        format_source_with_config(&original, &config)
    };

    let changed = original != formatted;
    let original_len = original.len();
    let formatted_len = formatted.len();

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
    }
}
