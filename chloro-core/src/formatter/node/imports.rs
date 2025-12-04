use ra_ap_syntax::ast::{AstNode, Use};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportGroup {
    Internal(InternalKind), // self::, super::, crate::, - sorted first
    External,               // everything else (including std, core, alloc)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InternalKind {
    Self_,
    Super,
    Crate,
}

pub fn classify_import(use_: &Use) -> (ImportGroup, String) {
    let path = if let Some(tree) = use_.use_tree() {
        tree.syntax().text().to_string()
    } else {
        return (ImportGroup::External, String::new());
    };

    let group = if path.starts_with("self::") {
        ImportGroup::Internal(InternalKind::Self_)
    } else if path.starts_with("super::") {
        ImportGroup::Internal(InternalKind::Super)
    } else if path.starts_with("crate::") {
        ImportGroup::Internal(InternalKind::Crate)
    } else {
        ImportGroup::External
    };

    (group, path)
}
