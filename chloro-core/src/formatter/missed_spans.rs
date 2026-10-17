//! Output of the source text between formatted nodes (rustfmt's `missed_spans.rs`).
//!
//! The visitor emits each node it formats and then calls [`FmtVisitor::format_missing`]
//! for the source text up to the next node. That text holds only whitespace, comments,
//! and code a rewrite gave up on: blank lines are clamped to the configured bounds,
//! comments are re-indented, and code is copied with trailing whitespace removed.

use super::comment::{CodeCharKind, CommentCodeSlices, is_last_comment_block, rewrite_comment};
use super::config::{RustfmtVersion, StyleEdition};
use super::parallel::GapOp;
use super::shape::{Indent, Shape};
use super::span::{BytePos, Span, mk_sp};
use super::utils::{count_newlines, last_line_width, trim};
use super::visitor::FmtVisitor;

struct SnippetStatus {
    /// An offset to the current line from the beginning of the original snippet.
    line_start: usize,
    /// A length of trailing whitespaces on the current line.
    last_wspace: Option<usize>,
    /// F6: under `rustfmt_version = "1.8.0"`, the `line_start` rustfmt 1.8.0 computes
    /// after a run of blank lines falls inside a multi-byte whitespace character; the
    /// next slice of the snippet taken from it panics in rustfmt (#5739) and fails the
    /// file here. A comment resets `line_start` without slicing, which clears it.
    line_start_unaligned: bool,
}

impl FmtVisitor<'_> {
    fn output_at_start(&self) -> bool {
        self.buffer.is_empty()
    }

    pub(crate) fn format_missing(&mut self, end: BytePos) {
        if self.defer_gap(GapOp::NoIndent(end), end) {
            return;
        }
        // rustfmt uses `format_missing()` to extract a missing comment between a macro (or
        // similar) and a trailing semicolon, and avoids the general path in the common case
        // where there is no such comment.
        let missing_snippet = self.snippet(mk_sp(self.last_pos, end));
        if missing_snippet.trim() == ";" {
            self.push_str(";");
            self.last_pos = end;
            return;
        }
        self.format_missing_inner(end, |this, last_snippet, _| this.push_str(last_snippet))
    }

    pub(crate) fn format_missing_with_indent(&mut self, end: BytePos) {
        self.format_missing_indent(end, true)
    }

    pub(crate) fn format_missing_no_indent(&mut self, end: BytePos) {
        self.format_missing_indent(end, false)
    }

    fn format_missing_indent(&mut self, end: BytePos, should_indent: bool) {
        let op = if should_indent {
            GapOp::WithIndent(end)
        } else {
            GapOp::WithoutIndent(end)
        };
        if self.defer_gap(op, end) {
            return;
        }
        self.format_missing_inner(end, |this, last_snippet, snippet| {
            this.push_str(last_snippet.trim_end());
            if last_snippet == snippet && !this.output_at_start() {
                // No new lines in the snippet.
                this.push_str("\n");
            }
            if should_indent {
                let indent = this.block_indent.to_string(this.config);
                this.push_str(&indent);
            }
        })
    }

    fn format_missing_inner<F: Fn(&mut Self, &str, &str)>(
        &mut self,
        end: BytePos,
        process_last_snippet: F,
    ) {
        let start = self.last_pos;

        if start >= end {
            // rustfmt asserts `start <= end`; an inverted span has nothing to output.
            if start == end && !self.output_at_start() {
                process_last_snippet(self, "", "");
            }
            return;
        }

        self.last_pos = end;
        let span = mk_sp(start, end);
        let snippet = self.snippet(span);

        // Do nothing for spaces in the beginning of the file
        if start == 0 && end as usize == snippet.len() && trim(snippet).is_empty() {
            return;
        }

        if trim(snippet).is_empty() {
            // Keep vertical spaces within range.
            self.push_vertical_spaces(count_newlines(snippet));
            process_last_snippet(self, "", snippet);
        } else {
            self.write_snippet(span, &process_last_snippet);
        }
    }

    pub(crate) fn push_vertical_spaces(&mut self, mut newline_count: usize) {
        let offset = self
            .buffer
            .bytes()
            .rev()
            .take_while(|c| *c == b'\n')
            .count();
        let newline_upper_bound = self.config.blank_lines_upper_bound() + 1;
        let newline_lower_bound = self.config.blank_lines_lower_bound() + 1;

        if newline_count + offset > newline_upper_bound {
            if offset >= newline_upper_bound {
                newline_count = 0;
            } else {
                newline_count = newline_upper_bound - offset;
            }
        } else if newline_count + offset < newline_lower_bound {
            if offset >= newline_lower_bound {
                newline_count = 0;
            } else {
                newline_count = newline_lower_bound - offset;
            }
        }

        for _ in 0..newline_count {
            self.push_str("\n");
        }
    }

    fn write_snippet<F>(&mut self, span: Span, process_last_snippet: F)
    where
        F: Fn(&mut Self, &str, &str),
    {
        // The source from the file start to the span's lo determines what precedes the
        // current comment. If the comment follows code on the same line, it is not moved.
        let big_snippet = self.snippet_provider.entire_snippet();
        let big_diff = span.lo() as usize;
        let snippet = self.snippet(span);

        let mut status = SnippetStatus {
            line_start: 0,
            last_wspace: None,
            line_start_unaligned: false,
        };

        for (kind, offset, subslice) in CommentCodeSlices::new(snippet) {
            if CodeCharKind::Comment == kind {
                // 1: comment.
                self.process_comment(
                    &mut status,
                    snippet,
                    &big_snippet[..(offset + big_diff)],
                    offset,
                    subslice,
                );
            } else if subslice.trim().is_empty() && subslice.contains('\n') {
                // 2: blank lines.
                let newline_count = count_newlines(subslice);
                self.push_vertical_spaces(newline_count);
                match self.config.rustfmt_version() {
                    // To avoid any issues with whitespace unicode chars just add the len of the slice
                    RustfmtVersion::V1_9_0 | RustfmtVersion::V1_10_0 => {
                        status.line_start = offset + subslice.len()
                    }
                    // F6: rustfmt 1.8.0 advances by the newline count (`v1.8.0
                    // src/missed_spans.rs:209`; the source holds no `\r\n` after
                    // normalisation), which is not a char boundary when the run holds a
                    // multi-byte whitespace character such as U+2029; rustfmt 1.9.0
                    // advances by the slice length (commit `74931504`, CHANGELOG 1.9.0
                    // Fixed #5739).
                    RustfmtVersion::V1_8_0 => {
                        let line_start = offset + newline_count;
                        status.line_start_unaligned = !snippet.is_char_boundary(line_start);
                        status.line_start = if status.line_start_unaligned {
                            offset + subslice.len()
                        } else {
                            line_start
                        };
                    }
                }
            } else {
                // 3: code which we failed to format.
                if status.line_start_unaligned {
                    self.run.format_failure.set(true);
                }
                self.process_missing_code(&mut status, snippet, subslice, offset);
            }
        }

        if status.line_start_unaligned {
            self.run.format_failure.set(true);
        }
        let last_snippet = &snippet[status.line_start..];
        process_last_snippet(self, last_snippet, snippet);
    }

    fn process_comment(
        &mut self,
        status: &mut SnippetStatus,
        snippet: &str,
        big_snippet: &str,
        offset: usize,
        subslice: &str,
    ) {
        let last_char = big_snippet
            .chars()
            .rev()
            .find(|rev_c| ![' ', '\t'].contains(rev_c));

        let fix_indent = last_char.is_none_or(|rev_c| ['{', '\n'].contains(&rev_c));
        let mut on_same_line = false;

        let comment_indent = if fix_indent {
            if let Some('{') = last_char {
                self.push_str("\n");
            }
            let indent_str = self.block_indent.to_string(self.config);
            self.push_str(&indent_str);
            self.block_indent
        } else if self.config.style_edition() >= StyleEdition::Edition2024
            && !snippet.starts_with('\n')
        {
            // The comment appears on the same line as the previous formatted code.
            // Assuming that comment is logically associated with that code, we want to keep it on
            // the same level and avoid mixing it with possible other comment.
            on_same_line = true;
            self.push_str(" ");
            self.block_indent
        } else {
            self.push_str(" ");
            Indent::from_width(self.config, last_line_width(&self.buffer))
        };

        let comment_width = std::cmp::min(
            self.config.comment_width(),
            self.config
                .max_width()
                .saturating_sub(self.block_indent.width()),
        );
        let comment_shape = Shape::legacy(comment_width, comment_indent);

        if on_same_line {
            match subslice.find('\n') {
                None => {
                    self.push_str(subslice);
                }
                Some(offset) if offset + 1 == subslice.len() => {
                    self.push_str(&subslice[..offset]);
                }
                Some(offset) => {
                    // keep first line as is: if it were too long and wrapped, it may get mixed
                    // with the other lines.
                    let first_line = &subslice[..offset];
                    self.push_str(first_line);
                    self.push_str(&comment_indent.to_string_with_newline(self.config));

                    let other_lines = &subslice[offset + 1..];
                    let comment_str =
                        rewrite_comment(other_lines, false, comment_shape, self.config)
                            .unwrap_or_else(|| String::from(other_lines));
                    self.push_str(&comment_str);
                }
            }
        } else {
            let comment_str = rewrite_comment(subslice, false, comment_shape, self.config)
                .unwrap_or_else(|| String::from(subslice));
            self.push_str(&comment_str);
        }

        status.last_wspace = None;
        status.line_start = offset + subslice.len();
        status.line_start_unaligned = false;

        // Add a newline:
        // - if there isn't one already
        // - otherwise, only if the last line is a line comment
        if status.line_start <= snippet.len() {
            match snippet[status.line_start..]
                .chars()
                // skip trailing whitespaces
                .find(|c| !(*c == ' ' || *c == '\t'))
            {
                Some('\n') | Some('\r') => {
                    if !is_last_comment_block(subslice) {
                        self.push_str("\n");
                    }
                }
                _ => self.push_str("\n"),
            }
        }
    }

    fn process_missing_code(
        &mut self,
        status: &mut SnippetStatus,
        snippet: &str,
        subslice: &str,
        offset: usize,
    ) {
        for (mut i, c) in subslice.char_indices() {
            i += offset;

            if c == '\n' {
                if let Some(lw) = status.last_wspace {
                    self.push_str(&snippet[status.line_start..lw]);
                    self.push_str("\n");
                    status.last_wspace = None;
                } else {
                    self.push_str(&snippet[status.line_start..=i]);
                }

                status.line_start = i + 1;
            } else if c.is_whitespace() && status.last_wspace.is_none() {
                status.last_wspace = Some(i);
            } else {
                status.last_wspace = None;
            }
        }

        let remaining = trim(&snippet[status.line_start..subslice.len() + offset]);
        if !remaining.is_empty() {
            self.push_str(&self.block_indent.to_string(self.config));
            self.push_str(remaining);
            status.line_start = subslice.len() + offset;
        } else if self.config.rustfmt_version() >= RustfmtVersion::V1_10_0 {
            // F21: rustfmt 1.10.0 moves `line_start` past a rest of whitespace as well, which
            // the last snippet then leaves out (`1.99.0 src/missed_spans.rs:367`, written for
            // `--file-lines`); rustfmt 1.9.0 moves it only past code.
            status.line_start = subslice.len() + offset;
        }
    }
}
