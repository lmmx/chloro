//! Completes environment variables defined by Cargo
//! (<https://doc.rust-lang.org/cargo/reference/environment-variables.html>)

use ide_db::syntax_helpers::node_ext::macro_call_for_string_token;
use syntax::{
    AstToken,
    ast::{self, IsString},
};

use crate::{
    CompletionItem, CompletionItemKind, completions::Completions, context::CompletionContext,
};


pub(crate) fn complete_cargo_env_vars(acc: &mut Completions, ctx: &CompletionContext<'_>, original: &ast::String, expanded: &ast::String) -> Option<()> {
    let is_in_env_expansion = ctx
        .sema
        .hir_file_for(&expanded.syntax().parent()?)
        .macro_file()
        .is_some_and(|it| it.is_env_or_option_env(ctx.sema.db));
    if !is_in_env_expansion {
        let call = macro_call_for_string_token(expanded)?;
        let makro = ctx.sema.resolve_macro_call(&call)?;
        // We won't map into `option_env` as that generates `None` for non-existent env vars
        // so fall back to this lookup
        if !makro.is_env_or_option_env(ctx.sema.db) {
            return None;
        }
    }
    let range = original.text_range_between_quotes()?;
    CARGO_DEFINED_VARS.iter().for_each(|&(var, detail)| {
        let mut item = CompletionItem::new(CompletionItemKind::Keyword, range, var, ctx.edition);
        item.detail(detail);
        item.add_to(acc, ctx.db);
    });
    Some(())
}

#[cfg(test)]
mod tests {
    use crate::tests::{check_edit, completion_list};
    #[test]
    fn completes_env_variable_in_env() {
        check_edit(
            "CARGO_BIN_NAME",
            r#"
//- minicore: env
fn main() {
    let foo = env!("CAR$0");
}
        "#,
            r#"
fn main() {
    let foo = env!("CARGO_BIN_NAME");
}
        "#,
        );
    }
    #[test]
    fn completes_env_variable_in_option_env() {
        check_edit(
            "CARGO_BIN_NAME",
            r#"
//- minicore: env
fn main() {
    let foo = option_env!("CAR$0");
}
        "#,
            r#"
fn main() {
    let foo = option_env!("CARGO_BIN_NAME");
}
        "#,
        );
    }
    #[test]
    fn doesnt_complete_in_random_strings() {
        let fixture = r#"
            fn main() {
                let foo = "CA$0";
            }
        "#;
        let completions = completion_list(fixture);
        assert!(completions.is_empty(), "Completions weren't empty: {completions}");
    }
    #[test]
    fn doesnt_complete_in_random_macro() {
        let fixture = r#"
            macro_rules! bar {
                ($($arg:tt)*) => { 0 }
            }

            fn main() {
                let foo = bar!("CA$0");

            }
        "#;
        let completions = completion_list(fixture);
        assert!(completions.is_empty(), "Completions weren't empty: {completions}");
    }
    #[test]
    fn doesnt_complete_for_shadowed_macro() {
        let fixture = r#"
            macro_rules! env {
                ($var:literal) => { 0 }
            }

            fn main() {
                let foo = env!("CA$0");
            }
        "#;
        let completions = completion_list(fixture);
        assert!(completions.is_empty(), "Completions weren't empty: {completions}")
    }
}
