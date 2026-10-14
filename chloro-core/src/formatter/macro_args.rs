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
//! 3. The consumed tokens are parsed again with the matching [`TopEntryPoint`] into a tree
//!    of their own ([`Tree::from_steps_at`]), laid out at the argument's offset in the file.
//!
//! Every argument node therefore has its original text offset, so spans, snippets and
//! comment recovery work on macro arguments exactly as on the rest of the file. The
//! fragment trees are kept by the run ([`RunState::keep_tree`]) for as long as the
//! argument nodes are cached.

use ra_ap_parser::{Edition, LexedStr, PrefixEntryPoint, Step, SyntaxKind, T, TopEntryPoint};

use crate::cst::{AstNode, Node, Tree, TreeBuilder, ast};

use super::config::RustfmtVersion;
use super::context::{RewriteContext, RunState};
use super::macros::MacroArg;
use super::rustc_compat::{doc_comment_style, subtree_rejected_by_rustc};
use super::rustc_syntax::lexer_rejects;
use super::span::{BytePos, Span, mk_sp};

/// The arguments of a macro call, as rustfmt's `ParsedMacroArgs`.
#[derive(Clone)]
pub(crate) struct ParsedMacroArgs<'a> {
    pub(crate) vec_with_semi: bool,
    pub(crate) trailing_comma: bool,
    pub(crate) args: Vec<MacroArg<'a>>,
}

/// One entry of `lazy_static! { [vis] static ref NAME: TY = EXPR; ... }`.
pub(crate) struct LazyStatic<'a> {
    pub(crate) vis: String,
    pub(crate) name: String,
    pub(crate) ty: ast::Type<'a>,
    pub(crate) expr: ast::Expr<'a>,
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
    /// Start of the first doc comment between the previous token and this one. rustc's
    /// lexer makes a doc comment a token (`DocComment`); rust-analyzer's makes it trivia.
    doc_lo: Option<usize>,
}

/// A parsed fragment: its byte range in the interior and its node, laid out at its file
/// offset in a tree of its own.
struct Fragment<'a> {
    kind: FragmentKind,
    lo: usize,
    hi: usize,
    node: Node<'a>,
}

/// The token stream of one token tree interior. `'a` is the run (the fragment trees are
/// kept by it), `'t` the token tree's own tree.
struct MacroTokens<'a, 't> {
    tt: ast::TokenTree<'t>,
    /// The whole file: every fragment tree is laid out over it.
    file: &'a str,
    /// Text of the interior, between the delimiters.
    text: &'a str,
    /// Absolute offset of `text`.
    base: BytePos,
    toks: Vec<Tok>,
    /// Start of the first doc comment after the last token, if any.
    doc_at_end: Option<usize>,
    edition: Edition,
    rustfmt_version: RustfmtVersion,
    run: std::rc::Rc<RunState<'a>>,
}

impl<'a, 't> MacroTokens<'a, 't> {
    fn new(context: &RewriteContext<'a>, tt: &ast::TokenTree<'t>) -> Option<MacroTokens<'a, 't>> {
        let (open, close) = delimiters(tt)?;
        let base = open.text_range().end();
        let end = close.text_range().start();
        let text = context.snippet(mk_sp(base, end));
        let edition = context.config.edition().to_ra();
        let lexed = LexedStr::new(edition, text);
        if lexer_rejects(&lexed, edition) {
            return None;
        }
        let mut toks = Vec::with_capacity(lexed.len());
        let mut depth = 0usize;
        let mut doc_lo = None;
        for i in 0..lexed.len() {
            let kind = lexed.kind(i);
            if kind.is_trivia() {
                if kind == SyntaxKind::COMMENT
                    && doc_lo.is_none()
                    && doc_comment_style(lexed.text(i)).is_some()
                {
                    doc_lo = Some(lexed.text_range(i).start);
                }
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
                doc_lo: doc_lo.take(),
            });
        }
        Some(MacroTokens {
            tt: *tt,
            file: context.snippet_provider.entire_snippet(),
            text,
            base,
            toks,
            doc_at_end: doc_lo,
            edition,
            rustfmt_version: context.config.rustfmt_version(),
            run: context.run.clone(),
        })
    }

    fn kind(&self, pos: usize) -> Option<SyntaxKind> {
        self.toks.get(pos).map(|t| t.kind)
    }

    /// The start of the doc comment that rustc sees as the token at `pos` (before the token
    /// at `pos`, or at the end when `pos` is past the last token).
    fn doc_at(&self, pos: usize) -> Option<usize> {
        match self.toks.get(pos) {
            Some(t) => t.doc_lo,
            None => self.doc_at_end,
        }
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
    fn parse_fragment(
        &self,
        kind: FragmentKind,
        start: usize,
        limit: usize,
    ) -> Option<Fragment<'a>> {
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
        // An item's doc comments are its attributes.
        let lo = match self.toks[start].doc_lo {
            Some(doc_lo) if kind == FragmentKind::Item => doc_lo,
            _ => self.toks[start].lo,
        };
        let hi = self.toks[end - 1].hi;
        let node = self.build(kind, lo, hi)?;
        Some(Fragment { kind, lo, hi, node })
    }

    /// rustc's type grammar accepts `?Trait`, a trait object with a single "maybe" bound;
    /// rust-analyzer's does not. This matters for `tracing`'s `?value` arguments, which
    /// rustfmt formats as types. Builds the node rust-analyzer uses for a bare trait object.
    fn parse_maybe_bound(&self, start: usize, limit: usize) -> Option<Fragment<'a>> {
        let path = self.parse_fragment(FragmentKind::Ty, start + 1, limit)?;
        if path.node.kind() != SyntaxKind::PATH_TYPE {
            return None;
        }
        let gap = &self.text[self.toks[start].hi..path.lo];
        if !gap.trim().is_empty() {
            return None;
        }
        let lexed = LexedStr::new(self.edition, &self.text[path.lo..path.hi]);
        let input = lexed.to_input(self.edition);
        let output = TopEntryPoint::Type.parse(&input, self.edition);
        let lo = self.toks[start].lo;
        let mut builder = TreeBuilder::new(
            self.file,
            self.base + lo as BytePos,
            self.run.next_tree_id(),
            lexed.len() * 2 + 8,
        );
        builder.start_node(SyntaxKind::DYN_TRAIT_TYPE);
        builder.start_node(SyntaxKind::TYPE_BOUND_LIST);
        builder.start_node(SyntaxKind::TYPE_BOUND);
        builder.token(T![?], 1);
        if !gap.is_empty() {
            builder.token(SyntaxKind::WHITESPACE, gap.len() as u32);
        }
        builder.steps(&lexed, &output)?;
        builder.finish_node();
        builder.finish_node();
        builder.finish_node();
        let tree = self.run.keep_tree(builder.finish());
        Some(Fragment {
            kind: FragmentKind::Ty,
            lo,
            hi: path.hi,
            node: tree.root(),
        })
    }

    /// Parses `text[lo..hi]` as a `kind` fragment into a tree of its own at its offset in
    /// the file, kept by the run, and returns its node: the root, or the single item under
    /// a `MacroItems` root.
    fn build(&self, kind: FragmentKind, lo: usize, hi: usize) -> Option<Node<'a>> {
        let lexed = LexedStr::new(self.edition, &self.text[lo..hi]);
        let input = lexed.to_input(self.edition);
        let output = kind.top_entry().parse(&input, self.edition);
        let base = self.base + lo as BytePos;
        let tree = Tree::from_steps_at(self.file, &lexed, &output, base, self.run.next_tree_id())?;
        let tree = self.run.keep_tree(tree);
        let root = tree.root();
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
        if node.kind() == SyntaxKind::ERROR
            || subtree_rejected_by_rustc(node, self.edition, self.rustfmt_version)
        {
            return None;
        }
        Some(node)
    }

    /// The fragment nodes, in order, or `None` when an element of the token tree straddles
    /// a fragment boundary (the fragments would not be a substitution for runs of the token
    /// tree's children).
    fn splice(&self, fragments: &[Fragment<'a>]) -> Option<Vec<Node<'a>>> {
        if fragments.is_empty() {
            return Some(Vec::new());
        }
        let base = self.base as usize;
        let mut frags = fragments.iter().peekable();
        for child in self.tt.syntax().children_with_tokens() {
            let range = child.text_range();
            let (lo, hi) = (range.start() as usize, range.end() as usize);
            if let Some(frag) = frags.peek() {
                let (flo, fhi) = (base + frag.lo, base + frag.hi);
                if lo >= flo && hi <= fhi {
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
        }
        if frags.next().is_some() {
            return None;
        }
        Some(fragments.iter().map(|f| f.node).collect())
    }

    /// rustc's `check_keyword`: a reserved word followed by `,` or the end.
    fn keyword_at(&self, pos: usize) -> bool {
        self.doc_at(pos).is_none()
            && self.kind(pos).is_some_and(|k| {
                (k.is_strict_keyword(self.edition) || k == T![_])
                    && self.doc_at(pos + 1).is_none()
                    && self.kind(pos + 1).is_none_or(|next| next == T![,])
            })
    }
}

fn delimiters<'t>(
    tt: &ast::TokenTree<'t>,
) -> Option<(crate::cst::Token<'t>, crate::cst::Token<'t>)> {
    let open = tt.syntax().first_token()?;
    let close = tt.syntax().last_token()?;
    let ok = matches!(
        (open.kind(), close.kind()),
        (T!['('], T![')']) | (T!['['], T![']']) | (T!['{'], T!['}'])
    );
    (ok && open.text_range().end() <= close.text_range().start()).then_some((open, close))
}

fn to_macro_arg<'a>(kind: FragmentKind, node: Node<'a>) -> Option<MacroArg<'a>> {
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
///
/// A doc comment cannot begin an expression, a type or a pattern
/// (`Parser::nonterminal_may_begin_with`), so an argument that starts with one can only be
/// an item.
fn parse_macro_arg<'a>(tokens: &MacroTokens<'a, '_>, pos: usize) -> Option<Fragment<'a>> {
    let limit = tokens.toks.len();
    if tokens.doc_at(pos).is_some() {
        return tokens.parse_fragment(FragmentKind::Item, pos, limit);
    }
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
pub(crate) fn parse_macro_args<'a>(
    context: &RewriteContext<'a>,
    tt: &ast::TokenTree<'_>,
    forced_bracket: bool,
) -> Option<ParsedMacroArgs<'a>> {
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
        // A doc comment after an argument is a token that is neither a separator nor the end.
        if tokens.doc_at(pos).is_some() {
            if last_is_item {
                continue;
            }
            return None;
        }
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
        if pos >= n && tokens.doc_at(pos).is_none() {
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
pub(crate) fn parse_expr<'a>(
    context: &RewriteContext<'a>,
    tt: &ast::TokenTree<'_>,
) -> Option<ast::Expr<'a>> {
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
    let node = tokens.build(FragmentKind::Expr, lo, hi)?;
    let frag = Fragment {
        kind: FragmentKind::Expr,
        lo,
        hi,
        node,
    };
    ast::Expr::cast(tokens.splice(&[frag])?.pop()?)
}

/// Parses the body of `lazy_static!`.
pub(crate) fn parse_lazy_static<'a>(
    context: &RewriteContext<'a>,
    tt: &ast::TokenTree<'_>,
) -> Option<Vec<LazyStatic<'a>>> {
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
            node: tokens.build(FragmentKind::Expr, lo, hi)?,
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
fn visibility_text(tokens: &MacroTokens<'_, '_>, start: usize, end: usize) -> String {
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
    use crate::cst::TreeArena;
    use crate::formatter::config::Settings;
    use crate::formatter::span::SnippetProvider;

    fn with_macro<R>(
        src: &str,
        f: impl for<'a> FnOnce(&RewriteContext<'a>, ast::TokenTree<'a>) -> R,
    ) -> R {
        let settings = Settings::default();
        let tree = Tree::parse(src, Edition::Edition2024).unwrap();
        let call = tree
            .root()
            .descendants()
            .find_map(ast::MacroCall::cast)
            .unwrap();
        let arena = TreeArena::new();
        let run = std::rc::Rc::new(RunState::new(&arena));
        let context = RewriteContext::new(&settings, SnippetProvider::new(src), run);
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
            assert_eq!(e.syntax().text(), "|x, y| x * y");
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
