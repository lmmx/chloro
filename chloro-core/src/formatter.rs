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
//! | `skip` | names listed by `#[rustfmt::skip::macros(..)]` and `#[rustfmt::skip::attributes(..)]` |
//! | `rustc_compat` | syntax rust-analyzer accepts but rustc's parser (and so rustfmt) rejects |
//! | `rustc_syntax` | syntax rustc's parser accepts and rust-analyzer's rejects, rebuilt from the parser's steps |
//! | `shape`, `context`, `span`, `nodes`, `utils`, `sort` | shared infrastructure |
//! | `width` | chloro only: a lower bound on a node's one-line width, from its tokens, which lets an attempt that needs one line be skipped when no one-line rendering can fit |
//! | `scratch` | not from rustfmt: a per-thread bump arena for intermediates that stay inside one module (EXP-025) |
//!
//! `nodes` and `span` adapt the syntax tree (`crate::cst`) to rustc's AST shape (doc comments as attributes,
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
mod parallel;
mod patterns;
pub mod printer;
mod reorder;
mod rustc_compat;
mod rustc_syntax;
mod scratch;
mod shape;
mod skip;
mod sort;
mod span;
mod stmt;
mod types;
mod unit_cache;
mod utils;
mod visitor;
mod width;

use std::borrow::Cow;

use config::{Config, NewlineStyle, Settings};
pub use formatting::ParseError;
pub use unit_cache::{CachedUnit, UnitCache, UnitKey};

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
/// `#![rustfmt::skip]` attribute or a configuration with `disable_all_formatting`;
/// [`try_format_source_with_config`] tells the first case from the others.
///
/// As in rustfmt, the formatted text never starts with a byte order mark, and `\r\n` line
/// endings in the source (including inside literals) read as `\n`; the output's line
/// endings are set by `newline_style`.
pub fn format_source_with_config(source: &str, config: &Config) -> String {
    format_whole(source, config, format_sequential).unwrap_or_else(|_| source.to_owned())
}

/// [`format_source_with_config`], or the syntax error for which it returns the source
/// unchanged: a file that rustc's parser rejects, which rustfmt leaves as it is and reports
/// with exit status 1. Every other source is `Ok` with the same text as
/// [`format_source_with_config`], including one returned unchanged for an inner
/// `#![rustfmt::skip]` or `disable_all_formatting`.
///
/// The error is found by parsing the source again, so it costs nothing on a source that
/// parses.
pub fn try_format_source_with_config(source: &str, config: &Config) -> Result<String, ParseError> {
    format_whole(source, config, format_sequential)
        .or_else(|refused| refused.reason(source, config))
}

/// Whether rustfmt counts `formatted`, the output of a formatting function for `source`
/// under `config`, as leaving `source` unchanged: what `--check` reports and whether
/// overwriting the file rewrites it (rustfmt's `source_file::write_file`).
///
/// Under `newline_style = Auto`, rustfmt compares its output with the source as rustc reads
/// it, without a leading byte order mark and with `\r\n` read as `\n`. A source that differs
/// from its formatted output only in those, such as a formatted file with `\r\n` line
/// endings, is therefore already formatted and stays as it is, although the formatted text
/// has `\n` line endings. Under any other style rustfmt compares with the source's bytes. An
/// output equal to the source, as for `#![rustfmt::skip]` or `disable_all_formatting`, is
/// always unchanged.
pub fn is_unchanged(source: &str, formatted: &str, config: &Config) -> bool {
    source == formatted
        || (config.newline_style == NewlineStyle::Auto
            && formatting::normalize_src(source) == formatted)
}

/// Parses `source` as [`format_source_with_config`] does before it formats anything, and
/// stops there. The same steps run in the same order: the configuration's settings, the
/// source's normalisation (a leading byte order mark dropped, `\r\n` read as `\n`), lexing
/// and parsing for the configured edition, the flat syntax tree ([`crate::cst::Tree`]), the
/// second parse of syntax rustc accepts and rust-analyzer rejects, and the check for syntax
/// rustc rejects. The tree is dropped before returning.
///
/// Returns the number of elements (nodes and tokens) of the file's tree, or `None` where
/// [`format_source_with_config`] returns the source unchanged without formatting it:
/// `disable_all_formatting`, a text longer than `u32::MAX` bytes, a syntax error, or syntax
/// rustc rejects. A file with an inner `#![rustfmt::skip]` is parsed (the attribute is read
/// from the tree, after this point).
///
/// For measuring the parser's share of the formatter's cost (`examples/bench --phase
/// parse`, `benchmarks/profile.sh --phase parse`); formatting does not call it.
pub fn parse_only(source: &str, config: &Config) -> Option<usize> {
    let settings = Settings::new(config);
    if settings.disable_all_formatting() {
        return None;
    }
    let normalized = formatting::normalize_src(source);
    formatting::parse_text(&normalized, &settings)
}

/// [`format_source_with_config`] with the file's top-level items formatted on up to
/// `threads` threads. The output is identical; `threads <= 1`, a file with fewer than two
/// top-level items, or one returned unchanged costs no thread. The threads are spawned
/// for the call and joined before it returns (`std::thread::scope`).
///
/// Each item is formatted on its own, as it is in `format_source_with_config`: the text
/// between items and the sorting of `use`, `mod` and `extern crate` groups stay on the
/// calling thread. Worth it for a large file; for a small one the thread start-up costs
/// more than it saves, so a caller formatting many files should keep file-level
/// parallelism and use this only above a size of its choosing.
pub fn format_source_with_config_parallel(source: &str, config: &Config, threads: usize) -> String {
    let format = |text: &str, settings: &Settings| {
        formatting::format_text_parallel(text, settings, threads, None)
    };
    format_whole(source, config, format).unwrap_or_else(|_| source.to_owned())
}

/// [`format_source_with_config_parallel`], or the syntax error for which it returns the
/// source unchanged, as [`try_format_source_with_config`] reports it.
pub fn try_format_source_with_config_parallel(
    source: &str,
    config: &Config,
    threads: usize,
) -> Result<String, ParseError> {
    let format = |text: &str, settings: &Settings| {
        formatting::format_text_parallel(text, settings, threads, None)
    };
    format_whole(source, config, format).or_else(|refused| refused.reason(source, config))
}

/// [`format_source_with_config_parallel`] with a cache of formatted top-level units. The
/// output is identical; only the units `cache` does not hold are formatted (on up to
/// `threads` threads, or on the calling thread when `threads <= 1`), and each one formatted
/// is stored in `cache`.
///
/// A unit is one top-level item, or one group of `use`, `mod x;` or `extern crate` items
/// sorted together. It is looked up by its source text, the whole configuration and the
/// file's `#![rustfmt::skip::macros(..)]`/`#![rustfmt::skip::attributes(..)]` names
/// ([`UnitKey`]); its record holds positions relative to its start, so it applies wherever
/// the unit has moved in the file. Everything else is computed afresh on every call: the
/// parse and the whole-file checks (a syntax error, a construct rustc rejects,
/// `#![rustfmt::skip]`, `disable_all_formatting` return the source unchanged before any
/// lookup), the text between units, and the trailing-newline and line-ending fix-ups. A
/// re-format after an edit to one item therefore costs the parse, a lookup per unit and
/// the edited unit.
///
/// The key does not identify the build of chloro-core: a cache kept across builds must be
/// keyed by the build as well.
pub fn format_source_with_unit_cache(
    source: &str,
    config: &Config,
    threads: usize,
    cache: &dyn UnitCache,
) -> String {
    format_whole(source, config, format_with_units(config, threads, cache))
        .unwrap_or_else(|_| source.to_owned())
}

/// [`format_source_with_unit_cache`], or the syntax error for which it returns the source
/// unchanged, as [`try_format_source_with_config`] reports it.
pub fn try_format_source_with_unit_cache(
    source: &str,
    config: &Config,
    threads: usize,
    cache: &dyn UnitCache,
) -> Result<String, ParseError> {
    format_whole(source, config, format_with_units(config, threads, cache))
        .or_else(|refused| refused.reason(source, config))
}

/// A source the formatter returns unchanged without formatting it (it does not parse, or the
/// formatting failed): its normalised text.
struct Refused<'s>(Cow<'s, str>);

impl Refused<'_> {
    /// The syntax error for which `source` was refused under `config`, or `source` unchanged
    /// when it parses.
    fn reason(self, source: &str, config: &Config) -> Result<String, ParseError> {
        match formatting::parse_error(&self.0, &Settings::new(config)) {
            Some(error) => Err(error),
            None => Ok(source.to_owned()),
        }
    }
}

/// Formats the whole of `source` with `format`, one of `formatting`'s functions, which is
/// given the normalised text: the formatted text with the configured line endings, the
/// source unchanged where it is not to be formatted, or `Err` when `format` refuses it.
fn format_whole<'s>(
    source: &'s str,
    config: &Config,
    format: impl FnOnce(&str, &Settings) -> Option<formatting::Formatted>,
) -> Result<String, Refused<'s>> {
    let settings = Settings::new(config);
    if settings.disable_all_formatting() {
        return Ok(source.to_owned());
    }
    let normalized = formatting::normalize_src(source);
    match format(&normalized, &settings) {
        Some(formatted) if formatted.echoed => Ok(source.to_owned()),
        Some(formatted) => {
            let mut text = formatted.text;
            // rustfmt detects the `Auto` style on the normalised source, in which no
            // `\r\n` is left: `Auto` produces `\n` line endings.
            formatting::apply_newline_style(settings.newline_style(), &mut text, &normalized);
            Ok(text)
        }
        None => Err(Refused(normalized)),
    }
}

/// [`format_whole`]'s `format` for [`format_source_with_config`].
fn format_sequential(text: &str, settings: &Settings) -> Option<formatting::Formatted> {
    formatting::format_text(text, settings, false, true)
}

/// [`format_whole`]'s `format` for [`format_source_with_unit_cache`].
fn format_with_units<'a>(
    config: &'a Config,
    threads: usize,
    cache: &'a dyn UnitCache,
) -> impl FnOnce(&str, &Settings) -> Option<formatting::Formatted> + 'a {
    move |text, settings| {
        let cache = parallel::CacheUse {
            cache,
            context: unit_cache::key_context(config),
        };
        formatting::format_text_parallel(text, settings, threads, Some(cache))
    }
}

/// Write indentation to buffer
pub(crate) fn write_indent(buf: &mut String, indent: usize) {
    for _ in 0..indent {
        buf.push(' ');
    }
}
