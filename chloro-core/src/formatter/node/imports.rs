use ra_ap_syntax::ast::{AstNode, AstToken, Comment, Use};

use crate::formatter::node::format_use;

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

pub fn sort_and_format_imports(
    use_items: &[(Vec<Comment>, Use, Vec<Comment>)],
    buf: &mut String,
    indent: usize,
) {
    // Sort the use statements by group and path
    let mut sorted_uses = use_items.to_vec();
    sorted_uses.sort_by(|(_, a, _), (_, b, _)| {
        let (group_a, path_a) = classify_import(a);
        let (group_b, path_b) = classify_import(b);

        group_a.cmp(&group_b).then_with(|| path_a.cmp(&path_b))
    });

    let mut last_group = None;
    for (before_comments, use_, trailing_comments) in sorted_uses {
        // Check if we need a blank line between groups
        let (group, _) = classify_import(&use_);
        if let Some(last) = last_group {
            if last != group {
                buf.push('\n');
            }
        }
        last_group = Some(group);

        // Output preceding comments
        for comment in &before_comments {
            buf.push_str(comment.text());
            buf.push('\n');
        }

        // Output the use statement
        format_use(use_.syntax(), buf, indent);

        // Output trailing comments
        for comment in &trailing_comments {
            buf.push_str(comment.text());
            buf.push('\n');
        }
    }
}
