//! Formatting a whole source text (rustfmt's `formatting.rs` and the snippet helpers of
//! its `lib.rs`).

use std::rc::Rc;

use ra_ap_parser::{LexedStr, StrStep, TopEntryPoint};
use ra_ap_syntax::ast::AstNode;
use ra_ap_syntax::{Edition, SourceFile, SyntaxTreeBuilder};

use super::comment::{CharClasses, LineClasses};
use super::config::{NewlineStyle, Settings};
use super::context::RunState;
use super::nodes::{contains_skip, inner_attributes};
use super::rustc_compat::rejected_by_rustc;
use super::shape::Indent;
use super::span::SnippetProvider;
use super::utils::indent_next_line;
use super::visitor::FmtVisitor;

/// The result of formatting a source text.
pub(crate) struct Formatted {
    pub(crate) text: String,
    /// The input was returned as written (`#![rustfmt::skip]` on the file).
    pub(crate) echoed: bool,
    /// A macro call or definition could not be formatted and was copied as written.
    pub(crate) macro_failure: bool,
    /// A rewrite dropped a comment and fell back to the original text.
    pub(crate) lost_comment: bool,
    /// Line ranges (1-based, inclusive) of the output that were copied as written.
    pub(crate) skipped_range: Vec<(usize, usize)>,
}

/// Parses `source`, or returns `None` when it does not parse. rustfmt refuses to format
/// a file with a syntax error; so does chloro.
///
/// Only lexer and parser errors count. `SourceFile::parse(..).errors()` would also run
/// rust-analyzer's semantic validation (e.g. `crate` in the middle of a path), which
/// rustc's parser does not do, and would cost a second traversal of the tree.
fn parse(source: &str, edition: Edition) -> Option<SourceFile> {
    let lexed = LexedStr::new(edition, source);
    if lexed.errors().next().is_some() {
        return None;
    }
    let input = lexed.to_input(edition);
    let output = TopEntryPoint::SourceFile.parse(&input, edition);
    let mut builder = SyntaxTreeBuilder::default();
    let mut has_error = false;
    lexed.intersperse_trivia(&output, &mut |step| match step {
        StrStep::Token { kind, text } => builder.token(kind, text),
        StrStep::Enter { kind } => builder.start_node(kind),
        StrStep::Exit => builder.finish_node(),
        StrStep::Error { .. } => has_error = true,
    });
    if has_error {
        return None;
    }
    let file = SourceFile::cast(builder.finish().syntax_node())?;
    (!rejected_by_rustc(&file)).then_some(file)
}

/// Formats a parsed source text. Returns `None` when the text does not parse.
pub(crate) fn format_text(
    source: &str,
    config: &Settings,
    is_macro_def: bool,
) -> Option<Formatted> {
    let file = parse(source, config.edition().to_ra())?;

    // `#![rustfmt::skip]` on the file: echo the input.
    if contains_skip(&inner_attributes(file.syntax())) {
        return Some(Formatted {
            text: source.to_owned(),
            echoed: true,
            macro_failure: false,
            lost_comment: false,
            skipped_range: Vec::new(),
        });
    }

    let run = Rc::new(RunState::default());
    let snippet_provider = SnippetProvider::new(source);
    let end_pos = snippet_provider.end_pos();
    let mut visitor = FmtVisitor::new(config, snippet_provider, run.clone());
    visitor.is_macro_def = is_macro_def;
    visitor.last_pos = 0;
    visitor.skip_empty_lines(end_pos);
    visitor.format_separate_mod(&file, end_pos);

    let mut text = std::mem::take(&mut visitor.buffer);
    let macro_failure = visitor.macro_rewrite_failure;
    drop(visitor);

    // The source map of rustc does not include terminating newlines, so rustfmt adds one
    // for each file and then truncates any run of trailing newlines to one.
    text.push('\n');
    truncate_trailing_newlines(&mut text);

    let skipped_range = run.skipped_range.borrow().clone();
    Some(Formatted {
        text,
        echoed: false,
        macro_failure,
        lost_comment: run.lost_comment.get(),
        skipped_range,
    })
}

/// rustfmt's `format_lines` (without its error reporting): a run of trailing newlines
/// becomes a single newline. `\r` characters do not interrupt the run.
fn truncate_trailing_newlines(text: &mut String) {
    let mut newline_count = 0;
    for c in text.chars().rev() {
        match c {
            '\n' => newline_count += 1,
            '\r' => {}
            _ => break,
        }
    }
    if newline_count > 1 {
        // Keep everything up to and including the first newline of the run.
        let mut seen = 0;
        let mut cut = text.len();
        for (i, c) in text.char_indices().rev() {
            match c {
                '\n' => {
                    seen += 1;
                    if seen == newline_count {
                        cut = i + 1;
                        break;
                    }
                }
                '\r' => {}
                _ => break,
            }
        }
        text.truncate(cut);
    }
}

/// rustfmt's `apply_newline_style`.
pub(crate) fn apply_newline_style(style: NewlineStyle, text: &mut String, original: &str) {
    let windows = match style {
        // The line ending of the first newline of the input, else the platform's.
        NewlineStyle::Auto => match original.find('\n') {
            Some(i) => i > 0 && original.as_bytes()[i - 1] == b'\r',
            None => cfg!(windows),
        },
        NewlineStyle::Native => cfg!(windows),
        NewlineStyle::Unix => false,
        NewlineStyle::Windows => true,
    };
    if windows {
        let mut out = String::with_capacity(text.len() * 11 / 10);
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\n' => out.push_str("\r\n"),
                '\r' if chars.peek() == Some(&'\n') => {}
                c => out.push(c),
            }
        }
        *text = out;
    } else if text.contains('\r') {
        // `\r\n` becomes `\n`; a lone `\r` is kept.
        *text = text.replace("\r\n", "\n");
    }
}

/// A formatted snippet and the lines (1-based) that were left as written.
pub(crate) struct FormattedSnippet {
    pub(crate) snippet: String,
    non_formatted_ranges: Vec<(usize, usize)>,
}

impl FormattedSnippet {
    /// Shifts the line numbers after removing the first line of the snippet.
    fn unwrap_code_block(&mut self) {
        for (low, high) in &mut self.non_formatted_ranges {
            *low -= 1;
            *high -= 1;
        }
    }

    /// Returns `true` if the line n did not get formatted.
    pub(crate) fn is_line_non_formatted(&self, n: usize) -> bool {
        self.non_formatted_ranges
            .iter()
            .any(|(low, high)| *low <= n && n <= *high)
    }
}

/// `true` if a line of `text` (outside skipped lines) ends in whitespace: rustfmt's
/// `TrailingWhitespace` error, which fails the formatting of a macro definition body.
fn has_trailing_whitespace(text: &str, skipped: &[(usize, usize)]) -> bool {
    let mut line = 1;
    let mut last_was_space = false;
    for (_, _, c) in CharClasses::new(text) {
        match c {
            '\r' => {}
            '\n' => {
                let is_skipped = skipped.iter().any(|&(lo, hi)| lo <= line && line <= hi);
                if last_was_space && !is_skipped {
                    return true;
                }
                line += 1;
                last_was_space = false;
            }
            c => last_was_space = c.is_whitespace(),
        }
    }
    false
}

/// Formats `snippet` as a source file (rustfmt's `format_snippet`). Used for the bodies of
/// macro definitions, where `is_macro_def` makes any unformatted code an error.
pub(crate) fn format_snippet(
    snippet: &str,
    config: &Settings,
    is_macro_def: bool,
) -> Option<FormattedSnippet> {
    let formatted = format_text(snippet, config, is_macro_def)?;
    if formatted.macro_failure
        || (is_macro_def
            && (formatted.lost_comment
                || has_trailing_whitespace(&formatted.text, &formatted.skipped_range)))
    {
        return None;
    }
    let mut text = formatted.text;
    // `newline_style` does not apply to snippets embedded in a larger output.
    if text.contains('\r') {
        text = text.replace("\r\n", "\n");
    }
    Some(FormattedSnippet {
        snippet: text,
        non_formatted_ranges: formatted.skipped_range,
    })
}

/// Formats a code block that may not parse as items by wrapping it in `fn main() { .. }`
/// (rustfmt's `format_code_block`). The returned code block does **not** end with a
/// newline.
pub(crate) fn format_code_block(
    code_snippet: &str,
    config: &Settings,
    is_macro_def: bool,
) -> Option<FormattedSnippet> {
    const FN_MAIN_PREFIX: &str = "fn main() {\n";

    fn enclose_in_main_block(s: &str, config: &Settings) -> String {
        let indent = Indent::from_width(config, config.tab_spaces());
        let mut result = String::with_capacity(s.len() * 2);
        result.push_str(FN_MAIN_PREFIX);
        let mut need_indent = true;
        for (kind, line) in LineClasses::new(s) {
            if need_indent {
                result.push_str(&indent.to_string(config));
            }
            result.push_str(line);
            result.push('\n');
            need_indent = indent_next_line(kind, line, config);
        }
        result.push('}');
        result
    }

    // Wrap the given code block with `fn main()` if it does not have one.
    let snippet = enclose_in_main_block(code_snippet, config);
    let mut result = String::with_capacity(snippet.len());
    let mut is_first = true;

    let mut formatted = format_snippet(&snippet, config, is_macro_def)?;
    // Remove wrapping main block
    formatted.unwrap_code_block();

    // Trim "fn main() {" on the first line and "}" on the last line,
    // then unindent the whole code block.
    let block_len = formatted
        .snippet
        .rfind('}')
        .unwrap_or(formatted.snippet.len());

    // It's possible that `block_len < FN_MAIN_PREFIX.len()`. This can happen if the code block was
    // formatted into the empty string, leading to the enclosing `fn main() {\n}` being formatted
    // into `fn main() {}`. In this case no unindentation is done.
    let block_start = std::cmp::min(FN_MAIN_PREFIX.len(), block_len);

    let mut is_indented = true;
    let indent_str = Indent::from_width(config, config.tab_spaces()).to_string(config);
    for (kind, line) in LineClasses::new(&formatted.snippet[block_start..block_len]) {
        if !is_first {
            result.push('\n');
        } else {
            is_first = false;
        }
        let trimmed_line = if !is_indented {
            line
        } else if line.len() > config.max_width() {
            // If there are lines that are larger than max width, we cannot tell
            // whether we have succeeded but have some comments or strings that
            // are too long, or we have failed to format code block. We will be
            // conservative and just return `None` in this case.
            return None;
        } else if line.len() > indent_str.len() {
            // Make sure that the line has leading whitespaces.
            if line.starts_with(indent_str.as_ref()) {
                let offset = if config.hard_tabs() {
                    1
                } else {
                    config.tab_spaces()
                };
                &line[offset..]
            } else {
                line
            }
        } else {
            line
        };
        result.push_str(trimmed_line);
        is_indented = indent_next_line(kind, line, config);
    }
    Some(FormattedSnippet {
        snippet: result,
        non_formatted_ranges: formatted.non_formatted_ranges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_newlines_are_truncated_to_one() {
        let mut s = String::from("fn a() {}\n\n\n");
        truncate_trailing_newlines(&mut s);
        assert_eq!(s, "fn a() {}\n");
        let mut s = String::from("a\r\n\r\n");
        truncate_trailing_newlines(&mut s);
        assert_eq!(s, "a\r\n");
    }

    #[test]
    fn newline_style_auto_follows_the_first_newline() {
        let mut s = String::from("a\nb\n");
        apply_newline_style(NewlineStyle::Auto, &mut s, "x\r\ny\n");
        assert_eq!(s, "a\r\nb\r\n");
        let mut s = String::from("a\nb\n");
        apply_newline_style(NewlineStyle::Auto, &mut s, "x\ny\r\n");
        assert_eq!(s, "a\nb\n");
    }

    #[test]
    fn trailing_whitespace_detection() {
        assert!(has_trailing_whitespace("a \nb\n", &[]));
        assert!(!has_trailing_whitespace("a \nb\n", &[(1, 1)]));
        assert!(!has_trailing_whitespace("a\nb\n", &[]));
    }
}
