//! Markdown format implementation using tree-sitter-md.
//!
//! This module provides tree-sitter queries for parsing markdown documents
//! and extracting section structure from ATX-style headings (# syntax).
use crate::formats::Format;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

pub struct MarkdownFormat;

impl MarkdownFormat {
    fn file_extension() -> &'static str {
        "md"
    }
    fn language() -> tree_sitter::Language {
        tree_sitter_md::LANGUAGE.into()
    }
    fn section_query() -> &'static str {
        "(atx_heading) @heading"
    }
    fn title_query() -> &'static str {
        "(atx_heading heading_content: (inline) @title)"
    }
    fn format_section_display(level: usize, title: &str) -> Line<'static> {
        // Cycle through colors for different heading levels
        let colors = [
            Color::Cyan,
            Color::Green,
            Color::LightYellow,
            Color::Magenta,
            Color::Blue,
            Color::Red,
        ];
        let color = colors[(level - 1) % colors.len()];
        let prefix = "#".repeat(level);
        let spans = vec![
            Span::styled(prefix, Style::default().fg(color)),
            Span::raw(" "),
            Span::raw(title.to_string()),
        ];
        Line::from(spans)
    }
}
