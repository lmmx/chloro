//! The formatter: a port of rustfmt's formatting logic onto rust-analyzer's syntax tree.
//!
//! The modules mirror rustfmt's source files of the same names (rustfmt 1.9.0), so that
//! each formatting decision can be compared with the code it reproduces:
//!
//! | module | role |
//! |---|---|
//! | `formatting` | entry point: parse, visit, trailing newlines, newline style |
//! | `visitor`, `missed_spans` | walk items/statements; copy comments and blank lines between them |
//! | `items`, `imports`, `reorder` | items, `use` trees, sorting of `use`/`mod`/`extern crate` |
//! | `expr`, `stmt`, `chains`, `closures`, `matches`, `pairs`, `patterns`, `types` | expressions and their parts |
//! | `macros`, `macro_args` | macro calls (re-parsed arguments) and `macro_rules!` bodies |
//! | `overflow`, `lists` | comma-separated lists and the "overflow the last item" heuristic |
//! | `attr`, `comment` | attributes, doc comments, comment re-indentation and recovery |
//! | `rustc_compat` | syntax rust-analyzer accepts but rustc's parser (and so rustfmt) rejects |
//! | `shape`, `context`, `span`, `nodes`, `utils`, `sort` | shared infrastructure |
//!
//! `nodes` and `span` adapt rowan nodes to rustc's AST shape (doc comments as attributes,
//! rustc's spans), which lets the ported modules keep rustfmt's control flow.

mod attr;
mod chains;
mod closures;
mod comment;
pub mod config;
mod context;
mod expr;
mod formatting;
mod imports;
mod items;
mod lists;
mod macro_args;
mod macros;
mod matches;
mod missed_spans;
mod nodes;
mod overflow;
mod pairs;
mod patterns;
pub mod printer;
mod reorder;
mod rustc_compat;
mod shape;
mod sort;
mod span;
mod stmt;
mod types;
mod utils;
mod visitor;

use config::{Config, Settings};

/// Format Rust source code with rustfmt's default style (`rustfmt --edition 2024`).
///
/// Source that does not parse is returned unchanged, as is a file with an inner
/// `#![rustfmt::skip]` attribute.
pub fn format_source(source: &str) -> String {
    format_source_with_config(source, &Config::default())
}

/// Format Rust source code with the given configuration.
///
/// Source that does not parse is returned unchanged, as is a file with an inner
/// `#![rustfmt::skip]` attribute or a configuration with `disable_all_formatting`.
///
/// As in rustfmt, the formatted text never starts with a byte order mark, and `\r\n` line
/// endings in the source (including inside literals) read as `\n`; the output's line
/// endings are set by `newline_style`.
pub fn format_source_with_config(source: &str, config: &Config) -> String {
    let settings = Settings::new(config);
    if settings.disable_all_formatting() {
        return source.to_owned();
    }
    let normalized = formatting::normalize_src(source);
    match formatting::format_text(&normalized, &settings, false) {
        Some(formatted) if formatted.echoed => source.to_owned(),
        Some(formatted) => {
            let mut text = formatted.text;
            // rustfmt detects the `Auto` style on the normalised source, in which no
            // `\r\n` is left: `Auto` produces `\n` line endings.
            formatting::apply_newline_style(settings.newline_style(), &mut text, &normalized);
            text
        }
        None => source.to_owned(),
    }
}

/// Write indentation to buffer
pub(crate) fn write_indent(buf: &mut String, indent: usize) {
    for _ in 0..indent {
        buf.push(' ');
    }
}
