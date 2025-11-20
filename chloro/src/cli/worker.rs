use super::args::Args;
use crate::vlog;
use chloro_core::format_source;
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

/// Process a single Rust source file
pub fn format_file(file_path: &Path, args: &Args) -> ProcessResult {
    vlog!(args, {"Processing: {}", file_path.display()});

    // Read the file
    let original = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            return ProcessResult::Error(format!("Failed to read {}: {}", file_path.display(), e));
        }
    };

    // Format it
    let formatted = format_source(&original);

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
