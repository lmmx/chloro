use hir::Semantics;
use ide_db::{FilePosition, RootDatabase};
use syntax::AstNode;

pub(crate) fn view_hir(db: &RootDatabase, position: FilePosition) -> String {
    (|| {
        let sema = Semantics::new(db);
        let source_file = sema.parse_guess_edition(position.file_id);
        sema.debug_hir_at(source_file.syntax().token_at_offset(position.offset).next()?)
    })()
    .unwrap_or_else(|| "Not inside a lowerable item".to_owned())
}
