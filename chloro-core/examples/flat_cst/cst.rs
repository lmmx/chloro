//! A flat concrete syntax tree: one preorder array of elements over a borrowed source text.
//!
//! Every node and token of the tree is one 16-byte [`Elem`] in a `Vec`, in preorder. A
//! node's children are found by skipping subtrees (`end` is the index after the subtree);
//! a token's text is a slice of the source at its byte offset, so no text is copied. Handles
//! ([`Node`], [`Token`]) are `Copy`: a reference to the tree and an index.
//!
//! The tree is built from the same `LexedStr` + `TopEntryPoint::parse` + `intersperse_trivia`
//! steps that `formatting::parse` feeds to rowan's `GreenNodeBuilder`, so both substrates
//! describe the same tree (`main.rs`'s `check` mode verifies it node by node).
//!
//! No `unsafe` code.

use std::fmt;

use ra_ap_parser::{Edition, LexedStr, StrStep, TopEntryPoint};
use ra_ap_syntax::{SyntaxKind, TextRange, TextSize};

/// One node or token of the tree, in preorder.
#[derive(Clone, Copy, Debug)]
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

const NO_PARENT: u32 = u32::MAX;

/// A flat tree over a borrowed source text.
pub struct Tree<'s> {
    src: &'s str,
    elems: Vec<Elem>,
    /// Parent index of every element (`NO_PARENT` for the root); empty when the tree was
    /// built without the column.
    parents: Vec<u32>,
}

/// The outcome of building: the tree, or why the source does not form one (the same
/// conditions under which `formatting::parse` returns `None`, apart from `rustc_compat`).
pub enum Built<'s> {
    Tree(Tree<'s>),
    LexError,
    ParseError,
}

impl<'s> Tree<'s> {
    /// Lexes and parses `src` as a source file for `edition` and lays the tree out flat.
    /// `with_parents` also fills the parent column.
    pub fn parse(src: &'s str, edition: Edition, with_parents: bool) -> Built<'s> {
        let lexed = LexedStr::new(edition, src);
        if lexed.errors().next().is_some() {
            return Built::LexError;
        }
        let input = lexed.to_input(edition);
        let output = TopEntryPoint::SourceFile.parse(&input, edition);
        Self::from_steps(src, &lexed, &output, with_parents)
    }

    /// Lays out the tree described by `output` over `lexed` (the step sequence
    /// `intersperse_trivia` produces: tokens with their text, node enters and exits).
    pub fn from_steps(
        src: &'s str,
        lexed: &LexedStr<'_>,
        output: &ra_ap_parser::Output,
        with_parents: bool,
    ) -> Built<'s> {
        Self::from_steps_at(src, lexed, output, 0, with_parents)
    }

    /// A fragment tree: `lexed` and `output` describe `src[base..base + lexed text]`, a
    /// re-parsed slice of the file (a macro argument). Every element gets its offset in the
    /// whole file, so `text_range()` and `text()` answer as they do for the file's own tree;
    /// no padding tokens and no copy of the enclosing token tree are needed (rowan's
    /// `macro_args::splice` builds both). The fragment is a tree of its own: `parent()` of
    /// its root is `None`.
    pub fn from_steps_at(
        src: &'s str,
        lexed: &LexedStr<'_>,
        output: &ra_ap_parser::Output,
        base: u32,
        with_parents: bool,
    ) -> Built<'s> {
        // Nodes are at most as many as the non-trivia tokens plus a few; tokens are
        // `lexed.len()` at most (a float split adds one). Over-reserving by 2x of the token
        // count avoids regrowth for every fixture file (measured: the ratio of elements to
        // lexed tokens is below 2 on all 1238 fixtures).
        let mut elems: Vec<Elem> = Vec::with_capacity(lexed.len() * 2 + 8);
        let mut parents: Vec<u32> = if with_parents {
            Vec::with_capacity(lexed.len() * 2 + 8)
        } else {
            Vec::new()
        };
        let mut open: Vec<u32> = Vec::with_capacity(64);
        let mut at: u32 = base;
        let mut has_error = false;
        lexed.intersperse_trivia(output, &mut |step| match step {
            StrStep::Token { kind, text } => {
                // Tokens arrive in source order and cover the source without gaps, so the
                // offset is the running total of token lengths. (`check` verifies that every
                // token's text is the source slice at that offset, split floats included.)
                let i = elems.len() as u32;
                elems.push(Elem {
                    kind,
                    is_node: false,
                    end: i + 1,
                    start: at,
                    len: text.len() as u32,
                });
                if with_parents {
                    parents.push(open.last().copied().unwrap_or(NO_PARENT));
                }
                at += text.len() as u32;
            }
            StrStep::Enter { kind } => {
                let i = elems.len() as u32;
                elems.push(Elem {
                    kind,
                    is_node: true,
                    end: 0,
                    start: at,
                    len: 0,
                });
                if with_parents {
                    parents.push(open.last().copied().unwrap_or(NO_PARENT));
                }
                open.push(i);
            }
            StrStep::Exit => {
                let i = open.pop().expect("balanced enter/exit") as usize;
                elems[i].end = elems.len() as u32;
                elems[i].len = at - elems[i].start;
            }
            StrStep::Error { .. } => has_error = true,
        });
        if has_error {
            return Built::ParseError;
        }
        debug_assert!(open.is_empty());
        debug_assert!(at as usize <= src.len());
        Built::Tree(Tree {
            src,
            elems,
            parents,
        })
    }

    pub fn source(&self) -> &'s str {
        self.src
    }

    pub fn len(&self) -> usize {
        self.elems.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elems.is_empty()
    }

    pub fn has_parent_column(&self) -> bool {
        !self.parents.is_empty()
    }

    /// Bytes the tree holds besides the source text.
    pub fn heap_bytes(&self) -> usize {
        self.elems.capacity() * std::mem::size_of::<Elem>()
            + self.parents.capacity() * std::mem::size_of::<u32>()
    }

    pub fn elem(&self, i: u32) -> Elem {
        self.elems[i as usize]
    }

    pub fn root(&self) -> Node<'_> {
        Node { tree: self, idx: 0 }
    }

    /// The element whose index is `i`, as a node (`None` for a token).
    pub fn node_at(&self, i: u32) -> Option<Node<'_>> {
        self.elems[i as usize]
            .is_node
            .then_some(Node { tree: self, idx: i })
    }

    /// Every node of the tree in preorder.
    pub fn nodes(&self) -> impl Iterator<Item = Node<'_>> + '_ {
        (0..self.elems.len() as u32)
            .filter(move |&i| self.elems[i as usize].is_node)
            .map(move |idx| Node { tree: self, idx })
    }

    /// The parent of element `i` found by searching backwards: the nearest earlier element
    /// whose subtree extends past `i`. Costs the number of elements between `i` and the
    /// start of its parent (the subtrees of the preceding siblings).
    fn parent_by_search(&self, i: u32) -> Option<u32> {
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

    fn parent_of(&self, i: u32) -> Option<u32> {
        if self.parents.is_empty() {
            self.parent_by_search(i)
        } else {
            match self.parents[i as usize] {
                NO_PARENT => None,
                p => Some(p),
            }
        }
    }

    /// The parent of `i` by search even when the column exists (for measuring).
    pub fn parent_by_search_of(&self, i: u32) -> Option<u32> {
        self.parent_by_search(i)
    }
}

/// A node or a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeOrToken<N, T> {
    Node(N),
    Token(T),
}

impl<N, T> NodeOrToken<N, T> {
    pub fn into_node(self) -> Option<N> {
        match self {
            NodeOrToken::Node(n) => Some(n),
            NodeOrToken::Token(_) => None,
        }
    }

    pub fn into_token(self) -> Option<T> {
        match self {
            NodeOrToken::Node(_) => None,
            NodeOrToken::Token(t) => Some(t),
        }
    }

    pub fn as_node(&self) -> Option<&N> {
        match self {
            NodeOrToken::Node(n) => Some(n),
            NodeOrToken::Token(_) => None,
        }
    }

    pub fn as_token(&self) -> Option<&T> {
        match self {
            NodeOrToken::Node(_) => None,
            NodeOrToken::Token(t) => Some(t),
        }
    }
}

pub type Element<'t> = NodeOrToken<Node<'t>, Token<'t>>;

impl<'t> Element<'t> {
    pub fn kind(&self) -> SyntaxKind {
        match self {
            NodeOrToken::Node(n) => n.kind(),
            NodeOrToken::Token(t) => t.kind(),
        }
    }

    pub fn text_range(&self) -> TextRange {
        match self {
            NodeOrToken::Node(n) => n.text_range(),
            NodeOrToken::Token(t) => t.text_range(),
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
        (self.tree as *const Tree<'_>).hash(state);
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
        (self.tree as *const Tree<'_>).hash(state);
        self.idx.hash(state);
    }
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}@{:?}", self.kind(), self.text_range())
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

pub fn range_of_elem(e: &Elem) -> TextRange {
    TextRange::at(TextSize::new(e.start), TextSize::new(e.len))
}

impl<'t> Node<'t> {
    #[inline]
    fn elem(&self) -> &'t Elem {
        &self.tree.elems[self.idx as usize]
    }

    /// The node's index in the tree: its identity (two handles into one tree with the same
    /// index are the same node).
    pub fn index(&self) -> u32 {
        self.idx
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
    pub fn text(&self) -> &'t str {
        let e = self.elem();
        &self.tree.src[e.start as usize..(e.start + e.len) as usize]
    }

    pub fn parent(&self) -> Option<Node<'t>> {
        self.tree.parent_of(self.idx).map(|idx| Node {
            tree: self.tree,
            idx,
        })
    }

    pub fn ancestors(&self) -> impl Iterator<Item = Node<'t>> + 't {
        std::iter::successors(Some(*self), |n| n.parent())
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

    /// The first token under the node: the first token-element of its subtree in preorder.
    pub fn first_token(&self) -> Option<Token<'t>> {
        let end = self.elem().end;
        let elems = &self.tree.elems;
        let mut i = self.idx + 1;
        while i < end {
            if !elems[i as usize].is_node {
                return Some(Token {
                    tree: self.tree,
                    idx: i,
                });
            }
            i += 1;
        }
        None
    }

    /// The last token under the node: the last token-element of its subtree.
    pub fn last_token(&self) -> Option<Token<'t>> {
        let elems = &self.tree.elems;
        let mut i = self.elem().end;
        while i > self.idx + 1 {
            i -= 1;
            if !elems[i as usize].is_node {
                return Some(Token {
                    tree: self.tree,
                    idx: i,
                });
            }
        }
        None
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

    pub fn parent(&self) -> Option<Node<'t>> {
        self.tree.parent_of(self.idx).map(|idx| Node {
            tree: self.tree,
            idx,
        })
    }

    /// The next token of the tree in source order (the next token-element).
    pub fn next_token(&self) -> Option<Token<'t>> {
        let elems = &self.tree.elems;
        let mut i = self.idx + 1;
        while (i as usize) < elems.len() {
            if !elems[i as usize].is_node {
                return Some(Token {
                    tree: self.tree,
                    idx: i,
                });
            }
            i += 1;
        }
        None
    }

    pub fn prev_token(&self) -> Option<Token<'t>> {
        let elems = &self.tree.elems;
        let mut i = self.idx;
        while i > 0 {
            i -= 1;
            if !elems[i as usize].is_node {
                return Some(Token {
                    tree: self.tree,
                    idx: i,
                });
            }
        }
        None
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
    use ra_ap_syntax::SyntaxKind;

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
pub trait AstNode<'t>: Sized + Copy {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(syntax: Node<'t>) -> Option<Self>;
    fn syntax(&self) -> Node<'t>;
}

/// A typed view of a token.
pub trait AstToken<'t>: Sized + Copy {
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
    ph: std::marker::PhantomData<N>,
}

impl<'t, N> AstChildren<'t, N> {
    fn new(parent: Node<'t>) -> Self {
        AstChildren {
            inner: parent.children(),
            ph: std::marker::PhantomData,
        }
    }
}

impl<'t, N: AstNode<'t>> Iterator for AstChildren<'t, N> {
    type Item = N;

    #[inline]
    fn next(&mut self) -> Option<N> {
        self.inner.find_map(N::cast)
    }
}
