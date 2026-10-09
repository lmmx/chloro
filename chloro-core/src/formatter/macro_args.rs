//! Parsing of macro call arguments (rustfmt's `parse/macros`).
//!
//! rust-analyzer keeps the body of a macro call as an opaque token tree. rustfmt formats
//! a call such as `foo!(a + b, Vec<u8>)` by running rustc's parser over the tokens and
//! taking each comma-separated argument as an expression, a type, a pattern or an item,
//! in that order of preference.
//!
//! The same is done here with rust-analyzer's parser:
//!
//! 1. The token tree interior is lexed once into a list of non-trivia tokens.
//! 2. Each argument is parsed with a [`PrefixEntryPoint`] over a window of tokens ending
//!    at the next top-level `,` or `;`. A parse that fails at the end of the window is
//!    retried with the window extended to the following separator, since a separator can
//!    belong to the argument (`|a, b| a + b`, `HashMap<K, V>`).
//! 3. The consumed tokens are parsed again with the matching [`TopEntryPoint`] to build a
//!    green node, and all argument nodes are spliced into a copy of the token tree.
//!
//! Splicing keeps every argument node at its original text offset, so spans, snippets
//! and comment recovery work on macro arguments exactly as on the rest of the file.

use ra_ap_parser::{Edition, LexedStr, PrefixEntryPoint, Step, StrStep, TopEntryPoint};
use ra_ap_syntax::{
    AstNode, GreenNode, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxTreeBuilder, T, ast,
};

use super::context::RewriteContext;
use super::macros::MacroArg;
use super::rustc_compat::subtree_rejected_by_rustc;
use super::span::{BytePos, Span, mk_sp};

/// The arguments of a macro call, as rustfmt's `ParsedMacroArgs`.
#[derive(Clone)]
pub(crate) struct ParsedMacroArgs {
    pub(crate) vec_with_semi: bool,
    pub(crate) trailing_comma: bool,
    pub(crate) args: Vec<MacroArg>,
}

/// One entry of `lazy_static! { [vis] static ref NAME: TY = EXPR; ... }`.
pub(crate) struct LazyStatic {
    pub(crate) vis: String,
    pub(crate) name: String,
    pub(crate) ty: ast::Type,
    pub(crate) expr: ast::Expr,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FragmentKind {
    Expr,
    Ty,
    Pat,
    Item,
}

impl FragmentKind {
    fn prefix_entry(self) -> PrefixEntryPoint {
        match self {
            FragmentKind::Expr => PrefixEntryPoint::Expr,
            FragmentKind::Ty => PrefixEntryPoint::Ty,
            FragmentKind::Pat => PrefixEntryPoint::Pat,
            FragmentKind::Item => PrefixEntryPoint::Item,
        }
    }

    fn top_entry(self) -> TopEntryPoint {
        match self {
            FragmentKind::Expr => TopEntryPoint::Expr,
            FragmentKind::Ty => TopEntryPoint::Type,
            FragmentKind::Pat => TopEntryPoint::Pattern,
            FragmentKind::Item => TopEntryPoint::MacroItems,
        }
    }
}

#[derive(Clone, Copy)]
struct Tok {
    kind: SyntaxKind,
    /// Byte range relative to the token tree interior.
    lo: usize,
    hi: usize,
    /// `true` for `,` and `;` outside any nested delimiter.
    top_level_sep: bool,
}

/// A parsed fragment awaiting splicing: its token range and green node.
struct Fragment {
    kind: FragmentKind,
    lo: usize,
    hi: usize,
    green: GreenNode,
}

/// The token stream of one token tree interior.
struct MacroTokens<'a> {
    tt: ast::TokenTree,
    /// Text of the interior, between the delimiters.
    text: &'a str,
    /// Absolute offset of `text`.
    base: BytePos,
    toks: Vec<Tok>,
    edition: Edition,
}

impl<'a> MacroTokens<'a> {
    fn new(context: &RewriteContext<'a>, tt: &ast::TokenTree) -> Option<MacroTokens<'a>> {
        let (open, close) = delimiters(tt)?;
        let base = u32::from(open.text_range().end());
        let end = u32::from(close.text_range().start());
        let text = context.snippet(mk_sp(base, end));
        let edition = context.config.edition().to_ra();
        let lexed = LexedStr::new(edition, text);
        if lexed.errors().next().is_some() {
            return None;
        }
        let mut toks = Vec::with_capacity(lexed.len());
        let mut depth = 0usize;
        for i in 0..lexed.len() {
            let kind = lexed.kind(i);
            if kind.is_trivia() {
                continue;
            }
            match kind {
                T!['('] | T!['['] | T!['{'] => depth += 1,
                T![')'] | T![']'] | T!['}'] => depth = depth.saturating_sub(1),
                _ => {}
            }
            let range = lexed.text_range(i);
            toks.push(Tok {
                kind,
                lo: range.start,
                hi: range.end,
                top_level_sep: depth == 0 && matches!(kind, T![,] | T![;]),
            });
        }
        Some(MacroTokens {
            tt: tt.clone(),
            text,
            base,
            toks,
            edition,
        })
    }

    fn kind(&self, pos: usize) -> Option<SyntaxKind> {
        self.toks.get(pos).map(|t| t.kind)
    }

    fn span_of(&self, start: usize, end: usize) -> Span {
        let lo = self.toks[start].lo as BytePos;
        let hi = self.toks[end - 1].hi as BytePos;
        mk_sp(self.base + lo, self.base + hi)
    }

    /// Index of the first top-level separator at or after `from`, or the end.
    fn next_sep(&self, from: usize, limit: usize) -> usize {
        (from..limit)
            .find(|&i| self.toks[i].top_level_sep)
            .unwrap_or(limit)
    }

    /// Prefix-parses `kind` over tokens `start..end`, returning the number of tokens
    /// consumed and, if the parser reported an error, the token index where it did.
    fn prefix_parse(&self, kind: FragmentKind, start: usize, end: usize) -> (usize, Option<usize>) {
        let text = &self.text[self.toks[start].lo..self.toks[end - 1].hi];
        let lexed = LexedStr::new(self.edition, text);
        let input = lexed.to_input(self.edition);
        let output = kind.prefix_entry().parse(&input, self.edition);
        let mut consumed = 0usize;
        let mut error = None;
        for step in output.iter() {
            match step {
                Step::Token { n_input_tokens, .. } => consumed += n_input_tokens as usize,
                Step::FloatSplit { .. } => consumed += 1,
                Step::Error { .. } => {
                    error.get_or_insert(consumed);
                }
                Step::Enter { .. } | Step::Exit => {}
            }
        }
        (consumed, error)
    }

    /// Parses one `kind` fragment starting at token `start`, ending no later than `limit`.
    ///
    /// Emulates rustc's greedy parse over the remaining stream: success means the parser
    /// stopped without an error, wherever that is.
    fn parse_fragment(&self, kind: FragmentKind, start: usize, limit: usize) -> Option<Fragment> {
        if start >= limit {
            return None;
        }
        if kind == FragmentKind::Ty && self.kind(start) == Some(T![?]) {
            return self.parse_maybe_bound(start, limit);
        }
        let consumed = if kind == FragmentKind::Item {
            // Items contain top-level `;`, so they are parsed against the whole remainder.
            match self.prefix_parse(kind, start, limit) {
                (n, None) if n > 0 => n,
                _ => return None,
            }
        } else {
            let mut end = self.next_sep(start, limit);
            loop {
                if end == start {
                    return None;
                }
                let window = end - start;
                match self.prefix_parse(kind, start, end) {
                    (n, None) if n > 0 => break n,
                    (_, Some(at)) if at >= window && end < limit => {
                        // The window ended inside the fragment: include the separator.
                        end = self.next_sep(end + 1, limit);
                    }
                    _ => return None,
                }
            }
        };
        let end = start + consumed;
        let (lo, hi) = (self.toks[start].lo, self.toks[end - 1].hi);
        let green = self.build(kind, &self.text[lo..hi])?;
        Some(Fragment {
            kind,
            lo,
            hi,
            green,
        })
    }

    /// rustc's type grammar accepts `?Trait`, a trait object with a single "maybe" bound;
    /// rust-analyzer's does not. This matters for `tracing`'s `?value` arguments, which
    /// rustfmt formats as types. Builds the node rust-analyzer uses for a bare trait object.
    fn parse_maybe_bound(&self, start: usize, limit: usize) -> Option<Fragment> {
        let path = self.parse_fragment(FragmentKind::Ty, start + 1, limit)?;
        if SyntaxNode::new_root(path.green.clone()).kind() != SyntaxKind::PATH_TYPE {
            return None;
        }
        let gap = &self.text[self.toks[start].hi..path.lo];
        if !gap.trim().is_empty() {
            return None;
        }
        let lexed = LexedStr::new(self.edition, &self.text[path.lo..path.hi]);
        let input = lexed.to_input(self.edition);
        let output = TopEntryPoint::Type.parse(&input, self.edition);
        let mut builder = SyntaxTreeBuilder::default();
        builder.start_node(SyntaxKind::DYN_TRAIT_TYPE);
        builder.start_node(SyntaxKind::TYPE_BOUND_LIST);
        builder.start_node(SyntaxKind::TYPE_BOUND);
        builder.token(T![?], "?");
        if !gap.is_empty() {
            builder.token(SyntaxKind::WHITESPACE, gap);
        }
        let mut has_error = false;
        lexed.intersperse_trivia(&output, &mut |step| match step {
            StrStep::Token { kind, text } => builder.token(kind, text),
            StrStep::Enter { kind } => builder.start_node(kind),
            StrStep::Exit => builder.finish_node(),
            StrStep::Error { .. } => has_error = true,
        });
        builder.finish_node();
        builder.finish_node();
        builder.finish_node();
        if has_error {
            return None;
        }
        let node = builder.finish().syntax_node();
        Some(Fragment {
            kind: FragmentKind::Ty,
            lo: self.toks[start].lo,
            hi: path.hi,
            green: node.green().into_owned(),
        })
    }

    fn build(&self, kind: FragmentKind, text: &str) -> Option<GreenNode> {
        let lexed = LexedStr::new(self.edition, text);
        let input = lexed.to_input(self.edition);
        let output = kind.top_entry().parse(&input, self.edition);
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
        let root = builder.finish().syntax_node();
        let node = if kind == FragmentKind::Item {
            // `MacroItems` wraps the item; the fragment is the single item inside.
            let mut items = root.children();
            let item = items.next()?;
            if items.next().is_some() || item.text_range() != root.text_range() {
                return None;
            }
            item
        } else {
            root
        };
        if node.kind() == SyntaxKind::ERROR || subtree_rejected_by_rustc(&node, self.edition) {
            return None;
        }
        Some(node.green().into_owned())
    }

    /// Replaces the parsed ranges of the token tree with the fragment nodes, in a copy of
    /// the whole tree, and returns the fragment nodes of the copy.
    fn splice(&self, fragments: &[Fragment]) -> Option<Vec<SyntaxNode>> {
        if fragments.is_empty() {
            return Some(Vec::new());
        }
        let tt = self.tt.syntax();
        let base = self.base as usize;
        let mut children = Vec::new();
        let mut positions = Vec::with_capacity(fragments.len());
        let mut frags = fragments.iter().peekable();
        for child in tt.children_with_tokens() {
            let range = child.text_range();
            let (lo, hi) = (usize::from(range.start()), usize::from(range.end()));
            if let Some(frag) = frags.peek() {
                let (flo, fhi) = (base + frag.lo, base + frag.hi);
                if lo >= flo && hi <= fhi {
                    if lo == flo {
                        positions.push(children.len());
                        children.push(NodeOrToken::Node(frag.green.clone()));
                    }
                    if hi == fhi {
                        frags.next();
                    }
                    continue;
                }
                if lo < fhi && hi > flo {
                    // A tree element straddles a fragment boundary.
                    return None;
                }
            }
            children.push(match child {
                NodeOrToken::Node(n) => NodeOrToken::Node(n.green().into_owned()),
                NodeOrToken::Token(t) => NodeOrToken::Token(t.green().to_owned()),
            });
        }
        if frags.next().is_some() {
            return None;
        }
        let new_tt = GreenNode::new(tt.green().kind(), children);

        // The ancestors of the token tree below the root. The fragments cover the text they
        // replace, so every ancestor in the copy has the range and kind it had before; a
        // binary search on ranges finds it without creating a red node per sibling.
        let ancestors: Vec<(SyntaxKind, ra_ap_syntax::TextRange)> = tt
            .ancestors()
            .take_while(|node| node.parent().is_some())
            .map(|node| (node.kind(), node.text_range()))
            .collect();
        let root = SyntaxNode::new_root(tt.replace_with(new_tt));
        let mut new = root;
        for &(kind, range) in ancestors.iter().rev() {
            new = new.child_or_token_at_range(range)?.into_node()?;
            if new.kind() != kind || new.text_range() != range {
                return None;
            }
        }
        let elements: Vec<_> = new.children_with_tokens().collect();
        positions
            .iter()
            .map(|&i| elements.get(i)?.clone().into_node())
            .collect()
    }

    /// rustc's `check_keyword`: a reserved word followed by `,` or the end.
    fn keyword_at(&self, pos: usize) -> bool {
        self.kind(pos).is_some_and(|k| {
            (k.is_strict_keyword(self.edition) || k == T![_])
                && self.kind(pos + 1).is_none_or(|next| next == T![,])
        })
    }
}

fn delimiters(
    tt: &ast::TokenTree,
) -> Option<(ra_ap_syntax::SyntaxToken, ra_ap_syntax::SyntaxToken)> {
    let open = tt.syntax().first_token()?;
    let close = tt.syntax().last_token()?;
    let ok = matches!(
        (open.kind(), close.kind()),
        (T!['('], T![')']) | (T!['['], T![']']) | (T!['{'], T!['}'])
    );
    (ok && open.text_range().end() <= close.text_range().start()).then_some((open, close))
}

fn to_macro_arg(kind: FragmentKind, node: SyntaxNode) -> Option<MacroArg> {
    Some(match kind {
        FragmentKind::Expr => MacroArg::Expr(ast::Expr::cast(node)?),
        FragmentKind::Ty => MacroArg::Ty(ast::Type::cast(node)?),
        FragmentKind::Pat => MacroArg::Pat(ast::Pat::cast(node)?),
        FragmentKind::Item => MacroArg::Item(ast::Item::cast(node)?),
    })
}

/// Argument slots before splicing: a keyword or the index of a fragment.
enum Slot {
    Keyword(String, Span),
    Fragment(usize),
}

/// rustc's `parse_macro_arg`: the first of expression, type, pattern and item that parses.
fn parse_macro_arg(tokens: &MacroTokens<'_>, pos: usize) -> Option<Fragment> {
    let limit = tokens.toks.len();
    [
        FragmentKind::Expr,
        FragmentKind::Ty,
        FragmentKind::Pat,
        FragmentKind::Item,
    ]
    .into_iter()
    .find_map(|kind| tokens.parse_fragment(kind, pos, limit))
}

/// Parses the arguments of a macro call delimited by `(` or `[`.
pub(crate) fn parse_macro_args(
    context: &RewriteContext<'_>,
    tt: &ast::TokenTree,
    forced_bracket: bool,
) -> Option<ParsedMacroArgs> {
    let tokens = MacroTokens::new(context, tt)?;
    let mut slots = Vec::new();
    let mut fragments: Vec<Fragment> = Vec::new();
    let mut vec_with_semi = false;
    let mut trailing_comma = false;
    let mut pos = 0;
    let n = tokens.toks.len();

    loop {
        if tokens.keyword_at(pos) {
            let t = tokens.toks[pos];
            slots.push(Slot::Keyword(
                tokens.text[t.lo..t.hi].to_owned(),
                tokens.span_of(pos, pos + 1),
            ));
            pos += 1;
        } else {
            let frag = parse_macro_arg(&tokens, pos)?;
            pos = tokens.toks.partition_point(|t| t.hi <= frag.hi);
            slots.push(Slot::Fragment(fragments.len()));
            fragments.push(frag);
        }

        let last_is_item = matches!(
            slots.last(),
            Some(Slot::Fragment(i)) if fragments[*i].kind == FragmentKind::Item
        );
        match tokens.kind(pos) {
            None => break,
            Some(T![,]) => {}
            Some(T![;]) => {
                // Try to parse `vec![expr; expr]`
                if forced_bracket {
                    pos += 1;
                    if pos < n {
                        let frag = parse_macro_arg(&tokens, pos)?;
                        pos = tokens.toks.partition_point(|t| t.hi <= frag.hi);
                        slots.push(Slot::Fragment(fragments.len()));
                        fragments.push(frag);
                        if pos >= n && slots.len() == 2 {
                            vec_with_semi = true;
                            break;
                        }
                    }
                }
                return None;
            }
            Some(_) if last_is_item => continue,
            Some(_) => return None,
        }

        pos += 1;
        if pos >= n {
            trailing_comma = true;
            break;
        }
    }

    let mut nodes = tokens.splice(&fragments)?.into_iter();
    let mut fragment_args = Vec::with_capacity(fragments.len());
    for frag in &fragments {
        fragment_args.push(to_macro_arg(frag.kind, nodes.next()?)?);
    }
    let mut fragment_args = fragment_args.into_iter();
    let args = slots
        .into_iter()
        .map(|slot| match slot {
            Slot::Keyword(text, span) => Some(MacroArg::Keyword(text, span)),
            Slot::Fragment(_) => fragment_args.next(),
        })
        .collect::<Option<Vec<_>>>()?;

    Some(ParsedMacroArgs {
        vec_with_semi,
        trailing_comma,
        args,
    })
}

/// rustc's `parse_expr` over a whole token tree: the leading expression, ignoring any
/// tokens after it (used for `try!`).
pub(crate) fn parse_expr(context: &RewriteContext<'_>, tt: &ast::TokenTree) -> Option<ast::Expr> {
    let tokens = MacroTokens::new(context, tt)?;
    let n = tokens.toks.len();
    if n == 0 {
        return None;
    }
    let (consumed, error) = tokens.prefix_parse(FragmentKind::Expr, 0, n);
    if error.is_some() || consumed == 0 {
        return None;
    }
    let (lo, hi) = (tokens.toks[0].lo, tokens.toks[consumed - 1].hi);
    let green = tokens.build(FragmentKind::Expr, &tokens.text[lo..hi])?;
    let frag = Fragment {
        kind: FragmentKind::Expr,
        lo,
        hi,
        green,
    };
    ast::Expr::cast(tokens.splice(&[frag])?.pop()?)
}

/// Parses the body of `lazy_static!`.
pub(crate) fn parse_lazy_static(
    context: &RewriteContext<'_>,
    tt: &ast::TokenTree,
) -> Option<Vec<LazyStatic>> {
    let tokens = MacroTokens::new(context, tt)?;
    let n = tokens.toks.len();
    let mut entries = Vec::new();
    let mut fragments = Vec::new();
    let mut pos = 0;
    let eat = |pos: &mut usize, kind: SyntaxKind| {
        if tokens.kind(*pos) == Some(kind) {
            *pos += 1;
        }
    };
    while pos < n {
        let vis = if tokens.kind(pos) == Some(T![pub]) {
            let start = pos;
            pos += 1;
            if tokens.kind(pos) == Some(T!['(']) {
                // `pub(crate)`, `pub(super)`, `pub(in path)`
                let close = (pos..n).find(|&i| tokens.kind(i) == Some(T![')']))?;
                pos = close + 1;
            }
            visibility_text(&tokens, start, pos)
        } else {
            String::new()
        };
        eat(&mut pos, T![static]);
        eat(&mut pos, T![ref]);
        if tokens.kind(pos) != Some(SyntaxKind::IDENT) {
            return None;
        }
        let t = tokens.toks[pos];
        let name = tokens.text[t.lo..t.hi].to_owned();
        pos += 1;
        eat(&mut pos, T![:]);
        let limit = tokens.next_sep(pos, n);
        let ty_limit = (pos..limit)
            .find(|&i| tokens.kind(i) == Some(T![=]))
            .unwrap_or(limit);
        let ty = tokens.parse_fragment(FragmentKind::Ty, pos, ty_limit)?;
        pos = tokens.toks.partition_point(|t| t.hi <= ty.hi);
        eat(&mut pos, T![=]);
        let limit = tokens.next_sep(pos, n);
        let (consumed, error) = if pos < limit {
            tokens.prefix_parse(FragmentKind::Expr, pos, limit)
        } else {
            (0, None)
        };
        if error.is_some() || consumed == 0 {
            return None;
        }
        let (lo, hi) = (tokens.toks[pos].lo, tokens.toks[pos + consumed - 1].hi);
        let expr = Fragment {
            kind: FragmentKind::Expr,
            lo,
            hi,
            green: tokens.build(FragmentKind::Expr, &tokens.text[lo..hi])?,
        };
        pos += consumed;
        eat(&mut pos, T![;]);
        entries.push((vis, name));
        fragments.push(ty);
        fragments.push(expr);
    }

    let mut nodes = tokens.splice(&fragments)?.into_iter();
    entries
        .into_iter()
        .map(|(vis, name)| {
            let ty = ast::Type::cast(nodes.next()?)?;
            let expr = ast::Expr::cast(nodes.next()?)?;
            Some(LazyStatic {
                vis,
                name,
                ty,
                expr,
            })
        })
        .collect()
}

/// The visibility text with rustfmt's normalisation (`pub(in crate)` is `pub(crate)`),
/// followed by a space.
fn visibility_text(tokens: &MacroTokens<'_>, start: usize, end: usize) -> String {
    let words: Vec<&str> = (start..end)
        .map(|i| &tokens.text[tokens.toks[i].lo..tokens.toks[i].hi])
        .collect();
    match words.as_slice() {
        ["pub"] => "pub ".to_owned(),
        ["pub", "(", "in", rest @ .., ")"] => {
            let path = rest.concat();
            match path.as_str() {
                "crate" | "self" | "super" => format!("pub({path}) "),
                _ => format!("pub(in {path}) "),
            }
        }
        ["pub", "(", rest @ .., ")"] => format!("pub({}) ", rest.concat()),
        _ => words.concat() + " ",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatter::config::Settings;
    use crate::formatter::span::SnippetProvider;
    use ra_ap_syntax::SourceFile;

    fn with_macro<R>(src: &str, f: impl FnOnce(&RewriteContext<'_>, ast::TokenTree) -> R) -> R {
        let settings = Settings::default();
        let file = SourceFile::parse(src, ra_ap_syntax::Edition::Edition2024).tree();
        let call = file
            .syntax()
            .descendants()
            .find_map(ast::MacroCall::cast)
            .unwrap();
        let context = RewriteContext::new(&settings, SnippetProvider::new(src), Default::default());
        f(&context, call.token_tree().unwrap())
    }

    fn arg_kinds(args: &ParsedMacroArgs) -> Vec<&'static str> {
        args.args
            .iter()
            .map(|a| match a {
                MacroArg::Expr(_) => "expr",
                MacroArg::Ty(_) => "ty",
                MacroArg::Pat(_) => "pat",
                MacroArg::Item(_) => "item",
                MacroArg::Keyword(..) => "kw",
            })
            .collect()
    }

    #[test]
    fn args_keep_their_offsets() {
        let src = "fn f() { foo!(a + b, |x, y| x * y, Vec<u8>, self); }";
        with_macro(src, |ctx, tt| {
            let args = parse_macro_args(ctx, &tt, false).unwrap();
            assert_eq!(arg_kinds(&args), ["expr", "expr", "ty", "kw"]);
            let MacroArg::Expr(e) = &args.args[1] else {
                unreachable!()
            };
            let range = e.syntax().text_range();
            assert_eq!(
                &src[usize::from(range.start())..usize::from(range.end())],
                "|x, y| x * y"
            );
            assert!(!args.trailing_comma);
        });
    }

    #[test]
    fn trailing_comma_and_vec_semi() {
        with_macro("fn f() { foo!(a, b,); }", |ctx, tt| {
            let args = parse_macro_args(ctx, &tt, false).unwrap();
            assert!(args.trailing_comma);
            assert_eq!(args.args.len(), 2);
        });
        with_macro("fn f() { vec![0u8; 4]; }", |ctx, tt| {
            let args = parse_macro_args(ctx, &tt, true).unwrap();
            assert!(args.vec_with_semi);
        });
    }

    #[test]
    fn non_list_macro_is_rejected() {
        with_macro("fn f() { foo!(key => value); }", |ctx, tt| {
            assert!(parse_macro_args(ctx, &tt, false).is_none());
        });
    }

    #[test]
    fn items_need_no_separator() {
        with_macro("foo!(fn a() {} struct B;);", |ctx, tt| {
            let args = parse_macro_args(ctx, &tt, false).unwrap();
            assert_eq!(arg_kinds(&args), ["item", "item"]);
        });
    }

    #[test]
    fn maybe_bound_is_a_type() {
        // `tracing::info_span!("name", ?value)`: rustc parses `?value` as a type.
        with_macro("fn f() { foo!(\"a\", ?name); }", |ctx, tt| {
            let args = parse_macro_args(ctx, &tt, false).unwrap();
            assert_eq!(arg_kinds(&args), ["expr", "ty"]);
        });
    }

    #[test]
    fn lazy_static_entries() {
        let src =
            "lazy_static! { pub(crate) static ref A: u8 = 1; static ref B: Vec<u8> = vec![]; }";
        with_macro(src, |ctx, tt| {
            let entries = parse_lazy_static(ctx, &tt).unwrap();
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].vis, "pub(crate) ");
            assert_eq!(entries[1].name, "B");
            assert_eq!(entries[1].ty.syntax().text(), "Vec<u8>");
        });
    }
}
