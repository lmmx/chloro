use hir::{EditionedFileId, Semantics, db::DefDatabase};
use ide_db::{FileId, RootDatabase};

pub(crate) fn view_item_tree(
    db: &RootDatabase,
    file_id: FileId,
) -> String {
    let sema = Semantics::new(db);
    let file_id = sema
        .attach_first_edition(file_id)
        .unwrap_or_else(|| EditionedFileId::current_edition(db, file_id));
    db.file_item_tree(file_id.into()).pretty_print(db, file_id.edition(db))
}
