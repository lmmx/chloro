//! Version sorting, as specified by the Rust Style Guide for the 2024 style edition
//! (rustfmt's `sort.rs`).

use std::cmp::Ordering;

/// Iterator which breaks an identifier into various [`VersionChunk`]s.
struct VersionChunkIter<'a> {
    ident: &'a str,
    start: usize,
}

impl<'a> VersionChunkIter<'a> {
    fn new(ident: &'a str) -> Self {
        Self { ident, start: 0 }
    }

    fn take_chunk(
        &mut self,
        mut chars: std::str::CharIndices<'a>,
        is_end: impl Fn(char) -> bool,
    ) -> &'a str {
        let mut end = None;
        for (idx, c) in chars.by_ref() {
            if is_end(c) {
                end = Some(self.start + idx);
                break;
            }
        }
        let end = end.unwrap_or(self.ident.len());
        let value = &self.ident[self.start..end];
        self.start = end;
        value
    }
}

impl<'a> Iterator for VersionChunkIter<'a> {
    type Item = VersionChunk<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut chars = self.ident[self.start..].char_indices();
        let (_, next) = chars.next()?;

        if next == '_' {
            self.start += next.len_utf8();
            return Some(VersionChunk::Underscore);
        }

        if next.is_ascii_digit() {
            let source = self.take_chunk(chars, |c| !c.is_ascii_digit());
            let zeros = source.chars().take_while(|c| *c == '0').count();
            // As in rustfmt, a digit run too long for `usize` ends the iteration.
            let value = source.parse::<usize>().ok()?;
            return Some(VersionChunk::Number {
                value,
                zeros,
                source,
            });
        }

        Some(VersionChunk::Str(
            self.take_chunk(chars, |c| c == '_' || c.is_ascii_digit()),
        ))
    }
}

/// Represents a chunk in the version-sort algorithm
#[derive(Debug, PartialEq, Eq)]
enum VersionChunk<'a> {
    /// A single `_` in an identifier. Underscores are sorted before all other characters.
    Underscore,
    /// A &str chunk in the version sort.
    Str(&'a str),
    /// A numeric chunk in the version sort. Keeps track of the numeric value and leading zeros.
    Number {
        value: usize,
        zeros: usize,
        source: &'a str,
    },
}

/// Determine which side of the version-sort comparison had more leading zeros.
#[derive(Debug, PartialEq, Eq)]
enum MoreLeadingZeros {
    Left,
    Right,
    Equal,
}

/// Compare two identifiers based on the version sorting algorithm described in
/// [the style guide](https://doc.rust-lang.org/nightly/style-guide/#sorting).
pub(crate) fn version_sort(a: &str, b: &str) -> Ordering {
    let mut iter_a = VersionChunkIter::new(a);
    let mut iter_b = VersionChunkIter::new(b);
    let mut more_leading_zeros = MoreLeadingZeros::Equal;

    loop {
        let (ca, cb) = match (iter_a.next(), iter_b.next()) {
            (None, None) => break,
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (Some(ca), Some(cb)) => (ca, cb),
        };
        match (ca, cb) {
            (VersionChunk::Underscore, VersionChunk::Underscore) => continue,
            (VersionChunk::Underscore, _) => return Ordering::Less,
            (_, VersionChunk::Underscore) => return Ordering::Greater,
            (
                VersionChunk::Number {
                    value: va,
                    zeros: lza,
                    ..
                },
                VersionChunk::Number {
                    value: vb,
                    zeros: lzb,
                    ..
                },
            ) => match va.cmp(&vb) {
                Ordering::Equal => {
                    if lza == lzb {
                        continue;
                    }
                    if more_leading_zeros == MoreLeadingZeros::Equal {
                        more_leading_zeros = if lza > lzb {
                            MoreLeadingZeros::Left
                        } else {
                            MoreLeadingZeros::Right
                        };
                    }
                    continue;
                }
                order => return order,
            },
            (ca, cb) => {
                let sa = match ca {
                    VersionChunk::Str(s) | VersionChunk::Number { source: s, .. } => s,
                    VersionChunk::Underscore => unreachable!(),
                };
                let sb = match cb {
                    VersionChunk::Str(s) | VersionChunk::Number { source: s, .. } => s,
                    VersionChunk::Underscore => unreachable!(),
                };
                match sa.cmp(sb) {
                    Ordering::Equal => continue,
                    order => return order,
                }
            }
        }
    }

    match more_leading_zeros {
        MoreLeadingZeros::Equal => Ordering::Equal,
        MoreLeadingZeros::Left => Ordering::Less,
        MoreLeadingZeros::Right => Ordering::Greater,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(mut v: Vec<&str>) -> Vec<&str> {
        v.sort_by(|a, b| version_sort(a, b));
        v
    }

    #[test]
    fn chunks() {
        let chunks: Vec<_> = VersionChunkIter::new("x86_128").collect();
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[2], VersionChunk::Underscore);
        assert_eq!(
            VersionChunkIter::new("w005").nth(1),
            Some(VersionChunk::Number {
                value: 5,
                zeros: 2,
                source: "005"
            })
        );
        assert_eq!(
            VersionChunkIter::new("x๙v").next(),
            Some(VersionChunk::Str("x๙v"))
        );
    }

    #[test]
    fn style_guide_examples() {
        assert_eq!(sorted(vec!["", "b", "a"]), ["", "a", "b"]);
        assert_eq!(sorted(vec!["x7x", "xxx"]), ["x7x", "xxx"]);
        assert_eq!(sorted(vec!["applesauce", "apple"]), ["apple", "applesauce"]);
        assert_eq!(sorted(vec!["aaaaa", "aaa_a"]), ["aaa_a", "aaaaa"]);
        assert_eq!(
            sorted(vec!["AAAAA", "AAA1A", "BBBBB", "BB_BB", "C3CCC"]),
            ["AAA1A", "AAAAA", "BB_BB", "BBBBB", "C3CCC"]
        );
        assert_eq!(
            sorted(vec![
                "5", "50", "500", "5_000", "5_005", "5_050", "5_500", "50_000", "50_005", "50_050",
                "50_500",
            ]),
            [
                "5", "5_000", "5_005", "5_050", "5_500", "50", "50_000", "50_005", "50_050",
                "50_500", "500",
            ]
        );
        assert_eq!(
            sorted(vec!["X86_64", "x86_64", "X86_128", "x86_128"]),
            ["X86_64", "X86_128", "x86_64", "x86_128"]
        );
        assert_eq!(sorted(vec!["__", "_"]), ["_", "__"]);
        assert_eq!(sorted(vec!["v010", "v10", "v09"]), ["v09", "v010", "v10"]);
    }
}
