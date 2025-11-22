/// Check if a string contains any lowercase ASCII characters
fn has_lowercase(s: &str) -> bool {
    s.as_bytes().iter().any(|&b| b.is_ascii_lowercase())
}

/// Generate a sort key that matches rustfmt's import sorting behavior
///
/// Key 1: ALL_CAPS identifiers sort last
/// Key 2: identifiers with uppercase initial sort after ones with lowercase initial
/// Key 3: then by full string ASCII order
pub fn sort_key(s: &str) -> (bool, bool, &str) {
    let first_is_lower = s
        .as_bytes()
        .first()
        .map_or(false, |&b| b.is_ascii_lowercase());

    // Skip expensive scan if first char is lowercase (guarantees it has lowercase)
    let is_all_caps = if first_is_lower {
        false
    } else {
        !has_lowercase(s)
    };

    (is_all_caps, !first_is_lower, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_key_ordering() {
        let mut items = vec!["ALL_CAPS", "Upper", "lower", "another"];
        items.sort_by_key(|&s| sort_key(s));
        assert_eq!(items, vec!["another", "lower", "Upper", "ALL_CAPS"]);
    }
}
