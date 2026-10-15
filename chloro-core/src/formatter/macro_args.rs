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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// A prefix parse of a window of tokens: what [`MacroTokens::prefix_parse`] reports, with
/// the window's lexed text and the parser's output.
struct PrefixParse<'a> {
    consumed: usize,
    error: Option<usize>,
    /// Tokens in the window.
    window: usize,
    /// Start of the window in the interior.
    lo: usize,
    lexed: LexedStr<'a>,
    output: ra_ap_parser::Output,
}

impl PrefixParse<'_> {
    /// Whether the build of the `kind` fragment starting at `lo` would lex and parse the
    /// same text into the same steps, so that it can take this parse instead.
    ///
    /// The fragment is the consumed tokens, so with the whole window consumed its text is
    /// the window's text and lexes the same. For `Expr` and `Ty` the top entry runs the
    /// prefix entry's grammar function (`expressions::expr`, `types::type_`) after opening
    /// a marker that it abandons when the parser stopped at the end of the input, as it
    /// has here; an abandoned marker is a tombstone that `event::process` drops, so the two
    /// outputs are the same steps (ra_ap_parser 0.0.307, `grammar.rs` `entry::prefix` and
    /// `entry::top`). `Pat` and `Item` use other grammar functions and keep the second
    /// parse.
    fn matches_top_parse(&self, kind: FragmentKind, lo: usize) -> bool {
        matches!(kind, FragmentKind::Expr | FragmentKind::Ty)
            && self.error.is_none()
            && self.consumed == self.window
            && self.lo == lo
    }
}

/// Whether two parser outputs are the same steps (a debug check of [`PrefixParse`]).
fn same_steps(a: &ra_ap_parser::Output, b: &ra_ap_parser::Output) -> bool {
    a.iter()
        .map(|s| format!("{s:?}"))
        .eq(b.iter().map(|s| format!("{s:?}")))
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
        let (toks, doc_lo) = if lexes_alone_as_in_file(text) {
            let from_tree = interior_tokens(tt, open.index(), close.index(), base);
            #[cfg(debug_assertions)]
            assert_eq!(
                Some(&from_tree),
                lex_interior(text, edition).as_ref(),
                "the token tree's tokens differ from a lexing of its interior: {text:?}"
            );
            from_tree
        } else {
            lex_interior(text, edition)?
        };
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
        let parse = self.prefix_parse_window(kind, start, end);
        (parse.consumed, parse.error)
    }

    /// [`Self::prefix_parse`], keeping the window's lexed text and parser output for
    /// [`Self::build`] to reuse.
    fn prefix_parse_window(&self, kind: FragmentKind, start: usize, end: usize) -> PrefixParse<'a> {
        let lo = self.toks[start].lo;
        let text = &self.text[lo..self.toks[end - 1].hi];
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
        PrefixParse {
            consumed,
            error,
            window: end - start,
            lo,
            lexed,
            output,
        }
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
        let mut reuse = None;
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
                let parse = self.prefix_parse_window(kind, start, end);
                match (parse.consumed, parse.error) {
                    (n, None) if n > 0 => {
                        reuse = Some(parse);
                        break n;
                    }
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
        let reuse = reuse.filter(|p| p.matches_top_parse(kind, lo));
        let node = self.build(kind, lo, hi, reuse)?;
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
    /// a `MacroItems` root. `prefix` is a prefix parse that [`PrefixParse::matches_top_parse`]
    /// over this text, whose lexed text and output are used instead of parsing again.
    fn build(
        &self,
        kind: FragmentKind,
        lo: usize,
        hi: usize,
        prefix: Option<PrefixParse<'a>>,
    ) -> Option<Node<'a>> {
        let (lexed, output) = match prefix {
            Some(p) => {
                debug_assert_eq!(p.lexed.as_str(), &self.text[lo..hi]);
                debug_assert!(same_steps(&p.output, &{
                    let input = p.lexed.to_input(self.edition);
                    kind.top_entry().parse(&input, self.edition)
                }));
                (p.lexed, p.output)
            }
            None => {
                let lexed = LexedStr::new(self.edition, &self.text[lo..hi]);
                let input = lexed.to_input(self.edition);
                let output = kind.top_entry().parse(&input, self.edition);
                (lexed, output)
            }
        };
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

/// The tokens of a token tree interior and the start of a doc comment after the last one,
/// from a lexing of the interior's text; `None` when rustc's lexer rejects it.
fn lex_interior(text: &str, edition: Edition) -> Option<(Vec<Tok>, Option<usize>)> {
    let lexed = LexedStr::new(edition, text);
    if lexer_rejects(&lexed, edition) {
        return None;
    }
    let mut tokens = TokCollector::with_capacity(lexed.len());
    for i in 0..lexed.len() {
        let range = lexed.text_range(i);
        tokens.push(lexed.kind(i), range.start, range.end, lexed.text(i));
    }
    Some(tokens.finish())
}

/// The same from the tokens the tree already holds between the delimiters at `open` and
/// `close` (element indices), at offsets relative to `base`, the start of the interior.
///
/// The file was lexed once; a token tree's interior is a run of whole tokens of that
/// lexing (no token crosses a delimiter), the parser keeps the tokens of a token tree as
/// the lexer made them, and the lexer is context-free except at the start of its input,
/// where it reads a shebang (`#!`) or a frontmatter fence (`---`, after blank lines):
/// [`lexes_alone_as_in_file`] sends those texts to [`lex_interior`]. A tree exists only
/// for text without lexer errors that rustc's lexer rejects (`Tree::parse`, and the
/// `lexer_rejects` test of the token tree that held a fragment), so the interior has none
/// either. Debug builds compare the two on every token tree formatted.
fn interior_tokens(
    tt: &ast::TokenTree<'_>,
    open: u32,
    close: u32,
    base: BytePos,
) -> (Vec<Tok>, Option<usize>) {
    let tree = tt.syntax().tree();
    let mut tokens = TokCollector::with_capacity((close - open) as usize);
    for i in open + 1..close {
        let e = tree.elem(i);
        if e.is_node {
            continue;
        }
        let lo = (e.start - base) as usize;
        let hi = lo + e.len as usize;
        let text = &tree.source()[e.start as usize..(e.start + e.len) as usize];
        tokens.push(e.kind, lo, hi, text);
    }
    tokens.finish()
}

/// `false` when lexing `text` alone may differ from lexing it inside a file: its first
/// character other than whitespace (Unicode's or the lexer's, which adds U+200E and
/// U+200F) is `#` (a shebang) or `-` (a frontmatter fence).
fn lexes_alone_as_in_file(text: &str) -> bool {
    let first = text
        .chars()
        .find(|&c| !(c.is_whitespace() || c == '\u{200e}' || c == '\u{200f}'));
    !matches!(first, Some('#' | '-'))
}

/// Builds the [`Tok`]s of an interior from its lexer tokens, trivia included.
struct TokCollector {
    toks: Vec<Tok>,
    depth: usize,
    doc_lo: Option<usize>,
}

impl TokCollector {
    fn with_capacity(n: usize) -> TokCollector {
        TokCollector {
            toks: Vec::with_capacity(n),
            depth: 0,
            doc_lo: None,
        }
    }

    fn push(&mut self, kind: SyntaxKind, lo: usize, hi: usize, text: &str) {
        if kind.is_trivia() {
            if kind == SyntaxKind::COMMENT
                && self.doc_lo.is_none()
                && doc_comment_style(text).is_some()
            {
                self.doc_lo = Some(lo);
            }
            return;
        }
        match kind {
            T!['('] | T!['['] | T!['{'] => self.depth += 1,
            T![')'] | T![']'] | T!['}'] => self.depth = self.depth.saturating_sub(1),
            _ => {}
        }
        self.toks.push(Tok {
            kind,
            lo,
            hi,
            top_level_sep: self.depth == 0 && matches!(kind, T![,] | T![;]),
            doc_lo: self.doc_lo.take(),
        });
    }

    fn finish(self) -> (Vec<Tok>, Option<usize>) {
        (self.toks, self.doc_lo)
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
    let parse = tokens.prefix_parse_window(FragmentKind::Expr, 0, n);
    let consumed = parse.consumed;
    if parse.error.is_some() || consumed == 0 {
        return None;
    }
    let (lo, hi) = (tokens.toks[0].lo, tokens.toks[consumed - 1].hi);
    let reuse = Some(parse).filter(|p| p.matches_top_parse(FragmentKind::Expr, lo));
    let node = tokens.build(FragmentKind::Expr, lo, hi, reuse)?;
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
        if pos >= limit {
            return None;
        }
        let parse = tokens.prefix_parse_window(FragmentKind::Expr, pos, limit);
        let consumed = parse.consumed;
        if parse.error.is_some() || consumed == 0 {
            return None;
        }
        let (lo, hi) = (tokens.toks[pos].lo, tokens.toks[pos + consumed - 1].hi);
        let reuse = Some(parse).filter(|p| p.matches_top_parse(FragmentKind::Expr, lo));
        let expr = Fragment {
            kind: FragmentKind::Expr,
            lo,
            hi,
            node: tokens.build(FragmentKind::Expr, lo, hi, reuse)?,
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

    /// The tokens of every token tree of every fixture file (and of a few sources with
    /// comments, doc comments, nested delimiters and raw identifiers) taken from the tree
    /// equal those of a lexing of the interior, at editions 2024 and 2018; interiors that
    /// start with `#` or `-` are left to the lexer.
    #[test]
    fn interior_tokens_match_a_lexing_of_the_interior() {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures");
        let mut sources: Vec<String> = walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
            .filter_map(|e| std::fs::read_to_string(e.path()).ok())
            .collect();
        sources.push(
            "m!(a::b => c >> d, x.0.1, r#try, union, 1.0e3, \"s\" /*c*/ /// d\n e, [f(g)]{h});\n"
                .to_owned(),
        );
        sources.push("m!(/// doc\n);\nm!( #!x);\nm!(\n---\n);\nm!(-1);\n".to_owned());
        let (mut compared, mut left) = (0, 0);
        for edition in [Edition::Edition2024, Edition::Edition2018] {
            for src in &sources {
                let Some(tree) = Tree::parse(src, edition) else {
                    continue;
                };
                for tt in tree.root().descendants().filter_map(ast::TokenTree::cast) {
                    let Some((open, close)) = delimiters(&tt) else {
                        continue;
                    };
                    let base = open.text_range().end();
                    let text = &src[base as usize..close.text_range().start() as usize];
                    if !lexes_alone_as_in_file(text) {
                        left += 1;
                        continue;
                    }
                    assert_eq!(
                        Some(interior_tokens(&tt, open.index(), close.index(), base)),
                        lex_interior(text, edition),
                        "{text:?}"
                    );
                    compared += 1;
                }
            }
        }
        assert!(
            compared > 10_000 && left > 0,
            "{compared} compared, {left} left"
        );
    }

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

    /// A build that reuses a whole-window prefix parse lays out the same tree as one that
    /// lexes and parses the text again with the top entry point: split floats, closures,
    /// casts, generic types, ranges, a trailing comment, a type that ends in `>>`.
    #[test]
    fn reused_prefix_parse_builds_the_same_tree() {
        let sources = [
            "m!(t.0.1);",
            "m!(x.0 + 1.5e3);",
            "m!(|a, b| a + b);",
            "m!(x as Vec<u8> /* c */);",
            "m!(a..=b);",
            "m!(Vec<Option<u8>>);",
            "m!(&'a mut [T; 3]);",
            "m!(async move { x.await });",
        ];
        let mut reused = 0;
        for src in sources {
            with_macro(src, |context, tt| {
                let tokens = MacroTokens::new(context, &tt).unwrap();
                let n = tokens.toks.len();
                let (lo, hi) = (tokens.toks[0].lo, tokens.toks[n - 1].hi);
                for kind in [FragmentKind::Expr, FragmentKind::Ty] {
                    let parse = tokens.prefix_parse_window(kind, 0, n);
                    if !parse.matches_top_parse(kind, lo) {
                        continue;
                    }
                    reused += 1;
                    let a = tokens.build(kind, lo, hi, Some(parse));
                    let b = tokens.build(kind, lo, hi, None);
                    let elems = |node: Option<Node<'_>>| {
                        node.map(|n| {
                            n.descendants_with_tokens()
                                .map(|e| (e.kind(), e.text_range()))
                                .collect::<Vec<_>>()
                        })
                    };
                    assert_eq!(elems(a), elems(b), "{src}");
                }
            });
        }
        assert!(reused >= sources.len(), "{reused} reused parses");
    }
}
