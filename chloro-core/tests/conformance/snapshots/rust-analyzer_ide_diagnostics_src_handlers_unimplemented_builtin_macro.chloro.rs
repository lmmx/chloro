use crate::{Diagnostic, DiagnosticCode, DiagnosticsContext, Severity};
pub(crate) fn unimplemented_builtin_macro(ctx: &DiagnosticsContext<'_>, d: &hir::UnimplementedBuiltinMacro) -> Diagnostic {
    Diagnostic::new_with_syntax_node_ptr(
        ctx,
        DiagnosticCode::Ra("unimplemented-builtin-macro", Severity::WeakWarning),
        "unimplemented built-in macro".to_owned(),
        d.node,
    )
    .stable()
}
