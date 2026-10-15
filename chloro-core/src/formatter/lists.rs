//! Format list-like expressions and items (rustfmt's `lists.rs`).

use std::cmp;
use std::iter::Peekable;

use super::comment::{FindUncommented, find_any_byte, find_comment_end, rewrite_comment};
use super::config::Settings;
use super::context::RewriteContext;
use super::shape::{Indent, Shape};
use super::span::{BytePos, SnippetProvider, mk_sp};
use super::utils::{
    count_newlines, first_line_width, last_line_width, starts_with_newline, unicode_str_width,
};

/// The definitive formatting tactic for lists.
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub(crate) enum DefinitiveListTactic {
    Vertical,
    Horizontal,
    Mixed,
    /// Special case tactic for `format!()`, `write!()` style macros.
    SpecialMacro(usize),
}

impl DefinitiveListTactic {
    /// Block indent style: everything but a horizontal list ends with a newline.
    pub(crate) fn ends_with_newline(&self) -> bool {
        *self != DefinitiveListTactic::Horizontal
    }
}

/// Formatting tactic for lists. This will be cast down to a
/// `DefinitiveListTactic` depending on the number and length of the items and
/// their comments.
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub(crate) enum ListTactic {
    /// One item per row.
    Vertical,
    /// All items on one row.
    Horizontal,
    /// Try Horizontal layout, if that fails then vertical.
    HorizontalVertical,
    /// HorizontalVertical with a soft limit of n characters.
    LimitedHorizontalVertical(usize),
    /// Pack as many items as possible per row over (possibly) many rows.
    Mixed,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub(crate) enum SeparatorTactic {
    Always,
    Never,
    Vertical,
}

impl SeparatorTactic {}

/// Where to put separator.
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub(crate) enum SeparatorPlace {
    Front,
    Back,
}

impl SeparatorPlace {
    pub(crate) fn is_front(self) -> bool {
        self == SeparatorPlace::Front
    }

    pub(crate) fn is_back(self) -> bool {
        self == SeparatorPlace::Back
    }

    pub(crate) fn from_tactic(
        default: SeparatorPlace,
        tactic: DefinitiveListTactic,
        sep: &str,
    ) -> SeparatorPlace {
        match tactic {
            DefinitiveListTactic::Vertical => default,
            _ => {
                if sep == "," {
                    SeparatorPlace::Back
                } else {
                    default
                }
            }
        }
    }
}

pub(crate) struct ListFormatting<'a> {
    tactic: DefinitiveListTactic,
    separator: &'a str,
    trailing_separator: SeparatorTactic,
    separator_place: SeparatorPlace,
    shape: Shape,
    /// Non-expressions, e.g., items, will have a new line at the end of the list.
    /// Important for comment styles.
    ends_with_newline: bool,
    /// Remove newlines between list elements for expressions.
    preserve_newline: bool,
    /// Nested import lists get some special handling for the "Mixed" list type
    nested: bool,
    /// Whether comments should be visually aligned.
    align_comments: bool,
    config: &'a Settings,
}

impl<'a> ListFormatting<'a> {
    pub(crate) fn new(shape: Shape, config: &'a Settings) -> Self {
        ListFormatting {
            tactic: DefinitiveListTactic::Vertical,
            separator: ",",
            trailing_separator: SeparatorTactic::Never,
            separator_place: SeparatorPlace::Back,
            shape,
            ends_with_newline: true,
            preserve_newline: false,
            nested: false,
            align_comments: true,
            config,
        }
    }

    pub(crate) fn tactic(mut self, tactic: DefinitiveListTactic) -> Self {
        self.tactic = tactic;
        self
    }

    pub(crate) fn separator(mut self, separator: &'a str) -> Self {
        self.separator = separator;
        self
    }

    pub(crate) fn trailing_separator(mut self, trailing_separator: SeparatorTactic) -> Self {
        self.trailing_separator = trailing_separator;
        self
    }

    pub(crate) fn separator_place(mut self, separator_place: SeparatorPlace) -> Self {
        self.separator_place = separator_place;
        self
    }

    pub(crate) fn ends_with_newline(mut self, ends_with_newline: bool) -> Self {
        self.ends_with_newline = ends_with_newline;
        self
    }

    pub(crate) fn preserve_newline(mut self, preserve_newline: bool) -> Self {
        self.preserve_newline = preserve_newline;
        self
    }

    pub(crate) fn nested(mut self, nested: bool) -> Self {
        self.nested = nested;
        self
    }

    pub(crate) fn align_comments(mut self, align_comments: bool) -> Self {
        self.align_comments = align_comments;
        self
    }

    pub(crate) fn needs_trailing_separator(&self) -> bool {
        match self.trailing_separator {
            // We always put separator in front.
            SeparatorTactic::Always => true,
            SeparatorTactic::Vertical => self.tactic == DefinitiveListTactic::Vertical,
            SeparatorTactic::Never => {
                self.tactic == DefinitiveListTactic::Vertical && self.separator_place.is_front()
            }
        }
    }
}

impl AsRef<ListItem> for ListItem {
    fn as_ref(&self) -> &ListItem {
        self
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub(crate) enum ListItemCommentStyle {
    /// Try to keep the comment on the same line with the item.
    SameLine,
    /// Put the comment on the previous or the next line of the item.
    DifferentLine,
    /// No comment available.
    None,
}

#[derive(Debug, Clone)]
pub(crate) struct ListItem {
    /// None for comments mean that they are not present.
    pub(crate) pre_comment: Option<String>,
    pub(crate) pre_comment_style: ListItemCommentStyle,
    /// Item should include attributes and doc comments. None indicates a failed rewrite.
    pub(crate) item: Option<String>,
    pub(crate) post_comment: Option<String>,
    /// Whether there is extra whitespace before this item.
    pub(crate) new_lines: bool,
}

impl ListItem {
    pub(crate) fn from_item(item: Option<String>) -> ListItem {
        ListItem {
            pre_comment: None,
            pre_comment_style: ListItemCommentStyle::None,
            item,
            post_comment: None,
            new_lines: false,
        }
    }

    pub(crate) fn inner_as_ref(&self) -> &str {
        self.item.as_deref().unwrap_or("")
    }

    pub(crate) fn is_different_group(&self) -> bool {
        self.inner_as_ref().contains('\n')
            || self.pre_comment.is_some()
            || self.post_comment.as_ref().is_some_and(|s| s.contains('\n'))
    }

    pub(crate) fn is_multiline(&self) -> bool {
        self.inner_as_ref().contains('\n')
            || self.pre_comment.as_ref().is_some_and(|s| s.contains('\n'))
            || self.post_comment.as_ref().is_some_and(|s| s.contains('\n'))
    }

    pub(crate) fn has_single_line_comment(&self) -> bool {
        self.pre_comment
            .as_ref()
            .is_some_and(|comment| comment.trim_start().starts_with("//"))
            || self
                .post_comment
                .as_ref()
                .is_some_and(|comment| comment.trim_start().starts_with("//"))
    }

    pub(crate) fn has_comment(&self) -> bool {
        self.pre_comment.is_some() || self.post_comment.is_some()
    }

    pub(crate) fn from_str<S: Into<String>>(s: S) -> ListItem {
        ListItem::from_item(Some(s.into()))
    }

    /// Returns `true` if the item causes something to be written.
    fn is_substantial(&self) -> bool {
        fn empty(s: &Option<String>) -> bool {
            !matches!(*s, Some(ref s) if !s.is_empty())
        }
        !(empty(&self.pre_comment) && empty(&self.item) && empty(&self.post_comment))
    }
}

/// The type of separator for lists.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum Separator {
    Comma,
    VerticalBar,
}

impl Separator {
    pub(crate) fn len(self) -> usize {
        match self {
            // 2 = `, `
            Separator::Comma => 2,
            // 3 = ` | `
            Separator::VerticalBar => 3,
        }
    }
}

pub(crate) fn definitive_tactic<I, T>(
    items: I,
    tactic: ListTactic,
    sep: Separator,
    width: usize,
) -> DefinitiveListTactic
where
    I: IntoIterator<Item = T> + Clone,
    T: AsRef<ListItem>,
{
    let pre_line_comments = items
        .clone()
        .into_iter()
        .any(|item| item.as_ref().has_single_line_comment());

    let limit = match tactic {
        _ if pre_line_comments => return DefinitiveListTactic::Vertical,
        ListTactic::Horizontal => return DefinitiveListTactic::Horizontal,
        ListTactic::Vertical => return DefinitiveListTactic::Vertical,
        ListTactic::LimitedHorizontalVertical(limit) => cmp::min(width, limit),
        ListTactic::Mixed | ListTactic::HorizontalVertical => width,
    };

    // rustfmt sums the widths of all items and their separators, then tests every item for
    // a newline: horizontal iff the sum fits `limit` and no item is multi-line. Both tests
    // are made item by item here and stop at the first item that decides: a multi-line
    // item, or a prefix whose widths and separators already exceed `limit` (the sum only
    // grows). Neither test has an effect, so the answer is rustfmt's.
    let mut total_width = 0;
    let mut sep_total = 0;
    let horizontal = items.into_iter().enumerate().all(|(i, item)| {
        let item = item.as_ref();
        if item.is_multiline() {
            return false;
        }
        if i > 0 {
            sep_total += sep.len();
        }
        total_width += total_item_width(item);
        total_width + sep_total <= limit
    });

    if horizontal {
        DefinitiveListTactic::Horizontal
    } else {
        match tactic {
            ListTactic::Mixed => DefinitiveListTactic::Mixed,
            _ => DefinitiveListTactic::Vertical,
        }
    }
}

/// Format a list of commented items into a string.
pub(crate) fn write_list<I, T>(items: I, formatting: &ListFormatting<'_>) -> Option<String>
where
    I: IntoIterator<Item = T> + Clone,
    T: AsRef<ListItem>,
{
    let tactic = formatting.tactic;
    let sep_len = formatting.separator.len();

    // Now that we know how we will layout, we can decide for sure if there
    // will be a trailing separator.
    let mut trailing_separator = formatting.needs_trailing_separator();
    let cloned_items = items.clone();
    let mut iter = items.into_iter().enumerate().peekable();
    let mut item_max_width: Option<usize> = None;
    let sep_place =
        SeparatorPlace::from_tactic(formatting.separator_place, tactic, formatting.separator);
    let mut prev_item_had_post_comment = false;
    let mut prev_item_is_nested_import = false;

    let mut line_len = 0;
    let indent_str = &formatting.shape.indent.to_string(formatting.config);
    // Room for every item with its comments, a separator, a newline and an indent, so the
    // loop rarely grows the buffer; at least rustfmt's 128 bytes.
    let estimate: usize = cloned_items
        .clone()
        .into_iter()
        .map(|item| {
            let item = item.as_ref();
            let len = |s: &Option<String>| s.as_deref().map_or(0, str::len);
            len(&item.item)
                + len(&item.pre_comment)
                + len(&item.post_comment)
                + sep_len
                + 1
                + indent_str.len()
        })
        .sum();
    let mut result = String::with_capacity(128.max(estimate));
    while let Some((i, item)) = iter.next() {
        let item = item.as_ref();
        let inner_item = item.item.as_deref()?;
        let first = i == 0;
        let last = iter.peek().is_none();
        let mut separate = match sep_place {
            SeparatorPlace::Front => !first,
            SeparatorPlace::Back => !last || trailing_separator,
        };
        let item_sep_len = if separate { sep_len } else { 0 };

        // Item string may be multi-line. Its length (used for block comment alignment)
        // should be only the length of the last line.
        let item_last_line = if item.is_multiline() {
            inner_item.lines().last().unwrap_or("")
        } else {
            inner_item
        };
        let mut item_last_line_width = unicode_str_width(item_last_line) + item_sep_len;
        if item_last_line.starts_with(&**indent_str) {
            item_last_line_width -= unicode_str_width(indent_str);
        }

        if !item.is_substantial() {
            continue;
        }

        match tactic {
            DefinitiveListTactic::Horizontal if !first => {
                result.push(' ');
            }
            DefinitiveListTactic::SpecialMacro(num_args_before) => {
                if i == 0 {
                    // Nothing
                } else if i < num_args_before {
                    result.push(' ');
                } else if i <= num_args_before + 1 {
                    result.push('\n');
                    result.push_str(indent_str);
                } else {
                    result.push(' ');
                }
            }
            DefinitiveListTactic::Vertical
                if !first && !inner_item.is_empty() && !result.is_empty() =>
            {
                result.push('\n');
                result.push_str(indent_str);
            }
            DefinitiveListTactic::Mixed => {
                let total_width = total_item_width(item) + item_sep_len;

                // 1 is space between separator and item.
                if (line_len > 0 && line_len + 1 + total_width > formatting.shape.width)
                    || prev_item_had_post_comment
                    || (formatting.nested
                        && (prev_item_is_nested_import
                            || (!first && inner_item.contains(':') && inner_item.contains("::"))))
                {
                    result.push('\n');
                    result.push_str(indent_str);
                    line_len = 0;
                    if formatting.ends_with_newline {
                        trailing_separator = true;
                    }
                } else if line_len > 0 {
                    result.push(' ');
                    line_len += 1;
                }

                if last && formatting.ends_with_newline {
                    separate = formatting.trailing_separator != SeparatorTactic::Never;
                }

                line_len += total_width;
            }
            _ => {}
        }

        // Pre-comments
        if let Some(ref comment) = item.pre_comment {
            // Block style in non-vertical mode.
            let block_mode = tactic == DefinitiveListTactic::Horizontal;
            // Width restriction is only relevant in vertical mode.
            let comment =
                rewrite_comment(comment, block_mode, formatting.shape, formatting.config)?;
            result.push_str(&comment);

            if !inner_item.is_empty() {
                use DefinitiveListTactic::*;
                if matches!(tactic, Vertical | Mixed | SpecialMacro(_)) {
                    // We cannot keep pre-comments on the same line if the comment is normalized.
                    let keep_comment = if formatting.config.normalize_comments()
                        || item.pre_comment_style == ListItemCommentStyle::DifferentLine
                    {
                        false
                    } else {
                        // We will try to keep the comment on the same line with the item here.
                        // 1 = ` `
                        let total_width = total_item_width(item) + item_sep_len + 1;
                        total_width <= formatting.shape.width
                    };
                    if keep_comment {
                        result.push(' ');
                    } else {
                        result.push('\n');
                        result.push_str(indent_str);
                        // This is the width of the item (without comments).
                        line_len = item.item.as_deref().map_or(0, unicode_str_width);
                    }
                } else {
                    result.push(' ')
                }
            }
            item_max_width = None;
        }

        if separate && sep_place.is_front() && !first {
            result.push_str(formatting.separator.trim());
            result.push(' ');
        }
        result.push_str(inner_item);

        // Post-comments
        if tactic == DefinitiveListTactic::Horizontal
            && let Some(comment) = item.post_comment.as_ref()
        {
            let formatted_comment = rewrite_comment(
                comment,
                true,
                Shape::legacy(formatting.shape.width, Indent::empty()),
                formatting.config,
            )?;

            result.push(' ');
            result.push_str(&formatted_comment);
        }

        if separate && sep_place.is_back() {
            result.push_str(formatting.separator);
        }

        if tactic != DefinitiveListTactic::Horizontal
            && let Some(comment) = item.post_comment.as_ref()
        {
            let overhead = last_line_width(&result) + first_line_width(comment.trim());

            let rewrite_post_comment = |item_max_width: &mut Option<usize>| {
                if item_max_width.is_none() && !last && !inner_item.contains('\n') {
                    *item_max_width = Some(max_width_of_item_with_post_comment(
                        &cloned_items,
                        i,
                        overhead,
                        formatting.config.max_width(),
                    ));
                }
                let overhead = if starts_with_newline(comment) {
                    0
                } else if let Some(max_width) = *item_max_width {
                    max_width + 2
                } else {
                    // 1 = space between item and comment.
                    item_last_line_width + 1
                };
                let width = formatting.shape.width.checked_sub(overhead).unwrap_or(1);
                let offset = formatting.shape.indent + overhead;
                let comment_shape = Shape::legacy(width, offset);

                let block_style = if !formatting.ends_with_newline && last {
                    true
                } else if starts_with_newline(comment) {
                    false
                } else {
                    comment.trim().contains('\n') || unicode_str_width(comment.trim()) > width
                };

                rewrite_comment(
                    comment.trim_start(),
                    block_style,
                    comment_shape,
                    formatting.config,
                )
            };

            let mut formatted_comment = rewrite_post_comment(&mut item_max_width)?;

            if !starts_with_newline(comment) {
                if formatting.align_comments {
                    let mut comment_alignment =
                        post_comment_alignment(item_max_width, unicode_str_width(inner_item));
                    if first_line_width(&formatted_comment)
                        + last_line_width(&result)
                        + comment_alignment
                        + 1
                        > formatting.config.max_width()
                    {
                        item_max_width = None;
                        formatted_comment = rewrite_post_comment(&mut item_max_width)?;
                        comment_alignment =
                            post_comment_alignment(item_max_width, unicode_str_width(inner_item));
                    }
                    for _ in 0..=comment_alignment {
                        result.push(' ');
                    }
                }
                // An additional space for the missing trailing separator (or
                // if we skipped alignment above).
                if !formatting.align_comments
                    || (last
                        && item_max_width.is_some()
                        && !separate
                        && !formatting.separator.is_empty())
                {
                    result.push(' ');
                }
            } else {
                result.push('\n');
                result.push_str(indent_str);
            }
            if formatted_comment.contains('\n') {
                item_max_width = None;
            }
            result.push_str(&formatted_comment);
        } else {
            item_max_width = None;
        }

        if formatting.preserve_newline
            && !last
            && tactic == DefinitiveListTactic::Vertical
            && item.new_lines
        {
            item_max_width = None;
            result.push('\n');
        }

        prev_item_had_post_comment = item.post_comment.is_some();
        // `contains(':')` is a `memchr`; most items have no `:` and skip the substring
        // search.
        prev_item_is_nested_import = inner_item.contains(':') && inner_item.contains("::");
    }

    Some(result)
}

fn max_width_of_item_with_post_comment<I, T>(
    items: &I,
    i: usize,
    overhead: usize,
    max_budget: usize,
) -> usize
where
    I: IntoIterator<Item = T> + Clone,
    T: AsRef<ListItem>,
{
    let mut max_width = 0;
    let mut first = true;
    for item in items.clone().into_iter().skip(i) {
        let item = item.as_ref();
        let inner_item_width = unicode_str_width(item.inner_as_ref());
        if !first
            && (item.is_different_group()
                || item.post_comment.is_none()
                || inner_item_width + overhead > max_budget)
        {
            return max_width;
        }
        if max_width < inner_item_width {
            max_width = inner_item_width;
        }
        if item.new_lines {
            return max_width;
        }
        first = false;
    }
    max_width
}

fn post_comment_alignment(item_max_width: Option<usize>, inner_item_width: usize) -> usize {
    item_max_width.unwrap_or(0).saturating_sub(inner_item_width)
}

pub(crate) struct ListItems<'a, I, F1, F2, F3>
where
    I: Iterator,
{
    snippet_provider: SnippetProvider<'a>,
    inner: Peekable<I>,
    get_lo: F1,
    get_hi: F2,
    get_item_string: F3,
    prev_span_end: BytePos,
    next_span_start: BytePos,
    terminator: &'a str,
    separator: &'a str,
    leave_last: bool,
}

pub(crate) fn extract_pre_comment(pre_snippet: &str) -> (Option<String>, ListItemCommentStyle) {
    // Each of the three tests below needs a `/` in the snippet.
    if !has_byte(pre_snippet, b'/') {
        return (None, ListItemCommentStyle::None);
    }
    extract_pre_comment_full(pre_snippet)
}

/// Whether `s` contains the ASCII byte `b`, eight bytes at a time.
#[inline]
fn has_byte(s: &str, b: u8) -> bool {
    find_any_byte(s.as_bytes(), 0, [b]) < s.len()
}

fn extract_pre_comment_full(pre_snippet: &str) -> (Option<String>, ListItemCommentStyle) {
    let trimmed_pre_snippet = pre_snippet.trim();
    // Both start and end are checked to support keeping a block comment inline with
    // the item, even if there are preceding line comments, while still supporting
    // a snippet that starts with a block comment but also contains one or more
    // trailing single line comments.
    let starts_with_block_comment = trimmed_pre_snippet.starts_with("/*");
    let ends_with_block_comment = trimmed_pre_snippet.ends_with("*/");
    let starts_with_single_line_comment = trimmed_pre_snippet.starts_with("//");
    if ends_with_block_comment {
        let comment_end = pre_snippet.rfind('/').unwrap_or(0);
        if pre_snippet[comment_end..].contains('\n') {
            (
                Some(trimmed_pre_snippet.to_owned()),
                ListItemCommentStyle::DifferentLine,
            )
        } else {
            (
                Some(trimmed_pre_snippet.to_owned()),
                ListItemCommentStyle::SameLine,
            )
        }
    } else if starts_with_single_line_comment || starts_with_block_comment {
        (
            Some(trimmed_pre_snippet.to_owned()),
            ListItemCommentStyle::DifferentLine,
        )
    } else {
        (None, ListItemCommentStyle::None)
    }
}

pub(crate) fn extract_post_comment(
    post_snippet: &str,
    comment_end: usize,
    separator: &str,
    is_last: bool,
) -> Option<String> {
    // A post-comment is returned only when a part of the snippet starts with `//` or `/*`.
    if !has_byte(&post_snippet[..comment_end], b'/') {
        return None;
    }
    extract_post_comment_full(post_snippet, comment_end, separator, is_last)
}

fn extract_post_comment_full(
    post_snippet: &str,
    comment_end: usize,
    separator: &str,
    is_last: bool,
) -> Option<String> {
    let white_space: &[_] = &[' ', '\t'];

    // Cleanup post-comment: strip separators and whitespace.
    let post_snippet = post_snippet[..comment_end].trim();

    let last_inline_comment_ends_with_separator = if is_last {
        if let Some(line) = post_snippet.lines().last() {
            line.ends_with(separator) && line.trim().starts_with("//")
        } else {
            false
        }
    } else {
        false
    };

    let post_snippet_trimmed = if post_snippet.starts_with([',', ':']) {
        post_snippet[1..].trim_matches(white_space)
    } else if let Some(stripped) = post_snippet.strip_prefix(separator) {
        stripped.trim_matches(white_space)
    } else if last_inline_comment_ends_with_separator {
        // since we're on the last item it's fine to keep any trailing separators in comments
        post_snippet.trim_matches(white_space)
    }
    // not comment or over two lines
    else if post_snippet.ends_with(separator)
        && (!post_snippet.trim().starts_with("//") || post_snippet.trim().contains('\n'))
    {
        post_snippet[..(post_snippet.len() - 1)].trim_matches(white_space)
    } else {
        post_snippet
    };
    let removed_newline_snippet = post_snippet_trimmed.trim();
    if !post_snippet_trimmed.is_empty()
        && (removed_newline_snippet.starts_with("//") || removed_newline_snippet.starts_with("/*"))
    {
        Some(post_snippet_trimmed.to_owned())
    } else {
        None
    }
}

pub(crate) fn get_comment_end(
    post_snippet: &str,
    separator: &str,
    terminator: &str,
    is_last: bool,
) -> usize {
    // Without `/` the snippet holds no comment: `CharClasses` yields every character as
    // `Normal` (code and character literals) or `InString` (strings, raw strings and
    // their prefixes), the two kinds `find_uncommented` matches, so its result for a
    // one-byte pattern is `find`'s; and there is no block comment.
    let pattern = if is_last { terminator } else { separator };
    if let [pattern] = *pattern.as_bytes()
        && !has_byte(post_snippet, b'/')
    {
        let bytes = post_snippet.as_bytes();
        if is_last {
            return memchr_or(bytes, pattern, bytes.len());
        }
        let newline_index = memchr_or(bytes, b'\n', usize::MAX);
        return match memchr_or(bytes, pattern, usize::MAX) {
            usize::MAX if newline_index != usize::MAX => newline_index + 1,
            usize::MAX => 0,
            separator_index if newline_index != usize::MAX && newline_index > separator_index => {
                newline_index + 1
            }
            _ => bytes.len(),
        };
    }
    get_comment_end_full(post_snippet, separator, terminator, is_last)
}

/// The position of the first `b` in `bytes`, or `default`.
#[inline]
fn memchr_or(bytes: &[u8], b: u8, default: usize) -> usize {
    match find_any_byte(bytes, 0, [b]) {
        i if i < bytes.len() => i,
        _ => default,
    }
}

fn get_comment_end_full(
    post_snippet: &str,
    separator: &str,
    terminator: &str,
    is_last: bool,
) -> usize {
    if is_last {
        return post_snippet
            .find_uncommented(terminator)
            .unwrap_or(post_snippet.len());
    }

    let mut block_open_index = post_snippet.find("/*");
    // check if it really is a block comment (and not `//*` or a nested comment)
    if let Some(i) = block_open_index {
        match post_snippet.find('/') {
            Some(j) if j < i => block_open_index = None,
            _ if post_snippet[..i].ends_with('/') => block_open_index = None,
            _ => (),
        }
    }
    let newline_index = post_snippet.find('\n');
    if let Some(separator_index) = post_snippet.find_uncommented(separator) {
        let block_end =
            |i: usize| find_comment_end(&post_snippet[i..]).unwrap_or(post_snippet.len() - i) + i;
        match (block_open_index, newline_index) {
            // Separator before comment, with the next item on same line.
            // Comment belongs to next item.
            (Some(i), None) if i > separator_index => separator_index + 1,
            // Block-style post-comment before the separator.
            (Some(i), None) => cmp::max(block_end(i), separator_index + 1),
            // Block-style post-comment. Either before or after the separator.
            (Some(i), Some(j)) if i < j => cmp::max(block_end(i), separator_index + 1),
            // Potential *single* line comment.
            (_, Some(j)) if j > separator_index => j + 1,
            _ => post_snippet.len(),
        }
    } else if let Some(newline_index) = newline_index {
        // Match arms may not have trailing comma. In any case, for match arms,
        // we will assume that the post comment belongs to the next arm if they
        // do not end with trailing comma.
        newline_index + 1
    } else {
        0
    }
}

/// Account for extra whitespace between items. This is fiddly
/// because of the way we divide pre- and post- comments.
pub(crate) fn has_extra_newline(post_snippet: &str, comment_end: usize) -> bool {
    if post_snippet.is_empty() || comment_end == 0 {
        return false;
    }
    // The same steps on bytes while the characters met are ASCII: the last character
    // before `comment_end` is one byte, the first line ends at the first `\n`, and the
    // whitespace that follows is `char::is_whitespace`'s ASCII set, which has `\x0B` and
    // `\x0C`. A non-ASCII byte on the way takes the character-based steps.
    let bytes = post_snippet.as_bytes();
    if bytes[comment_end - 1].is_ascii() {
        let rest = &bytes[comment_end - 1..];
        let first_newline = memchr_or(rest, b'\n', rest.len());
        let mut newlines = 0;
        for &b in &rest[first_newline..] {
            match b {
                b'\n' => {
                    newlines += 1;
                    if newlines > 1 {
                        return true;
                    }
                }
                b' ' | b'\t' | b'\r' | b'\x0B' | b'\x0C' => {}
                0x80.. => return has_extra_newline_full(post_snippet, comment_end),
                _ => return false,
            }
        }
        return false;
    }
    has_extra_newline_full(post_snippet, comment_end)
}

fn has_extra_newline_full(post_snippet: &str, comment_end: usize) -> bool {
    if post_snippet.is_empty() || comment_end == 0 {
        return false;
    }

    let len_last = post_snippet[..comment_end]
        .chars()
        .last()
        .map_or(1, char::len_utf8);
    // Everything from the separator to the next item.
    let test_snippet = &post_snippet[comment_end - len_last..];
    let first_newline = test_snippet.find('\n').unwrap_or(test_snippet.len());
    // From the end of the first line of comments.
    let test_snippet = &test_snippet[first_newline..];
    let first = test_snippet
        .find(|c: char| !c.is_whitespace())
        .unwrap_or(test_snippet.len());
    // From the end of the first line of comments to the next non-whitespace char.
    let test_snippet = &test_snippet[..first];

    // There were multiple line breaks which got trimmed to nothing.
    count_newlines(test_snippet) > 1
}

impl<T, I, F1, F2, F3> Iterator for ListItems<'_, I, F1, F2, F3>
where
    I: Iterator<Item = T>,
    F1: Fn(&T) -> BytePos,
    F2: Fn(&T) -> BytePos,
    F3: Fn(&T) -> Option<String>,
{
    type Item = ListItem;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|item| {
            // Pre-comment
            let pre_snippet = self
                .snippet_provider
                .span_to_snippet(mk_sp(self.prev_span_end, (self.get_lo)(&item)))
                .unwrap_or("");
            let (pre_comment, pre_comment_style) = extract_pre_comment(pre_snippet);

            // Post-comment
            let next_start = match self.inner.peek() {
                Some(next_item) => (self.get_lo)(next_item),
                None => self.next_span_start,
            };
            let post_snippet = self
                .snippet_provider
                .span_to_snippet(mk_sp((self.get_hi)(&item), next_start))
                .unwrap_or("");
            let is_last = self.inner.peek().is_none();
            let comment_end =
                get_comment_end(post_snippet, self.separator, self.terminator, is_last);
            let new_lines = has_extra_newline(post_snippet, comment_end);
            let post_comment =
                extract_post_comment(post_snippet, comment_end, self.separator, is_last);

            self.prev_span_end = (self.get_hi)(&item) + comment_end as BytePos;

            ListItem {
                pre_comment,
                pre_comment_style,
                // leave_last is set to true only for rewrite_items
                item: if self.inner.peek().is_none() && self.leave_last {
                    None
                } else {
                    (self.get_item_string)(&item)
                },
                post_comment,
                new_lines,
            }
        })
    }
}

/// Creates an iterator over a list's items with associated comments.
#[allow(clippy::too_many_arguments)]
pub(crate) fn itemize_list<'a, T, I, F1, F2, F3>(
    snippet_provider: SnippetProvider<'a>,
    inner: I,
    terminator: &'a str,
    separator: &'a str,
    get_lo: F1,
    get_hi: F2,
    get_item_string: F3,
    prev_span_end: BytePos,
    next_span_start: BytePos,
    leave_last: bool,
) -> ListItems<'a, I, F1, F2, F3>
where
    I: Iterator<Item = T>,
    F1: Fn(&T) -> BytePos,
    F2: Fn(&T) -> BytePos,
    F3: Fn(&T) -> Option<String>,
{
    ListItems {
        snippet_provider,
        inner: inner.peekable(),
        get_lo,
        get_hi,
        get_item_string,
        prev_span_end,
        next_span_start,
        terminator,
        separator,
        leave_last,
    }
}

/// Returns the count and total width of the list items.
#[cfg(test)]
fn calculate_width<I, T>(items: I) -> (usize, usize)
where
    I: IntoIterator<Item = T>,
    T: AsRef<ListItem>,
{
    items
        .into_iter()
        .map(|item| total_item_width(item.as_ref()))
        .fold((0, 0), |acc, l| (acc.0 + 1, acc.1 + l))
}

pub(crate) fn total_item_width(item: &ListItem) -> usize {
    comment_len(item.pre_comment.as_deref())
        + comment_len(item.post_comment.as_deref())
        + item.item.as_deref().map_or(0, unicode_str_width)
}

fn comment_len(comment: Option<&str>) -> usize {
    match comment {
        Some(s) => {
            let text_len = s.trim().len();
            if text_len > 0 {
                // We'll put " /*" before and " */" after inline comments.
                text_len + 6
            } else {
                text_len
            }
        }
        None => 0,
    }
}

/// Compute horizontal and vertical shapes for a struct-lit-like thing.
pub(crate) fn struct_lit_shape(
    shape: Shape,
    context: &RewriteContext<'_>,
    prefix_width: usize,
    suffix_width: usize,
) -> Option<(Option<Shape>, Shape)> {
    let v_shape = {
        let shape = shape.block_indent(context.config.tab_spaces());
        Shape {
            width: context.budget(shape.indent.width()),
            ..shape
        }
    };
    let h_shape = shape
        .width
        .checked_sub(prefix_width + suffix_width)
        .map(|w| {
            let shape_width = cmp::min(w, context.config.struct_lit_width());
            Shape::legacy(shape_width, shape.indent)
        });
    Some((h_shape, v_shape))
}

/// Compute the tactic for the internals of a struct-lit-like thing.
pub(crate) fn struct_lit_tactic(
    h_shape: Option<Shape>,
    context: &RewriteContext<'_>,
    items: &[ListItem],
) -> DefinitiveListTactic {
    if let Some(h_shape) = h_shape {
        let prelim_tactic = if context.config.struct_lit_single_line() {
            ListTactic::HorizontalVertical
        } else {
            ListTactic::Vertical
        };
        definitive_tactic(items, prelim_tactic, Separator::Comma, h_shape.width)
    } else {
        DefinitiveListTactic::Vertical
    }
}

/// Given a tactic and possible shapes for horizontal and vertical layout,
/// come up with the actual shape to use.
pub(crate) fn shape_for_tactic(
    tactic: DefinitiveListTactic,
    h_shape: Option<Shape>,
    v_shape: Shape,
) -> Shape {
    match tactic {
        DefinitiveListTactic::Horizontal => h_shape.unwrap_or(v_shape),
        _ => v_shape,
    }
}

/// Create a ListFormatting object for formatting the internals of a
/// struct-lit-like thing, that is a series of fields.
pub(crate) fn struct_lit_formatting<'a>(
    shape: Shape,
    tactic: DefinitiveListTactic,
    context: &'a RewriteContext<'_>,
    force_no_trailing_comma: bool,
) -> ListFormatting<'a> {
    let ends_with_newline = tactic == DefinitiveListTactic::Vertical;
    ListFormatting {
        tactic,
        separator: ",",
        trailing_separator: if force_no_trailing_comma {
            SeparatorTactic::Never
        } else {
            context.config.trailing_comma()
        },
        separator_place: SeparatorPlace::Back,
        shape,
        ends_with_newline,
        preserve_newline: true,
        nested: false,
        align_comments: true,
        config: context.config,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// rustfmt's `definitive_tactic`: the sum of every width, then the newline test.
    fn definitive_tactic_reference(
        items: &[ListItem],
        tactic: ListTactic,
        sep: Separator,
        width: usize,
    ) -> DefinitiveListTactic {
        let pre_line_comments = items.iter().any(|item| item.has_single_line_comment());
        let limit = match tactic {
            _ if pre_line_comments => return DefinitiveListTactic::Vertical,
            ListTactic::Horizontal => return DefinitiveListTactic::Horizontal,
            ListTactic::Vertical => return DefinitiveListTactic::Vertical,
            ListTactic::LimitedHorizontalVertical(limit) => cmp::min(width, limit),
            ListTactic::Mixed | ListTactic::HorizontalVertical => width,
        };
        let (sep_count, total_width) = calculate_width(items);
        let real_total = total_width + sep.len() * sep_count.saturating_sub(1);
        if real_total <= limit && !items.iter().any(|item| item.is_multiline()) {
            DefinitiveListTactic::Horizontal
        } else {
            match tactic {
                ListTactic::Mixed => DefinitiveListTactic::Mixed,
                _ => DefinitiveListTactic::Vertical,
            }
        }
    }

    /// `definitive_tactic`'s early exits answer as the full sum does, on every list of up to
    /// three items over items of several widths, multi-line and non-ASCII items and items
    /// with block and line comments, at every tactic, both separators and every width up
    /// to 40.
    #[test]
    fn definitive_tactic_matches_the_full_sum() {
        let mut kinds: Vec<ListItem> = [
            "",
            "a",
            "abcd",
            "abcdefghijklmnop",
            "x\ny",
            "\u{e9}\u{3000}z",
            "a\r\nb",
        ]
        .iter()
        .map(|s| ListItem::from_str(*s))
        .collect();
        kinds.push(ListItem::from_item(None));
        let mut commented = ListItem::from_str("abc");
        commented.post_comment = Some("/* note */".to_owned());
        kinds.push(commented);
        let mut line_comment = ListItem::from_str("ab");
        line_comment.pre_comment = Some("// first".to_owned());
        kinds.push(line_comment);
        let mut multi_comment = ListItem::from_str("ab");
        multi_comment.post_comment = Some("/* a\nb */".to_owned());
        kinds.push(multi_comment);
        let mut lists: Vec<Vec<ListItem>> = vec![vec![]];
        for len in 1..=3 {
            let mut next = Vec::new();
            for list in lists.iter().filter(|l| l.len() == len - 1) {
                for kind in &kinds {
                    let mut list = list.clone();
                    list.push(kind.clone());
                    next.push(list);
                }
            }
            lists.extend(next);
        }
        let tactics = [
            ListTactic::Vertical,
            ListTactic::Horizontal,
            ListTactic::HorizontalVertical,
            ListTactic::LimitedHorizontalVertical(10),
            ListTactic::LimitedHorizontalVertical(25),
            ListTactic::Mixed,
        ];
        for list in &lists {
            for tactic in tactics {
                for sep in [Separator::Comma, Separator::VerticalBar] {
                    for width in 0..=40 {
                        assert_eq!(
                            definitive_tactic(list.as_slice(), tactic, sep, width),
                            definitive_tactic_reference(list, tactic, sep, width),
                            "{:?} {tactic:?} {sep:?} {width}",
                            list.iter().map(|i| i.item.clone()).collect::<Vec<_>>()
                        );
                    }
                }
            }
        }
    }

    /// Every string of up to five characters over the bytes that drive the list-gap
    /// functions, and a few with `\r`, `\x0B` and non-ASCII characters.
    fn gap_strings() -> Vec<String> {
        const ALPHABET: [char; 13] = [
            ',', ';', ')', ']', '}', ' ', '\n', '/', '*', '"', '\'', 'r', '#',
        ];
        let mut strings: Vec<String> = [
            ", // c\n",
            ",\n\n    ",
            ",\r\n\r\n",
            ",\n\x0B\n",
            ",\n\x0C \n",
            ",\n\u{a0}\n",
            ",\n\u{3000}\n x",
            "é,\n\n",
            ", é\n",
            ",é",
            "/* a */,\n",
            ", r#\"a,\"#\n",
            ", 'a',\n",
            " r#, \n",
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
        strings
    }

    #[test]
    fn list_gap_fast_paths_match_the_full_functions() {
        let patterns = [(",", ")"), (";", "}"), ("|", "}"), (",", ""), (",", "|")];
        for s in &gap_strings() {
            assert_eq!(extract_pre_comment(s), extract_pre_comment_full(s), "{s:?}");
            for (separator, terminator) in patterns {
                for is_last in [false, true] {
                    assert_eq!(
                        get_comment_end(s, separator, terminator, is_last),
                        get_comment_end_full(s, separator, terminator, is_last),
                        "{s:?} {separator:?} {terminator:?} {is_last}"
                    );
                }
            }
            for comment_end in (0..=s.len()).filter(|&i| s.is_char_boundary(i)) {
                assert_eq!(
                    has_extra_newline(s, comment_end),
                    has_extra_newline_full(s, comment_end),
                    "{s:?} {comment_end}"
                );
                for is_last in [false, true] {
                    assert_eq!(
                        extract_post_comment(s, comment_end, ",", is_last),
                        extract_post_comment_full(s, comment_end, ",", is_last),
                        "{s:?} {comment_end} {is_last}"
                    );
                }
            }
        }
    }
}
