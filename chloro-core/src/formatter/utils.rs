//! Small helpers shared by the rewrite modules (rustfmt's `utils.rs`).

use std::borrow::Cow;

use crate::cst::ast::{self, AstNode};
use unicode_width::UnicodeWidthStr;

use super::comment::{
    CharClasses, FullCodeCharKind, LineClasses, filter_normal_code, find_any_byte,
};
use super::config::{Settings, StyleEdition};
use super::context::RewriteContext;
use super::nodes::{is_repeat_array, node_text};
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
    // A first line of ASCII characters is as wide as it is long: a `\r` before the `\n`
    // belongs to the line and counts one column there.
    match first_newline_or_non_ascii(s.as_bytes()) {
        None => s.len(),
        Some(i) if s.as_bytes()[i] == b'\n' => i,
        Some(_) => unicode_str_width(s.split('\n').next().unwrap_or("")),
    }
}

/// The width of the last line in s.
#[inline]
pub(crate) fn last_line_width(s: &str) -> usize {
    // A last line of ASCII characters is as wide as it is long.
    match last_newline_or_non_ascii(s.as_bytes()) {
        None => s.len(),
        Some(i) if s.as_bytes()[i] == b'\n' => s.len() - i - 1,
        Some(_) => unicode_str_width(s.rsplit('\n').next().unwrap_or("")),
    }
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
    if is_ascii_single_line(s.as_bytes()) {
        s.len()
    } else {
        multi_line_or_unicode_str_width(s)
    }
}

/// `unicode_str_width` for a string with a `\n` or a non-ASCII character. Kept out of line
/// so that the common case does not pay for its registers.
#[inline(never)]
fn multi_line_or_unicode_str_width(s: &str) -> usize {
    if !s.is_ascii() {
        return s.width();
    }
    // unicode-width 0.1.14 counts every ASCII character as one column, except `\n` and a
    // `\r` before `\n`, which count as none.
    let bytes = s.as_bytes();
    // Without `\r`, the width is the length minus the number of `\n`, counted eight bytes
    // at a time: `(found >> 7)` holds 0 or 1 per byte, and the multiplication sums the
    // bytes into the top byte.
    const ONES: u64 = 0x0101_0101_0101_0101;
    let mut chunks = bytes.chunks_exact(8);
    let mut newlines = 0;
    let mut carriage_returns = 0;
    for chunk in &mut chunks {
        let w = u64::from_le_bytes(chunk.try_into().unwrap());
        let found = newline_or_non_ascii(w);
        newlines += ((found >> 7).wrapping_mul(ONES) >> 56) as usize;
        carriage_returns |= newline_or_non_ascii(w ^ 0x0707_0707_0707_0707);
    }
    let rest = chunks.remainder();
    if carriage_returns == 0 && !rest.contains(&b'\r') {
        return bytes.len() - newlines - rest.iter().filter(|&&b| b == b'\n').count();
    }
    let mut width = bytes.len();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            width -= 1 + usize::from(i > 0 && bytes[i - 1] == b'\r');
        }
    }
    width
}

/// Bit 7 of a byte of the result is set iff that byte of `w` is non-ASCII or `\n`; the
/// other bits are clear. `(y & 0x7f) + 0x7f` has bit 7 set iff `y & 0x7f != 0`, and no byte
/// carries into the next, so `!(... | y)` has bit 7 set iff the byte of `y = w ^ 0x0a..` is
/// zero. Every byte is flagged exactly, so both the lowest and the highest flag can be read.
fn newline_or_non_ascii(w: u64) -> u64 {
    const LOW7: u64 = 0x7f7f_7f7f_7f7f_7f7f;
    const NEWLINES: u64 = 0x0a0a_0a0a_0a0a_0a0a;
    let y = w ^ NEWLINES;
    (w | !(((y & LOW7) + LOW7) | y)) & !LOW7
}

/// The position of the first byte that is `\n` or non-ASCII, eight bytes at a time.
fn first_newline_or_non_ascii(bytes: &[u8]) -> Option<usize> {
    let mut chunks = bytes.chunks_exact(8);
    let mut pos = 0;
    for chunk in &mut chunks {
        // Read little-endian, the lowest flag is the first byte.
        let found = newline_or_non_ascii(u64::from_le_bytes(chunk.try_into().unwrap()));
        if found != 0 {
            return Some(pos + (found.trailing_zeros() / 8) as usize);
        }
        pos += 8;
    }
    chunks
        .remainder()
        .iter()
        .position(|&b| b == b'\n' || !b.is_ascii())
        .map(|i| pos + i)
}

/// The position of the last byte that is `\n` or non-ASCII, eight bytes at a time from
/// the end.
fn last_newline_or_non_ascii(bytes: &[u8]) -> Option<usize> {
    let mut end = bytes.len();
    while end >= 8 {
        // Read little-endian, the highest flag is the last byte.
        let found =
            newline_or_non_ascii(u64::from_le_bytes(bytes[end - 8..end].try_into().unwrap()));
        if found != 0 {
            return Some(end - 1 - (found.leading_zeros() / 8) as usize);
        }
        end -= 8;
    }
    bytes[..end]
        .iter()
        .rposition(|&b| b == b'\n' || !b.is_ascii())
}

/// Returns `true` if `bytes` is ASCII and contains no `\n`: the case where
/// `unicode_str_width` is the byte length. Tests eight bytes at a time.
fn is_ascii_single_line(bytes: &[u8]) -> bool {
    let flags = newline_or_non_ascii;
    if bytes.len() < 8 {
        return bytes.iter().all(|&b| b < 0x80 && b != b'\n');
    }
    let mut acc = 0;
    let mut chunks = bytes.chunks_exact(8);
    for chunk in &mut chunks {
        acc |= flags(u64::from_ne_bytes(chunk.try_into().unwrap()));
    }
    if !chunks.remainder().is_empty() {
        // The last eight bytes overlap bytes already tested, which leaves `acc` as it is.
        let last = &bytes[bytes.len() - 8..];
        acc |= flags(u64::from_ne_bytes(last.try_into().unwrap()));
    }
    acc == 0
}

/// Joins string pieces with one allocation of the exact length. Gives what `format!` gives
/// for a format string of literal text and `{}` placeholders whose arguments are strings
/// (`str`'s `Display` writes the text unchanged when no width or precision is given),
/// without the formatting machinery.
pub(crate) fn concat(parts: &[&str]) -> String {
    parts.concat()
}

pub(crate) fn count_newlines(input: &str) -> usize {
    count_newline_bytes(input.as_bytes())
}

/// The number of `\n` bytes, eight at a time: `newline_flags(w) >> 7` holds 1 in each
/// byte of `w` that is `\n` and 0 in the others, so the byte lanes of the sum over at most
/// 255 words hold at most 255; they are then added in pairs into 16-bit lanes (at most
/// 510) and the four 16-bit lanes into the top one by a multiplication (at most 2040).
pub(crate) fn count_newline_bytes(bytes: &[u8]) -> usize {
    const BYTE_LANES: u64 = 0x00ff_00ff_00ff_00ff;
    const WORD_ONES: u64 = 0x0001_0001_0001_0001;
    let mut count = 0;
    for block in bytes.chunks(8 * 255) {
        let mut words = block.chunks_exact(8);
        let mut lanes = 0;
        for word in &mut words {
            lanes += newline_flags(u64::from_le_bytes(word.try_into().unwrap())) >> 7;
        }
        let pairs = (lanes & BYTE_LANES) + ((lanes >> 8) & BYTE_LANES);
        count += (pairs.wrapping_mul(WORD_ONES) >> 48) as usize;
        count += words.remainder().iter().filter(|&&b| b == b'\n').count();
    }
    count
}

/// Bit 7 of a byte of the result is set iff that byte of `w` is `\n` (see
/// `newline_or_non_ascii`); the other bits are clear.
fn newline_flags(w: u64) -> u64 {
    const LOW7: u64 = 0x7f7f_7f7f_7f7f_7f7f;
    let y = w ^ 0x0a0a_0a0a_0a0a_0a0a;
    !(((y & LOW7) + LOW7) | y) & !LOW7
}

/// `count_newlines(s) == 1`, stopping at a second `\n`.
pub(crate) fn has_one_newline(s: &str) -> bool {
    let bytes = s.as_bytes();
    let first = find_any_byte(bytes, 0, *b"\n");
    first < bytes.len() && find_any_byte(bytes, first + 1, *b"\n") == bytes.len()
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
    match lines_fit(snippet, max_width, shape) {
        Some(fits) => fits,
        None => str_fits_by_lines(snippet, max_width, shape),
    }
}

/// `str_fits_by_lines` in one pass for a text without `\r`, or `None` when it has one.
/// Without `\r`, `lines()` yields exactly the pieces between `\n` bytes (an empty piece
/// after a final `\n` is not yielded, and its width 0 passes both of its tests), and
/// `first_line_width`, `last_line_width` and `unicode_str_width` give each piece the width
/// it has alone: its length when it is ASCII, its `unicode_str_width` otherwise. The tests
/// are conjunctions without side effects, so the first piece that fails one decides the
/// result, whatever comes after it.
fn lines_fit(snippet: &str, max_width: usize, shape: Shape) -> Option<bool> {
    let bytes = snippet.as_bytes();
    let mut start = 0;
    let mut pos = 0;
    let mut ascii = true;
    let mut first = true;
    loop {
        pos = first_newline_return_or_non_ascii(bytes, pos);
        let end = match bytes.get(pos) {
            Some(b'\n') => pos,
            Some(b'\r') => return None,
            Some(_) => {
                ascii = false;
                pos += 1;
                continue;
            }
            None => bytes.len(),
        };
        let width = if ascii {
            end - start
        } else {
            unicode_str_width(&snippet[start..end])
        };
        if end == bytes.len() {
            return Some(if first {
                width <= shape.width
            } else {
                width <= max_width && width <= shape.used_width() + shape.width
            });
        }
        if width > if first { shape.width } else { max_width } {
            return Some(false);
        }
        first = false;
        ascii = true;
        pos += 1;
        start = pos;
    }
}

/// The position of the first byte at or after `pos` that is `\n`, `\r` or non-ASCII, or
/// `bytes.len()`, eight bytes at a time. `newline_or_non_ascii(w ^ 0x07..)` flags `\r`
/// (`0x0d ^ 0x07 = 0x0a`) and the non-ASCII bytes, exactly per byte.
fn first_newline_return_or_non_ascii(bytes: &[u8], mut pos: usize) -> usize {
    while let Some(chunk) = bytes.get(pos..pos + 8) {
        let w = u64::from_le_bytes(chunk.try_into().unwrap());
        let found = newline_or_non_ascii(w) | newline_or_non_ascii(w ^ 0x0707_0707_0707_0707);
        if found != 0 {
            return pos + (found.trailing_zeros() / 8) as usize;
        }
        pos += 8;
    }
    while let Some(&b) = bytes.get(pos) {
        if b == b'\n' || b == b'\r' || !b.is_ascii() {
            break;
        }
        pos += 1;
    }
    pos
}

/// `filtered_str_fits` after the comment filter, line by line (rustfmt's form).
fn str_fits_by_lines(snippet: &str, max_width: usize, shape: Shape) -> bool {
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
pub(crate) fn left_most_sub_expr<'a>(e: &ast::Expr<'a>) -> ast::Expr<'a> {
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
        None => *e,
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
        | ast::Expr::RecordExpr(..)
        | ast::Expr::WhileExpr(..)
        | ast::Expr::IfExpr(..)
        | ast::Expr::BlockExpr(..)
        | ast::Expr::LoopExpr(..)
        | ast::Expr::ForExpr(..)
        | ast::Expr::MatchExpr(..) => repr.contains('\n'),
        ast::Expr::ArrayExpr(a) => !is_repeat_array(a) && repr.contains('\n'),
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
    b.op_kind().is_some_and(ast::is_assign_op)
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

    #[test]
    fn ascii_width_matches_unicode_width() {
        for s in [
            "", "abc", "a\nb", "a\r\nb", "\r", "a\rb", "\t x\n\n", "\r\n\r\n", "\n\r",
        ] {
            assert_eq!(unicode_str_width(s), s.width(), "{s:?}");
        }
    }

    use super::*;

    #[test]
    fn ascii_width_fast_path_matches_unicode_width() {
        // Every length around the eight-byte words, with one special character at every
        // position: `\n`, `\r\n`, a lone `\r`, the last ASCII byte, NUL, a tab, and non-ASCII
        // characters whose UTF-8 bytes are `0xc2 0x8a`, `0xc4 0x8a` and `0xe2 0x80 0x8b`.
        for len in 0..40 {
            let plain: String = (0..len)
                .map(|i| char::from(b'a' + (i % 26) as u8))
                .collect();
            assert_eq!(unicode_str_width(&plain), plain.width(), "{plain:?}");
            for at in 0..=len {
                for special in [
                    "\n", "\r\n", "\r", "\x7f", "\0", "\t", "\u{8a}", "\u{10a}", "\u{200b}",
                ] {
                    let mut s = plain.clone();
                    s.insert_str(at, special);
                    assert_eq!(unicode_str_width(&s), s.width(), "{s:?}");
                }
            }
        }
    }

    #[test]
    fn newline_counts_match_the_byte_filter() {
        // Lengths across the eight-byte words and the 255-word blocks, with `\n` at a
        // stride, next to bytes one bit away from it (`\x0B`, `\x08`, `\x8a`).
        for len in (0..40).chain([2039, 2040, 2041, 4100, 20_000]) {
            for stride in [1, 2, 3, 7, 8, 9, 1000] {
                let bytes: Vec<u8> = (0..len)
                    .map(|i| match i % stride {
                        0 => b'\n',
                        1 => b'\x0B',
                        2 => b'\x08',
                        3 => 0x8a,
                        _ => b'a',
                    })
                    .collect();
                let expected = bytes.iter().filter(|&&b| b == b'\n').count();
                assert_eq!(count_newline_bytes(&bytes), expected, "{len} {stride}");
                if let Ok(s) = std::str::from_utf8(&bytes) {
                    assert_eq!(has_one_newline(s), expected == 1, "{len} {stride}");
                }
            }
        }
        for s in [
            "",
            "a",
            "\n",
            "a\nb",
            "\n\n",
            "a\nb\nc",
            "abcdefgh\n",
            "abcdefghi\nj\n",
        ] {
            assert_eq!(has_one_newline(s), count_newlines(s) == 1, "{s:?}");
        }
    }

    #[test]
    fn one_pass_fit_matches_the_line_by_line_fit() {
        // Every string of up to five characters over `a`, space, `\n`, `\r`, a two-column
        // character, a combining mark, a variation selector and `/`, and a few longer ones
        // around the eight-byte words, against every small shape.
        const ALPHABET: [char; 8] = ['a', ' ', '\n', '\r', '\u{3000}', '\u{301}', '\u{fe0f}', '/'];
        let mut strings: Vec<String> = [
            "abcdefghijklmnop\nabcdefgh",
            "abcdefgh\nabcdefghijklmnopq\n",
            "abcdefg\u{3000}\nabcdefghij\rk\nabc",
            "a // b\nc",
            "\u{2764}\u{fe0f}\nab",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        for len in 0..=5 {
            for mut n in 0..ALPHABET.len().pow(len) {
                let mut s = String::new();
                for _ in 0..len {
                    s.push(ALPHABET[n % ALPHABET.len()]);
                    n /= ALPHABET.len();
                }
                strings.push(s);
            }
        }
        for s in &strings {
            let filtered = filter_normal_code(s);
            for width in 0..=4 {
                for max_width in 0..=4 {
                    for offset in [0, 1] {
                        let shape = Shape {
                            width,
                            indent: Indent::new(0, 0),
                            offset,
                        };
                        assert_eq!(
                            filtered_str_fits(s, max_width, shape),
                            str_fits_by_lines(&filtered, max_width, shape),
                            "{s:?} {width} {max_width} {offset}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn line_widths_match_the_split_lines_and_unicode_width() {
        // Lengths around the eight-byte words, with one or two of `\n`, `\r\n`, `\r` and
        // non-ASCII characters (bytes `0xc2 0x8a`, `0xe2 0x80 0x8b`) at every position.
        let specials = ["\n", "\r\n", "\r", "\u{8a}", "\u{200b}", "\t"];
        let mut strings = vec![];
        for len in 0..40 {
            let plain: String = (0..len)
                .map(|i| char::from(b'a' + (i % 26) as u8))
                .collect();
            strings.push(plain.clone());
            for at in 0..=len {
                for first in specials {
                    let mut s = plain.clone();
                    s.insert_str(at, first);
                    strings.push(s.clone());
                    for second in specials {
                        for at2 in (0..=s.len()).filter(|&i| s.is_char_boundary(i)).step_by(3) {
                            let mut t = s.clone();
                            t.insert_str(at2, second);
                            strings.push(t);
                        }
                    }
                }
            }
        }
        for s in &strings {
            let first = unicode_str_width(s.split('\n').next().unwrap_or(""));
            let last = unicode_str_width(s.rsplit('\n').next().unwrap_or(""));
            assert_eq!(first_line_width(s), first, "{s:?}");
            assert_eq!(last_line_width(s), last, "{s:?}");
            assert_eq!(unicode_str_width(s), s.width(), "{s:?}");
        }
    }

    #[test]
    fn concat_matches_format_on_strings() {
        let owned = String::from("é {x} ");
        let borrowed = "}{";
        let cow: Cow<'_, str> = Cow::Owned("\n    ".to_owned());
        for (a, b) in [("", ""), ("a", "b"), ("ünï", "\t\r\n"), ("{}", "%s")] {
            assert_eq!(
                concat(&[a, "{", &owned, borrowed, &cow, b, "}"]),
                format!("{a}{{{owned}{borrowed}{cow}{b}}}")
            );
        }
    }

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
