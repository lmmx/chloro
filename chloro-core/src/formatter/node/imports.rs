use ra_ap_syntax::ast::{AstNode, Use};

use crate::formatter::node::format_use;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ImportGroup {
    Std,
    External,
    Internal,
}

fn classify_import(use_: &Use) -> (ImportGroup, String) {
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

pub fn sort_and_format_imports(uses: &[Use], buf: &mut String, indent: usize) {
    // Classify and sort imports
    let mut classified: Vec<_> = uses.iter().map(|u| (classify_import(u), u)).collect();

    // Sort by group first, then by path
    classified.sort_by(|a, b| {
        let (group_a, path_a) = &a.0;
        let (group_b, path_b) = &b.0;
        group_a.cmp(group_b).then_with(|| path_a.cmp(path_b))
    });

    // Format with blank lines between groups
    let mut last_group = None;

    for ((group, _), use_) in classified {
        // Add blank line between groups
        if let Some(last) = last_group {
            if last != group {
                buf.push('\n');
            }
        }

        format_use(use_.syntax(), buf, indent);
        last_group = Some(group);
    }
}
