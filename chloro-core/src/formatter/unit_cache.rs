//! A cache of formatted top-level units, for re-formatting a file after an edit.
//!
//! A unit is what `parallel.rs` formats on its own: one top-level item, or one group of
//! consecutive `use`, `mod x;` or `extern crate` items that `reorder.rs` sorts together. A
//! unit's output is a function of its own source text, the configuration and the file's
//! `#![rustfmt::skip::macros(..)]`/`#![rustfmt::skip::attributes(..)]` names
//! (`docs/journal/2026-10-10-item-independence.md`; the dependency table in
//! `docs/journal/2026-10-15-extensions-theory/incremental-and-process.md` §1). The text
//! between units, the file-level checks (parse errors, constructs rustc rejects,
//! `#![rustfmt::skip]`, `disable_all_formatting`) and the trailing-newline and line-ending
//! fix-ups are not cached: they are recomputed from the fresh parse on every call.
//!
//! [`format_source_with_unit_cache`](crate::format_source_with_unit_cache) looks each unit
//! up by a [`UnitKey`] (the context bytes and the unit's text), formats only the units it
//! does not find, stores what it formatted, and stitches the result exactly as the parallel
//! path does. A [`CachedUnit`] holds every position relative to the unit's start, so a hit
//! applies wherever the unit has moved to in the file.
//!
//! What the key does not identify is the build of chloro-core: a cache that outlives the
//! process must be keyed by the build too (the CLI keys its cache file by the digest of
//! its executable).

use super::config::Config;
use super::parallel::{GapOp, UnitOutput};
use super::span::BytePos;

/// Version of the [`CachedUnit`] layout and of the key's context bytes. Part of every key.
const FORMAT_VERSION: u32 = 1;

/// A store of formatted units. The formatter calls it from the calling thread only.
///
/// An implementation must return from [`get`](UnitCache::get) only a value that
/// [`insert`](UnitCache::insert) was given for a key with equal [`UnitKey::context`] and
/// [`UnitKey::text`] (for example by hashing both with a collision-resistant hash).
pub trait UnitCache {
    /// The unit stored for `key`, if any.
    fn get(&self, key: &UnitKey<'_>) -> Option<CachedUnit>;
    /// Stores `unit` for `key`.
    fn insert(&self, key: &UnitKey<'_>, unit: CachedUnit);
}

/// What a unit's output depends on, apart from the build of chloro-core.
#[derive(Clone, Copy, Debug)]
pub struct UnitKey<'a> {
    context: &'a [u8],
    text: &'a str,
}

impl<'a> UnitKey<'a> {
    /// A key of `context` and `text` (for an implementation's tests; the formatter builds
    /// the keys it looks up).
    pub fn new(context: &'a [u8], text: &'a str) -> UnitKey<'a> {
        UnitKey { context, text }
    }

    /// Everything but the unit's text: the cache format version, the crate version, the
    /// whole configuration ([`Config::key_bytes`]) and the file's skip names. Equal for
    /// every unit of one file.
    pub fn context(&self) -> &'a [u8] {
        self.context
    }

    /// The unit's source text, after the normalisation every entry point applies (no byte
    /// order mark, `\n` line endings), from the start of its first item (outer attributes
    /// and doc comments included) to the end of its last.
    pub fn text(&self) -> &'a str {
        self.text
    }
}

/// The context bytes of a file's keys before the skip names.
pub(crate) fn key_context(config: &Config) -> Vec<u8> {
    let mut out = Vec::with_capacity(128);
    out.extend_from_slice(b"chloro unit cache\0");
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(env!("CARGO_PKG_VERSION").as_bytes());
    out.push(0);
    let config = config.key_bytes();
    out.extend_from_slice(&(config.len() as u64).to_le_bytes());
    out.extend_from_slice(&config);
    out
}

/// The formatted output of one unit, with every position relative to the unit's start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedUnit {
    /// The first gap call (`GapOp` tag) and its position minus the unit's start.
    gap: Option<(u8, i64)>,
    /// The visitor's `last_pos` after the unit, minus the unit's start.
    last_pos: u32,
    /// The source from the unit's end to `last_pos`, when the unit consumed text past its
    /// last item; checked against the source on a hit.
    tail: String,
    /// The unit's text, or `None` when it equals the source from the unit's start to
    /// `last_pos` (a unit already formatted), which a hit reads from the source.
    text: Option<String>,
    macro_failure: bool,
    lost_comment: bool,
    format_failure: bool,
    /// The line ranges left as written (`RunState::skipped_range`), as recorded: source
    /// lines of the file, or output lines of a nested visitor's buffer, or, for the indices
    /// in `output_line_ranges`, output lines of the unit's own text, which the stitching
    /// shifts by the line the unit starts at in the output.
    skipped_range: Vec<(usize, usize)>,
    output_line_ranges: Vec<usize>,
    /// The 1-based source line of the unit's start when `skipped_range` is not empty, else
    /// 0: those ranges hold absolute source lines, so the record applies only to a unit
    /// starting on the same line.
    line: usize,
}

const FLAG_MACRO_FAILURE: u8 = 1;
const FLAG_LOST_COMMENT: u8 = 2;
const FLAG_FORMAT_FAILURE: u8 = 4;
const FLAG_GAP: u8 = 8;
const FLAG_TEXT: u8 = 16;

impl CachedUnit {
    /// The record of `out`, the output of the unit spanning `lo..hi` of `source`, which
    /// starts on source line `line_of_lo()`; `None` for an output that cannot be stitched
    /// (`broken`).
    pub(crate) fn from_output(
        out: &UnitOutput,
        source: &str,
        lo: BytePos,
        hi: BytePos,
        line_of_lo: impl FnOnce() -> usize,
    ) -> Option<CachedUnit> {
        if out.broken || out.last_pos < lo || u32::try_from(out.text.len()).is_err() {
            return None;
        }
        let tail = if out.last_pos > hi {
            source.get(hi as usize..out.last_pos as usize)?.to_owned()
        } else {
            String::new()
        };
        let verbatim = source.get(lo as usize..out.last_pos as usize) == Some(out.text.as_str());
        let gap = out.gap.map(|op| {
            let (tag, pos) = match op {
                GapOp::WithIndent(p) => (0, p),
                GapOp::NoIndent(p) => (1, p),
                GapOp::WithoutIndent(p) => (2, p),
                GapOp::RemovedImport(p) => (3, p),
            };
            (tag, i64::from(pos) - i64::from(lo))
        });
        Some(CachedUnit {
            gap,
            last_pos: out.last_pos - lo,
            tail,
            text: (!verbatim).then(|| out.text.clone()),
            macro_failure: out.macro_failure,
            lost_comment: out.lost_comment,
            format_failure: out.format_failure,
            skipped_range: out.skipped_range.clone(),
            output_line_ranges: out.output_line_ranges.clone(),
            line: if out.skipped_range.is_empty() {
                0
            } else {
                line_of_lo()
            },
        })
    }

    /// The output this record gives for the unit spanning `lo..hi` of `source`, which
    /// starts on source line `line_of_lo()`, with its positions moved to `lo`; `None` when a
    /// position falls outside the source, the source past `hi` is not the recorded tail, or
    /// the record holds skipped ranges and the unit has moved to another line (the caller
    /// then formats the unit).
    pub(crate) fn to_output(
        &self,
        source: &str,
        lo: BytePos,
        hi: BytePos,
        line_of_lo: impl FnOnce() -> usize,
    ) -> Option<UnitOutput> {
        if self.line != 0 && line_of_lo() != self.line {
            return None;
        }
        let last_pos = lo.checked_add(self.last_pos)?;
        if last_pos > hi {
            if source.get(hi as usize..last_pos as usize)? != self.tail {
                return None;
            }
        } else if !self.tail.is_empty() {
            return None;
        }
        let gap = match self.gap {
            None => None,
            Some((tag, delta)) => {
                let pos = BytePos::try_from(i64::from(lo) + delta).ok()?;
                if pos as usize > source.len() {
                    return None;
                }
                Some(match tag {
                    0 => GapOp::WithIndent(pos),
                    1 => GapOp::NoIndent(pos),
                    2 => GapOp::WithoutIndent(pos),
                    3 => GapOp::RemovedImport(pos),
                    _ => return None,
                })
            }
        };
        let text = match &self.text {
            Some(text) => text.clone(),
            None => source.get(lo as usize..last_pos as usize)?.to_owned(),
        };
        Some(UnitOutput {
            gap,
            text,
            last_pos,
            macro_failure: self.macro_failure,
            lost_comment: self.lost_comment,
            format_failure: self.format_failure,
            skipped_range: self.skipped_range.clone(),
            output_line_ranges: self.output_line_ranges.clone(),
            broken: false,
        })
    }

    /// Bytes held on the heap by the record (its strings).
    pub fn heap_size(&self) -> usize {
        self.tail.capacity()
            + self.text.as_ref().map_or(0, String::capacity)
            + self.skipped_range.capacity() * size_of::<(usize, usize)>()
            + self.output_line_ranges.capacity() * size_of::<usize>()
    }

    /// A self-contained serialisation, for a cache kept on disk; read back by
    /// [`from_bytes`](CachedUnit::from_bytes).
    pub fn to_bytes(&self) -> Vec<u8> {
        let text_len = self.text.as_ref().map_or(0, String::len);
        let mut out = Vec::with_capacity(26 + self.tail.len() + text_len);
        let mut flags = 0;
        if self.macro_failure {
            flags |= FLAG_MACRO_FAILURE;
        }
        if self.lost_comment {
            flags |= FLAG_LOST_COMMENT;
        }
        if self.format_failure {
            flags |= FLAG_FORMAT_FAILURE;
        }
        if self.gap.is_some() {
            flags |= FLAG_GAP;
        }
        if self.text.is_some() {
            flags |= FLAG_TEXT;
        }
        out.push(flags);
        let (tag, delta) = self.gap.unwrap_or((0, 0));
        out.push(tag);
        out.extend_from_slice(&delta.to_le_bytes());
        out.extend_from_slice(&self.last_pos.to_le_bytes());
        out.extend_from_slice(&(self.tail.len() as u32).to_le_bytes());
        out.extend_from_slice(self.tail.as_bytes());
        if let Some(text) = &self.text {
            out.extend_from_slice(&(text.len() as u32).to_le_bytes());
            out.extend_from_slice(text.as_bytes());
        }
        out.extend_from_slice(&(self.line as u64).to_le_bytes());
        out.extend_from_slice(&(self.skipped_range.len() as u64).to_le_bytes());
        for &(lo, hi) in &self.skipped_range {
            out.extend_from_slice(&(lo as u64).to_le_bytes());
            out.extend_from_slice(&(hi as u64).to_le_bytes());
        }
        out.extend_from_slice(&(self.output_line_ranges.len() as u64).to_le_bytes());
        for &i in &self.output_line_ranges {
            out.extend_from_slice(&(i as u64).to_le_bytes());
        }
        out
    }

    /// Reads a record written by [`to_bytes`](CachedUnit::to_bytes); `None` for anything
    /// else (a truncated or corrupted record, text that is not UTF-8, trailing bytes).
    pub fn from_bytes(bytes: &[u8]) -> Option<CachedUnit> {
        let mut rest = bytes;
        let mut take = |n: usize| -> Option<&[u8]> {
            let (head, tail) = rest.split_at_checked(n)?;
            rest = tail;
            Some(head)
        };
        let flags = take(1)?[0];
        if flags
            & !(FLAG_MACRO_FAILURE | FLAG_LOST_COMMENT | FLAG_FORMAT_FAILURE | FLAG_GAP | FLAG_TEXT)
            != 0
        {
            return None;
        }
        let tag = take(1)?[0];
        let delta = i64::from_le_bytes(take(8)?.try_into().ok()?);
        let last_pos = u32::from_le_bytes(take(4)?.try_into().ok()?);
        let tail_len = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
        let tail = String::from_utf8(take(tail_len)?.to_vec()).ok()?;
        let text = if flags & FLAG_TEXT != 0 {
            let len = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
            Some(String::from_utf8(take(len)?.to_vec()).ok()?)
        } else {
            None
        };
        let mut word = || -> Option<usize> {
            usize::try_from(u64::from_le_bytes(take(8)?.try_into().ok()?)).ok()
        };
        let line = word()?;
        let n = word()?;
        let mut skipped_range = Vec::new();
        for _ in 0..n {
            skipped_range.push((word()?, word()?));
        }
        let n = word()?;
        let mut output_line_ranges = Vec::new();
        for _ in 0..n {
            output_line_ranges.push(word()?);
        }
        if !rest.is_empty() || tag > 3 || (flags & FLAG_GAP == 0 && (tag, delta) != (0, 0)) {
            return None;
        }
        Some(CachedUnit {
            gap: (flags & FLAG_GAP != 0).then_some((tag, delta)),
            last_pos,
            tail,
            text,
            macro_failure: flags & FLAG_MACRO_FAILURE != 0,
            lost_comment: flags & FLAG_LOST_COMMENT != 0,
            format_failure: flags & FLAG_FORMAT_FAILURE != 0,
            skipped_range,
            output_line_ranges,
            line,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use super::*;
    use crate::{format_source_with_config, format_source_with_unit_cache};

    /// An exact cache that also keeps every key's context and every record it was given.
    #[derive(Default)]
    struct Recording {
        map: RefCell<HashMap<(Vec<u8>, String), CachedUnit>>,
        contexts: RefCell<Vec<Vec<u8>>>,
        hits: RefCell<usize>,
    }

    impl UnitCache for Recording {
        fn get(&self, key: &UnitKey<'_>) -> Option<CachedUnit> {
            self.contexts.borrow_mut().push(key.context().to_vec());
            let found = self
                .map
                .borrow()
                .get(&(key.context().to_vec(), key.text().to_owned()))
                .cloned();
            if found.is_some() {
                *self.hits.borrow_mut() += 1;
            }
            found
        }

        fn insert(&self, key: &UnitKey<'_>, unit: CachedUnit) {
            self.map
                .borrow_mut()
                .insert((key.context().to_vec(), key.text().to_owned()), unit);
        }
    }

    impl Recording {
        fn record(&self, text: &str) -> CachedUnit {
            let map = self.map.borrow();
            let mut found = map.iter().filter(|((_, t), _)| t == text).map(|(_, u)| u);
            let unit = found.next().expect("a record for the text").clone();
            assert!(found.next().is_none());
            unit
        }

        fn take_hits(&self) -> usize {
            std::mem::take(&mut *self.hits.borrow_mut())
        }
    }

    const FILE: &str = "\
//! A file.

use std::fmt;
use std::io;

/// A function.
fn a(x: u32) -> u32 {
    x + 1
}

struct S {
    a: u32,
}

fn b(){let y=2;}
";

    #[test]
    fn cold_and_warm_passes_give_the_reference() {
        let config = Config::default();
        let expected = format_source_with_config(FILE, &config);
        let cache = Recording::default();
        for threads in [1, 4] {
            assert_eq!(
                format_source_with_unit_cache(FILE, &config, threads, &cache),
                expected
            );
        }
        // `use` group, `a`, `S`, `b`: four units, all found by the second pass.
        assert_eq!(cache.map.borrow().len(), 4);
        assert_eq!(cache.take_hits(), 4);
    }

    #[test]
    fn the_key_covers_the_configuration_and_the_skip_names() {
        let cache = Recording::default();
        let mut two = Config::default();
        two.set("tab_spaces", "2").unwrap();
        let skipping = format!("#![rustfmt::skip::macros(foo)]\n{FILE}");
        let _ = format_source_with_unit_cache(FILE, &Config::default(), 1, &cache);
        let _ = format_source_with_unit_cache(FILE, &two, 1, &cache);
        let _ = format_source_with_unit_cache(&skipping, &Config::default(), 1, &cache);
        {
            let contexts = cache.contexts.borrow();
            assert_eq!(contexts.len(), 12);
            let (default, two, skip) = (&contexts[0], &contexts[4], &contexts[8]);
            assert!(contexts[..4].iter().all(|c| c == default));
            assert_ne!(default, two);
            assert_ne!(default, skip);
            assert!(
                default.ends_with(&[0; 16]),
                "no skip names: two zero counts"
            );
        }
        // Nothing was found across the three: each key differs.
        assert_eq!(cache.take_hits(), 0);
        assert_eq!(
            format_source_with_unit_cache(&skipping, &Config::default(), 1, &cache),
            format_source_with_config(&skipping, &Config::default())
        );
    }

    #[test]
    fn records_are_relative_to_the_unit_and_rebased_on_a_hit() {
        let config = Config::default();
        let cache = Recording::default();
        let _ = format_source_with_unit_cache(FILE, &config, 1, &cache);
        let fn_a = "/// A function.\nfn a(x: u32) -> u32 {\n    x + 1\n}";
        let at_first = cache.record(fn_a);
        // Already formatted: the text is read from the source, and the gap and `last_pos`
        // are offsets from the unit's start.
        assert_eq!(at_first.text, None);
        assert_eq!(at_first.last_pos as usize, fn_a.len());
        assert_eq!(at_first.gap, Some((0, 0)));

        // The same units 40 bytes and three lines further down: every one is found, and the
        // output is the reference.
        let moved = FILE.replace(
            "//! A file.\n",
            "//! A file.\n\n// thirty-seven bytes of comment.\n\n",
        );
        assert_eq!(
            format_source_with_unit_cache(&moved, &config, 1, &cache),
            format_source_with_config(&moved, &config)
        );
        assert_eq!(cache.take_hits(), 4);
        assert_eq!(cache.record(fn_a), at_first);

        // An unformatted unit keeps its formatted text.
        let fn_b = cache.record("fn b(){let y=2;}");
        assert_eq!(fn_b.text.as_deref(), Some("fn b() {\n    let y = 2;\n}"));

        let lo = moved.find("/// A function.").unwrap() as BytePos;
        let hi = lo + fn_a.len() as BytePos;
        let out = at_first.to_output(&moved, lo, hi, || 0).unwrap();
        assert_eq!(out.gap, Some(GapOp::WithIndent(lo)));
        assert_eq!(out.last_pos, hi);
        assert_eq!(out.text, fn_a);
    }

    #[test]
    fn a_tail_is_checked_against_the_source() {
        let unit = CachedUnit {
            gap: Some((1, 0)),
            last_pos: 5,
            tail: ";".to_owned(),
            text: Some("x!()".to_owned()),
            macro_failure: false,
            lost_comment: false,
            format_failure: false,
            skipped_range: Vec::new(),
            output_line_ranges: Vec::new(),
            line: 0,
        };
        let out = unit.to_output("  x!();", 2, 6, || 1).unwrap();
        assert_eq!((out.last_pos, out.gap), (7, Some(GapOp::NoIndent(2))));
        assert!(unit.to_output("  x!(),", 2, 6, || 1).is_none());
        // Past the end of the source.
        assert!(unit.to_output("  x!()", 2, 6, || 1).is_none());
        // A gap before the start of the source.
        let before = CachedUnit {
            gap: Some((0, -3)),
            ..unit.clone()
        };
        assert!(before.to_output("  x!();", 2, 6, || 1).is_none());
    }

    #[test]
    fn skipped_ranges_pin_the_record_to_its_line() {
        let config = Config::default();
        let source = "fn a() {}\n\nfoo!(a b c d e f g h i j k l m n o p q r s t u v w x y z a b c d e f g h i j k l m n o p q r s t u v w x y z);\n";
        let cache = Recording::default();
        assert_eq!(
            format_source_with_unit_cache(source, &config, 1, &cache),
            format_source_with_config(source, &config)
        );
        let text = source.lines().nth(2).unwrap();
        let record = cache.record(text);
        assert!(
            !record.skipped_range.is_empty(),
            "the macro call is left as written"
        );
        assert_eq!(record.line, 3);
        let _ = format_source_with_unit_cache(source, &config, 1, &cache);
        assert_eq!(cache.take_hits(), 2);
        // One line down: found, not used, formatted again and stored for the new line.
        let moved = format!("\n{source}");
        assert_eq!(
            format_source_with_unit_cache(&moved, &config, 1, &cache),
            format_source_with_config(&moved, &config)
        );
        assert_eq!(cache.record(text).line, 4);
    }

    #[test]
    fn records_round_trip_through_bytes() {
        let config = Config::default();
        let cache = Recording::default();
        let moved = format!(
            "\n{FILE}fn c(){{x!(a b c d e f g h i j k l m n o p q r s t u v w x y z a b c d e f g h i j k l m n o p q r s t u v w x y z);}}\n"
        );
        let _ = format_source_with_unit_cache(&moved, &config, 1, &cache);
        let map = cache.map.borrow();
        assert!(map.len() >= 5);
        for unit in map.values() {
            let bytes = unit.to_bytes();
            assert_eq!(CachedUnit::from_bytes(&bytes).as_ref(), Some(unit));
            for cut in 0..bytes.len() {
                assert_eq!(CachedUnit::from_bytes(&bytes[..cut]), None, "{cut}");
            }
            let mut longer = bytes.clone();
            longer.push(0);
            assert_eq!(CachedUnit::from_bytes(&longer), None);
            let mut flags = bytes.clone();
            flags[0] |= 0x80;
            assert_eq!(CachedUnit::from_bytes(&flags), None);
        }
    }
}
