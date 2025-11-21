//! Completes function abi strings.

use syntax::{
    AstNode, AstToken, SmolStr,
    ast::{self, IsString},
};

use crate::{
    CompletionItem, CompletionItemKind, completions::Completions, context::CompletionContext,
};

// Most of these are feature gated, we should filter/add feature gate completions once we have them.

pub(crate) fn complete_extern_abi(
    acc: &mut Completions,
    ctx: &CompletionContext<'_>,
    expanded: &ast::String,
) -> Option<()> {
    if !expanded.syntax().parent().is_some_and(|it| ast::Abi::can_cast(it.kind())) {
        return None;
    }
    let abi_str = expanded;
    let source_range = abi_str.text_range_between_quotes()?;
    for &abi in SUPPORTED_CALLING_CONVENTIONS {
        CompletionItem::new(
            CompletionItemKind::Keyword,
            source_range,
            SmolStr::new_static(abi),
            ctx.edition,
        )
        .add_to(acc, ctx.db);
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use expect_test::expect;
    use crate::tests::{check_edit, check_no_kw};
    #[test]
    fn only_completes_in_string_literals() {
        check_no_kw(
            r#"
$0 fn foo {}
"#,
            expect![[]],
        );
    }
    #[test]
    fn requires_extern_prefix() {
        check_no_kw(
            r#"
"$0" fn foo {}
"#,
            expect![[]],
        );
    }
    #[test]
    fn works() {
        check_no_kw(
            r#"
extern "$0" fn foo {}
"#,
            expect![[]],
        );
        check_edit(
            "Rust",
            r#"
extern "$0" fn foo {}
"#,
            r#"
extern "Rust" fn foo {}
"#,
        );
    }
}
