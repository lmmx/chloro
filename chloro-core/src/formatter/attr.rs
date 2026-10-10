//! Format attributes and meta items (rustfmt's `attr.rs`).
//!
//! rustc parses an attribute's arguments into a *meta item* tree when they have the shape
//! `path`, `path = literal` or `path(meta, ...)`; only such attributes are reformatted.
//! rust-analyzer keeps attribute arguments as an unstructured token tree, so this module
//! includes a small parser that rebuilds rustc's meta item tree from it.

use ra_ap_syntax::ast::{self, AstNode};
use ra_ap_syntax::{NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken};

use super::comment::{contains_comment, recover_missing_comment_in_span, rewrite_doc_comment};
use super::context::{Rewrite, RewriteContext};
use super::expr::{LitKind, rewrite_literal, span_ends_with_comma};
use super::lists::{
    ListFormatting, ListTactic, Separator, SeparatorTactic, definitive_tactic, itemize_list,
    write_list,
};
use super::nodes::{AttrStyle, Attribute, child};
use super::overflow;
use super::shape::Shape;
use super::span::{Span, Spanned, mk_sp, node_range_span, token_span};
use super::utils::count_newlines;

/// A literal inside a meta item.
#[derive(Clone, Debug)]
pub(crate) struct MetaLit {
    pub(crate) kind: LitKind,
    pub(crate) span: Span,
}

#[derive(Clone, Debug)]
pub(crate) enum MetaItemKind {
    Word,
    List(Vec<MetaItemInner>),
    NameValue(MetaLit),
}

#[derive(Clone, Debug)]
pub(crate) struct MetaItem {
    /// The path with insignificant whitespace removed, e.g. `rustfmt::skip`.
    pub(crate) path: String,
    pub(crate) kind: MetaItemKind,
    pub(crate) span: Span,
}

#[derive(Clone, Debug)]
pub(crate) enum MetaItemInner {
    MetaItem(MetaItem),
    Lit(MetaLit),
}

impl MetaItemInner {
    pub(crate) fn is_word(&self) -> bool {
        matches!(
            self,
            MetaItemInner::MetaItem(MetaItem {
                kind: MetaItemKind::Word,
                ..
            })
        )
    }

    pub(crate) fn is_lit(&self) -> bool {
        matches!(self, MetaItemInner::Lit(_))
    }
}

impl Spanned for MetaItemInner {
    fn span(&self) -> Span {
        match self {
            MetaItemInner::MetaItem(m) => m.span,
            MetaItemInner::Lit(l) => l.span,
        }
    }
}

fn lit_kind(kind: SyntaxKind) -> Option<LitKind> {
    Some(match kind {
        SyntaxKind::STRING => LitKind::Str,
        SyntaxKind::INT_NUMBER => LitKind::Integer,
        SyntaxKind::FLOAT_NUMBER => LitKind::Float,
        SyntaxKind::CHAR
        | SyntaxKind::BYTE
        | SyntaxKind::BYTE_STRING
        | SyntaxKind::C_STRING
        | SyntaxKind::TRUE_KW
        | SyntaxKind::FALSE_KW => LitKind::Other,
        _ => return None,
    })
}

/// A cursor over the significant elements of a token tree.
struct MetaParser {
    elems: Vec<NodeOrToken<SyntaxNode, SyntaxToken>>,
    pos: usize,
}

impl MetaParser {
    /// Elements strictly between the delimiters of a `TOKEN_TREE`.
    fn new(tt: &SyntaxNode) -> Option<MetaParser> {
        let children: Vec<_> = tt
            .children_with_tokens()
            .filter(|e| !matches!(e, NodeOrToken::Token(t) if t.kind() == SyntaxKind::WHITESPACE))
            .collect();
        let first = children.first()?.as_token()?;
        if first.kind() != SyntaxKind::L_PAREN {
            return None;
        }
        let inner = children.get(1..children.len().checked_sub(1)?)?.to_vec();
        Some(MetaParser {
            elems: inner,
            pos: 0,
        })
    }

    fn peek(&self) -> Option<&NodeOrToken<SyntaxNode, SyntaxToken>> {
        self.elems.get(self.pos)
    }

    fn peek_kind(&self) -> Option<SyntaxKind> {
        self.peek().map(|e| e.kind())
    }

    fn bump(&mut self) -> Option<NodeOrToken<SyntaxNode, SyntaxToken>> {
        let e = self.elems.get(self.pos).cloned();
        self.pos += 1;
        e
    }

    fn at_end(&self) -> bool {
        self.pos >= self.elems.len()
    }

    /// `Lit | Path ['=' Lit | '(' list ')']`
    fn parse_inner(&mut self) -> Option<MetaItemInner> {
        let first = self.peek()?.as_token()?.clone();
        if let Some(kind) = lit_kind(first.kind()) {
            self.bump();
            return Some(MetaItemInner::Lit(MetaLit {
                kind,
                span: token_span(&first),
            }));
        }
        let lo = token_span(&first).lo();
        let mut path = String::new();
        let mut hi;
        if first.kind() == SyntaxKind::COLON {
            self.eat_path_sep(&mut path)?;
        }
        loop {
            let seg = self.bump()?.into_token()?;
            if !(seg.kind() == SyntaxKind::IDENT
                || seg.kind().is_keyword(ra_ap_syntax::Edition::CURRENT))
            {
                return None;
            }
            path.push_str(seg.text());
            hi = token_span(&seg).hi();
            if self.peek_kind() == Some(SyntaxKind::COLON) {
                self.eat_path_sep(&mut path)?;
            } else {
                break;
            }
        }
        let kind = match self.peek() {
            Some(NodeOrToken::Token(t)) if t.kind() == SyntaxKind::EQ => {
                self.bump();
                let lit = self.bump()?.into_token()?;
                let kind = lit_kind(lit.kind())?;
                hi = token_span(&lit).hi();
                MetaItemKind::NameValue(MetaLit {
                    kind,
                    span: token_span(&lit),
                })
            }
            Some(NodeOrToken::Node(n)) if n.kind() == SyntaxKind::TOKEN_TREE => {
                let n = n.clone();
                self.bump();
                hi = node_range_span(&n).hi();
                MetaItemKind::List(parse_meta_list(&n)?)
            }
            _ => MetaItemKind::Word,
        };
        Some(MetaItemInner::MetaItem(MetaItem {
            path,
            kind,
            span: mk_sp(lo, hi),
        }))
    }

    /// Consumes a `::` (two joint `:` tokens).
    fn eat_path_sep(&mut self, path: &mut String) -> Option<()> {
        for _ in 0..2 {
            let t = self.bump()?.into_token()?;
            if t.kind() != SyntaxKind::COLON {
                return None;
            }
        }
        path.push_str("::");
        Some(())
    }
}

/// Parses the comma-separated contents of a parenthesised token tree as meta items.
fn parse_meta_list(tt: &SyntaxNode) -> Option<Vec<MetaItemInner>> {
    let mut parser = MetaParser::new(tt)?;
    let mut items = Vec::new();
    while !parser.at_end() {
        items.push(parser.parse_inner()?);
        match parser.bump() {
            None => break,
            Some(NodeOrToken::Token(t)) if t.kind() == SyntaxKind::COMMA => {}
            Some(_) => return None,
        }
    }
    Some(items)
}

/// rustc's `Attribute::meta()`: the attribute as a meta item, if its arguments have the
/// required shape.
pub(crate) fn attr_meta(attr: &ast::Attr) -> Option<MetaItem> {
    let meta = child::<ast::Meta>(attr.syntax())?;
    let path_node = meta.path()?;
    let path: String = path_node
        .syntax()
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
        .map(|t| t.text().to_string())
        .collect();
    let kind = if let Some(tt) = meta.token_tree() {
        MetaItemKind::List(parse_meta_list(tt.syntax())?)
    } else if let Some(expr) = meta.expr() {
        let ast::Expr::Literal(lit) = expr else {
            return None;
        };
        let token = lit.token();
        let kind = lit_kind(token.kind())?;
        MetaItemKind::NameValue(MetaLit {
            kind,
            span: token_span(&token),
        })
    } else {
        MetaItemKind::Word
    };
    // rustc gives an attribute's meta item the span of the whole attribute, `#[...]`
    // included. This is observable: `span_ends_with_comma` never sees a trailing comma
    // of a top-level list, which is why rustfmt removes it.
    Some(MetaItem {
        path,
        kind,
        span: node_range_span(attr.syntax()),
    })
}

/// Returns the meta items listed inside a list attribute such as `#[derive(A, B)]`.
fn meta_item_list(attr: &ast::Attr) -> Option<Vec<MetaItemInner>> {
    match attr_meta(attr)?.kind {
        MetaItemKind::List(list) => Some(list),
        _ => None,
    }
}

impl Rewrite for MetaItemInner {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        match self {
            MetaItemInner::MetaItem(meta_item) => meta_item.rewrite(context, shape),
            MetaItemInner::Lit(l) => rewrite_literal(context, l.kind, l.span, shape),
        }
    }
}

impl Rewrite for MetaItem {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        Some(match self.kind {
            MetaItemKind::Word => self.path.clone(),
            MetaItemKind::List(ref list) => {
                let has_trailing_comma = span_ends_with_comma(context, self.span);
                overflow::rewrite_with_parens(
                    context,
                    &self.path,
                    list.iter()
                        .cloned()
                        .map(overflow::OverflowableItem::MetaItemInner),
                    // 1 = "]"
                    shape.sub_width(1)?,
                    self.span,
                    context.config.attr_fn_like_width(),
                    Some(if has_trailing_comma {
                        SeparatorTactic::Always
                    } else {
                        SeparatorTactic::Never
                    }),
                )?
            }
            MetaItemKind::NameValue(ref lit) => {
                // 3 = ` = `
                let lit_shape = shape.shrink_left(self.path.len() + 3)?;
                // `rewrite_literal` returns `None` when `lit` exceeds max
                // width. Since a literal is basically unformattable unless it
                // is a string literal (and only if `format_strings` is set),
                // we might be better off ignoring the fact that the attribute
                // is longer than the max width and continue on formatting.
                let value = rewrite_literal(context, lit.kind, lit.span, lit_shape)
                    .unwrap_or_else(|| context.snippet(lit.span).to_owned());
                format!("{} = {}", self.path, value)
            }
        })
    }
}

/// The shape of the arguments to a function-like attribute.
fn argument_shape(
    left: usize,
    combine: bool,
    shape: Shape,
    context: &RewriteContext<'_>,
) -> Option<Shape> {
    if combine {
        shape.offset_left(left)
    } else {
        Some(
            shape
                .block_indent(context.config.tab_spaces())
                .with_max_width(context.config),
        )
    }
}

fn attr_prefix(attr: &Attribute) -> &'static str {
    match attr.style() {
        AttrStyle::Inner => "#!",
        AttrStyle::Outer => "#",
    }
}

fn format_derive(
    derives: &[Attribute],
    shape: Shape,
    context: &RewriteContext<'_>,
) -> Option<String> {
    // Collect all items from all attributes
    let mut all_items = Vec::new();
    for attr in derives {
        let a = attr.as_attr()?;
        // If any attribute is not parseable, none of the attributes will be reformatted.
        let spans: Vec<Span> = meta_item_list(a)?.iter().map(Spanned::span).collect();
        let attr_span = attr.span();
        all_items.extend(itemize_list(
            context.snippet_provider,
            spans.into_iter(),
            ")",
            ",",
            |span| span.lo(),
            |span| span.hi(),
            |span| Some(context.snippet(*span).to_owned()),
            // We update derive attribute spans to start after the opening '('
            // This helps us focus parsing to just what's inside #[derive(...)]
            context.snippet_provider.span_after(attr_span, "("),
            attr_span.hi(),
            false,
        ));
    }

    // Collect formatting parameters.
    let prefix = attr_prefix(&derives[0]);
    let argument_shape = argument_shape("[derive()]".len() + prefix.len(), false, shape, context)?;
    let one_line_shape = shape
        .offset_left("[derive()]".len() + prefix.len())?
        .sub_width("()]".len())?;
    let one_line_budget = one_line_shape.width;

    let tactic = definitive_tactic(
        &all_items,
        ListTactic::HorizontalVertical,
        Separator::Comma,
        argument_shape.width,
    );
    // We always add the trailing comma and remove it if it is not needed.
    let trailing_separator = SeparatorTactic::Always;

    // Format the collection of items.
    let fmt = ListFormatting::new(argument_shape, context.config)
        .tactic(tactic)
        .trailing_separator(trailing_separator)
        .ends_with_newline(false);
    let item_str = write_list(&all_items, &fmt)?;

    // Determine if the result will be nested, i.e. if the items are on multiple lines or we've
    // exceeded our budget to fit on a single line.
    let nested = item_str.contains('\n') || item_str.len() > one_line_budget;

    // Format the final result.
    let mut result = String::with_capacity(128);
    result.push_str(prefix);
    result.push_str("[derive(");
    if nested {
        let nested_indent = argument_shape.indent.to_string_with_newline(context.config);
        result.push_str(&nested_indent);
        result.push_str(&item_str);
        result.push_str(&shape.indent.to_string_with_newline(context.config));
    } else if let SeparatorTactic::Always = context.config.trailing_comma() {
        // Retain the trailing comma.
        result.push_str(&item_str);
    } else if let Some(stripped) = item_str.strip_suffix(',') {
        // Remove the trailing comma.
        result.push_str(stripped);
    } else {
        result.push_str(&item_str);
    }
    result.push_str(")]");

    Some(result)
}

/// Returns the first group of attributes that fills the given predicate.
/// We consider two doc comments are in different group if they are separated by normal comments.
fn take_while_with_pred<'a, P>(
    context: &RewriteContext<'_>,
    attrs: &'a [Attribute],
    pred: P,
) -> &'a [Attribute]
where
    P: Fn(&Attribute) -> bool,
{
    let mut len = 0;
    let mut iter = attrs.iter().peekable();

    while let Some(attr) = iter.next() {
        if pred(attr) {
            len += 1;
        } else {
            break;
        }
        if let Some(next_attr) = iter.peek() {
            // Extract comments between two attributes.
            let span_between_attr = mk_sp(attr.span().hi(), next_attr.span().lo());
            let snippet = context.snippet(span_between_attr);
            if count_newlines(snippet) >= 2 || snippet.contains('/') {
                break;
            }
        }
    }

    &attrs[..len]
}

/// Rewrite the any doc comments which come before any other attributes.
fn rewrite_initial_doc_comments(
    context: &RewriteContext<'_>,
    attrs: &[Attribute],
    shape: Shape,
) -> Option<(usize, Option<String>)> {
    if attrs.is_empty() {
        return Some((0, None));
    }
    // Rewrite doc comments
    let sugared_docs = take_while_with_pred(context, attrs, Attribute::is_doc_comment);
    if !sugared_docs.is_empty() {
        let snippet = sugared_docs
            .iter()
            .map(|a| context.snippet(a.span()))
            .collect::<Vec<_>>()
            .join("\n");
        return Some((
            sugared_docs.len(),
            Some(rewrite_doc_comment(
                &snippet,
                shape.comment(context.config),
                context.config,
            )?),
        ));
    }

    Some((0, None))
}

fn has_newlines_before_after_comment(comment: &str) -> (&str, &str) {
    // Look at before and after comment and see if there are any empty lines.
    let comment_begin = comment.find('/');
    let len = comment_begin.unwrap_or(comment.len());
    let mlb = count_newlines(&comment[..len]) > 1;
    let mla = if comment_begin.is_none() {
        mlb
    } else {
        comment
            .chars()
            .rev()
            .take_while(|c| c.is_whitespace())
            .filter(|&c| c == '\n')
            .count()
            > 1
    };
    (if mlb { "\n" } else { "" }, if mla { "\n" } else { "" })
}

impl Rewrite for Attribute {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        let snippet = context.snippet(self.span());
        let attr = match self {
            Attribute::Doc(_) => {
                return rewrite_doc_comment(snippet, shape.comment(context.config), context.config);
            }
            Attribute::Normal(attr) => attr,
        };
        let prefix = attr_prefix(self);

        if contains_comment(snippet) {
            return Some(snippet.to_owned());
        }

        let Some(meta) = attr_meta(attr) else {
            return Some(snippet.to_owned());
        };
        debug_assert!(!context.config.normalize_doc_attributes());

        // 1 = `[`
        let shape = shape.offset_left(prefix.len() + 1)?;
        let is_unsafe = child::<ast::Meta>(attr.syntax())
            .and_then(|m| m.unsafe_token())
            .is_some();
        Some(match meta.rewrite(context, shape) {
            Some(rw) if is_unsafe => format!("{prefix}[unsafe({rw})]"),
            Some(rw) => format!("{prefix}[{rw}]"),
            None => snippet.to_owned(),
        })
    }
}

/// Rewrites a run of attributes, merging derives and grouping doc comments as rustfmt does.
pub(crate) fn rewrite_attrs(
    attrs: &[Attribute],
    context: &RewriteContext<'_>,
    shape: Shape,
) -> Option<String> {
    if attrs.is_empty() {
        return Some(String::new());
    }

    // The current remaining attributes.
    let mut attrs = attrs;
    let mut result = String::new();

    // This is not just a simple map because we need to handle doc comments
    // (where we take as many doc comment attributes as possible) and possibly
    // merging derives into a single attribute.
    loop {
        if attrs.is_empty() {
            return Some(result);
        }

        // Handle doc comments.
        let (doc_comment_len, doc_comment_str) =
            rewrite_initial_doc_comments(context, attrs, shape)?;
        if doc_comment_len > 0 {
            result.push_str(&doc_comment_str?);

            let missing_span = attrs
                .get(doc_comment_len)
                .map(|next| mk_sp(attrs[doc_comment_len - 1].span().hi(), next.span().lo()));
            if let Some(missing_span) = missing_span {
                let snippet = context.snippet(missing_span);
                let (mla, mlb) = has_newlines_before_after_comment(snippet);
                let comment = recover_missing_comment_in_span(
                    missing_span,
                    shape.with_max_width(context.config),
                    context,
                    0,
                )?;
                let comment = if comment.is_empty() {
                    format!("\n{mlb}")
                } else {
                    format!("{mla}{comment}\n{mlb}")
                };
                result.push_str(&comment);
                result.push_str(&shape.indent.to_string(context.config));
            }

            attrs = &attrs[doc_comment_len..];

            continue;
        }

        // Handle derives if we will merge them.
        let is_derive = |a: &Attribute| a.has_name("derive");
        if context.config.merge_derives() && is_derive(&attrs[0]) {
            let derives = take_while_with_pred(context, attrs, is_derive);
            let derive_str = format_derive(derives, shape, context)?;
            result.push_str(&derive_str);

            let missing_span = attrs
                .get(derives.len())
                .map(|next| mk_sp(attrs[derives.len() - 1].span().hi(), next.span().lo()));
            if let Some(missing_span) = missing_span {
                let comment = recover_missing_comment_in_span(
                    missing_span,
                    shape.with_max_width(context.config),
                    context,
                    0,
                )?;
                result.push_str(&comment);
                if let Some(next) = attrs.get(derives.len())
                    && next.is_doc_comment()
                {
                    let snippet = context.snippet(missing_span);
                    let (_, mlb) = has_newlines_before_after_comment(snippet);
                    result.push_str(mlb);
                }
                result.push('\n');
                result.push_str(&shape.indent.to_string(context.config));
            }

            attrs = &attrs[derives.len()..];

            continue;
        }

        // If we get here, then we have a regular attribute, just handle one at a time.
        let formatted_attr = attrs[0].rewrite(context, shape)?;
        result.push_str(&formatted_attr);

        let missing_span = attrs
            .get(1)
            .map(|next| mk_sp(attrs[0].span().hi(), next.span().lo()));
        if let Some(missing_span) = missing_span {
            let comment = recover_missing_comment_in_span(
                missing_span,
                shape.with_max_width(context.config),
                context,
                0,
            )?;
            result.push_str(&comment);
            if let Some(next) = attrs.get(1)
                && next.is_doc_comment()
            {
                let snippet = context.snippet(missing_span);
                let (_, mlb) = has_newlines_before_after_comment(snippet);
                result.push_str(mlb);
            }
            result.push('\n');
            result.push_str(&shape.indent.to_string(context.config));
        }

        attrs = &attrs[1..];
    }
}

impl Rewrite for [Attribute] {
    fn rewrite(&self, context: &RewriteContext<'_>, shape: Shape) -> Option<String> {
        rewrite_attrs(self, context, shape)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_ap_syntax::{Edition, SourceFile};

    fn first_attr(src: &str) -> ast::Attr {
        SourceFile::parse(src, Edition::CURRENT)
            .tree()
            .syntax()
            .descendants()
            .find_map(ast::Attr::cast)
            .unwrap()
    }

    #[test]
    fn meta_items_parse_like_rustc() {
        let meta = attr_meta(&first_attr(
            "#[cfg_attr(feature = \"x\", derive(Debug, ::a::B), true)] fn f() {}",
        ))
        .unwrap();
        assert_eq!(meta.path, "cfg_attr");
        let MetaItemKind::List(list) = meta.kind else {
            panic!("expected a list");
        };
        assert_eq!(list.len(), 3);
        assert!(list[2].is_lit());
        let MetaItemInner::MetaItem(ref derive) = list[1] else {
            panic!("expected a meta item");
        };
        let MetaItemKind::List(ref derives) = derive.kind else {
            panic!("expected a list");
        };
        assert!(matches!(&derives[1], MetaItemInner::MetaItem(m) if m.path == "::a::B"));
    }

    #[test]
    fn non_literal_values_are_not_meta_items() {
        assert!(attr_meta(&first_attr("#[path = -1] fn f() {}")).is_none());
        assert!(attr_meta(&first_attr("#[doc = concat!(\"a\")] fn f() {}")).is_none());
        assert!(attr_meta(&first_attr("#[foo(a + b)] fn f() {}")).is_none());
    }
}
