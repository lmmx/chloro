//! Completion for representations.

use ide_db::SymbolKind;
use syntax::ast;

use crate::{Completions, context::CompletionContext, item::CompletionItem};
pub(super) fn complete_repr(acc: &mut Completions, ctx: &CompletionContext<'_>, input: ast::TokenTree) {
    if let Some(existing_reprs) = super::parse_comma_sep_expr(input) {
        for &ReprCompletion { label, snippet, lookup, collides } in REPR_COMPLETIONS {
            let repr_already_annotated = existing_reprs
                .iter()
                .filter_map(|expr| match expr {
                    ast::Expr::PathExpr(path) => path.path()?.as_single_name_ref(),
                    ast::Expr::CallExpr(call) => match call.expr()? {
                        ast::Expr::PathExpr(path) => path.path()?.as_single_name_ref(),
                        _ => None,
                    },
                    _ => None,
                })
                .any(|it| {
                    let text = it.text();
                    lookup.unwrap_or(label) == text || collides.contains(&text.as_str())
                });
            if repr_already_annotated {
                continue;
            }

            let mut item = CompletionItem::new(
                SymbolKind::BuiltinAttr,
                ctx.source_range(),
                label,
                ctx.edition,
            );
            if let Some(lookup) = lookup {
                item.lookup_by(lookup);
            }
            if let Some((snippet, cap)) = snippet.zip(ctx.config.snippet_cap) {
                item.insert_snippet(cap, snippet);
            }
            item.add_to(acc, ctx.db);
        }
    }
}

struct ReprCompletion {
    label: &'static str,
    snippet: Option<&'static str>,
    lookup: Option<&'static str>,
    collides: &'static [&'static str],
}

const fn attr(label: &'static str, collides: &'static [&'static str]) -> ReprCompletion {
    ReprCompletion { label, snippet: None, lookup: None, collides }
}

#[rustfmt::skip]
