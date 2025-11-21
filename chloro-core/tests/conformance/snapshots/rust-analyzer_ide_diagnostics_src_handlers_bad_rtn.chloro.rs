use ide_db::Severity;

use crate::{Diagnostic, DiagnosticCode, DiagnosticsContext};

pub(crate) fn bad_rtn(
    ctx: &DiagnosticsContext<'_>,
    d: &hir::BadRtn,
) -> Diagnostic {
    Diagnostic::new_with_syntax_node_ptr(
        ctx,
        DiagnosticCode::Ra("bad-rtn", Severity::Error),
        "return type notation not allowed in this position yet",
        d.rtn.map(Into::into),
    )
    .stable()
}

#[cfg(test)]
mod tests {
    use crate::tests::check_diagnostics;
    #[test]
    fn fn_traits_also_emit() {
        check_diagnostics(
            r#"
//- minicore: fn
fn foo<
    A: Fn(..),
      // ^^^^ error: return type notation not allowed in this position yet
>() {}
        "#,
        );
    }
    #[test]
    fn bad_rtn() {
        check_diagnostics(
            r#"
mod module {
    pub struct Type;
}
trait Trait {}

fn foo()
where
    module(..)::Type: Trait
       // ^^^^ error: return type notation not allowed in this position yet
{
}
        "#,
        );
    }
}
