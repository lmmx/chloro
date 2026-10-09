//! Comment classification and light-touch comment rewriting.
//!
//! Ported from rustfmt's `comment.rs`, restricted to the default configuration
//! (`normalize_comments = false`, `wrap_comments = false`): comments are only
//! re-indented and stripped of trailing whitespace, never re-flowed.

use std::borrow::Cow;

use super::config::Settings;
use super::context::RewriteContext;
use super::shape::{Indent, Shape};
use super::span::Span;
use super::utils::{
    first_line_width, last_line_width, trim_left_preserve_layout, trimmed_last_line_width,
};

fn is_custom_comment(comment: &str) -> bool {
    if !comment.starts_with("//") {
        false
    } else if let Some(c) = comment.chars().nth(2) {
        !c.is_alphanumeric() && !c.is_whitespace()
    } else {
        false
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum CommentStyle<'a> {
    DoubleSlash,
    TripleSlash,
    Doc,
    SingleBullet,
    DoubleBullet,
    Exclamation,
    Custom(&'a str),
}

fn custom_opener(s: &str) -> &str {
    s.lines().next().map_or("", |first_line| {
        first_line
            .find(' ')
            .map_or(first_line, |space_index| &first_line[0..=space_index])
    })
}

impl<'a> CommentStyle<'a> {
    /// Returns `true` if the commenting style cannot span multiple lines.
    pub(crate) fn is_line_comment(&self) -> bool {
        matches!(
            self,
            CommentStyle::DoubleSlash
                | CommentStyle::TripleSlash
                | CommentStyle::Doc
                | CommentStyle::Custom(_)
        )
    }

    /// Returns `true` if the commenting style can span multiple lines.
    pub(crate) fn is_block_comment(&self) -> bool {
        matches!(
            self,
            CommentStyle::SingleBullet | CommentStyle::DoubleBullet | CommentStyle::Exclamation
        )
    }

    pub(crate) fn opener(&self) -> &'a str {
        match *self {
            CommentStyle::DoubleSlash => "// ",
            CommentStyle::TripleSlash => "/// ",
            CommentStyle::Doc => "//! ",
            CommentStyle::SingleBullet => "/* ",
            CommentStyle::DoubleBullet => "/** ",
            CommentStyle::Exclamation => "/*! ",
            CommentStyle::Custom(opener) => opener,
        }
    }

    pub(crate) fn closer(&self) -> &'a str {
        match *self {
            CommentStyle::DoubleSlash
            | CommentStyle::TripleSlash
            | CommentStyle::Custom(..)
            | CommentStyle::Doc => "",
            CommentStyle::SingleBullet | CommentStyle::DoubleBullet | CommentStyle::Exclamation => {
                " */"
            }
        }
    }

    pub(crate) fn line_start(&self) -> &'a str {
        match *self {
            CommentStyle::DoubleSlash => "// ",
            CommentStyle::TripleSlash => "/// ",
            CommentStyle::Doc => "//! ",
            CommentStyle::SingleBullet | CommentStyle::DoubleBullet | CommentStyle::Exclamation => {
                " * "
            }
            CommentStyle::Custom(opener) => opener,
        }
    }
}

pub(crate) fn comment_style(orig: &str) -> CommentStyle<'_> {
    if orig.starts_with("/**") && !orig.starts_with("/**/") {
        CommentStyle::DoubleBullet
    } else if orig.starts_with("/*!") {
        CommentStyle::Exclamation
    } else if orig.starts_with("/*") {
        CommentStyle::SingleBullet
    } else if orig.starts_with("///") && orig.chars().nth(3).is_none_or(|c| c != '/') {
        CommentStyle::TripleSlash
    } else if orig.starts_with("//!") {
        CommentStyle::Doc
    } else if is_custom_comment(orig) {
        CommentStyle::Custom(custom_opener(orig))
    } else {
        CommentStyle::DoubleSlash
    }
}

/// Returns true if the last line of the passed string finishes with a block-comment.
pub(crate) fn is_last_comment_block(s: &str) -> bool {
    s.trim_end().ends_with("*/")
}

/// Combine `prev_str` and `next_str` into a single `String`. `span` may contain
/// comments between two strings. If there are such comments, then that will be
/// recovered. If `allow_extend` is true and there is no comment between the two
/// strings, then they will be put on a single line as long as doing so does not
/// exceed max width.
pub(crate) fn combine_strs_with_missing_comments(
    context: &RewriteContext<'_>,
    prev_str: &str,
    next_str: &str,
    span: Span,
    shape: Shape,
    allow_extend: bool,
) -> Option<String> {
    let mut result =
        String::with_capacity(prev_str.len() + next_str.len() + shape.indent.width() + 128);
    result.push_str(prev_str);
    let mut allow_one_line = !prev_str.contains('\n') && !next_str.contains('\n');
    let first_sep =
        if prev_str.is_empty() || next_str.is_empty() || trimmed_last_line_width(prev_str) == 0 {
            ""
        } else {
            " "
        };
    let mut one_line_width =
        last_line_width(prev_str) + first_line_width(next_str) + first_sep.len();

    let indent = shape.indent;
    let missing_comment = rewrite_missing_comment(span, shape, context)?;

    if missing_comment.is_empty() {
        if allow_extend && one_line_width <= shape.width {
            result.push_str(first_sep);
        } else if !prev_str.is_empty() {
            result.push_str(&indent.to_string_with_newline(context.config))
        }
        result.push_str(next_str);
        return Some(result);
    }

    // We have a missing comment between the first expression and the second expression.
    // Peek the original source code and find out whether there is a newline between the first
    // expression and the second expression or the missing comment. We will preserve the original
    // layout whenever possible.
    let original_snippet = context.snippet(span);
    let prefer_same_line = if let Some(pos) = original_snippet.find('/') {
        !original_snippet[..pos].contains('\n')
    } else {
        !original_snippet.contains('\n')
    };

    one_line_width -= first_sep.len();
    let first_sep = if prev_str.is_empty() || missing_comment.is_empty() {
        Cow::from("")
    } else {
        let one_line_width = last_line_width(prev_str) + first_line_width(&missing_comment) + 1;
        if prefer_same_line && one_line_width <= shape.width {
            Cow::from(" ")
        } else {
            indent.to_string_with_newline(context.config)
        }
    };
    result.push_str(&first_sep);
    result.push_str(&missing_comment);

    let second_sep = if missing_comment.is_empty() || next_str.is_empty() {
        Cow::from("")
    } else if missing_comment.starts_with("//") {
        indent.to_string_with_newline(context.config)
    } else {
        one_line_width += missing_comment.len() + first_sep.len() + 1;
        allow_one_line &= !missing_comment.starts_with("//") && !missing_comment.contains('\n');
        if prefer_same_line && allow_one_line && one_line_width <= shape.width {
            Cow::from(" ")
        } else {
            indent.to_string_with_newline(context.config)
        }
    };
    result.push_str(&second_sep);
    result.push_str(next_str);

    Some(result)
}

pub(crate) fn rewrite_doc_comment(orig: &str, shape: Shape, config: &Settings) -> Option<String> {
    identify_comment(orig, shape, config, true)
}

/// Rewrites a comment at the given shape.
///
/// `block_style` only affects comment normalisation and wrapping, which are unstable rustfmt
/// options fixed to `false` (see [`Settings::normalize_comments`]); with those disabled the
/// comment is only re-indented.
pub(crate) fn rewrite_comment(
    orig: &str,
    _block_style: bool,
    shape: Shape,
    config: &Settings,
) -> Option<String> {
    identify_comment(orig, shape, config, false)
}

fn identify_comment(
    orig: &str,
    shape: Shape,
    config: &Settings,
    is_doc_comment: bool,
) -> Option<String> {
    let style = comment_style(orig);

    // Computes the byte length of line taking into account a newline if the line is part of a
    // paragraph.
    fn compute_len(orig: &str, line: &str) -> usize {
        if orig.len() > line.len() {
            if orig.as_bytes()[line.len()] == b'\r' {
                line.len() + 2
            } else {
                line.len() + 1
            }
        } else {
            line.len()
        }
    }

    // Get the first group of line comments having the same commenting style.
    //
    // Returns a tuple with:
    // - a boolean indicating if there is a blank line
    // - a number indicating the size of the first group of comments
    fn consume_same_line_comments(
        style: CommentStyle<'_>,
        orig: &str,
        line_start: &str,
    ) -> (bool, usize) {
        let mut first_group_ending = 0;
        let mut hbl = false;

        for line in orig.lines() {
            let trimmed_line = line.trim_start();
            if trimmed_line.is_empty() {
                hbl = true;
                break;
            } else if trimmed_line.starts_with(line_start) || comment_style(trimmed_line) == style {
                first_group_ending += compute_len(&orig[first_group_ending..], line);
            } else {
                break;
            }
        }
        (hbl, first_group_ending)
    }

    let (has_bare_lines, first_group_ending) = match style {
        CommentStyle::DoubleSlash | CommentStyle::TripleSlash | CommentStyle::Doc => {
            let line_start = style.line_start().trim_start();
            consume_same_line_comments(style, orig, line_start)
        }
        CommentStyle::Custom(opener) => {
            let trimmed_opener = opener.trim_end();
            consume_same_line_comments(style, orig, trimmed_opener)
        }
        // for a block comment, search for the closing symbol
        CommentStyle::DoubleBullet | CommentStyle::SingleBullet | CommentStyle::Exclamation => {
            let closer = style.closer().trim_start();
            let mut count = orig.matches(closer).count();
            let mut closing_symbol_offset = 0;
            let mut hbl = false;
            let mut first = true;
            for line in orig.lines() {
                closing_symbol_offset += compute_len(&orig[closing_symbol_offset..], line);
                let mut trimmed_line = line.trim_start();
                if !trimmed_line.starts_with('*')
                    && !trimmed_line.starts_with("//")
                    && !trimmed_line.starts_with("/*")
                {
                    hbl = true;
                }

                // Remove opener from consideration when searching for closer
                if first {
                    let opener = style.opener().trim_end();
                    trimmed_line = trimmed_line.get(opener.len()..).unwrap_or("");
                    first = false;
                }
                if trimmed_line.ends_with(closer) {
                    count = count.saturating_sub(1);
                    if count == 0 {
                        break;
                    }
                }
            }
            (hbl, closing_symbol_offset)
        }
    };

    let (first_group, rest) = orig.split_at(first_group_ending.min(orig.len()));
    debug_assert!(!config.normalize_comments() && !config.wrap_comments());
    let rewritten_first_group = if has_bare_lines && style.is_block_comment() {
        trim_left_preserve_layout(first_group, shape.indent, config)?
    } else {
        light_rewrite_comment(first_group, shape.indent, config, is_doc_comment)
    };
    if rest.is_empty() {
        Some(rewritten_first_group)
    } else {
        identify_comment(rest.trim_start(), shape, config, is_doc_comment).map(|rest_str| {
            format!(
                "{}\n{}{}{}",
                rewritten_first_group,
                // insert back the blank line
                if has_bare_lines && style.is_line_comment() {
                    "\n"
                } else {
                    ""
                },
                shape.indent.to_string(config),
                rest_str
            )
        })
    }
}

pub(crate) fn rewrite_missing_comment(
    span: Span,
    shape: Shape,
    context: &RewriteContext<'_>,
) -> Option<String> {
    let missing_snippet = context.snippet(span);
    let trimmed_snippet = missing_snippet.trim();
    // check the span starts with a comment
    let pos = trimmed_snippet.find('/');
    if !trimmed_snippet.is_empty() && pos.is_some() {
        rewrite_comment(trimmed_snippet, false, shape, context.config)
    } else {
        Some(String::new())
    }
}

/// Recover the missing comments in the specified span, if available.
/// The layout of the comments will be preserved as long as it does not break the code
/// and its total width does not exceed the max width.
pub(crate) fn recover_missing_comment_in_span(
    span: Span,
    shape: Shape,
    context: &RewriteContext<'_>,
    used_width: usize,
) -> Option<String> {
    let missing_comment = rewrite_missing_comment(span, shape, context)?;
    if missing_comment.is_empty() {
        Some(String::new())
    } else {
        let missing_snippet = context.snippet(span);
        let pos = missing_snippet.find('/')?;
        // 1 = ` `
        let total_width = missing_comment.len() + used_width + 1;
        let force_new_line_before_comment =
            missing_snippet[..pos].contains('\n') || total_width > context.config.max_width();
        let sep = if force_new_line_before_comment {
            shape.indent.to_string_with_newline(context.config)
        } else {
            Cow::from(" ")
        };
        Some(format!("{sep}{missing_comment}"))
    }
}

/// Trim trailing whitespaces unless they consist of two or more whitespaces.
fn trim_end_unless_two_whitespaces(s: &str, is_doc_comment: bool) -> &str {
    if is_doc_comment && s.ends_with("  ") {
        s
    } else {
        s.trim_end()
    }
}

/// Trims whitespace and aligns to indent, but otherwise does not change comments.
fn light_rewrite_comment(
    orig: &str,
    offset: Indent,
    config: &Settings,
    is_doc_comment: bool,
) -> String {
    let sep = format!("\n{}", offset.to_string(config));
    let mut out = String::with_capacity(orig.len() + 16);
    for (i, l) in orig.lines().enumerate() {
        if i > 0 {
            out.push_str(&sep);
        }
        // This is basically just l.trim(), but in the case that a line starts
        // with `*` we want to leave one space before it, so it aligns with the
        // `*` in `/*`.
        let left_trimmed = match l.find(|c: char| !c.is_whitespace()) {
            Some(fnw) if l.as_bytes()[fnw] == b'*' && fnw > 0 => &l[fnw - 1..],
            Some(fnw) => &l[fnw..],
            None => "",
        };
        // Preserve markdown's double-space line break syntax in doc comment.
        out.push_str(trim_end_unless_two_whitespaces(
            left_trimmed,
            is_doc_comment,
        ));
    }
    out
}

pub(crate) trait FindUncommented {
    fn find_uncommented(&self, pat: &str) -> Option<usize>;
    fn find_last_uncommented(&self, pat: &str) -> Option<usize>;
}

impl FindUncommented for str {
    fn find_uncommented(&self, pat: &str) -> Option<usize> {
        let first = pat.bytes().next().filter(u8::is_ascii);
        let mut needle_iter = pat.chars();
        let mut classes = CharClasses::new(self);
        loop {
            // A skipped character is `Normal` and differs from the first character of the
            // pattern, so it would only reset a needle that is already reset.
            if let Some(first) = first
                && needle_iter.as_str().len() == pat.len()
            {
                classes.skip_plain(Some(first));
            }
            let Some((kind, i, b)) = classes.next() else {
                break;
            };
            match needle_iter.next() {
                None => {
                    return Some(i - pat.len());
                }
                Some(c) => match kind {
                    FullCodeCharKind::Normal | FullCodeCharKind::InString if b == c => {}
                    _ => {
                        needle_iter = pat.chars();
                    }
                },
            }
        }

        // Handle case where the pattern is a suffix of the search string
        match needle_iter.next() {
            Some(_) => None,
            None => Some(self.len() - pat.len()),
        }
    }

    fn find_last_uncommented(&self, pat: &str) -> Option<usize> {
        if let Some(left) = self.find_uncommented(pat) {
            let mut result = left;
            // add 1 to use find_last_uncommented for &str after pat
            while let Some(next) = self[(result + 1)..].find_last_uncommented(pat) {
                result += next + 1;
            }
            Some(result)
        } else {
            None
        }
    }
}

/// Returns the first byte position after the first comment. The given string
/// is expected to be prefixed by a comment, including delimiters.
pub(crate) fn find_comment_end(s: &str) -> Option<usize> {
    let mut iter = CharClasses::new(s);
    for (kind, i, _c) in &mut iter {
        if kind == FullCodeCharKind::Normal || kind == FullCodeCharKind::InString {
            return Some(i);
        }
    }

    // Handle case where the comment ends at the end of `s`.
    if iter.status == CharClassesStatus::Normal {
        Some(s.len())
    } else {
        None
    }
}

/// Returns `true` if text contains any comment.
pub(crate) fn contains_comment(text: &str) -> bool {
    // Fast path: every comment starts with `/`.
    if !text.contains('/') {
        return false;
    }
    let mut classes = CharClasses::new(text);
    loop {
        classes.skip_plain(None);
        match classes.next() {
            Some((kind, _, _)) if kind.is_comment() => return true,
            Some(_) => {}
            None => return false,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum CharClassesStatus {
    Normal,
    /// Character is within a string
    LitString,
    LitStringEscape,
    /// Character is within a raw string
    LitRawString(u32),
    RawStringPrefix(u32),
    RawStringSuffix(u32),
    LitChar,
    LitCharEscape,
    /// Character inside a block comment, with the integer indicating the nesting deepness of the
    /// comment
    BlockComment(u32),
    /// Character inside a block-commented string, with the integer indicating the nesting deepness
    /// of the comment
    StringInBlockComment(u32),
    /// Status when the '/' has been consumed, but not yet the '*', deepness is
    /// the new deepness (after the comment opening).
    BlockCommentOpening(u32),
    /// Status when the '*' has been consumed, but not yet the '/', deepness is
    /// the new deepness (after the comment closing).
    BlockCommentClosing(u32),
    /// Character is within a line comment
    LineComment,
}

/// Distinguish between functional part of code and comments
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub(crate) enum CodeCharKind {
    Normal,
    Comment,
}

/// Distinguish between functional part of code and comments,
/// describing opening and closing of comments for ease when chunking
/// code from tagged characters
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub(crate) enum FullCodeCharKind {
    Normal,
    /// The first character of a comment, there is only one for a comment (always '/')
    StartComment,
    /// Any character inside a comment including the second character of comment
    /// marks ("//", "/*")
    InComment,
    /// Last character of a comment, '\n' for a line comment, '/' for a block comment.
    EndComment,
    /// Start of a multiline string inside a comment
    StartStringCommented,
    /// End of a multiline string inside a comment
    EndStringCommented,
    /// Inside a commented string
    InStringCommented,
    /// Start of a multiline string
    StartString,
    /// End of a multiline string
    EndString,
    /// Inside a string.
    InString,
}

impl FullCodeCharKind {
    pub(crate) fn is_comment(self) -> bool {
        matches!(
            self,
            FullCodeCharKind::StartComment
                | FullCodeCharKind::InComment
                | FullCodeCharKind::EndComment
                | FullCodeCharKind::StartStringCommented
                | FullCodeCharKind::InStringCommented
                | FullCodeCharKind::EndStringCommented
        )
    }

    /// Returns true if the character is inside a comment
    pub(crate) fn inside_comment(self) -> bool {
        matches!(
            self,
            FullCodeCharKind::InComment
                | FullCodeCharKind::StartStringCommented
                | FullCodeCharKind::InStringCommented
                | FullCodeCharKind::EndStringCommented
        )
    }

    pub(crate) fn is_string(self) -> bool {
        self == FullCodeCharKind::InString || self == FullCodeCharKind::StartString
    }

    /// Returns true if the character is within a commented string
    pub(crate) fn is_commented_string(self) -> bool {
        self == FullCodeCharKind::InStringCommented
            || self == FullCodeCharKind::StartStringCommented
    }

    fn to_codecharkind(self) -> CodeCharKind {
        if self.is_comment() {
            CodeCharKind::Comment
        } else {
            CodeCharKind::Normal
        }
    }
}

/// Classifies each character of a string as code, string literal or comment.
///
/// Yields `(kind, byte_index, char)`. Lookahead is done directly on the
/// underlying `&str`, so the iterator never allocates.
pub(crate) struct CharClasses<'a> {
    src: &'a str,
    pos: usize,
    status: CharClassesStatus,
}

impl<'a> CharClasses<'a> {
    pub(crate) fn new(src: &'a str) -> CharClasses<'a> {
        CharClasses {
            src,
            pos: 0,
            status: CharClassesStatus::Normal,
        }
    }

    /// The character `n` positions after the one just consumed (0 = next).
    fn peek_nth(&self, n: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(n)
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn is_raw_string_suffix(&self, count: u32) -> bool {
        (0..count as usize).all(|n| self.peek_nth(n) == Some('#'))
    }

    /// In the `Normal` state, moves past the characters that `next` would classify as
    /// `Normal` without leaving the state: everything but `r`, `"`, `'` and `/`, all ASCII,
    /// so the scan can work on bytes. Also stops at `stop`, a byte the caller looks for.
    fn skip_plain(&mut self, stop: Option<u8>) {
        if self.status != CharClassesStatus::Normal {
            return;
        }
        let bytes = self.src.as_bytes();
        let mut pos = self.pos;
        while let Some(&b) = bytes.get(pos) {
            if matches!(b, b'r' | b'"' | b'\'' | b'/') || Some(b) == stop {
                break;
            }
            pos += 1;
        }
        // `pos` is at a char boundary: it is at the end or at an ASCII byte.
        self.pos = pos;
    }
}

impl Iterator for CharClasses<'_> {
    type Item = (FullCodeCharKind, usize, char);

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.pos;
        let chr = self.src[idx..].chars().next()?;
        self.pos += chr.len_utf8();
        let mut char_kind = FullCodeCharKind::Normal;
        self.status = match self.status {
            CharClassesStatus::LitRawString(sharps) => {
                char_kind = FullCodeCharKind::InString;
                match chr {
                    '"' => {
                        if sharps == 0 {
                            char_kind = FullCodeCharKind::Normal;
                            CharClassesStatus::Normal
                        } else if self.is_raw_string_suffix(sharps) {
                            CharClassesStatus::RawStringSuffix(sharps)
                        } else {
                            CharClassesStatus::LitRawString(sharps)
                        }
                    }
                    _ => CharClassesStatus::LitRawString(sharps),
                }
            }
            CharClassesStatus::RawStringPrefix(sharps) => {
                char_kind = FullCodeCharKind::InString;
                match chr {
                    '#' => CharClassesStatus::RawStringPrefix(sharps + 1),
                    '"' => CharClassesStatus::LitRawString(sharps),
                    _ => CharClassesStatus::Normal, // Unreachable.
                }
            }
            CharClassesStatus::RawStringSuffix(sharps) => match chr {
                '#' => {
                    if sharps == 1 {
                        CharClassesStatus::Normal
                    } else {
                        char_kind = FullCodeCharKind::InString;
                        CharClassesStatus::RawStringSuffix(sharps - 1)
                    }
                }
                _ => CharClassesStatus::Normal, // Unreachable
            },
            CharClassesStatus::LitString => {
                char_kind = FullCodeCharKind::InString;
                match chr {
                    '"' => CharClassesStatus::Normal,
                    '\\' => CharClassesStatus::LitStringEscape,
                    _ => CharClassesStatus::LitString,
                }
            }
            CharClassesStatus::LitStringEscape => {
                char_kind = FullCodeCharKind::InString;
                CharClassesStatus::LitString
            }
            CharClassesStatus::LitChar => match chr {
                '\\' => CharClassesStatus::LitCharEscape,
                '\'' => CharClassesStatus::Normal,
                _ => CharClassesStatus::LitChar,
            },
            CharClassesStatus::LitCharEscape => CharClassesStatus::LitChar,
            CharClassesStatus::Normal => match chr {
                // rustfmt 1.9 treats any `r#` as a raw string opener, including
                // raw identifiers; the state machine falls back to `Normal` at the
                // first character that is neither `#` nor `"`.
                'r' => match self.peek() {
                    Some('#') | Some('"') => {
                        char_kind = FullCodeCharKind::InString;
                        CharClassesStatus::RawStringPrefix(0)
                    }
                    _ => CharClassesStatus::Normal,
                },
                '"' => {
                    char_kind = FullCodeCharKind::InString;
                    CharClassesStatus::LitString
                }
                '\'' => match self.peek() {
                    Some('\\') => CharClassesStatus::LitChar,
                    // rustfmt peeks twice here (multipeek), so the second
                    // peek looks one character further ahead.
                    _ if self.peek_nth(1) == Some('\'') => CharClassesStatus::LitChar,
                    _ => CharClassesStatus::Normal,
                },
                '/' => match self.peek() {
                    Some('*') => {
                        self.status = CharClassesStatus::BlockCommentOpening(1);
                        return Some((FullCodeCharKind::StartComment, idx, chr));
                    }
                    Some('/') => {
                        self.status = CharClassesStatus::LineComment;
                        return Some((FullCodeCharKind::StartComment, idx, chr));
                    }
                    _ => CharClassesStatus::Normal,
                },
                _ => CharClassesStatus::Normal,
            },
            CharClassesStatus::StringInBlockComment(deepness) => {
                char_kind = FullCodeCharKind::InStringCommented;
                if chr == '"' {
                    CharClassesStatus::BlockComment(deepness)
                } else if chr == '*' && self.peek() == Some('/') {
                    char_kind = FullCodeCharKind::InComment;
                    CharClassesStatus::BlockCommentClosing(deepness - 1)
                } else {
                    CharClassesStatus::StringInBlockComment(deepness)
                }
            }
            CharClassesStatus::BlockComment(deepness) => {
                char_kind = FullCodeCharKind::InComment;
                match self.peek() {
                    Some('/') if chr == '*' => CharClassesStatus::BlockCommentClosing(deepness - 1),
                    Some('*') if chr == '/' => CharClassesStatus::BlockCommentOpening(deepness + 1),
                    _ if chr == '"' => CharClassesStatus::StringInBlockComment(deepness),
                    _ => self.status,
                }
            }
            CharClassesStatus::BlockCommentOpening(deepness) => {
                self.status = CharClassesStatus::BlockComment(deepness);
                return Some((FullCodeCharKind::InComment, idx, chr));
            }
            CharClassesStatus::BlockCommentClosing(deepness) => {
                if deepness == 0 {
                    self.status = CharClassesStatus::Normal;
                    return Some((FullCodeCharKind::EndComment, idx, chr));
                } else {
                    self.status = CharClassesStatus::BlockComment(deepness);
                    return Some((FullCodeCharKind::InComment, idx, chr));
                }
            }
            CharClassesStatus::LineComment => match chr {
                '\n' => {
                    self.status = CharClassesStatus::Normal;
                    return Some((FullCodeCharKind::EndComment, idx, chr));
                }
                _ => {
                    self.status = CharClassesStatus::LineComment;
                    return Some((FullCodeCharKind::InComment, idx, chr));
                }
            },
        };
        Some((char_kind, idx, chr))
    }
}

/// An iterator over the lines of a string, paired with the char kind at the
/// end of the line.
pub(crate) struct LineClasses<'a> {
    base: std::iter::Peekable<CharClasses<'a>>,
    src: &'a str,
    kind: FullCodeCharKind,
}

impl<'a> LineClasses<'a> {
    pub(crate) fn new(s: &'a str) -> Self {
        LineClasses {
            base: CharClasses::new(s).peekable(),
            src: s,
            kind: FullCodeCharKind::Normal,
        }
    }
}

impl<'a> Iterator for LineClasses<'a> {
    type Item = (FullCodeCharKind, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let &(start_kind, start, _) = self.base.peek()?;
        let mut end = self.src.len();

        for (kind, i, c) in self.base.by_ref() {
            // needed to set the kind of the ending character on the last line
            self.kind = kind;
            if c == '\n' {
                end = i;
                self.kind = match (start_kind, kind) {
                    (FullCodeCharKind::Normal, FullCodeCharKind::InString) => {
                        FullCodeCharKind::StartString
                    }
                    (FullCodeCharKind::InString, FullCodeCharKind::Normal) => {
                        FullCodeCharKind::EndString
                    }
                    (FullCodeCharKind::InComment, FullCodeCharKind::InStringCommented) => {
                        FullCodeCharKind::StartStringCommented
                    }
                    (FullCodeCharKind::InStringCommented, FullCodeCharKind::InComment) => {
                        FullCodeCharKind::EndStringCommented
                    }
                    _ => kind,
                };
                break;
            }
        }

        let mut line = &self.src[start..end];
        // Workaround for CRLF newline.
        if let Some(stripped) = line.strip_suffix('\r') {
            line = stripped;
        }

        Some((self.kind, line))
    }
}

/// Iterator over functional and commented parts of a string. Any part of a string is either
/// functional code, either *one* block comment, either *one* line comment. Whitespace between
/// comments is functional code. Line comments contain their ending newlines.
struct UngroupedCommentCodeSlices<'a> {
    slice: &'a str,
    iter: CharClasses<'a>,
    /// The next item of `iter`, once looked at.
    peeked: Option<Option<(FullCodeCharKind, usize, char)>>,
}

impl<'a> UngroupedCommentCodeSlices<'a> {
    fn new(code: &'a str) -> UngroupedCommentCodeSlices<'a> {
        UngroupedCommentCodeSlices {
            slice: code,
            iter: CharClasses::new(code),
            peeked: None,
        }
    }

    fn advance(&mut self) -> Option<(FullCodeCharKind, usize, char)> {
        self.peeked.take().unwrap_or_else(|| self.iter.next())
    }

    fn peek(&mut self) -> Option<(FullCodeCharKind, usize, char)> {
        *self.peeked.get_or_insert_with(|| self.iter.next())
    }
}

impl<'a> Iterator for UngroupedCommentCodeSlices<'a> {
    type Item = (CodeCharKind, usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let (kind, start_idx, _) = self.advance()?;
        match kind {
            FullCodeCharKind::Normal | FullCodeCharKind::InString => {
                // Consume all the Normal code. Characters skipped by `skip_plain` are
                // `Normal`, so they belong to this slice.
                loop {
                    if self.peeked.is_none() {
                        self.iter.skip_plain(None);
                    }
                    match self.peek() {
                        Some((char_kind, _, _)) if !char_kind.is_comment() => {
                            self.peeked = None;
                        }
                        _ => break,
                    }
                }
            }
            FullCodeCharKind::StartComment => {
                // Consume the whole comment
                loop {
                    match self.advance() {
                        Some((kind, ..)) if kind.inside_comment() => continue,
                        _ => break,
                    }
                }
            }
            _ => {}
        }
        let slice = match self.peek() {
            Some((_, end_idx, _)) => &self.slice[start_idx..end_idx],
            None => &self.slice[start_idx..],
        };
        Some((
            if kind.is_comment() {
                CodeCharKind::Comment
            } else {
                CodeCharKind::Normal
            },
            start_idx,
            slice,
        ))
    }
}

/// Iterator over an alternating sequence of functional and commented parts of
/// a string. The first item is always a, possibly zero length, subslice of
/// functional text. Line style comments contain their ending newlines.
pub(crate) struct CommentCodeSlices<'a> {
    slice: &'a str,
    last_slice_kind: CodeCharKind,
    last_slice_end: usize,
}

impl<'a> CommentCodeSlices<'a> {
    pub(crate) fn new(slice: &'a str) -> CommentCodeSlices<'a> {
        CommentCodeSlices {
            slice,
            last_slice_kind: CodeCharKind::Comment,
            last_slice_end: 0,
        }
    }
}

impl<'a> Iterator for CommentCodeSlices<'a> {
    type Item = (CodeCharKind, usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.last_slice_end == self.slice.len() {
            return None;
        }

        let mut sub_slice_end = self.last_slice_end;
        let mut first_whitespace = None;
        let subslice = &self.slice[self.last_slice_end..];
        let mut iter = CharClasses::new(subslice);

        for (kind, i, c) in &mut iter {
            let is_comment_connector = self.last_slice_kind == CodeCharKind::Normal
                && subslice.starts_with("//")
                && [' ', '\t'].contains(&c);

            if is_comment_connector && first_whitespace.is_none() {
                first_whitespace = Some(i);
            }

            if kind.to_codecharkind() == self.last_slice_kind && !is_comment_connector {
                let last_index = match first_whitespace {
                    Some(j) => j,
                    None => i,
                };
                sub_slice_end = self.last_slice_end + last_index;
                break;
            }

            if !is_comment_connector {
                first_whitespace = None;
            }
        }

        if let (None, true) = (iter.next(), sub_slice_end == self.last_slice_end) {
            // This was the last subslice.
            sub_slice_end = match first_whitespace {
                Some(i) => self.last_slice_end + i,
                None => self.slice.len(),
            };
        }

        let kind = match self.last_slice_kind {
            CodeCharKind::Comment => CodeCharKind::Normal,
            CodeCharKind::Normal => CodeCharKind::Comment,
        };
        let res = (
            kind,
            self.last_slice_end,
            &self.slice[self.last_slice_end..sub_slice_end],
        );
        self.last_slice_end = sub_slice_end;
        self.last_slice_kind = kind;

        Some(res)
    }
}

/// Checks is `new` didn't miss any comment from `span`, if it removed any, return previous text
pub(crate) fn recover_comment_removed(
    new: String,
    span: Span,
    context: &RewriteContext<'_>,
) -> String {
    let snippet = context.snippet(span);
    if snippet != new && changed_comment_content(snippet, &new) {
        // We missed some comments. Keep the original text. rustfmt reports this as a
        // `LostComment` error, which only matters when formatting a macro definition body.
        context.run.lost_comment.set(true);
        snippet.to_owned()
    } else {
        new
    }
}

pub(crate) fn filter_normal_code(code: &str) -> Cow<'_, str> {
    if !code.contains('/') {
        return Cow::Borrowed(code);
    }
    let mut buffer = String::with_capacity(code.len());
    LineClasses::new(code).for_each(|(kind, line)| match kind {
        FullCodeCharKind::Normal
        | FullCodeCharKind::StartString
        | FullCodeCharKind::InString
        | FullCodeCharKind::EndString => {
            buffer.push_str(line);
            buffer.push('\n');
        }
        _ => (),
    });
    if !code.ends_with('\n') && buffer.ends_with('\n') {
        buffer.pop();
    }
    Cow::Owned(buffer)
}

/// Returns `true` if the two strings of code have the same payload of comments.
/// The payload of comments is everything in the string except:
/// - actual code (not comments),
/// - comment start/end marks,
/// - whitespace,
/// - '*' at the beginning of lines in block comments.
fn changed_comment_content(orig: &str, new: &str) -> bool {
    // Fast path: every comment starts with `/`, and two texts without comments have the
    // same (empty) comment content.
    if !orig.contains('/') && !new.contains('/') {
        return false;
    }
    let code_comment_content = |code| {
        UngroupedCommentCodeSlices::new(code)
            .filter(|(kind, _, _)| *kind == CodeCharKind::Comment)
            .flat_map(|(_, _, s)| CommentReducer::new(s))
    };
    code_comment_content(orig).ne(code_comment_content(new))
}

/// Iterator over the 'payload' characters of a comment.
/// It skips whitespace, comment start/end marks, and '*' at the beginning of lines.
/// The comment must be one comment, ie not more than one start mark (no multiple line comments,
/// for example).
struct CommentReducer<'a> {
    is_block: bool,
    at_start_line: bool,
    iter: std::str::Chars<'a>,
}

impl<'a> CommentReducer<'a> {
    fn new(comment: &'a str) -> CommentReducer<'a> {
        let is_block = comment.starts_with("/*");
        let comment = remove_comment_header(comment);
        CommentReducer {
            is_block,
            // There are no supplementary '*' on the first line.
            at_start_line: false,
            iter: comment.chars(),
        }
    }
}

impl Iterator for CommentReducer<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut c = self.iter.next()?;
            if self.is_block && self.at_start_line {
                while c.is_whitespace() {
                    c = self.iter.next()?;
                }
                // Ignore leading '*'.
                if c == '*' {
                    c = self.iter.next()?;
                }
            } else if c == '\n' {
                self.at_start_line = true;
            }
            if !c.is_whitespace() {
                return Some(c);
            }
        }
    }
}

fn remove_comment_header(comment: &str) -> &str {
    if comment.starts_with("///") || comment.starts_with("//!") {
        &comment[3..]
    } else if let Some(stripped) = comment.strip_prefix("//") {
        stripped
    } else if ((comment.starts_with("/**") && !comment.starts_with("/**/"))
        || comment.starts_with("/*!"))
        && comment.len() >= 5
    {
        &comment[3..comment.len() - 2]
    } else if comment.len() >= 4 {
        &comment[2..comment.len() - 2]
    } else {
        // Unterminated block comment at end of input.
        comment.get(2..).unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_classes() {
        let kinds: Vec<_> = CharClasses::new("//\n\n").map(|(k, _, c)| (k, c)).collect();
        assert_eq!(
            kinds,
            [
                (FullCodeCharKind::StartComment, '/'),
                (FullCodeCharKind::InComment, '/'),
                (FullCodeCharKind::EndComment, '\n'),
                (FullCodeCharKind::Normal, '\n'),
            ]
        );
    }

    #[test]
    fn raw_string_is_not_a_comment() {
        assert!(contains_comment("r#struct // c"));
        assert!(!contains_comment("r#\"// not a comment\"#"));
    }

    #[test]
    fn find_uncommented_skips_comments() {
        assert_eq!("/* , */ a, b".find_uncommented(","), Some(9));
        assert_eq!("a /* = */ = b".find_uncommented("="), Some(10));
    }

    #[test]
    fn comment_payload_comparison() {
        assert!(!changed_comment_content("a /* x */ b", "a\n/*  x */\nb"));
        assert!(changed_comment_content("a // x\n", "a\n"));
    }
}
