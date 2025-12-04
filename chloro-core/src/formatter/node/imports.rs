use ra_ap_syntax::ast::{AstNode, Use};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportGroup {
    Std,
    External,
    Internal,
}

pub fn classify_import(use_: &Use) -> (ImportGroup, String) {
    let path = if let Some(tree) = use_.use_tree() {
        tree.syntax().text().to_string()
    } else {
        return (ImportGroup::External, String::new());
    };

    let group =
        if path.starts_with("std::") || path.starts_with("core::") || path.starts_with("alloc::") {
            ImportGroup::Std
        } else if path.starts_with("crate::")
            || path.starts_with("self::")
            || path.starts_with("super::")
        {
            ImportGroup::Internal
        } else {
            ImportGroup::External
        };

    (group, path)
}
