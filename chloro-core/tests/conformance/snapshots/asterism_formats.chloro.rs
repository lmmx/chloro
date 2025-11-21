//! Format trait and implementations for different document types.
//!
//! This module defines the `Format` trait which abstracts over different
//! document formats (markdown, org-mode, restructuredtext, etc.) by providing
//! tree-sitter queries specific to each format.
pub mod difftastic;

pub mod markdown;
/// Abstracts document type differences through tree-sitter queries.
///
/// Enables support for markdown and other structured formats by providing format-specific parsing
/// queries (tree-sitter uses SCM lisp queries).
fn file_extension() -> &'static str;
fn language() -> tree_sitter::Language;
fn section_query() -> &str;
fn title_query() -> &str;
fn format_section_display(level: usize, title: &str) -> ratatui::text::Line<'static>;
