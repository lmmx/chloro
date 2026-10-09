//! Small helpers shared by the rewrite modules (rustfmt's `utils.rs`).

use std::borrow::Cow;

use ra_ap_syntax::ast::{self, AstNode};
use unicode_width::UnicodeWidthStr;

use super::comment::{CharClasses, FullCodeCharKind, LineClasses, filter_normal_code};
use super::config::{Settings, StyleEdition};
use super::context::RewriteContext;
use super::nodes::node_text;
use super::shape::{Indent, Shape};

/// Computes the length of a string's last line, minus offset.
pub(crate) fn extra_offset(text: &str, shape: Shape) -> usize {
    match text.rfind('\n') {
        // 1 for newline character
        Some(idx) => text.len().saturating_sub(idx + 1 + shape.used_width()),
        None => text.len(),
    }
}

/// Formats a visibility (`pub`, `pub(crate)`, `pub(in path)`) followed by a space.
///
/// As in rustfmt, `pub(in crate)`, `pub(in self)` and `pub(in super)` lose the `in`.
pub(crate) fn format_visibility(vis: Option<&ast::Visibility>) -> Cow<'static, str> {
    let Some(vis) = vis else {
        return Cow::from("");
    };
    let Some(path) = vis.path() else {
        return Cow::from("pub ");
    };
    let path_str = path
        .segments()
        .map(|seg| node_text(seg.syntax()))
        .collect::<Vec<_>>()
        .join("::");
    let is_keyword = |s: &str| s == "crate" || s == "self" || s == "super";
    let in_str = if is_keyword(&path_str) { "" } else { "in " };
    Cow::from(format!("pub({in_str}{path_str}) "))
}

#[inline]
pub(crate) fn is_single_line(s: &str) -> bool {
    !s.contains('\n')
}

#[inline]
pub(crate) fn first_line_contains_single_line_comment(s: &str) -> bool {
    s.lines().next().is_some_and(|l| l.contains("//"))
}

#[inline]
pub(crate) fn last_line_contains_single_line_comment(s: &str) -> bool {
    s.lines().last().is_some_and(|l| l.contains("//"))
}

#[inline]
pub(crate) fn is_attributes_extendable(attrs_str: &str) -> bool {
    !attrs_str.contains('\n') && !last_line_contains_single_line_comment(attrs_str)
}

/// The width of the first line in s.
#[inline]
pub(crate) fn first_line_width(s: &str) -> usize {
    unicode_str_width(s.split('\n').next().unwrap_or(""))
}

/// The width of the last line in s.
#[inline]
pub(crate) fn last_line_width(s: &str) -> usize {
    unicode_str_width(s.rsplit('\n').next().unwrap_or(""))
}

/// The total used width of the last line.
#[inline]
pub(crate) fn last_line_used_width(s: &str, offset: usize) -> usize {
    if s.contains('\n') {
        last_line_width(s)
    } else {
        offset + unicode_str_width(s)
    }
}

#[inline]
pub(crate) fn trimmed_last_line_width(s: &str) -> usize {
    unicode_str_width(match s.rfind('\n') {
        Some(n) => s[(n + 1)..].trim(),
        None => s.trim(),
    })
}

#[inline]
pub(crate) fn last_line_extendable(s: &str) -> bool {
    if s.ends_with("\"#") {
        return true;
    }
    for c in s.chars().rev() {
        match c {
            '(' | ')' | ']' | '}' | '?' | '>' => continue,
            '\n' => break,
            _ if c.is_whitespace() => continue,
            _ => return false,
        }
    }
    true
}

pub(crate) fn semicolon_for_expr(context: &RewriteContext<'_>, expr: &ast::Expr) -> bool {
    // Never try to insert semicolons on expressions when we're inside
    // a macro definition - this can prevent the macro from compiling
    // when used in expression position
    if context.is_macro_def {
        return false;
    }

    match expr {
        ast::Expr::ReturnExpr(..) | ast::Expr::ContinueExpr(..) | ast::Expr::BreakExpr(..) => {
            context.config.trailing_semicolon()
        }
        _ => false,
    }
}

/// Computes the length of the given string, as it would appear in the output.
pub(crate) fn unicode_str_width(s: &str) -> usize {
    // `width()` counts every ASCII character except `\n` as one column (unicode-width 0.1).
    if s.is_ascii() && !s.contains('\n') {
        s.len()
    } else {
        s.width()
    }
}

pub(crate) fn count_newlines(input: &str) -> usize {
    input.bytes().filter(|&b| b == b'\n').count()
}

/// Wraps String in an Option. Returns Some when the string adheres to the
/// Rewrite constraints defined for the Rewrite trait and None otherwise.
pub(crate) fn wrap_str(s: String, max_width: usize, shape: Shape) -> Option<String> {
    if filtered_str_fits(&s, max_width, shape) {
        Some(s)
    } else {
        None
    }
}

pub(crate) fn filtered_str_fits(snippet: &str, max_width: usize, shape: Shape) -> bool {
    let snippet = &*filter_normal_code(snippet);
    if !snippet.is_empty() {
        // First line must fits with `shape.width`.
        if first_line_width(snippet) > shape.width {
            return false;
        }
        // If the snippet does not include newline, we are done.
        if is_single_line(snippet) {
            return true;
        }
        // The other lines must fit within the maximum width.
        if snippet
            .lines()
            .skip(1)
            .any(|line| unicode_str_width(line) > max_width)
        {
            return false;
        }
        // A special check for the last line, since the caller may
        // place trailing characters on this line.
        if last_line_width(snippet) > shape.used_width() + shape.width {
            return false;
        }
    }
    true
}

#[inline]
pub(crate) fn colon_spaces(config: &Settings) -> &'static str {
    let before = config.space_before_colon();
    let after = config.space_after_colon();
    match (before, after) {
        (true, true) => " : ",
        (true, false) => " :",
        (false, true) => ": ",
        (false, false) => ":",
    }
}

/// The left-most sub-expression whose text starts the given expression.
pub(crate) fn left_most_sub_expr(e: &ast::Expr) -> ast::Expr {
    let next = match e {
        ast::Expr::CallExpr(c) => c.expr(),
        ast::Expr::BinExpr(b) => b.lhs(),
        ast::Expr::CastExpr(c) => c.expr(),
        ast::Expr::FieldExpr(f) => f.expr(),
        ast::Expr::IndexExpr(i) => i.base(),
        ast::Expr::RangeExpr(r) => ast::RangeItem::start(r),
        ast::Expr::TryExpr(t) => t.expr(),
        _ => None,
    };
    match next {
        Some(next) => left_most_sub_expr(&next),
        None => e.clone(),
    }
}

#[inline]
pub(crate) fn starts_with_newline(s: &str) -> bool {
    s.starts_with('\n') || s.starts_with("\r\n")
}

#[inline]
pub(crate) fn first_line_ends_with(s: &str, c: char) -> bool {
    s.lines().next().is_some_and(|l| l.ends_with(c))
}

/// States whether an expression's last line exclusively consists of closing
/// parens, braces, and brackets in its idiomatic formatting.
pub(crate) fn is_block_expr(context: &RewriteContext<'_>, expr: &ast::Expr, repr: &str) -> bool {
    match expr {
        ast::Expr::MacroExpr(..)
        | ast::Expr::FormatArgsExpr(..)
        | ast::Expr::CallExpr(..)
        | ast::Expr::MethodCallExpr(..)
        | ast::Expr::ArrayExpr(..)
        | ast::Expr::RecordExpr(..)
        | ast::Expr::WhileExpr(..)
        | ast::Expr::IfExpr(..)
        | ast::Expr::BlockExpr(..)
        | ast::Expr::LoopExpr(..)
        | ast::Expr::ForExpr(..)
        | ast::Expr::MatchExpr(..) => repr.contains('\n'),
        ast::Expr::ParenExpr(p) => p.expr().is_some_and(|e| is_block_expr(context, &e, repr)),
        ast::Expr::BinExpr(b) if !is_assignment(b) => {
            b.rhs().is_some_and(|e| is_block_expr(context, &e, repr))
        }
        ast::Expr::IndexExpr(i) => i.index().is_some_and(|e| is_block_expr(context, &e, repr)),
        ast::Expr::PrefixExpr(p) => p.expr().is_some_and(|e| is_block_expr(context, &e, repr)),
        ast::Expr::TryExpr(t) => t.expr().is_some_and(|e| is_block_expr(context, &e, repr)),
        ast::Expr::YieldExpr(y) => y.expr().is_some_and(|e| is_block_expr(context, &e, repr)),
        ast::Expr::ClosureExpr(c) => c.body().is_some_and(|e| is_block_expr(context, &e, repr)),
        // This can only be a string lit
        ast::Expr::Literal(_) => {
            repr.contains('\n') && trimmed_last_line_width(repr) <= context.config.tab_spaces()
        }
        _ => false,
    }
}

/// `true` for `=` and compound assignment operators, which rustc models as
/// `ExprKind::Assign` / `ExprKind::AssignOp` rather than `ExprKind::Binary`.
pub(crate) fn is_assignment(b: &ast::BinExpr) -> bool {
    matches!(b.op_kind(), Some(ast::BinaryOp::Assignment { .. }))
}

/// Removes trailing spaces from the specified snippet. We do not remove spaces
/// inside strings or comments.
pub(crate) fn remove_trailing_white_spaces(text: &str) -> String {
    let mut buffer = String::with_capacity(text.len());
    let mut space_buffer = String::with_capacity(128);
    for (char_kind, _, c) in CharClasses::new(text) {
        match c {
            '\n' => {
                if char_kind == FullCodeCharKind::InString {
                    buffer.push_str(&space_buffer);
                }
                space_buffer.clear();
                buffer.push('\n');
            }
            _ if c.is_whitespace() => {
                space_buffer.push(c);
            }
            _ => {
                if !space_buffer.is_empty() {
                    buffer.push_str(&space_buffer);
                    space_buffer.clear();
                }
                buffer.push(c);
            }
        }
    }
    buffer
}

/// Indent each line according to the specified `indent`, preserving the relative
/// indentation of the lines after the first.
pub(crate) fn trim_left_preserve_layout(
    orig: &str,
    indent: Indent,
    config: &Settings,
) -> Option<String> {
    let mut lines = LineClasses::new(orig);
    let first_line = lines.next().map(|(_, s)| s.trim_end().to_owned())?;
    let mut trimmed_lines = Vec::with_capacity(16);

    let mut veto_trim = false;
    let min_prefix_space_width = lines
        .filter_map(|(kind, line)| {
            let mut trimmed = true;
            let prefix_space_width = if is_empty_line(line) {
                None
            } else {
                Some(get_prefix_space_width(config, line))
            };

            // just InString{Commented} in order to allow the start of a string to be indented
            let new_veto_trim_value = (kind == FullCodeCharKind::InString
                || (config.style_edition() >= StyleEdition::Edition2024
                    && kind == FullCodeCharKind::InStringCommented))
                && !line.ends_with('\\');
            let line = if veto_trim || new_veto_trim_value {
                veto_trim = new_veto_trim_value;
                trimmed = false;
                line
            } else {
                line.trim()
            };
            trimmed_lines.push((trimmed, line, prefix_space_width));

            // Because there is a veto against trimming and indenting lines within a string,
            // such lines should not be taken into account when computing the minimum.
            match kind {
                FullCodeCharKind::InStringCommented | FullCodeCharKind::EndStringCommented
                    if config.style_edition() >= StyleEdition::Edition2024 =>
                {
                    None
                }
                FullCodeCharKind::InString | FullCodeCharKind::EndString => None,
                _ => prefix_space_width,
            }
        })
        .min()?;

    let mut out = first_line;
    out.push('\n');
    for (i, &(trimmed, line, prefix_space_width)) in trimmed_lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match prefix_space_width {
            _ if !trimmed => out.push_str(line),
            Some(original_indent_width) => {
                let new_indent_width =
                    indent.width() + original_indent_width.saturating_sub(min_prefix_space_width);
                let new_indent = Indent::from_width(config, new_indent_width);
                out.push_str(&new_indent.to_string(config));
                out.push_str(line);
            }
            None => {}
        }
    }
    Some(out)
}

/// Based on the given line, determine if the next line can be indented or not.
/// This allows to preserve the indentation of multi-line literals when
/// re-inserted a code block that has been formatted separately from the rest
/// of the code, such as code in macro defs or code blocks doc comments.
pub(crate) fn indent_next_line(kind: FullCodeCharKind, line: &str, config: &Settings) -> bool {
    if kind.is_string() {
        // If the string ends with '\', the string has been wrapped over
        // multiple lines. If `format_strings = true`, then the indentation of
        // strings wrapped over multiple lines will have been adjusted while
        // formatting the code block, therefore the string's indentation needs
        // to be adjusted for the code surrounding the code block.
        config.format_strings() && line.ends_with('\\')
    } else if config.style_edition() >= StyleEdition::Edition2024 {
        !kind.is_commented_string()
    } else {
        true
    }
}

pub(crate) fn is_empty_line(s: &str) -> bool {
    s.is_empty() || s.chars().all(char::is_whitespace)
}

fn get_prefix_space_width(config: &Settings, s: &str) -> usize {
    let mut width = 0;
    for c in s.chars() {
        match c {
            ' ' => width += 1,
            '\t' => width += config.tab_spaces(),
            _ => return width,
        }
    }
    width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_trailing_white_spaces_keeps_raw_strings() {
        let s = "    r#\"\n        test\n    \"#";
        assert_eq!(remove_trailing_white_spaces(s), s);
    }

    #[test]
    fn trim_left_preserve_layout_reindents() {
        let config = Settings::default();
        let indent = Indent::new(4, 0);
        assert_eq!(
            trim_left_preserve_layout("aaa\n\tbbb\n    ccc", indent, &config),
            Some("aaa\n    bbb\n    ccc".to_string())
        );
    }
}
