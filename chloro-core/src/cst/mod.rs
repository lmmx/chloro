//! A flat concrete syntax tree: one preorder array of elements over a borrowed source text.
//!
//! Every node and token of a tree is one 16-byte [`Elem`] in a `Vec`, in preorder. A node's
//! children are found by skipping subtrees (`end` is the index after the subtree); a token's
//! text is a slice of the source at its byte offset, so no text is copied. Handles ([`Node`],
//! [`Token`]) are `Copy`: a reference to the tree and an index. Creating one costs nothing,
//! which is what distinguishes this substrate from rowan's red nodes (one 64-byte allocation
//! per handle).
//!
//! A tree is built from the same `LexedStr` + `TopEntryPoint::parse` output that
//! `ra_ap_syntax` feeds through `intersperse_trivia` to rowan's `GreenNodeBuilder`, so both
//! describe the same tree (the tests in this module verify it node by node while rowan is a
//! dev-dependency). [`TreeBuilder::steps`] ports `intersperse_trivia`'s trivia attachment
//! into the build loop; debug builds check it against `intersperse_trivia` on every tree.
//!
//! The typed layer over the tree ([`ast`]) keeps `ra_ap_syntax::ast`'s names, so the
//! formatter modules read `expr.lhs()`, `item.name()`, `block.stmt_list()` as before.
//!
//! Ownership: a run of the formatter builds the file's tree and, for every macro call whose
//! arguments it formats, fragment trees laid out at the argument's file offset
//! ([`Tree::from_steps_at`]). The fragment trees are appended to a [`TreeArena`] owned by the
//! run's frame, and every handle borrows the arena's lifetime, which is the `'a` that
//! `RewriteContext<'a>` already carries for the source text. Handles are stored in the memo
//! and the macro-argument cache of the run; the arena outlives both.
//!
//! No `unsafe` code.

pub mod ast;
mod generated;

use std::fmt;
use std::marker::PhantomData;

use ra_ap_parser::{Edition, LexedStr, Output, Step, StrStep, SyntaxKind, TopEntryPoint};

/// Trees allocated during one formatting run, with stable addresses for the run's lifetime.
pub type TreeArena<'s> = typed_arena::Arena<Tree<'s>>;

/// One node or token of the tree, in preorder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Elem {
    pub kind: SyntaxKind,
    pub is_node: bool,
    /// Index after this element's subtree (`index + 1` for a token or an empty node).
    pub end: u32,
    /// Byte offset of the element's text in the source.
    pub start: u32,
    /// Byte length of the element's text.
    pub len: u32,
}

/// A byte range of the source: the `TextRange` of the tree's elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextRange {
    start: u32,
    end: u32,
}

impl TextRange {
    #[inline]
    pub fn new(start: u32, end: u32) -> TextRange {
        debug_assert!(start <= end);
        TextRange { start, end }
    }

    #[inline]
    pub fn start(self) -> u32 {
        self.start
    }

    #[inline]
    pub fn end(self) -> u32 {
        self.end
    }

    #[inline]
    pub fn len(self) -> u32 {
        self.end - self.start
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// A flat tree over a borrowed source text.
pub struct Tree<'s> {
    src: &'s str,
    elems: Vec<Elem>,
    /// The tree's number within its run (0 for the file, then one per fragment): with an
    /// element index, the identity of a node for the run's memo.
    id: u32,
}

impl<'s> Tree<'s> {
    /// Lexes and parses `src` as a source file for `edition` and lays the tree out flat.
    /// `None` when the source does not lex or parse: rustfmt refuses to format a file with
    /// a syntax error, and so does chloro. Only lexer and parser errors count;
    /// rust-analyzer's semantic validation is not run.
    pub fn parse(src: &'s str, edition: Edition) -> Option<Tree<'s>> {
        let lexed = LexedStr::new(edition, src);
        if lexed.errors().next().is_some() {
            return None;
        }
        let input = lexed.to_input(edition);
        let output = TopEntryPoint::SourceFile.parse(&input, edition);
        Self::from_steps_at(src, &lexed, &output, 0, 0)
    }

    /// The tree of `src` whether or not it lexes and parses without error (an error leaves
    /// `ERROR` nodes in it), as `ra_ap_syntax::SourceFile::parse(..).tree()` answers. The
    /// formatter never formats such a tree; tests look at one.
    pub fn parse_with_errors(src: &'s str, edition: Edition) -> Tree<'s> {
        let lexed = LexedStr::new(edition, src);
        let input = lexed.to_input(edition);
        let output = TopEntryPoint::SourceFile.parse(&input, edition);
        let mut builder = TreeBuilder::new(src, 0, 0, lexed.len() * 2 + 8);
        let _ = builder.steps(&lexed, &output);
        builder.finish()
    }

    /// Lays out the tree described by `output` over `lexed` (the step sequence
    /// `intersperse_trivia` produces: tokens with their text, node enters and exits), as a
    /// tree numbered `id`.
    ///
    /// `lexed` and `output` describe `src[base..base + lexed text]`: the whole file when
    /// `base` is 0, or a re-parsed slice of it (a macro argument). Every element gets its
    /// offset in the whole file, so `text_range()` and `text()` answer as they do for the
    /// file's own tree; no padding tokens and no copy of the enclosing token tree are
    /// needed. A fragment is a tree of its own: `parent()` of its root is `None`.
    ///
    /// `None` when the parser reported an error.
    pub fn from_steps_at(
        src: &'s str,
        lexed: &LexedStr<'_>,
        output: &ra_ap_parser::Output,
        base: u32,
        id: u32,
    ) -> Option<Tree<'s>> {
        let mut builder = TreeBuilder::new(src, base, id, lexed.len() * 2 + 8);
        builder.steps(lexed, output)?;
        Some(builder.finish())
    }

    pub fn source(&self) -> &'s str {
        self.src
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn len(&self) -> usize {
        self.elems.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elems.is_empty()
    }

    /// Bytes the tree holds besides the source text.
    pub fn heap_bytes(&self) -> usize {
        self.elems.capacity() * std::mem::size_of::<Elem>()
    }

    pub fn elem(&self, i: u32) -> Elem {
        self.elems[i as usize]
    }

    pub fn root(&self) -> Node<'_> {
        Node { tree: self, idx: 0 }
    }

    /// The node at index `i` (a handle recovered from [`Node::index`]).
    #[inline]
    pub fn node_at(&self, i: u32) -> Node<'_> {
        debug_assert!(self.elems[i as usize].is_node);
        Node { tree: self, idx: i }
    }

    /// Every node of the tree in preorder.
    pub fn nodes(&self) -> impl Iterator<Item = Node<'_>> + '_ {
        (0..self.elems.len() as u32)
            .filter(move |&i| self.elems[i as usize].is_node)
            .map(move |idx| Node { tree: self, idx })
    }

    /// The parent of element `i`: the nearest earlier node whose subtree extends past `i`,
    /// found by searching backwards. Costs the number of elements between `i` and the start
    /// of its parent (the subtrees of the preceding siblings); the tree keeps no parent
    /// column.
    fn parent_of(&self, i: u32) -> Option<u32> {
        let mut j = i;
        while j > 0 {
            j -= 1;
            let e = &self.elems[j as usize];
            if e.is_node && e.end > i {
                return Some(j);
            }
        }
        None
    }
}

/// Builds a [`Tree`] element by element: parser steps ([`TreeBuilder::steps`]) or single
/// nodes and tokens around them (the synthetic node `macro_args::parse_maybe_bound` builds).
/// Token offsets are the running total of token lengths from `base`, so every token's text
/// is the source slice at its offset (checked in debug builds, split floats included).
#[derive(Clone)]
pub struct TreeBuilder<'s> {
    src: &'s str,
    elems: Vec<Elem>,
    open: Vec<u32>,
    at: u32,
    id: u32,
}

impl<'s> TreeBuilder<'s> {
    pub fn new(src: &'s str, base: u32, id: u32, capacity: usize) -> TreeBuilder<'s> {
        TreeBuilder {
            src,
            elems: Vec::with_capacity(capacity),
            open: Vec::with_capacity(64),
            at: base,
            id,
        }
    }

    /// The offset the next token starts at.
    pub fn offset(&self) -> u32 {
        self.at
    }

    /// A token of `len` bytes at the current offset.
    #[inline]
    pub fn token(&mut self, kind: SyntaxKind, len: u32) {
        let i = self.elems.len() as u32;
        self.elems.push(Elem {
            kind,
            is_node: false,
            end: i + 1,
            start: self.at,
            len,
        });
        self.at += len;
    }

    #[inline]
    pub fn start_node(&mut self, kind: SyntaxKind) {
        let i = self.elems.len() as u32;
        self.elems.push(Elem {
            kind,
            is_node: true,
            end: 0,
            start: self.at,
            len: 0,
        });
        self.open.push(i);
    }

    #[inline]
    pub fn finish_node(&mut self) {
        let i = self.open.pop().expect("balanced enter/exit") as usize;
        self.elems[i].end = self.elems.len() as u32;
        self.elems[i].len = self.at - self.elems[i].start;
    }

    /// Lays out the tree `output` describes over `lexed`, which describe the source from the
    /// current offset, with trivia attached as `intersperse_trivia` attaches it (see
    /// [`Self::steps_fused`]). `None` when the parser reported an error (the builder is
    /// then unusable). Debug builds also run [`Self::steps_interspersed`] on a copy of the
    /// builder and assert that both give the same elements.
    pub fn steps(&mut self, lexed: &LexedStr<'_>, output: &Output) -> Option<()> {
        #[cfg(debug_assertions)]
        let mut reference = self.clone();
        #[cfg(debug_assertions)]
        let expected = reference.steps_interspersed(lexed, output);
        let result = self.steps_fused(lexed, output);
        #[cfg(debug_assertions)]
        {
            assert_eq!(
                result, expected,
                "fused and interspersed build disagree on errors"
            );
            assert!(
                self.elems == reference.elems
                    && self.open == reference.open
                    && self.at == reference.at,
                "fused and interspersed build disagree"
            );
        }
        result
    }

    /// Feeds the steps `intersperse_trivia` produces for `output` over `lexed`, which
    /// describe the source from the current offset. `None` when the parser reported an
    /// error (the builder is then unusable). The reference for [`Self::steps_fused`].
    pub fn steps_interspersed(&mut self, lexed: &LexedStr<'_>, output: &Output) -> Option<()> {
        let mut has_error = false;
        lexed.intersperse_trivia(output, &mut |step| match step {
            StrStep::Token { kind, text } => {
                // Tokens arrive in source order and cover the source without gaps, so the
                // offset is the running total of token lengths.
                debug_assert_eq!(
                    self.src
                        .get(self.at as usize..self.at as usize + text.len()),
                    Some(text),
                    "token text is the source slice at its offset"
                );
                self.token(kind, text.len() as u32);
            }
            StrStep::Enter { kind } => self.start_node(kind),
            StrStep::Exit => self.finish_node(),
            StrStep::Error { .. } => has_error = true,
        });
        (!has_error).then_some(())
    }

    /// [`Self::steps_interspersed`] in one pass: decodes `output` and writes the elements
    /// directly, with token lengths from `LexedStr::text_start`, instead of producing
    /// `StrStep`s with their text for a callback.
    ///
    /// A port of `LexedStr::intersperse_trivia` and its `Builder` (ra_ap_parser 0.0.307,
    /// `shortcuts.rs`), whose rules it must reproduce: the first `Enter` takes no trivia;
    /// an `Exit` is deferred until the next step, so trivia after a node's last token is
    /// never inside it; before a token, the trivia ahead is emitted at the current depth;
    /// before an `Enter`, the trivia run ahead is split by [`n_attached_trivias`] into the
    /// part left in the parent and the part attached inside the new node; a split float
    /// becomes `NAME_REF`/`INT_NUMBER`, `.` and an optional second `NAME_REF`; at the end
    /// the remaining trivia goes inside the root. The panics of the original
    /// (`unreachable!` states, the float asserts) are kept.
    pub fn steps_fused(&mut self, lexed: &LexedStr<'_>, output: &Output) -> Option<()> {
        let len = |i: usize| (lexed.text_start(i + 1) - lexed.text_start(i)) as u32;
        let n = lexed.len();
        let mut pos = 0usize;
        let mut state = BuildState::PendingEnter;
        let mut has_error = false;
        for step in output.iter() {
            match step {
                Step::Token {
                    kind,
                    n_input_tokens,
                } => {
                    self.leave_pending(&mut state);
                    pos = self.trivias(lexed, pos, n);
                    let end = pos + n_input_tokens as usize;
                    self.token(kind, (lexed.text_start(end) - lexed.text_start(pos)) as u32);
                    pos = end;
                }
                Step::FloatSplit { ends_in_dot } => {
                    self.leave_pending(&mut state);
                    pos = self.trivias(lexed, pos, n);
                    let text = lexed.text(pos);
                    match text.split_once('.') {
                        Some((left, right)) => {
                            assert!(!left.is_empty());
                            self.start_node(SyntaxKind::NAME_REF);
                            self.token(SyntaxKind::INT_NUMBER, left.len() as u32);
                            self.finish_node();
                            // The parser's own `Exit` of the field expression, moved up.
                            self.finish_node();
                            self.token(SyntaxKind::DOT, 1);
                            if ends_in_dot {
                                assert!(right.is_empty(), "{left}.{right}");
                                state = BuildState::Normal;
                            } else {
                                assert!(!right.is_empty(), "{left}.{right}");
                                self.start_node(SyntaxKind::NAME_REF);
                                self.token(SyntaxKind::INT_NUMBER, right.len() as u32);
                                self.finish_node();
                                state = BuildState::PendingExit;
                            }
                        }
                        None => {
                            // A float without a dot (`1e0`): an error and an `ERROR` node.
                            has_error = true;
                            self.start_node(SyntaxKind::ERROR);
                            self.token(SyntaxKind::FLOAT_NUMBER, text.len() as u32);
                            self.finish_node();
                            self.finish_node();
                            state = if ends_in_dot {
                                BuildState::Normal
                            } else {
                                BuildState::PendingExit
                            };
                        }
                    }
                    pos += 1;
                }
                Step::Enter { kind } => {
                    match std::mem::replace(&mut state, BuildState::Normal) {
                        BuildState::PendingEnter => {
                            // The root: no previous node to leave trivia in.
                            self.start_node(kind);
                            continue;
                        }
                        BuildState::PendingExit => self.finish_node(),
                        BuildState::Normal => {}
                    }
                    let run = (pos..n).take_while(|&i| lexed.kind(i).is_trivia()).count();
                    let attached = n_attached_trivias(kind, lexed, pos, run);
                    for i in pos..pos + run - attached {
                        self.token(lexed.kind(i), len(i));
                    }
                    self.start_node(kind);
                    for i in pos + run - attached..pos + run {
                        self.token(lexed.kind(i), len(i));
                    }
                    pos += run;
                }
                Step::Exit => match std::mem::replace(&mut state, BuildState::PendingExit) {
                    BuildState::PendingEnter => unreachable!(),
                    BuildState::PendingExit => self.finish_node(),
                    BuildState::Normal => {}
                },
                Step::Error { .. } => {
                    // `intersperse_trivia` reports the error at `text_start(pos)`.
                    let _ = lexed.text_start(pos);
                    has_error = true;
                }
            }
        }
        match state {
            BuildState::PendingExit => {
                self.trivias(lexed, pos, n);
                self.finish_node();
            }
            BuildState::PendingEnter | BuildState::Normal => unreachable!(),
        }
        (!has_error).then_some(())
    }

    /// Finishes the node whose `Exit` is pending, before a token or a split float.
    #[inline]
    fn leave_pending(&mut self, state: &mut BuildState) {
        match std::mem::replace(state, BuildState::Normal) {
            BuildState::PendingEnter => unreachable!(),
            BuildState::PendingExit => self.finish_node(),
            BuildState::Normal => {}
        }
    }

    /// Emits the trivia tokens from `pos` up to the next non-trivia token; returns its index.
    #[inline]
    fn trivias(&mut self, lexed: &LexedStr<'_>, mut pos: usize, n: usize) -> usize {
        while pos < n {
            let kind = lexed.kind(pos);
            if !kind.is_trivia() {
                break;
            }
            self.token(
                kind,
                (lexed.text_start(pos + 1) - lexed.text_start(pos)) as u32,
            );
            pos += 1;
        }
        pos
    }

    pub fn finish(self) -> Tree<'s> {
        debug_assert!(self.open.is_empty(), "every node is finished");
        debug_assert!(self.at as usize <= self.src.len());
        Tree {
            src: self.src,
            elems: self.elems,
            id: self.id,
        }
    }
}

/// A node or a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeOrToken<N, T> {
    Node(N),
    Token(T),
}

impl<N, T> NodeOrToken<N, T> {
    #[inline]
    pub fn into_node(self) -> Option<N> {
        match self {
            NodeOrToken::Node(n) => Some(n),
            NodeOrToken::Token(_) => None,
        }
    }

    #[inline]
    pub fn into_token(self) -> Option<T> {
        match self {
            NodeOrToken::Node(_) => None,
            NodeOrToken::Token(t) => Some(t),
        }
    }

    #[inline]
    pub fn as_node(&self) -> Option<&N> {
        match self {
            NodeOrToken::Node(n) => Some(n),
            NodeOrToken::Token(_) => None,
        }
    }

    #[inline]
    pub fn as_token(&self) -> Option<&T> {
        match self {
            NodeOrToken::Node(_) => None,
            NodeOrToken::Token(t) => Some(t),
        }
    }
}

/// A child of a node: a node or a token.
pub type Element<'t> = NodeOrToken<Node<'t>, Token<'t>>;

impl<'t> Element<'t> {
    #[inline]
    pub fn kind(&self) -> SyntaxKind {
        match self {
            NodeOrToken::Node(n) => n.kind(),
            NodeOrToken::Token(t) => t.kind(),
        }
    }

    #[inline]
    pub fn text_range(&self) -> TextRange {
        match self {
            NodeOrToken::Node(n) => n.text_range(),
            NodeOrToken::Token(t) => t.text_range(),
        }
    }

    /// The element's text: a slice of the source.
    #[inline]
    pub fn text(&self) -> &'t str {
        match self {
            NodeOrToken::Node(n) => n.text(),
            NodeOrToken::Token(t) => t.text(),
        }
    }
}

/// A node handle: the tree and the node's index. `Copy`; creating one costs nothing.
#[derive(Clone, Copy)]
pub struct Node<'t> {
    tree: &'t Tree<'t>,
    idx: u32,
}

/// A token handle.
#[derive(Clone, Copy)]
pub struct Token<'t> {
    tree: &'t Tree<'t>,
    idx: u32,
}

impl PartialEq for Node<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.tree, other.tree) && self.idx == other.idx
    }
}
impl Eq for Node<'_> {}
impl std::hash::Hash for Node<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::from_ref(self.tree).hash(state);
        self.idx.hash(state);
    }
}
impl PartialEq for Token<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.tree, other.tree) && self.idx == other.idx
    }
}
impl Eq for Token<'_> {}
impl std::hash::Hash for Token<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::from_ref(self.tree).hash(state);
        self.idx.hash(state);
    }
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}@{:?}", self.kind(), self.text_range())
    }
}

/// The node's text, as rowan's `SyntaxNode` displays.
impl fmt::Display for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text())
    }
}

impl fmt::Debug for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}@{:?} {:?}",
            self.kind(),
            self.text_range(),
            self.text()
        )
    }
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text())
    }
}

#[inline]
fn range_of_elem(e: &Elem) -> TextRange {
    TextRange {
        start: e.start,
        end: e.start + e.len,
    }
}

impl<'t> Node<'t> {
    #[inline]
    fn elem(&self) -> &'t Elem {
        &self.tree.elems[self.idx as usize]
    }

    /// The node's index in the tree: with the tree's id, its identity (two handles into one
    /// tree with the same index are the same node).
    #[inline]
    pub fn index(&self) -> u32 {
        self.idx
    }

    /// The id of the tree the node belongs to ([`Tree::id`]).
    #[inline]
    pub fn tree_id(&self) -> u32 {
        self.tree.id
    }

    pub fn tree(&self) -> &'t Tree<'t> {
        self.tree
    }

    #[inline]
    pub fn kind(&self) -> SyntaxKind {
        self.elem().kind
    }

    #[inline]
    pub fn text_range(&self) -> TextRange {
        range_of_elem(self.elem())
    }

    /// The node's text: a slice of the source, no copy.
    #[inline]
    pub fn text(&self) -> &'t str {
        let e = self.elem();
        &self.tree.src[e.start as usize..(e.start + e.len) as usize]
    }

    /// The parent node, found by searching backwards from this node; `None` for the root
    /// of a tree (a file or a macro-argument fragment). See [`Tree::parent_of`] for the cost.
    pub fn parent(&self) -> Option<Node<'t>> {
        self.tree.parent_of(self.idx).map(|idx| Node {
            tree: self.tree,
            idx,
        })
    }

    /// Child nodes and tokens, in order.
    #[inline]
    pub fn children_with_tokens(&self) -> Children<'t> {
        let e = self.elem();
        Children {
            tree: self.tree,
            i: self.idx + 1,
            end: e.end,
        }
    }

    /// Child nodes, in order.
    #[inline]
    pub fn children(&self) -> NodeChildren<'t> {
        NodeChildren(self.children_with_tokens())
    }

    pub fn first_child(&self) -> Option<Node<'t>> {
        self.children().next()
    }

    pub fn last_child(&self) -> Option<Node<'t>> {
        self.children().last()
    }

    pub fn first_child_or_token(&self) -> Option<Element<'t>> {
        self.children_with_tokens().next()
    }

    pub fn last_child_or_token(&self) -> Option<Element<'t>> {
        self.children_with_tokens().last()
    }

    /// The first token under the node, as rowan answers it: the first token on the chain
    /// of first children (`None` when that chain ends at a node without children).
    pub fn first_token(&self) -> Option<Token<'t>> {
        let elems = &self.tree.elems;
        let mut i = self.idx;
        loop {
            let e = &elems[i as usize];
            if !e.is_node {
                return Some(Token {
                    tree: self.tree,
                    idx: i,
                });
            }
            if e.end == i + 1 {
                return None;
            }
            i += 1;
        }
    }

    /// The last token under the node, as rowan answers it: the last token on the chain of
    /// last children (`None` when that chain ends at a node without children). In preorder
    /// that is the last element of the subtree, if it is a token.
    pub fn last_token(&self) -> Option<Token<'t>> {
        let end = self.elem().end;
        if end == self.idx + 1 {
            return None;
        }
        let last = end - 1;
        (!self.tree.elems[last as usize].is_node).then_some(Token {
            tree: self.tree,
            idx: last,
        })
    }

    /// Every node of the subtree, this node first, in preorder.
    pub fn descendants(&self) -> impl Iterator<Item = Node<'t>> + 't {
        let tree = self.tree;
        let end = self.elem().end;
        (self.idx..end)
            .filter(move |&i| tree.elems[i as usize].is_node)
            .map(move |idx| Node { tree, idx })
    }

    /// Every element of the subtree, this node first, in preorder.
    pub fn descendants_with_tokens(&self) -> impl Iterator<Item = Element<'t>> + 't {
        let tree = self.tree;
        let end = self.elem().end;
        (self.idx..end).map(move |idx| {
            if tree.elems[idx as usize].is_node {
                NodeOrToken::Node(Node { tree, idx })
            } else {
                NodeOrToken::Token(Token { tree, idx })
            }
        })
    }

    /// The number of elements in the subtree, the node included.
    pub fn subtree_len(&self) -> u32 {
        self.elem().end - self.idx
    }
}

impl<'t> Token<'t> {
    #[inline]
    fn elem(&self) -> &'t Elem {
        &self.tree.elems[self.idx as usize]
    }

    #[inline]
    pub fn index(&self) -> u32 {
        self.idx
    }

    #[inline]
    pub fn kind(&self) -> SyntaxKind {
        self.elem().kind
    }

    #[inline]
    pub fn text_range(&self) -> TextRange {
        range_of_elem(self.elem())
    }

    /// The token's text: a slice of the source.
    #[inline]
    pub fn text(&self) -> &'t str {
        let e = self.elem();
        &self.tree.src[e.start as usize..(e.start + e.len) as usize]
    }

    /// The parent node (see [`Node::parent`]).
    pub fn parent(&self) -> Option<Node<'t>> {
        self.tree.parent_of(self.idx).map(|idx| Node {
            tree: self.tree,
            idx,
        })
    }
}

/// Iterator over a node's children, nodes and tokens.
#[derive(Clone)]
pub struct Children<'t> {
    tree: &'t Tree<'t>,
    i: u32,
    end: u32,
}

impl<'t> Iterator for Children<'t> {
    type Item = Element<'t>;

    #[inline]
    fn next(&mut self) -> Option<Element<'t>> {
        if self.i >= self.end {
            return None;
        }
        let idx = self.i;
        let e = &self.tree.elems[idx as usize];
        self.i = e.end;
        Some(if e.is_node {
            NodeOrToken::Node(Node {
                tree: self.tree,
                idx,
            })
        } else {
            NodeOrToken::Token(Token {
                tree: self.tree,
                idx,
            })
        })
    }
}

/// Iterator over a node's child nodes.
#[derive(Clone)]
pub struct NodeChildren<'t>(Children<'t>);

impl<'t> Iterator for NodeChildren<'t> {
    type Item = Node<'t>;

    #[inline]
    fn next(&mut self) -> Option<Node<'t>> {
        loop {
            let c = &mut self.0;
            if c.i >= c.end {
                return None;
            }
            let idx = c.i;
            let e = &c.tree.elems[idx as usize];
            c.i = e.end;
            if e.is_node {
                return Some(Node { tree: c.tree, idx });
            }
        }
    }
}

/// The typed layer's building blocks: the `support` module of `ra_ap_syntax::ast`, over
/// flat nodes.
pub mod support {
    use super::{AstChildren, AstNode, Node, Token};
    use ra_ap_parser::SyntaxKind;

    /// The first child node that casts to `N` (`ra_ap_syntax`'s `support::child`).
    #[inline]
    pub fn child<'t, N: AstNode<'t>>(parent: Node<'t>) -> Option<N> {
        parent.children().find_map(N::cast)
    }

    /// Every child node that casts to `N`.
    #[inline]
    pub fn children<'t, N: AstNode<'t>>(parent: Node<'t>) -> AstChildren<'t, N> {
        AstChildren::new(parent)
    }

    /// The first child token of `kind`.
    #[inline]
    pub fn token<'t>(parent: Node<'t>, kind: SyntaxKind) -> Option<Token<'t>> {
        parent
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|it| it.kind() == kind)
    }
}

/// A typed view of a node: `ra_ap_syntax::ast::AstNode` over flat nodes.
pub trait AstNode<'t>: Sized + Copy + fmt::Debug {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(syntax: Node<'t>) -> Option<Self>;
    fn syntax(&self) -> Node<'t>;
}

/// A typed view of a token.
pub trait AstToken<'t>: Sized + Copy + fmt::Debug {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(syntax: Token<'t>) -> Option<Self>;
    fn syntax(&self) -> Token<'t>;
    fn text(&self) -> &'t str {
        self.syntax().text()
    }
}

/// The child nodes of a node that cast to `N`.
#[derive(Clone)]
pub struct AstChildren<'t, N> {
    inner: NodeChildren<'t>,
    ph: PhantomData<N>,
}

impl<'t, N> AstChildren<'t, N> {
    fn new(parent: Node<'t>) -> Self {
        AstChildren {
            inner: parent.children(),
            ph: PhantomData,
        }
    }
}

/// The state of [`TreeBuilder::steps_fused`], as `intersperse_trivia`'s.
enum BuildState {
    PendingEnter,
    Normal,
    PendingExit,
}

/// How many of the `run` trivia tokens at `pos..pos + run` (the run before a node of
/// `kind`) go inside the node: rust-analyzer's `n_attached_trivias`, which walks the run
/// backwards from the node. Only items and fields take trivia. A blank line stops the walk
/// unless an outer doc comment precedes it; an inner doc comment stops it; any other
/// comment extends the attached part to include it.
fn n_attached_trivias(kind: SyntaxKind, lexed: &LexedStr<'_>, pos: usize, run: usize) -> usize {
    use SyntaxKind::*;
    match kind {
        CONST | ENUM | FN | IMPL | MACRO_CALL | MACRO_DEF | MACRO_RULES | MODULE | RECORD_FIELD
        | STATIC | STRUCT | TRAIT | TUPLE_FIELD | TYPE_ALIAS | UNION | USE | VARIANT
        | EXTERN_CRATE => {
            let mut res = 0;
            // `j` counts from the node backwards: lexed index `pos + run - 1 - j`.
            let mut j = 0;
            while j < run {
                let i = pos + run - 1 - j;
                match lexed.kind(i) {
                    WHITESPACE if lexed.text(i).contains("\n\n") => {
                        // Skip the blank line when the trivia before it is an outer doc
                        // comment.
                        if j + 1 < run
                            && lexed.kind(i - 1) == COMMENT
                            && is_outer(lexed.text(i - 1))
                        {
                            j += 1;
                            continue;
                        }
                        break;
                    }
                    COMMENT => {
                        if is_inner(lexed.text(i)) {
                            break;
                        }
                        res = j + 1;
                    }
                    _ => {}
                }
                j += 1;
            }
            res
        }
        _ => 0,
    }
}

fn is_outer(text: &str) -> bool {
    if text.starts_with("////") || text.starts_with("/***") {
        return false;
    }
    text.starts_with("///") || text.starts_with("/**")
}

fn is_inner(text: &str) -> bool {
    text.starts_with("//!") || text.starts_with("/*!")
}

impl<'t, N: AstNode<'t>> Iterator for AstChildren<'t, N> {
    type Item = N;

    #[inline]
    fn next(&mut self) -> Option<N> {
        self.inner.find_map(N::cast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rowan tree of `src`, as `ra_ap_syntax::SyntaxTreeBuilder` builds it from the
    /// same steps.
    fn rowan_tree(src: &str, edition: Edition) -> ra_ap_syntax::SyntaxNode {
        let lexed = LexedStr::new(edition, src);
        let input = lexed.to_input(edition);
        let output = TopEntryPoint::SourceFile.parse(&input, edition);
        let mut builder = ra_ap_syntax::SyntaxTreeBuilder::default();
        lexed.intersperse_trivia(&output, &mut |step| match step {
            StrStep::Token { kind, text } => builder.token(kind, text),
            StrStep::Enter { kind } => builder.start_node(kind),
            StrStep::Exit => builder.finish_node(),
            StrStep::Error { .. } => panic!("{src:?} does not parse"),
        });
        builder.finish().syntax_node()
    }

    /// Every element in preorder agrees with rowan's on kind, range and node/token class;
    /// every token's text is the source slice at its range; `parent()` agrees with rowan's.
    fn assert_same_tree(src: &str) {
        let edition = Edition::Edition2024;
        let root = rowan_tree(src, edition);
        let tree = Tree::parse(src, edition).expect("parses");
        let mut n = 0u32;
        for e in root.preorder_with_tokens() {
            let rowan::WalkEvent::Enter(el) = e else {
                continue;
            };
            let fe = tree.elem(n);
            assert_eq!(el.kind(), fe.kind, "kind at element {n} of {src:?}");
            let r = el.text_range();
            assert_eq!(
                (u32::from(r.start()), u32::from(r.end())),
                (fe.start, fe.start + fe.len),
                "range at element {n} of {src:?}"
            );
            match el {
                ra_ap_syntax::NodeOrToken::Node(_) => assert!(fe.is_node, "node at {n}"),
                ra_ap_syntax::NodeOrToken::Token(t) => {
                    assert!(!fe.is_node, "token at {n}");
                    let slice = &src[fe.start as usize..(fe.start + fe.len) as usize];
                    assert_eq!(t.text(), slice, "token text at {n}");
                }
            }
            n += 1;
        }
        assert_eq!(n as usize, tree.len(), "element count of {src:?}");
        for (el, fl) in root.descendants().zip(tree.nodes()) {
            assert_eq!(
                el.parent().map(|p| u32::from(p.text_range().start())),
                fl.parent().map(|p| p.text_range().start()),
                "parent of {fl:?} in {src:?}"
            );
            assert_eq!(
                el.first_token().map(|t| t.text().to_owned()),
                fl.first_token().map(|t| t.text().to_owned())
            );
            assert_eq!(
                el.last_token().map(|t| t.text().to_owned()),
                fl.last_token().map(|t| t.text().to_owned())
            );
            assert_eq!(el.text().to_string(), fl.text());
        }
    }

    const SOURCES: &[&str] = &[
        "",
        "fn f() {}\n",
        "//! doc\n#[cfg(x)]\nfn f<T: ?Sized>(a: &T) -> u8 { m!(a, b); 1 }\n",
        "fn f() { let x = t.0.1; let y = 1.5e3; let z = x.0.1.2; }\n",
        "mod m {\n    //! inner\n    /// outer\n    pub(crate) struct S<'a> { x: &'a u8 }\n}\n",
        "fn f() { match x { 1 | 2 => {} _ if let Some(y) = z && y => {} } }\n",
        "macro_rules! m { ($($x:tt)*) => { $($x)* }; }\nm! { fn g() {} }\n",
        "fn f() { 'a: loop { break 'a; } async move { x.await } }\n",
        "#![allow(x)]\nuse a::{b, c::*};\nimpl<T> Tr for S<T> where T: Clone { type A = u8; }\n",
    ];

    #[test]
    fn flat_tree_equals_the_rowan_tree() {
        for src in SOURCES {
            assert_same_tree(src);
        }
    }

    /// Every third fixture file (a debug build compares about 15 files per second).
    #[test]
    fn flat_tree_equals_the_rowan_tree_on_the_fixtures() {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures");
        let mut files = 0;
        let mut paths: Vec<_> = walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .map(|e| e.into_path())
            .filter(|p| p.extension().is_some_and(|e| e == "rs"))
            .collect();
        paths.sort();
        for entry in paths.iter().step_by(3) {
            let Ok(src) = std::fs::read_to_string(entry) else {
                continue;
            };
            let src = src
                .strip_prefix('\u{feff}')
                .unwrap_or(&src)
                .replace("\r\n", "\n");
            if Tree::parse(&src, Edition::Edition2024).is_none() {
                continue;
            }
            assert_same_tree(&src);
            files += 1;
        }
        assert!(files > 300, "{files} fixtures compared");
    }

    /// The elements, open nodes and result of [`TreeBuilder::steps_fused`] and of
    /// [`TreeBuilder::steps_interspersed`] over `src` parsed with `entry`; whether the
    /// parser output holds a split float that ends in a dot.
    fn assert_fused_equal(src: &str, entry: &TopEntryPoint) -> bool {
        let edition = Edition::Edition2024;
        let lexed = LexedStr::new(edition, src);
        let input = lexed.to_input(edition);
        let output = entry.parse(&input, edition);
        let mut fused = TreeBuilder::new(src, 0, 0, 16);
        let mut reference = TreeBuilder::new(src, 0, 0, 16);
        let a = fused.steps_fused(&lexed, &output);
        let b = reference.steps_interspersed(&lexed, &output);
        assert_eq!(a, b, "result of {src:?}");
        assert_eq!(fused.elems, reference.elems, "elements of {src:?}");
        assert_eq!(fused.open, reference.open);
        assert_eq!(fused.at, reference.at);
        output
            .iter()
            .any(|s| matches!(s, ra_ap_parser::Step::FloatSplit { ends_in_dot: true }))
    }

    #[test]
    fn fused_build_equals_the_interspersed_build() {
        let file = &TopEntryPoint::SourceFile;
        // Split floats: two fields, a field and a dot before whitespace, an exponent (an error).
        assert_fused_equal("fn f() { t.0.1; t.0.1.2; x.0 .1; }\n", file);
        assert!(
            assert_fused_equal("fn f() { x.0. bar; x.0. await; x.0.0.call(); }\n", file),
            "a split float that ends in a dot"
        );
        assert_fused_equal("fn f() { x.1e0; }\n", file);
        // A doc comment followed by a blank line, then an item; `////` and `/***` are not
        // doc comments; a plain comment after a blank line stays outside.
        assert_fused_equal("/// doc\n\nfn f() {}\n//// four\n\nfn g() {}\n", file);
        assert_fused_equal("/** doc */\n\nstruct S;\n/*** x */\n\nstruct T;\n", file);
        assert_fused_equal("// a\n\n// b\nfn f() {}\n", file);
        // An inner doc comment stops the attachment.
        assert_fused_equal("mod m {\n    //! inner\n    // c\n    fn f() {}\n}\n", file);
        // Trailing trivia at the end of a block and of the file.
        assert_fused_equal("fn f() {\n    x;\n    // end\n}\n\n// eof\n  ", file);
        // A comment between the attributes and the item, and above the attributes.
        assert_fused_equal("// above\n#[a]\n// between\n#[b]\nfn f() {}\n", file);
        // `macro_rules!` bodies and a macro call with comments.
        assert_fused_equal(
            "/// m\nmacro_rules! m {\n    // c\n    ($x:expr) => { $x };\n}\nm!(1 /* c */);\n",
            file,
        );
        // Fields and variants take comments; leading trivia of the file.
        assert_fused_equal(
            "\n// lead\nstruct S {\n    // a\n    a: u8,\n}\nenum E {\n    /// v\n\n    V,\n}\n",
            file,
        );
        // Fragments (macro arguments), errors, and the empty file.
        assert_fused_equal("a + /* c */ b", &TopEntryPoint::Expr);
        assert_fused_equal("Vec<u8> x", &TopEntryPoint::Type);
        assert_fused_equal("// c\nfn f() {}\n", &TopEntryPoint::MacroItems);
        assert_fused_equal("fn f( {", file);
        assert_fused_equal("", file);
        for src in SOURCES {
            assert_fused_equal(src, file);
        }
    }

    /// Every fixture file, whether or not it parses.
    #[test]
    fn fused_build_equals_the_interspersed_build_on_the_fixtures() {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures");
        let mut files = 0;
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(path) else {
                continue;
            };
            assert_fused_equal(&src, &TopEntryPoint::SourceFile);
            files += 1;
        }
        assert!(files > 1200, "{files} fixtures compared");
    }

    #[test]
    fn errors_give_no_tree() {
        assert!(Tree::parse("fn f( {", Edition::Edition2024).is_none());
        assert!(Tree::parse("let x = \"unterminated", Edition::Edition2024).is_none());
    }

    #[test]
    fn fragments_are_laid_out_at_their_offset() {
        let src = "fn f() { foo!(a + b, c) }";
        let base = src.find("a + b").unwrap() as u32;
        let edition = Edition::Edition2024;
        let lexed = LexedStr::new(edition, "a + b");
        let input = lexed.to_input(edition);
        let output = TopEntryPoint::Expr.parse(&input, edition);
        let frag = Tree::from_steps_at(src, &lexed, &output, base, 7).unwrap();
        assert_eq!(frag.id(), 7);
        let root = frag.root();
        assert_eq!(root.kind(), SyntaxKind::BIN_EXPR);
        assert_eq!(root.text_range(), TextRange::new(base, base + 5));
        assert_eq!(root.text(), "a + b");
        assert!(root.parent().is_none());
        for t in root
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
        {
            let r = t.text_range();
            assert_eq!(t.text(), &src[r.start() as usize..r.end() as usize]);
        }
    }

    #[test]
    fn children_and_tokens() {
        let tree = Tree::parse("fn f(a: u8) -> u8 { a }\n", Edition::Edition2024).unwrap();
        let root = tree.root();
        assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
        let f = root.first_child().unwrap();
        assert_eq!(f.kind(), SyntaxKind::FN);
        assert_eq!(f.first_token().unwrap().text(), "fn");
        assert_eq!(f.last_token().unwrap().text(), "}");
        assert_eq!(f.last_child().unwrap().kind(), SyntaxKind::BLOCK_EXPR);
        let kinds: Vec<_> = f.children().map(|n| n.kind()).collect();
        assert_eq!(
            kinds,
            [
                SyntaxKind::NAME,
                SyntaxKind::PARAM_LIST,
                SyntaxKind::RET_TYPE,
                SyntaxKind::BLOCK_EXPR
            ]
        );
        assert_eq!(f.children_with_tokens().count(), 8);
        assert_eq!(f.parent(), Some(root));
        assert_eq!(f.last_child().unwrap().parent(), Some(f));
        assert_eq!(support::token(f, SyntaxKind::FN_KW).unwrap().index(), 2);
        assert!(support::token(f, SyntaxKind::L_CURLY).is_none());
    }
}
