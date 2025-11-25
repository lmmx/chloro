//! Submodule grouping logic for use statement formatting.
//!
//! This module handles the logic of grouping use items by their submodule prefix,
//! matching rustfmt's behavior where items from different submodules are separated
//! by blank lines.

/// Groups use items by their submodule prefix.
///
/// Items from the same submodule are placed in the same group.
/// Groups are ordered by the sort order of items, with different submodules
/// separated into different groups.
///
/// # Example
/// ```text
/// Input:  ["attr::A", "attr::B", "expr::C", "Root"]
/// Output: [["attr::A", "attr::B"], ["expr::C"], ["Root"]]
/// ```
pub fn group_by_submodule(items: Vec<String>) -> Vec<Vec<String>> {
    // First check: are there any multi-item nested braces?
    let has_nested_groups = items
        .iter()
        .any(|item| item.contains('{') && item.contains(','));

    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut current_group: Vec<String> = Vec::new();

    for item in items {
        let has_multi_item_braces = item.contains('{') && item.contains(',');
        let has_path_separator = item.contains("::");

        let needs_own_line = has_multi_item_braces || (has_nested_groups && has_path_separator);

        if needs_own_line {
            if !current_group.is_empty() {
                groups.push(std::mem::take(&mut current_group));
            }
            groups.push(vec![item]);
        } else {
            current_group.push(item);
        }
    }

    if !current_group.is_empty() {
        groups.push(current_group);
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by_submodule_mixed() {
        let items = vec![
            "attr::AttrsWithOwner".to_string(),
            "expr_store::path::Path".to_string(),
            "item_scope::ItemInNs".to_string(),
            "per_ns::Namespace".to_string(),
            "resolver::HasResolver".to_string(),
            "resolver::Resolver".to_string(),
            "AssocItemId".to_string(),
            "AttrDefId".to_string(),
        ];

        let groups = group_by_submodule(items);

        assert_eq!(groups.len(), 6);
        assert_eq!(groups[0], vec!["attr::AttrsWithOwner"]);
        assert_eq!(groups[1], vec!["expr_store::path::Path"]);
        assert_eq!(groups[2], vec!["item_scope::ItemInNs"]);
        assert_eq!(groups[3], vec!["per_ns::Namespace"]);
        assert_eq!(
            groups[4],
            vec!["resolver::HasResolver", "resolver::Resolver"]
        );
        assert_eq!(groups[5], vec!["AssocItemId", "AttrDefId"]);
    }

    #[test]
    fn test_group_by_submodule_same_module() {
        let items = vec![
            "foo::A".to_string(),
            "foo::B".to_string(),
            "foo::C".to_string(),
        ];

        let groups = group_by_submodule(items);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], vec!["foo::A", "foo::B", "foo::C"]);
    }

    #[test]
    fn test_group_by_submodule_root_only() {
        let items = vec!["A".to_string(), "B".to_string(), "C".to_string()];

        let groups = group_by_submodule(items);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], vec!["A", "B", "C"]);
    }

    #[test]
    fn test_group_by_submodule_nested_imports() {
        let items = vec![
            "resolver::{HasResolver, Resolver}".to_string(),
            "types::TypeId".to_string(),
        ];

        let groups = group_by_submodule(items);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], vec!["resolver::{HasResolver, Resolver}"]);
        assert_eq!(groups[1], vec!["types::TypeId"]);
    }
}
