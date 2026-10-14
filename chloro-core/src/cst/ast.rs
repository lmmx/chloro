//! The typed layer over the flat tree, with `ra_ap_syntax::ast`'s names.
//!
//! The node structs, enums, `Has*` traits and their accessors are generated from
//! `rust.ungram` by `gen_ast.py` into `generated.rs` (re-exported here). This file holds
//! what `ra_ap_syntax` writes by hand and the formatter uses: the accessors of
//! `expr_ext.rs` and `node_ext.rs` with their child-selection rules, the token types of
//! `generated/tokens.rs` and `token_ext.rs` (comments, literals), and `traits.rs`'s doc
//! comment iterator.

pub use super::generated::*;

pub use super::{AstChildren, AstNode, AstToken};
use super::{Children, Node, Token, support};
use ra_ap_parser::SyntaxKind::{self, *};
use ra_ap_parser::T;

// ---------------------------------------------------------------- expr_ext.rs

/// The `else` branch of an `if`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElseBranch<'t> {
    Block(BlockExpr<'t>),
    IfExpr(IfExpr<'t>),
}

impl<'t> AstNode<'t> for ElseBranch<'t> {
    fn can_cast(kind: SyntaxKind) -> bool {
        BlockExpr::can_cast(kind) || IfExpr::can_cast(kind)
    }
    fn cast(syntax: Node<'t>) -> Option<Self> {
        if let Some(block) = BlockExpr::cast(syntax) {
            Some(Self::Block(block))
        } else {
            IfExpr::cast(syntax).map(Self::IfExpr)
        }
    }
    fn syntax(&self) -> Node<'t> {
        match self {
            ElseBranch::Block(b) => b.syntax(),
            ElseBranch::IfExpr(i) => i.syntax(),
        }
    }
}

impl<'t> IfExpr<'t> {
    pub fn condition(&self) -> Option<Expr<'t>> {
        // If the condition is a BlockExpr, and the then branch is missing,
        // the parser will attach the then branch as the condition (ra_ap_syntax).
        let mut exprs = support::children::<Expr<'t>>(self.syntax());
        let first = exprs.next();
        match first {
            Some(Expr::BlockExpr(_)) => exprs.next().and(first),
            first => first,
        }
    }
    pub fn then_branch(&self) -> Option<BlockExpr<'t>> {
        match support::children::<Expr<'t>>(self.syntax()).nth(1)? {
            Expr::BlockExpr(block) => Some(block),
            _ => None,
        }
    }
    pub fn else_branch(&self) -> Option<ElseBranch<'t>> {
        match support::children::<Expr<'t>>(self.syntax()).nth(2)? {
            Expr::BlockExpr(block) => Some(ElseBranch::Block(block)),
            Expr::IfExpr(elif) => Some(ElseBranch::IfExpr(elif)),
            _ => None,
        }
    }
}

impl<'t> PrefixExpr<'t> {
    pub fn op_token(&self) -> Option<Token<'t>> {
        self.syntax().first_child_or_token()?.into_token()
    }
}

/// The operator of a binary expression (the token kind; rust-analyzer's `BinaryOp` value is
/// a function of the kind).
pub fn is_bin_op(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        T![||]
            | T![&&]
            | T![==]
            | T![!=]
            | T![<=]
            | T![>=]
            | T![<]
            | T![>]
            | T![+]
            | T![*]
            | T![-]
            | T![/]
            | T![%]
            | T![<<]
            | T![>>]
            | T![^]
            | T![|]
            | T![&]
            | T![=]
            | T![+=]
            | T![*=]
            | T![-=]
            | T![/=]
            | T![%=]
            | T![<<=]
            | T![>>=]
            | T![^=]
            | T![|=]
            | T![&=]
    )
}

/// `=` and the compound assignment operators: rust-analyzer's `BinaryOp::Assignment`,
/// rustc's `ExprKind::Assign` and `ExprKind::AssignOp`.
pub fn is_assign_op(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        T![=]
            | T![+=]
            | T![*=]
            | T![-=]
            | T![/=]
            | T![%=]
            | T![<<=]
            | T![>>=]
            | T![^=]
            | T![|=]
            | T![&=]
    )
}

impl<'t> BinExpr<'t> {
    pub fn op_token(&self) -> Option<Token<'t>> {
        self.syntax()
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find(|t| is_bin_op(t.kind()))
    }
    pub fn op_kind(&self) -> Option<SyntaxKind> {
        self.op_token().map(|t| t.kind())
    }
    pub fn lhs(&self) -> Option<Expr<'t>> {
        support::children(self.syntax()).next()
    }
    pub fn rhs(&self) -> Option<Expr<'t>> {
        support::children(self.syntax()).nth(1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RangeOp {
    /// `..`
    Exclusive,
    /// `..=`
    Inclusive,
}

/// The parts of a range expression or pattern (`ast.rs`'s `RangeItem`).
pub trait RangeItem<'t>: AstNode<'t> {
    type Bound: AstNode<'t>;
    fn op_token(&self) -> Option<Token<'t>>;
    fn start(&self) -> Option<Self::Bound>;
    fn end(&self) -> Option<Self::Bound>;
    fn op_kind(&self) -> Option<RangeOp> {
        match self.op_token()?.kind() {
            T![..] => Some(RangeOp::Exclusive),
            T![..=] => Some(RangeOp::Inclusive),
            _ => None,
        }
    }
}

fn range_op_details<'t>(node: Node<'t>) -> Option<(usize, Token<'t>)> {
    node.children_with_tokens()
        .enumerate()
        .find_map(|(ix, child)| {
            let token = child.into_token()?;
            matches!(token.kind(), T![..] | T![..=]).then_some((ix, token))
        })
}

impl<'t> RangeItem<'t> for RangeExpr<'t> {
    type Bound = Expr<'t>;
    fn op_token(&self) -> Option<Token<'t>> {
        range_op_details(self.syntax()).map(|t| t.1)
    }
    fn start(&self) -> Option<Expr<'t>> {
        let op_ix = range_op_details(self.syntax())?.0;
        self.syntax()
            .children_with_tokens()
            .take(op_ix)
            .find_map(|it| Expr::cast(it.into_node()?))
    }
    fn end(&self) -> Option<Expr<'t>> {
        let op_ix = range_op_details(self.syntax())?.0;
        self.syntax()
            .children_with_tokens()
            .skip(op_ix + 1)
            .find_map(|it| Expr::cast(it.into_node()?))
    }
}

impl<'t> RangeItem<'t> for RangePat<'t> {
    type Bound = Pat<'t>;
    fn op_token(&self) -> Option<Token<'t>> {
        range_op_details(self.syntax()).map(|t| t.1)
    }
    fn start(&self) -> Option<Pat<'t>> {
        let op_ix = range_op_details(self.syntax())?.0;
        self.syntax()
            .children_with_tokens()
            .take(op_ix)
            .find_map(|it| Pat::cast(it.into_node()?))
    }
    fn end(&self) -> Option<Pat<'t>> {
        let op_ix = range_op_details(self.syntax())?.0;
        self.syntax()
            .children_with_tokens()
            .skip(op_ix + 1)
            .find_map(|it| Pat::cast(it.into_node()?))
    }
}

impl<'t> IndexExpr<'t> {
    pub fn base(&self) -> Option<Expr<'t>> {
        support::children(self.syntax()).next()
    }
    pub fn index(&self) -> Option<Expr<'t>> {
        support::children(self.syntax()).nth(1)
    }
}

/// The kind of a literal, as the token type of its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LiteralKind<'t> {
    String(String<'t>),
    ByteString(ByteString<'t>),
    CString(CString<'t>),
    IntNumber(IntNumber<'t>),
    FloatNumber(FloatNumber<'t>),
    Char(Char<'t>),
    Byte(Byte<'t>),
    Bool(bool),
}

impl<'t> Literal<'t> {
    pub fn token(&self) -> Token<'t> {
        self.syntax()
            .children_with_tokens()
            .find(|e| e.kind() != ATTR && !e.kind().is_trivia())
            .and_then(|e| e.into_token())
            .unwrap()
    }

    pub fn kind(&self) -> LiteralKind<'t> {
        let token = self.token();
        if let Some(t) = IntNumber::cast(token) {
            return LiteralKind::IntNumber(t);
        }
        if let Some(t) = FloatNumber::cast(token) {
            return LiteralKind::FloatNumber(t);
        }
        if let Some(t) = String::cast(token) {
            return LiteralKind::String(t);
        }
        if let Some(t) = ByteString::cast(token) {
            return LiteralKind::ByteString(t);
        }
        if let Some(t) = CString::cast(token) {
            return LiteralKind::CString(t);
        }
        if let Some(t) = Char::cast(token) {
            return LiteralKind::Char(t);
        }
        if let Some(t) = Byte::cast(token) {
            return LiteralKind::Byte(t);
        }
        match token.kind() {
            T![true] => LiteralKind::Bool(true),
            T![false] => LiteralKind::Bool(false),
            _ => unreachable!(),
        }
    }
}

impl<'t> ClosureExpr<'t> {
    pub fn body(&self) -> Option<Expr<'t>> {
        support::child(self.syntax())
    }
}

impl<'t> BlockExpr<'t> {
    pub fn statements(&self) -> impl Iterator<Item = Stmt<'t>> + 't {
        self.stmt_list().into_iter().flat_map(|it| it.statements())
    }
    pub fn tail_expr(&self) -> Option<Expr<'t>> {
        self.stmt_list()?.tail_expr()
    }
}

impl<'t> ForExpr<'t> {
    pub fn iterable(&self) -> Option<Expr<'t>> {
        // If the iterable is a BlockExpr, check if the body is missing.
        // If it is assume the iterable is the expression that is missing instead.
        let mut exprs = support::children::<Expr<'t>>(self.syntax());
        let first = exprs.next();
        match first {
            Some(Expr::BlockExpr(_)) => exprs.next().and(first),
            first => first,
        }
    }
}

impl<'t> HasLoopBody<'t> for ForExpr<'t> {
    fn loop_body(&self) -> Option<BlockExpr<'t>> {
        let mut exprs = support::children::<BlockExpr<'t>>(self.syntax());
        let first = exprs.next();
        let second = exprs.next();
        second.or(first)
    }
}

impl<'t> WhileExpr<'t> {
    pub fn condition(&self) -> Option<Expr<'t>> {
        // If the condition is a BlockExpr, and the body is missing,
        // the parser will attach the body as the condition.
        let mut exprs = support::children::<Expr<'t>>(self.syntax());
        let first = exprs.next();
        match first {
            Some(Expr::BlockExpr(_)) => exprs.next().and(first),
            first => first,
        }
    }
}

impl<'t> HasLoopBody<'t> for WhileExpr<'t> {
    fn loop_body(&self) -> Option<BlockExpr<'t>> {
        let mut exprs = support::children::<BlockExpr<'t>>(self.syntax());
        let first = exprs.next();
        let second = exprs.next();
        second.or(first)
    }
}

impl<'t> MatchGuard<'t> {
    pub fn condition(&self) -> Option<Expr<'t>> {
        support::child(self.syntax())
    }
}

impl<'t> ArgList<'t> {
    pub fn args(&self) -> AstChildren<'t, Expr<'t>> {
        support::children(self.syntax())
    }
}

// ---------------------------------------------------------------- node_ext.rs

impl<'t> Const<'t> {
    pub fn body(&self) -> Option<Expr<'t>> {
        support::child(self.syntax())
    }
}

impl<'t> Fn<'t> {
    pub fn body(&self) -> Option<BlockExpr<'t>> {
        support::child(self.syntax())
    }
}

impl<'t> Static<'t> {
    pub fn body(&self) -> Option<Expr<'t>> {
        support::child(self.syntax())
    }
}

impl<'t> MacroDef<'t> {
    fn tts(&self) -> (Option<TokenTree<'t>>, Option<TokenTree<'t>>) {
        let mut types = support::children(self.syntax());
        let first = types.next();
        let second = types.next();
        (first, second)
    }
    /// The matcher of `macro m(args) { body }`; `None` for `macro m { rules }`.
    pub fn args(&self) -> Option<TokenTree<'t>> {
        match self.tts() {
            (Some(args), Some(_)) => Some(args),
            _ => None,
        }
    }
    /// The last token tree: the transcriber, or the rules of `macro m { rules }`.
    pub fn body(&self) -> Option<TokenTree<'t>> {
        match self.tts() {
            (Some(body), None) | (_, Some(body)) => Some(body),
            _ => None,
        }
    }
}

impl<'t> Impl<'t> {
    pub fn self_ty(&self) -> Option<Type<'t>> {
        match self.target() {
            (Some(t), None) | (_, Some(t)) => Some(t),
            _ => None,
        }
    }
    pub fn trait_(&self) -> Option<Type<'t>> {
        match self.target() {
            (Some(t), Some(_)) => Some(t),
            _ => None,
        }
    }
    fn target(&self) -> (Option<Type<'t>>, Option<Type<'t>>) {
        let mut types = support::children(self.syntax());
        let first = types.next();
        let second = types.next();
        (first, second)
    }
}

impl<'t> Abi<'t> {
    /// The string literal token of `extern "C"`.
    pub fn abi_string(&self) -> Option<Token<'t>> {
        self.string_token()
    }
}

impl<'t> NameRef<'t> {
    pub fn text(&self) -> &'t str {
        self.syntax().text()
    }
}

impl<'t> Name<'t> {
    pub fn text(&self) -> &'t str {
        self.syntax().text()
    }
}

impl<'t> Path<'t> {
    /// The segments from the first (`a` in `a::b::c`) to the last.
    pub fn segments(&self) -> impl Iterator<Item = PathSegment<'t>> + 't {
        let first = self.first_segment();
        let path = *self;
        std::iter::successors(first, move |p| {
            // The next segment after `p` is the segment of the path whose qualifier is p's
            // path; found by walking from the top path down, as rust-analyzer does with
            // `parent_path`.
            let mut cur = path;
            let mut last = None;
            while let Some(q) = cur.qualifier() {
                if q.segment().map(|s| s.syntax()) == Some(p.syntax()) {
                    last = cur.segment();
                    break;
                }
                cur = q;
            }
            last
        })
    }
    pub fn first_segment(&self) -> Option<PathSegment<'t>> {
        let mut cur = *self;
        while let Some(q) = cur.qualifier() {
            cur = q;
        }
        cur.segment()
    }
}

// ---------------------------------------------------------------- tokens

macro_rules! token_types {
    ($($name:ident => $kind:ident),* $(,)?) => {
        $(
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
            pub struct $name<'t> {
                pub(crate) syntax: Token<'t>,
            }
            impl<'t> AstToken<'t> for $name<'t> {
                fn can_cast(kind: SyntaxKind) -> bool {
                    kind == $kind
                }
                fn cast(syntax: Token<'t>) -> Option<Self> {
                    if Self::can_cast(syntax.kind()) {
                        Some(Self { syntax })
                    } else {
                        None
                    }
                }
                fn syntax(&self) -> Token<'t> {
                    self.syntax
                }
            }
        )*
    };
}

token_types! {
    Whitespace => WHITESPACE,
    Comment => COMMENT,
    String => STRING,
    ByteString => BYTE_STRING,
    CString => C_STRING,
    IntNumber => INT_NUMBER,
    FloatNumber => FLOAT_NUMBER,
    Char => CHAR,
    Byte => BYTE,
    Ident => IDENT,
}

/// `token_ext.rs`: the shape and doc placement of a comment, from its prefix.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct CommentKind {
    pub shape: CommentShape,
    pub doc: Option<CommentPlacement>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CommentShape {
    Line,
    Block,
}

impl CommentShape {
    pub fn is_line(self) -> bool {
        self == CommentShape::Line
    }

    pub fn is_block(self) -> bool {
        self == CommentShape::Block
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CommentPlacement {
    Inner,
    Outer,
}

impl CommentKind {
    const BY_PREFIX: [(&'static str, CommentKind); 9] = [
        (
            "/**/",
            CommentKind {
                shape: CommentShape::Block,
                doc: None,
            },
        ),
        (
            "/***",
            CommentKind {
                shape: CommentShape::Block,
                doc: None,
            },
        ),
        (
            "////",
            CommentKind {
                shape: CommentShape::Line,
                doc: None,
            },
        ),
        (
            "///",
            CommentKind {
                shape: CommentShape::Line,
                doc: Some(CommentPlacement::Outer),
            },
        ),
        (
            "//!",
            CommentKind {
                shape: CommentShape::Line,
                doc: Some(CommentPlacement::Inner),
            },
        ),
        (
            "/**",
            CommentKind {
                shape: CommentShape::Block,
                doc: Some(CommentPlacement::Outer),
            },
        ),
        (
            "/*!",
            CommentKind {
                shape: CommentShape::Block,
                doc: Some(CommentPlacement::Inner),
            },
        ),
        (
            "//",
            CommentKind {
                shape: CommentShape::Line,
                doc: None,
            },
        ),
        (
            "/*",
            CommentKind {
                shape: CommentShape::Block,
                doc: None,
            },
        ),
    ];

    pub fn from_text(text: &str) -> CommentKind {
        let &(_prefix, kind) = CommentKind::BY_PREFIX
            .iter()
            .find(|&(prefix, _kind)| text.starts_with(prefix))
            .unwrap();
        kind
    }

    pub fn prefix(&self) -> &'static str {
        let &(prefix, _) = CommentKind::BY_PREFIX
            .iter()
            .rev()
            .find(|(_, kind)| kind == self)
            .unwrap();
        prefix
    }
}

impl Comment<'_> {
    pub fn kind(&self) -> CommentKind {
        CommentKind::from_text(self.text())
    }

    pub fn is_doc(&self) -> bool {
        self.kind().doc.is_some()
    }

    pub fn is_inner(&self) -> bool {
        self.kind().doc == Some(CommentPlacement::Inner)
    }

    pub fn is_outer(&self) -> bool {
        self.kind().doc == Some(CommentPlacement::Outer)
    }

    pub fn prefix(&self) -> &'static str {
        let &(prefix, _kind) = CommentKind::BY_PREFIX
            .iter()
            .find(|&(prefix, kind)| self.kind() == *kind && self.text().starts_with(prefix))
            .unwrap();
        prefix
    }
}

impl Whitespace<'_> {
    pub fn spans_multiple_lines(&self) -> bool {
        let text = self.text();
        text.find('\n')
            .is_some_and(|idx| text[idx + 1..].contains('\n'))
    }
}

/// `token_ext.rs`'s `IsString`: the raw-literal prefix of a string token.
pub trait IsString<'t>: AstToken<'t> {
    fn raw_prefix(&self) -> &'static str;
    fn is_raw(&self) -> bool {
        self.text().starts_with(self.raw_prefix())
    }
}

impl<'t> IsString<'t> for String<'t> {
    fn raw_prefix(&self) -> &'static str {
        "r"
    }
}

impl<'t> IsString<'t> for ByteString<'t> {
    fn raw_prefix(&self) -> &'static str {
        "br"
    }
}

impl<'t> IsString<'t> for CString<'t> {
    fn raw_prefix(&self) -> &'static str {
        "cr"
    }
}

/// `traits.rs`: the doc comment tokens among a node's children.
#[derive(Clone)]
pub struct DocCommentIter<'t> {
    iter: Children<'t>,
}

impl<'t> DocCommentIter<'t> {
    pub fn new(node: Node<'t>) -> DocCommentIter<'t> {
        DocCommentIter {
            iter: node.children_with_tokens(),
        }
    }
}

impl<'t> Iterator for DocCommentIter<'t> {
    type Item = Comment<'t>;
    fn next(&mut self) -> Option<Comment<'t>> {
        self.iter.by_ref().find_map(|el| {
            el.into_token()
                .and_then(Comment::cast)
                .filter(Comment::is_doc)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::Tree;
    use ra_ap_parser::Edition;

    /// `generated.rs` is what `gen_ast.py` produces from the checked-in `rust.ungram`.
    #[test]
    fn generated_layer_is_up_to_date() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cst");
        let out = tempfile::NamedTempFile::with_suffix(".rs").unwrap();
        let status = std::process::Command::new("python3")
            .arg(dir.join("gen_ast.py"))
            .arg(dir.join("rust.ungram"))
            .arg(out.path())
            .stdout(std::process::Stdio::null())
            .status()
            .expect("python3 runs gen_ast.py");
        assert!(status.success(), "gen_ast.py failed");
        let generated = std::fs::read_to_string(out.path()).unwrap();
        let checked_in = std::fs::read_to_string(dir.join("generated.rs")).unwrap();
        assert!(
            generated == checked_in,
            "src/cst/generated.rs differs from gen_ast.py's output; regenerate it"
        );
    }

    #[test]
    fn hand_written_accessors_select_the_same_children_as_ra_ap_syntax() {
        let src = "fn f() { if a { 1 } else if b { 2 } else { 3 }; while x { } for i in it { } \
                   a + b * c; -x; !y; r[0]; 1..2; 3..=4; |q| q; match m { _ if g => 0 } \
                   x.0.1; \"s\"; r\"raw\"; 1.5; b'a'; true; }\n\
                   impl<T> Tr for S<T> {}\nimpl S {}\nconst C: u8 = 1;\nstatic D: u8 = 2;\n\
                   macro m($x:tt) { $x }\nextern \"C\" fn g() {}\nfn h() { a::b::<u8>::c; }\n";
        let tree = Tree::parse(src, Edition::Edition2024).unwrap();
        let root = tree.root();
        let nodes: Vec<Node<'_>> = root.descendants().collect();
        let find =
            |f: &dyn std::ops::Fn(Node<'_>) -> bool| nodes.iter().copied().find(|&n| f(n)).unwrap();

        let if_expr = IfExpr::cast(find(&|n| n.kind() == IF_EXPR)).unwrap();
        assert_eq!(if_expr.condition().unwrap().syntax().text(), "a");
        assert_eq!(if_expr.then_branch().unwrap().syntax().text(), "{ 1 }");
        let Some(ElseBranch::IfExpr(elif)) = if_expr.else_branch() else {
            panic!("else if");
        };
        assert!(matches!(elif.else_branch(), Some(ElseBranch::Block(_))));

        let w = WhileExpr::cast(find(&|n| n.kind() == WHILE_EXPR)).unwrap();
        assert_eq!(w.condition().unwrap().syntax().text(), "x");
        assert_eq!(w.loop_body().unwrap().syntax().text(), "{ }");
        let f = ForExpr::cast(find(&|n| n.kind() == FOR_EXPR)).unwrap();
        assert_eq!(f.iterable().unwrap().syntax().text(), "it");
        assert_eq!(f.loop_body().unwrap().syntax().text(), "{ }");

        let bin = BinExpr::cast(find(&|n| n.kind() == BIN_EXPR)).unwrap();
        assert_eq!(bin.op_kind(), Some(T![+]));
        assert_eq!(bin.lhs().unwrap().syntax().text(), "a");
        assert_eq!(bin.rhs().unwrap().syntax().text(), "b * c");
        let neg = PrefixExpr::cast(find(&|n| n.kind() == PREFIX_EXPR)).unwrap();
        assert_eq!(neg.op_token().unwrap().kind(), T![-]);
        let idx = IndexExpr::cast(find(&|n| n.kind() == INDEX_EXPR)).unwrap();
        assert_eq!(idx.base().unwrap().syntax().text(), "r");
        assert_eq!(idx.index().unwrap().syntax().text(), "0");
        let ranges: Vec<RangeExpr<'_>> =
            nodes.iter().copied().filter_map(RangeExpr::cast).collect();
        assert_eq!(ranges[0].op_kind(), Some(RangeOp::Exclusive));
        assert_eq!(ranges[1].op_kind(), Some(RangeOp::Inclusive));
        assert_eq!(ranges[1].start().unwrap().syntax().text(), "3");
        assert_eq!(ranges[1].end().unwrap().syntax().text(), "4");
        let cl = ClosureExpr::cast(find(&|n| n.kind() == CLOSURE_EXPR)).unwrap();
        assert_eq!(cl.body().unwrap().syntax().text(), "q");
        let guard = MatchGuard::cast(find(&|n| n.kind() == MATCH_GUARD)).unwrap();
        assert_eq!(guard.condition().unwrap().syntax().text(), "g");

        let lits: Vec<LiteralKind<'_>> = nodes
            .iter()
            .copied()
            .filter_map(Literal::cast)
            .map(|l| l.kind())
            .collect();
        assert!(matches!(lits[0], LiteralKind::IntNumber(_)));
        let strings: Vec<String<'_>> = lits
            .iter()
            .filter_map(|l| match l {
                LiteralKind::String(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert_eq!(strings.len(), 2);
        assert!(!strings[0].is_raw());
        assert!(strings[1].is_raw());
        assert!(
            lits.iter()
                .any(|l| matches!(l, LiteralKind::FloatNumber(f) if f.text() == "1.5"))
        );
        assert!(lits.iter().any(|l| matches!(l, LiteralKind::Byte(_))));
        assert!(lits.contains(&LiteralKind::Bool(true)));

        let impls: Vec<Impl<'_>> = nodes.iter().copied().filter_map(Impl::cast).collect();
        assert_eq!(impls[0].trait_().unwrap().syntax().text(), "Tr");
        assert_eq!(impls[0].self_ty().unwrap().syntax().text(), "S<T>");
        assert!(impls[1].trait_().is_none());
        assert_eq!(impls[1].self_ty().unwrap().syntax().text(), "S");
        let c = Const::cast(find(&|n| n.kind() == CONST)).unwrap();
        assert_eq!(c.body().unwrap().syntax().text(), "1");
        let s = Static::cast(find(&|n| n.kind() == STATIC)).unwrap();
        assert_eq!(s.body().unwrap().syntax().text(), "2");
        let m = MacroDef::cast(find(&|n| n.kind() == MACRO_DEF)).unwrap();
        assert_eq!(m.args().unwrap().syntax().text(), "($x:tt)");
        assert_eq!(m.body().unwrap().syntax().text(), "{ $x }");
        let rules = Tree::parse("macro m { ($i:ident) => {} }", Edition::Edition2024).unwrap();
        let m = MacroDef::cast(
            rules
                .root()
                .descendants()
                .find(|n| n.kind() == MACRO_DEF)
                .unwrap(),
        )
        .unwrap();
        assert!(m.args().is_none());
        assert_eq!(m.body().unwrap().syntax().text(), "{ ($i:ident) => {} }");
        let abi = Abi::cast(find(&|n| n.kind() == ABI)).unwrap();
        assert_eq!(abi.abi_string().unwrap().text(), "\"C\"");
        let path = Path::cast(find(&|n| n.kind() == PATH && n.text() == "a::b::<u8>::c")).unwrap();
        let segs: Vec<&str> = path.segments().map(|s| s.syntax().text()).collect();
        assert_eq!(segs, ["a", "b::<u8>", "c"]);
        assert_eq!(path.first_segment().unwrap().syntax().text(), "a");
    }

    #[test]
    fn comment_kinds_and_doc_comments() {
        let src = "/// outer\n//! inner\n//// plain\n/** block */\n/*! inner block */\n/**/\n// c\nfn f() {}\n";
        let tree = Tree::parse(src, Edition::Edition2024).unwrap();
        let comments: Vec<Comment<'_>> = tree
            .root()
            .descendants_with_tokens()
            .filter_map(|e| e.into_token().and_then(Comment::cast))
            .collect();
        let kinds: Vec<(bool, bool, bool)> = comments
            .iter()
            .map(|c| (c.is_doc(), c.is_inner(), c.kind().shape.is_block()))
            .collect();
        assert_eq!(
            kinds,
            [
                (true, false, false),
                (true, true, false),
                (false, false, false),
                (true, false, true),
                (true, true, true),
                (false, false, true),
                (false, false, false)
            ]
        );
        assert_eq!(comments[0].prefix(), "///");
        assert_eq!(comments[5].prefix(), "/**/");
        // Inner doc comments keep the comment run on the file rather than the item.
        let file = SourceFile::cast(tree.root()).unwrap();
        let docs: Vec<&str> = file.doc_comments().map(|c| c.text()).collect();
        assert_eq!(
            docs,
            [
                "/// outer",
                "//! inner",
                "/** block */",
                "/*! inner block */"
            ]
        );
    }
}
