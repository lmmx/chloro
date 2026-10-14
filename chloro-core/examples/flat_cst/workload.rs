// The query workload. This file is `include!`d twice, in `mod red` and `mod flat`, so the same
// source compiles against `ra_ap_syntax::ast` over rowan's red nodes and against the generated
// `ast.rs` over flat nodes. The enclosing module provides `ast`, `AstNode`, `Node`, `Token`,
// `NodeOrToken`, `RangeItem`, `rustc_span`, `has_token` and `token_text`.
//
// It imitates the formatter's access pattern:
// - `visit_expr`: for every expression node, `kind`, `text_range`, and the typed child
//   accessors `format_expr_uncached`'s match calls for that kind (expr.rs);
// - `stmts_of`: for every block, the `children_with_tokens` walk of `nodes::stmts_of` with
//   `rustc_span` and `has_token` per statement;
// - `visit_path`: for every path, the segment walk of `types::rewrite_path_segments`
//   (`qualifier` chain, `name_ref` text, `generic_arg_list` and its args).
// Every result is folded into the checksum, so the two instantiations must agree.

use super::Acc;

#[inline]
fn mark(acc: &mut Acc, n: &Node) {
    let r = n.text_range();
    acc.accessor_calls += 1;
    acc.mix((n.kind() as u64) << 48 | u64::from(u32::from(r.start())) << 24 | u64::from(u32::from(r.end())));
}

#[inline]
fn markt(acc: &mut Acc, t: &Token) {
    let r = t.text_range();
    acc.accessor_calls += 1;
    acc.mix((t.kind() as u64) << 48 | u64::from(u32::from(r.start())) << 24 | u64::from(u32::from(r.end())) | 1 << 63);
}

#[inline]
fn miss(acc: &mut Acc) {
    acc.accessor_calls += 1;
    acc.mix(0xdead);
}

fn mark_text(acc: &mut Acc, text: &str) {
    for b in text.bytes() {
        acc.mix(u64::from(b));
    }
}

macro_rules! opt {
    ($acc:expr, $e:expr) => {
        match $e {
            Some(x) => mark($acc, &x.syntax()),
            None => miss($acc),
        }
    };
}

macro_rules! optt {
    ($acc:expr, $e:expr) => {
        match $e {
            Some(t) => markt($acc, &t),
            None => miss($acc),
        }
    };
}

/// Every node of the tree in preorder: expressions, blocks and paths are queried.
pub fn run(root: &Node, acc: &mut Acc) {
    for n in root.descendants() {
        let kind = n.kind();
        if ast::Expr::can_cast(kind) {
            let e = ast::Expr::cast(n.clone()).unwrap();
            visit_expr(&e, acc);
        } else if kind == SyntaxKind::PATH {
            let p = ast::Path::cast(n.clone()).unwrap();
            visit_path(&p, acc);
        }
    }
}

fn visit_expr(e: &ast::Expr, acc: &mut Acc) {
    mark(acc, &e.syntax());
    match e {
        ast::Expr::ArrayExpr(a) => {
            for x in a.exprs() {
                mark(acc, &x.syntax());
            }
            optt!(acc, a.semicolon_token());
        }
        ast::Expr::BinExpr(b) => {
            opt!(acc, b.lhs());
            opt!(acc, b.rhs());
            optt!(acc, b.op_token());
        }
        ast::Expr::CallExpr(c) => {
            opt!(acc, c.expr());
            match c.arg_list() {
                Some(args) => {
                    mark(acc, &args.syntax());
                    for x in args.args() {
                        mark(acc, &x.syntax());
                    }
                }
                None => miss(acc),
            }
        }
        ast::Expr::MethodCallExpr(m) => {
            opt!(acc, m.receiver());
            opt!(acc, m.name_ref());
            opt!(acc, m.generic_arg_list());
            match m.arg_list() {
                Some(args) => {
                    for x in args.args() {
                        mark(acc, &x.syntax());
                    }
                }
                None => miss(acc),
            }
        }
        ast::Expr::FieldExpr(f) => {
            opt!(acc, f.expr());
            opt!(acc, f.name_ref());
        }
        ast::Expr::IfExpr(i) => {
            opt!(acc, i.condition());
            opt!(acc, i.then_branch());
            match i.else_branch() {
                Some(b) => mark(acc, &b.syntax()),
                None => miss(acc),
            }
        }
        ast::Expr::MatchExpr(m) => {
            opt!(acc, m.expr());
            if let Some(arms) = m.match_arm_list() {
                for arm in arms.arms() {
                    opt!(acc, arm.pat());
                    match arm.guard() {
                        Some(g) => opt!(acc, g.condition()),
                        None => miss(acc),
                    }
                    opt!(acc, arm.expr());
                    optt!(acc, arm.comma_token());
                }
            }
        }
        ast::Expr::ClosureExpr(c) => {
            if let Some(params) = c.param_list() {
                for p in params.params() {
                    opt!(acc, p.pat());
                    opt!(acc, p.ty());
                }
            }
            opt!(acc, c.ret_type());
            opt!(acc, c.body());
            optt!(acc, c.move_token());
        }
        ast::Expr::BlockExpr(b) => {
            optt!(acc, b.unsafe_token());
            match b.stmt_list() {
                Some(list) => stmts_of(&list, acc),
                None => miss(acc),
            }
        }
        ast::Expr::PathExpr(p) => opt!(acc, p.path()),
        ast::Expr::RefExpr(r) => {
            opt!(acc, r.expr());
            optt!(acc, r.mut_token());
            optt!(acc, r.raw_token());
        }
        ast::Expr::PrefixExpr(p) => {
            opt!(acc, p.expr());
            optt!(acc, p.op_token());
        }
        ast::Expr::TryExpr(t) => opt!(acc, t.expr()),
        ast::Expr::AwaitExpr(a) => opt!(acc, a.expr()),
        ast::Expr::ParenExpr(p) => opt!(acc, p.expr()),
        ast::Expr::ReturnExpr(r) => opt!(acc, r.expr()),
        ast::Expr::BreakExpr(b) => {
            opt!(acc, b.expr());
            opt!(acc, b.lifetime());
        }
        ast::Expr::CastExpr(c) => {
            opt!(acc, c.expr());
            opt!(acc, c.ty());
        }
        ast::Expr::IndexExpr(i) => {
            opt!(acc, i.base());
            opt!(acc, i.index());
        }
        ast::Expr::RangeExpr(r) => {
            opt!(acc, r.start());
            opt!(acc, r.end());
            optt!(acc, r.op_token());
        }
        ast::Expr::TupleExpr(t) => {
            for x in t.fields() {
                mark(acc, &x.syntax());
            }
        }
        ast::Expr::RecordExpr(r) => {
            opt!(acc, r.path());
            if let Some(list) = r.record_expr_field_list() {
                for f in list.fields() {
                    opt!(acc, f.name_ref());
                    opt!(acc, f.expr());
                }
                opt!(acc, list.spread());
            }
        }
        ast::Expr::Literal(l) => markt(acc, &l.token()),
        ast::Expr::MacroExpr(m) => match m.macro_call() {
            Some(call) => {
                opt!(acc, call.path());
                match call.token_tree() {
                    Some(tt) => {
                        optt!(acc, tt.syntax().first_token());
                        optt!(acc, tt.syntax().last_token());
                    }
                    None => miss(acc),
                }
            }
            None => miss(acc),
        },
        ast::Expr::LetExpr(l) => {
            opt!(acc, l.pat());
            opt!(acc, l.expr());
        }
        ast::Expr::ForExpr(f) => {
            opt!(acc, f.pat());
            opt!(acc, f.iterable());
            opt!(acc, f.loop_body());
            opt!(acc, f.label());
        }
        ast::Expr::WhileExpr(w) => {
            opt!(acc, w.condition());
            opt!(acc, w.loop_body());
            opt!(acc, w.label());
        }
        ast::Expr::LoopExpr(l) => {
            opt!(acc, l.loop_body());
            opt!(acc, l.label());
        }
        _ => {}
    }
}

/// `nodes::stmts_of`: statements of a block in rustc's form.
fn stmts_of(list: &ast::StmtList, acc: &mut Acc) {
    let tail = list.tail_expr();
    let tail_range = tail.as_ref().map(|t| t.syntax().text_range());
    for child in list.syntax().children_with_tokens() {
        acc.elements += 1;
        match child {
            NodeOrToken::Token(t) => {
                if t.kind() == SyntaxKind::SEMICOLON {
                    markt(acc, &t);
                }
            }
            NodeOrToken::Node(n) => {
                if let Some(l) = ast::LetStmt::cast(n.clone()) {
                    let (lo, hi) = rustc_span(&n);
                    acc.mix(u64::from(lo) << 32 | u64::from(hi));
                    opt!(acc, l.pat());
                    opt!(acc, l.ty());
                    opt!(acc, l.initializer());
                    opt!(acc, l.let_else());
                } else if let Some(es) = ast::ExprStmt::cast(n.clone()) {
                    let (lo, hi) = rustc_span(&n);
                    acc.mix(u64::from(lo) << 32 | u64::from(hi));
                    let has_semi = has_token(&n, SyntaxKind::SEMICOLON);
                    acc.mix(u64::from(has_semi));
                    opt!(acc, es.expr());
                } else if ast::Item::can_cast(n.kind()) || tail_range == Some(n.text_range()) {
                    let (lo, hi) = rustc_span(&n);
                    acc.mix(u64::from(lo) << 32 | u64::from(hi));
                }
            }
        }
    }
}

/// `types::rewrite_path_segments`: the segments of a path, innermost qualifier first.
fn visit_path(path: &ast::Path, acc: &mut Acc) {
    let mut segments = Vec::new();
    let mut cur = Some(path.clone());
    while let Some(p) = cur {
        if let Some(seg) = p.segment() {
            segments.push(seg);
        } else {
            miss(acc);
        }
        cur = p.qualifier();
    }
    for seg in segments.iter().rev() {
        optt!(acc, seg.coloncolon_token());
        match seg.name_ref() {
            Some(name) => {
                mark(acc, &name.syntax());
                match name.syntax().first_token() {
                    Some(t) => mark_text(acc, token_text(&t)),
                    None => miss(acc),
                }
            }
            None => miss(acc),
        }
        match seg.generic_arg_list() {
            Some(list) => {
                mark(acc, &list.syntax());
                for arg in list.generic_args() {
                    mark(acc, &arg.syntax());
                    if let ast::GenericArg::TypeArg(t) = &arg {
                        opt!(acc, t.ty());
                    }
                }
            }
            None => miss(acc),
        }
        opt!(acc, seg.parenthesized_arg_list());
        opt!(acc, seg.ret_type());
    }
}
