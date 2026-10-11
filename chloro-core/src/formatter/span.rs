//! Byte spans into the source text, modelled on rustc's `Span`.
//!
//! rustfmt recovers comments, blank lines and unformatted code by slicing the original
//! source between AST nodes. rowan gives every node an exact `TextRange`, so the same
//! technique works here; the only adjustment is that rust-analyzer attaches leading
//! comments to the following item, whereas rustc spans start at the first outer attribute
//! (doc comments included). [`Spanned::span`] applies that adjustment.

use ra_ap_syntax::{
    AstNode, AstToken, NodeOrToken, RustLanguage, SyntaxKind, SyntaxNode, SyntaxToken, ast,
};
use rowan::Language;

use super::comment::FindUncommented;

pub(crate) type BytePos = u32;

/// A half-open byte range `lo..hi` into a source string.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Span {
    lo: BytePos,
    hi: BytePos,
}

impl Span {
    pub(crate) fn lo(self) -> BytePos {
        self.lo
    }

    pub(crate) fn hi(self) -> BytePos {
        self.hi
    }

    pub(crate) fn with_lo(self, lo: BytePos) -> Span {
        mk_sp(lo, self.hi)
    }

    pub(crate) fn with_hi(self, hi: BytePos) -> Span {
        mk_sp(self.lo, hi)
    }

    /// The span between the end of `self` and the start of `other`.
    pub(crate) fn between(self, other: Span) -> Span {
        mk_sp(self.hi, other.lo.max(self.hi))
    }
}

/// Creates a span; an inverted range is clamped to the empty span at `lo`.
pub(crate) fn mk_sp(lo: BytePos, hi: BytePos) -> Span {
    Span { lo, hi: hi.max(lo) }
}

pub(crate) fn token_span(token: &SyntaxToken) -> Span {
    let r = token.text_range();
    mk_sp(r.start().into(), r.end().into())
}

/// The full rowan range of a node, including attached leading comments.
pub(crate) fn node_range_span(node: &SyntaxNode) -> Span {
    let r = node.text_range();
    mk_sp(r.start().into(), r.end().into())
}

/// Returns `true` for comments that rustc parses as doc attributes (`///`, `/** */`).
pub(crate) fn is_outer_doc_comment(token: &SyntaxToken) -> bool {
    token.kind() == SyntaxKind::COMMENT
        && ast::Comment::cast(token.clone()).is_some_and(|c| c.is_outer() && c.kind().doc.is_some())
}

/// Returns `true` for comments that rustc parses as inner doc attributes (`//!`, `/*! */`).
pub(crate) fn is_inner_doc_comment(token: &SyntaxToken) -> bool {
    token.kind() == SyntaxKind::COMMENT
        && ast::Comment::cast(token.clone()).is_some_and(|c| c.is_inner() && c.kind().doc.is_some())
}

/// `true` for the text of an outer doc comment (`///`, `/** */`), with rust-analyzer's
/// classification: `////` and `/***` are plain comments.
pub(crate) fn is_outer_doc_text(text: &str) -> bool {
    (text.starts_with("///") && !text.starts_with("////"))
        || (text.starts_with("/**") && !text.starts_with("/***") && !text.starts_with("/**/"))
}

/// The rustc-style span of a node: leading trivia that rust-analyzer attached to the
/// node (plain comments and whitespace) is excluded.
///
/// Called for most nodes several times per rewrite, so it reads the green children
/// directly instead of creating a cursor node per child.
pub(crate) fn rustc_span(node: &SyntaxNode) -> Span {
    let full = node_range_span(node);
    let mut lo = full.lo();
    for child in node.green().children() {
        match child {
            NodeOrToken::Node(_) => return full.with_lo(lo),
            NodeOrToken::Token(t) => {
                let significant = match RustLanguage::kind_from_raw(t.kind()) {
                    SyntaxKind::WHITESPACE => false,
                    SyntaxKind::COMMENT => is_outer_doc_text(t.text()),
                    _ => true,
                };
                if significant {
                    return full.with_lo(lo);
                }
                lo += BytePos::from(t.text_len());
            }
        }
    }
    full
}

/// Span of a syntax element as rustfmt sees it.
pub(crate) trait Spanned {
    fn span(&self) -> Span;
}

impl<N: AstNode> Spanned for N {
    fn span(&self) -> Span {
        rustc_span(self.syntax())
    }
}

/// Slices of the source text addressed by [`Span`]s (rustfmt's `SnippetProvider` and
/// `SpanUtils`).
#[derive(Copy, Clone, Debug)]
pub(crate) struct SnippetProvider<'a> {
    src: &'a str,
}

impl<'a> SnippetProvider<'a> {
    pub(crate) fn new(src: &'a str) -> Self {
        SnippetProvider { src }
    }

    pub(crate) fn entire_snippet(&self) -> &'a str {
        self.src
    }

    pub(crate) fn end_pos(&self) -> BytePos {
        self.src.len() as BytePos
    }

    pub(crate) fn span_to_snippet(&self, span: Span) -> Option<&'a str> {
        self.src.get(span.lo as usize..span.hi as usize)
    }

    pub(crate) fn snippet(&self, span: Span) -> &'a str {
        self.span_to_snippet(span).unwrap_or("")
    }

    pub(crate) fn opt_span_before(&self, original: Span, needle: &str) -> Option<BytePos> {
        let snippet = self.span_to_snippet(original)?;
        let offset = snippet.find_uncommented(needle)?;
        Some(original.lo + offset as BytePos)
    }

    pub(crate) fn opt_span_after(&self, original: Span, needle: &str) -> Option<BytePos> {
        self.opt_span_before(original, needle)
            .map(|pos| pos + needle.len() as BytePos)
    }

    /// Position just after the first uncommented `needle` in `original`.
    ///
    /// rustfmt panics when the needle is absent; here the lookup falls back to the end of
    /// `original`, which only affects comment recovery around the needle.
    pub(crate) fn span_after(&self, original: Span, needle: &str) -> BytePos {
        let pos = self.opt_span_after(original, needle);
        debug_assert!(pos.is_some(), "`{needle}` not found in {original:?}");
        pos.unwrap_or(original.hi)
    }

    /// Position of the first uncommented `needle` in `original` (see [`Self::span_after`]).
    pub(crate) fn span_before(&self, original: Span, needle: &str) -> BytePos {
        let pos = self.opt_span_before(original, needle);
        debug_assert!(pos.is_some(), "`{needle}` not found in {original:?}");
        pos.unwrap_or(original.lo)
    }

    pub(crate) fn span_after_last(&self, original: Span, needle: &str) -> BytePos {
        let snippet = self.snippet(original);
        let mut offset = 0;
        while let Some(additional_offset) = snippet[offset..].find_uncommented(needle) {
            offset += additional_offset + needle.len();
        }
        original.lo + offset as BytePos
    }

    pub(crate) fn span_before_last(&self, original: Span, needle: &str) -> BytePos {
        let snippet = self.snippet(original);
        let mut offset = 0;
        while let Some(additional_offset) = snippet[offset..].find_uncommented(needle) {
            offset += additional_offset + needle.len();
        }
        (original.lo + offset as BytePos).saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_ap_syntax::ast::HasModuleItem;
    use ra_ap_syntax::{Edition, SourceFile};

    #[test]
    fn item_span_skips_plain_comments_but_keeps_doc_comments() {
        let src = "// plain\n/// doc\nfn f() {}\n";
        let file = SourceFile::parse(src, Edition::CURRENT).tree();
        let item = file.items().next().unwrap();
        let span = item.span();
        assert_eq!(
            &src[span.lo() as usize..span.hi() as usize],
            "/// doc\nfn f() {}"
        );
    }
}
