// /// Check if a string contains any lowercase ASCII characters
// fn has_lowercase(s: &str) -> bool {
//     s.as_bytes().iter().any(|&b| b.is_ascii_lowercase())
// }
//
// /// Generate a sort key that matches rustfmt's import sorting behavior
// ///
// /// Key 0: `self` always sorts first
// /// Key 1: ALL_CAPS identifiers sort last
// /// Key 2: identifiers with uppercase initial sort after ones with lowercase initial
// /// Key 3: then by full string ASCII order
// // pub fn sort_key(s: &str) -> (u8, bool, String, &str) {
// pub fn sort_key(s: &str) -> (u8, String, &str) {
//     // `self` always comes first
//     if s == "self" {
//         // return (0, false, "".to_string(), s);
//         return (0, "".to_string(), s);
//     }
//
//     // let first_is_lower = s
//     //     .as_bytes()
//     //     .first()
//     //     .is_some_and(|&b| b.is_ascii_lowercase());
//
//     // // Skip expensive scan if first char is lowercase (guarantees it has lowercase)
//     // let is_all_caps = if first_is_lower {
//     //     false
//     // } else {
//     //     !has_lowercase(s)
//     // };
//
//     // (1, is_all_caps, !first_is_lower, s) pre-2024
//     (1, s.to_ascii_lowercase(), s) // 2024
// }

pub fn sort_key(s: &str) -> (bool, Vec<(u8, char)>) {
    (
        s != "self",
        s.chars()
            .map(|c| match c {
                '_' => (0, c),
                'A'..='Z' => (1, c),
                'a'..='z' => (2, c.to_ascii_uppercase()),
                _ => (3, c),
            })
            .collect(),
    )
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
    #[test]
    fn test_sort_key_self_first() {
        let mut items = vec!["Foo", "self", "bar", "Baz"];
        items.sort_by_key(|&s| sort_key(s));
        assert_eq!(items, vec!["self", "bar", "Baz", "Foo"]);
    }
    #[test]
    fn test_sort_key_self_with_all_caps() {
        let mut items = vec!["ALL_CAPS", "self", "lower", "Upper"];
        items.sort_by_key(|&s| sort_key(s));
        assert_eq!(items, vec!["self", "lower", "Upper", "ALL_CAPS"]);
    }
}
