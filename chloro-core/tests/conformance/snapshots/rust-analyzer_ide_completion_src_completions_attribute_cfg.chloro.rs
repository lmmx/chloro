//! Completion for cfg

use ide_db::SymbolKind;
use itertools::Itertools;
use syntax::{AstToken, Direction, NodeOrToken, SyntaxKind, algo, ast::Ident};

use crate::{CompletionItem, completions::Completions, context::CompletionContext};
pub(crate) fn complete_cfg(acc: &mut Completions, ctx: &CompletionContext<'_>) {
    let add_completion = |item: &str| {
        let mut completion =
            CompletionItem::new(SymbolKind::BuiltinAttr, ctx.source_range(), item, ctx.edition);
        completion.insert_text(format!(r#""{item}""#));
        acc.add(completion.build(ctx.db));
    };
    // FIXME: Move this into context/analysis.rs
    let previous = ctx
        .original_token
        .prev_token()
        .and_then(|it| {
            if matches!(it.kind(), SyntaxKind::EQ) {
                Some(it.into())
            } else {
                algo::non_trivia_sibling(it.into(), Direction::Prev)
            }
        })
        .filter(|t| matches!(t.kind(), SyntaxKind::EQ))
        .and_then(|it| algo::non_trivia_sibling(it.prev_sibling_or_token()?, Direction::Prev))
        .map(|it| match it {
            NodeOrToken::Node(_) => None,
            NodeOrToken::Token(t) => Ident::cast(t),
        });
    match previous {
        Some(None) => (),
        Some(Some(p)) => match p.text() {
            "target_arch" => KNOWN_ARCH.iter().copied().for_each(add_completion),
            "target_env" => KNOWN_ENV.iter().copied().for_each(add_completion),
            "target_os" => KNOWN_OS.iter().copied().for_each(add_completion),
            "target_vendor" => KNOWN_VENDOR.iter().copied().for_each(add_completion),
            "target_endian" => ["little", "big"].into_iter().for_each(add_completion),
            name => ctx.krate.potential_cfg(ctx.db).get_cfg_values(name).for_each(|s| {
                let s = s.as_str();
                let insert_text = format!(r#""{s}""#);
                let mut item = CompletionItem::new(
                    SymbolKind::BuiltinAttr,
                    ctx.source_range(),
                    s,
                    ctx.edition,
                );
                item.insert_text(insert_text);

                acc.add(item.build(ctx.db));
            }),
        },
        None => ctx
            .krate
            .potential_cfg(ctx.db)
            .get_cfg_keys()
            .unique()
            .map(|s| (s.as_str(), ""))
            .chain(CFG_CONDITION.iter().copied())
            .for_each(|(s, snippet)| {
                let mut item = CompletionItem::new(
                    SymbolKind::BuiltinAttr,
                    ctx.source_range(),
                    s,
                    ctx.edition,
                );
                if let Some(cap) = ctx.config.snippet_cap
                    && !snippet.is_empty()
                {
                    item.insert_snippet(cap, snippet);
                }
                acc.add(item.build(ctx.db));
            }),
    }
}





