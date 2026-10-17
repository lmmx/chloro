//! Headers, sets of consecutive keywords and tokens such as `macro_rules! name` (rustfmt
//! 1.10.0's `header.rs`): placed on a single line, except where a comment stands between
//! two parts, which is kept as written.

use std::borrow::Cow;

use super::comment::{combine_strs_with_missing_comments, contains_comment};
use super::context::RewriteContext;
use super::shape::Shape;
use super::span::Span;

pub(crate) fn format_header(
    context: &RewriteContext<'_>,
    shape: Shape,
    parts: Vec<HeaderPart<'_>>,
) -> String {
    let shape = shape.infinite_width();

    // Empty `HeaderPart`s are ignored.
    let mut parts = parts.into_iter().filter(|x| !x.snippet.is_empty());
    let Some(part) = parts.next() else {
        return String::new();
    };

    let mut result = part.snippet.into_owned();
    let mut span = part.span;

    for part in parts {
        let comments_span = span.between(part.span);
        let comments_snippet = context.snippet(comments_span);
        result = if contains_comment(comments_snippet) {
            // rustfmt keeps the comments as written rather than reformatting them.
            format!("{result}{comments_snippet}{}", part.snippet)
        } else {
            combine_strs_with_missing_comments(
                context,
                &result,
                &part.snippet,
                comments_span,
                shape,
                true,
            )
            .unwrap_or_else(|| format!("{result} {}", part.snippet))
        };
        span = part.span;
    }

    result
}

#[derive(Debug)]
pub(crate) struct HeaderPart<'a> {
    /// snippet of this part without surrounding space
    snippet: Cow<'a, str>,
    span: Span,
}

impl<'a> HeaderPart<'a> {
    pub(crate) fn new(snippet: impl Into<Cow<'a, str>>, span: Span) -> Self {
        Self {
            snippet: snippet.into(),
            span,
        }
    }
}
