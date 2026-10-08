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
pub fn format_source_with_config(source: &str, config: &Config) -> String {
    let settings = Settings::new(config);
    if settings.disable_all_formatting() {
        return source.to_owned();
    }
    match formatting::format_text(source, &settings, false) {
        Some(formatted) if formatted.echoed => formatted.text,
        Some(formatted) => {
            let mut text = formatted.text;
            formatting::apply_newline_style(settings.newline_style(), &mut text, source);
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
