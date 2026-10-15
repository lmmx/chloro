//! Persistent unit cache for the CLI: formatted top-level units, for files the fixed-point
//! cache does not answer.
//!
//! The fixed-point cache (`cache.rs`) answers a file whose content was seen formatted. A
//! file that changed misses it; this cache then answers every top-level unit of the file
//! that did not change (`chloro_core::format_source_with_unit_cache`), so only the edited
//! units are formatted. The output is the same with and without it.
//!
//! It is opt-in (`--unit-cache`); `--no-cache` turns it off with the fixed-point cache. A run
//! that finds nothing in it pays for hashing and storing every unit and writing the file:
//! +10-15% wall time and +4.6% instructions on a cold `--check` of the 1238 conformance
//! fixtures (`docs/journal/2026-10-14-autoresearch/experiments/EXP-015-item-cache.md`), too
//! much for a default that cold CI runs would pay.
//!
//! # Key
//!
//! An entry's digest is `BLAKE3(len(context) || context || text)` of the library's
//! [`UnitKey`] (the unit's text, the whole configuration, the file's skip names and the
//! library's version). The build of chloro is in the file: the cache file is
//! `<cache-dir>/<32 hex chars>.chloro-units`, named by the fixed-point cache's file key
//! `BLAKE3(tool key || config key)`, whose whole 32 bytes are in the header and checked when
//! the file is read. Another binary or configuration reads another file.
//!
//! # On-disk format
//!
//! A 48-byte header (`CHLOROUC`, format version `u32`, run generation `u32`, file key),
//! then records: a 32-byte digest, the `u32` generation in which it was last stored or found,
//! the `u32` length of the record and the record ([`CachedUnit::to_bytes`]). A file that does
//! not fit this layout, or whose key differs, is ignored and overwritten.
//!
//! The file is read on the first file the fixed-point cache does not answer, so a run on an
//! unchanged tree never reads it. It is written at the end of a run that stored a unit, to a
//! temporary file renamed over it, as the fixed-point cache is, and only when the executable
//! is the one hashed at start-up. The records are bounded to [`MAX_BYTES`] in all; past it,
//! the records seen least recently are dropped.

use super::cache::{Cache, Digest, create_temp};
use chloro_core::{CachedUnit, UnitCache, UnitKey};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// Version of the on-disk layout. A layout change reads a new file.
pub const FORMAT_VERSION: u32 = 1;

/// Suffix of every unit cache file.
pub const FILE_SUFFIX: &str = ".chloro-units";

/// Upper bound on the bytes of the records kept in one unit cache file.
pub const MAX_BYTES: usize = 64 << 20;

const MAGIC: &[u8; 8] = b"CHLOROUC";
/// Magic (8), format version (4), generation (4), file key (32).
const HEADER_LEN: usize = 48;
/// Digest (32), generation (4), record length (4).
const RECORD_HEADER_LEN: usize = 40;

/// Digest of a unit key.
pub fn unit_digest(key: &UnitKey<'_>) -> Digest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(key.context().len() as u64).to_le_bytes());
    hasher.update(key.context());
    hasher.update(key.text().as_bytes());
    *hasher.finalize().as_bytes()
}

/// File name of the unit cache whose key is `key`.
pub fn file_name_of(key: &Digest) -> String {
    let mut name = String::with_capacity(32 + FILE_SUFFIX.len());
    for byte in &key[..16] {
        name.push_str(&format!("{byte:02x}"));
    }
    name.push_str(FILE_SUFFIX);
    name
}

/// The entries of one unit cache file, shared by the worker threads: an implementation of
/// [`UnitCache`] over a `HashMap` keyed by [`unit_digest`]. A run of the CLI uses one
/// generation; a resident server (`server.rs`) keeps the store and starts a generation per
/// request ([`Units::next_generation`]).
pub struct UnitStore {
    generation: AtomicU32,
    entries: Mutex<HashMap<Digest, (u32, CachedUnit)>>,
    /// [`held_size`] of every entry: the memory the entries take, the measure
    /// [`Units::trim`] bounds.
    held: AtomicUsize,
    /// Bytes of the records read from disk.
    file_len: usize,
    lookups: AtomicUsize,
    hits: AtomicUsize,
    stores: AtomicUsize,
    dirty: AtomicBool,
}

impl UnitCache for UnitStore {
    fn get(&self, key: &UnitKey<'_>) -> Option<CachedUnit> {
        let digest = unit_digest(key);
        self.lookups.fetch_add(1, Ordering::Relaxed);
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let (seen, unit) = entries.get_mut(&digest)?;
        *seen = self.generation.load(Ordering::Relaxed);
        self.hits.fetch_add(1, Ordering::Relaxed);
        Some(unit.clone())
    }

    fn insert(&self, key: &UnitKey<'_>, unit: CachedUnit) {
        let digest = unit_digest(key);
        self.stores.fetch_add(1, Ordering::Relaxed);
        self.dirty.store(true, Ordering::Relaxed);
        let size = held_size(&unit);
        let generation = self.generation.load(Ordering::Relaxed);
        let old = self
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(digest, (generation, unit));
        self.held.fetch_add(size, Ordering::Relaxed);
        if let Some((_, old)) = old {
            self.held.fetch_sub(held_size(&old), Ordering::Relaxed);
        }
    }
}

/// Counts of one run, for the verbose report.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UnitStats {
    /// Entries read from disk.
    pub entries_read: usize,
    /// Bytes of the file read.
    pub file_len: usize,
    /// Units looked up.
    pub lookups: usize,
    /// Lookups that found a record (a record found but not usable, e.g. its tail differs,
    /// is counted here and formatted).
    pub hits: usize,
    /// Units formatted and stored.
    pub stores: usize,
    /// Entries held at the end, and the heap bytes of their records.
    pub entries: usize,
    pub record_bytes: usize,
}

/// The bytes an entry is counted for in memory: the map's key and value and the record's
/// heap size. A record whose unit was already formatted holds no text, so its entry is
/// mostly the struct (EXP-021 measured about 0.7 KiB of resident memory per such entry,
/// table growth and the fixed-point entry of the same save included); the file's
/// [`RECORD_HEADER_LEN`] and serialised bytes would undercount it several times.
fn held_size(unit: &CachedUnit) -> usize {
    std::mem::size_of::<(Digest, (u32, CachedUnit))>() + unit.heap_size()
}

impl UnitStore {
    fn new(generation: u32, entries: Entries, file_len: usize) -> UnitStore {
        let held = entries.values().map(|(_, u)| held_size(u)).sum();
        UnitStore {
            generation: AtomicU32::new(generation),
            entries: Mutex::new(entries),
            held: AtomicUsize::new(held),
            file_len,
            lookups: AtomicUsize::new(0),
            hits: AtomicUsize::new(0),
            stores: AtomicUsize::new(0),
            dirty: AtomicBool::new(false),
        }
    }

    fn stats(&self, entries_read: usize) -> UnitStats {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        UnitStats {
            entries_read,
            file_len: self.file_len,
            lookups: self.lookups.load(Ordering::Relaxed),
            hits: self.hits.load(Ordering::Relaxed),
            stores: self.stores.load(Ordering::Relaxed),
            entries: entries.len(),
            record_bytes: entries.values().map(|(_, u)| u.heap_size()).sum(),
        }
    }
}

/// The unit cache of one (tool, config) pair, read on first use.
pub struct Units {
    path: PathBuf,
    key: Digest,
    store: OnceLock<(UnitStore, usize)>,
}

impl Units {
    /// The unit cache next to the fixed-point cache `cache`, not yet read.
    pub fn new(cache: &Cache) -> Units {
        let dir = cache.path().parent().unwrap_or(Path::new("."));
        Units {
            path: dir.join(file_name_of(cache.key())),
            key: *cache.key(),
            store: OnceLock::new(),
        }
    }

    /// Path of the unit cache file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The entries, read from the file on the first call.
    pub fn store(&self) -> &UnitStore {
        &self
            .store
            .get_or_init(|| {
                let bytes = fs::read(&self.path).ok();
                let file_len = bytes.as_ref().map_or(0, Vec::len);
                let (generation, entries) = bytes
                    .and_then(|bytes| parse(&bytes, &self.key))
                    .unwrap_or_default();
                let read = entries.len();
                (
                    UnitStore::new(generation.wrapping_add(1), entries, file_len),
                    read,
                )
            })
            .0
    }

    /// Counts of this run; `None` when the file was never read.
    pub fn stats(&self) -> Option<UnitStats> {
        self.store.get().map(|(store, read)| store.stats(*read))
    }

    /// Writes the file atomically if a unit was stored in this run. `Ok(false)` when there
    /// was nothing to write.
    ///
    /// # Errors
    ///
    /// The first I/O error from creating the directory, writing the temporary file or
    /// renaming it, and an error of kind `Other` when the executable changed since `cache`
    /// was opened (nothing is written then).
    pub fn save(self, cache: &Cache) -> io::Result<bool> {
        self.write(cache.executable_unchanged())
    }

    /// [`save`](Units::save), given whether the executable is unchanged.
    fn write(self, executable_unchanged: bool) -> io::Result<bool> {
        self.write_kept(executable_unchanged)
    }

    /// Starts a new generation of the store, if it was read: a resident server calls it
    /// before each request, so that [`trim`](Units::trim) and the file keep the units seen
    /// most recently.
    pub fn next_generation(&self) {
        if let Some((store, _)) = self.store.get() {
            store.generation.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// The memory the entries held take, as [`held_size`] counts it (0 when the file was
    /// never read).
    pub fn held_bytes(&self) -> usize {
        self.store
            .get()
            .map_or(0, |(store, _)| store.held.load(Ordering::Relaxed))
    }

    /// Drops the entries seen least recently while the entries held take more than
    /// `max_bytes` (as [`held_bytes`](Units::held_bytes) counts), down to three quarters of
    /// it: the bound a resident server keeps in memory, as [`MAX_BYTES`] bounds the file.
    /// The map's spare capacity is not counted (up to as much again after a doubling).
    /// Returns the number of entries dropped.
    pub fn trim(&self, max_bytes: usize) -> usize {
        let Some((store, _)) = self.store.get() else {
            return 0;
        };
        if store.held.load(Ordering::Relaxed) <= max_bytes {
            return 0;
        }
        let generation = store.generation.load(Ordering::Relaxed);
        let mut entries = store.entries.lock().unwrap_or_else(|e| e.into_inner());
        let mut ages: Vec<(u32, Digest)> = entries
            .iter()
            .map(|(digest, (seen, _))| (generation.wrapping_sub(*seen), *digest))
            .collect();
        // Oldest first.
        ages.sort_unstable_by(|a, b| b.cmp(a));
        let target = max_bytes / 4 * 3;
        let mut held = store.held.load(Ordering::Relaxed);
        let mut dropped = 0;
        for (_, digest) in ages {
            if held <= target {
                break;
            }
            if let Some((_, unit)) = entries.remove(&digest) {
                held -= held_size(&unit);
                dropped += 1;
            }
        }
        store.held.store(held, Ordering::Relaxed);
        dropped
    }

    /// Writes the file atomically if a unit was stored since the store was read or last
    /// written, keeping the entries in memory (a resident server writes when it goes idle).
    /// `Ok(false)` when there was nothing to write.
    ///
    /// # Errors
    ///
    /// As [`save`](Units::save).
    pub fn write_kept(&self, executable_unchanged: bool) -> io::Result<bool> {
        let Some((store, _)) = self.store.get() else {
            return Ok(false);
        };
        if !store.dirty.load(Ordering::Relaxed) {
            return Ok(false);
        }
        if !executable_unchanged {
            return Err(io::Error::other(
                "the chloro executable changed during the run; the unit cache was not written",
            ));
        }
        let generation = store.generation.load(Ordering::Relaxed);
        let entries = store.entries.lock().unwrap_or_else(|e| e.into_inner());
        let mut records: Vec<(Digest, u32, Vec<u8>)> = entries
            .iter()
            .map(|(digest, (seen, unit))| (*digest, *seen, unit.to_bytes()))
            .collect();
        drop(entries);
        // Newest first; `wrapping_sub` orders correctly across a wrap. Then keep records
        // while they fit the bound.
        records.sort_by_key(|(digest, seen, _)| (generation.wrapping_sub(*seen), *digest));
        let mut total = 0;
        records.retain(|(_, _, bytes)| {
            total += RECORD_HEADER_LEN + bytes.len();
            total <= MAX_BYTES
        });
        // A fixed order makes the file a function of its contents.
        records.sort_unstable_by_key(|a| a.0);

        let mut bytes = Vec::with_capacity(HEADER_LEN + total.min(MAX_BYTES));
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        bytes.extend_from_slice(&generation.to_le_bytes());
        bytes.extend_from_slice(&self.key);
        for (digest, seen, record) in &records {
            bytes.extend_from_slice(digest);
            bytes.extend_from_slice(&seen.to_le_bytes());
            bytes.extend_from_slice(&(record.len() as u32).to_le_bytes());
            bytes.extend_from_slice(record);
        }

        let dir = self.path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(dir)?;
        let target = self
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let (tmp, mut file) = create_temp(dir, target)?;
        let written = file.write_all(&bytes);
        drop(file);
        let written = written.and_then(|()| fs::rename(&tmp, &self.path));
        if written.is_err() {
            // The temporary file was created by this call (`create_new`), so it is ours.
            let _ = fs::remove_file(&tmp);
        }
        written?;
        store.dirty.store(false, Ordering::Relaxed);
        Ok(true)
    }
}

type Entries = HashMap<Digest, (u32, CachedUnit)>;

/// Parse a unit cache file into its generation and entries. `None` for any layout
/// mismatch and for a file whose header carries another key than `key`.
fn parse(bytes: &[u8], key: &Digest) -> Option<(u32, Entries)> {
    if bytes.len() < HEADER_LEN || &bytes[..8] != MAGIC {
        return None;
    }
    if u32::from_le_bytes(bytes[8..12].try_into().ok()?) != FORMAT_VERSION {
        return None;
    }
    let generation = u32::from_le_bytes(bytes[12..16].try_into().ok()?);
    if bytes[16..HEADER_LEN] != key[..] {
        return None;
    }
    let mut entries = HashMap::new();
    let mut rest = &bytes[HEADER_LEN..];
    while !rest.is_empty() {
        let (head, tail) = rest.split_at_checked(RECORD_HEADER_LEN)?;
        let digest: Digest = head[..32].try_into().ok()?;
        let seen = u32::from_le_bytes(head[32..36].try_into().ok()?);
        let len = u32::from_le_bytes(head[36..40].try_into().ok()?) as usize;
        let (record, tail) = tail.split_at_checked(len)?;
        entries.insert(digest, (seen, CachedUnit::from_bytes(record)?));
        rest = tail;
    }
    Some((generation, entries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chloro_core::Config;

    /// A unit store over `dir`, without a fixed-point cache (the tests' file key).
    fn units_at(dir: &Path, key: Digest) -> Units {
        Units {
            path: dir.join(file_name_of(&key)),
            key,
            store: OnceLock::new(),
        }
    }

    const SOURCE: &str = "fn a() {\n    let x = 1;\n}\n\nfn b(){let y=2;}\n";

    #[test]
    fn stored_units_are_read_back_and_found() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let expected = chloro_core::format_source_with_config(SOURCE, &config);

        let units = units_at(dir.path(), [7; 32]);
        let out = chloro_core::format_source_with_unit_cache(SOURCE, &config, 1, units.store());
        assert_eq!(out, expected);
        let stats = units.stats().unwrap();
        assert_eq!((stats.lookups, stats.hits, stats.stores), (2, 0, 2));
        assert!(units.write(true).unwrap());

        let units = units_at(dir.path(), [7; 32]);
        let edited = SOURCE.replace("y=2", "y=3");
        let out = chloro_core::format_source_with_unit_cache(&edited, &config, 1, units.store());
        assert_eq!(
            out,
            chloro_core::format_source_with_config(&edited, &config)
        );
        let stats = units.stats().unwrap();
        assert_eq!(stats.entries_read, 2);
        assert_eq!((stats.lookups, stats.hits, stats.stores), (2, 1, 1));

        // Another key (another binary or configuration) reads nothing.
        let units = units_at(dir.path(), [8; 32]);
        fs::copy(
            dir.path().join(file_name_of(&[7; 32])),
            dir.path().join(file_name_of(&[8; 32])),
        )
        .unwrap();
        assert_eq!(units.store().stats(0).entries, 0);
    }

    #[test]
    fn a_corrupt_file_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let units = units_at(dir.path(), [9; 32]);
        let _ = chloro_core::format_source_with_unit_cache(SOURCE, &config, 1, units.store());
        assert!(units.write(true).unwrap());
        let path = dir.path().join(file_name_of(&[9; 32]));
        let mut bytes = fs::read(&path).unwrap();
        assert!(parse(&bytes, &[9; 32]).is_some());
        bytes.truncate(bytes.len() - 1);
        assert!(parse(&bytes, &[9; 32]).is_none());
        fs::write(&path, &bytes).unwrap();
        let units = units_at(dir.path(), [9; 32]);
        assert_eq!(units.store().stats(0).entries, 0);
        let out = chloro_core::format_source_with_unit_cache(SOURCE, &config, 1, units.store());
        assert_eq!(out, chloro_core::format_source_with_config(SOURCE, &config));
    }

    /// `n` one-function sources: each formats as one unit of its own.
    fn functions(n: usize) -> Vec<String> {
        (0..n)
            .map(|i| format!("fn f{i}() {{\n    let x = {i};\n}}\n"))
            .collect()
    }

    #[test]
    fn a_resident_store_keeps_its_entries_across_requests_and_writes() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let units = units_at(dir.path(), [3; 32]);
        assert_eq!(units.held_bytes(), 0);
        for source in functions(3) {
            units.next_generation();
            let out =
                chloro_core::format_source_with_unit_cache(&source, &config, 1, units.store());
            assert_eq!(
                out,
                chloro_core::format_source_with_config(&source, &config)
            );
        }
        let held = units.held_bytes();
        assert!(held >= 3 * std::mem::size_of::<(Digest, (u32, CachedUnit))>());
        // Written, and still held: a second write has nothing new to write.
        assert!(units.write_kept(true).unwrap());
        assert!(!units.write_kept(true).unwrap());
        assert_eq!(units.stats().unwrap().entries, 3);
        // A unit seen again is found, and storing nothing leaves the count alone.
        let source = &functions(1)[0];
        let _ = chloro_core::format_source_with_unit_cache(source, &config, 1, units.store());
        let stats = units.stats().unwrap();
        assert_eq!((stats.hits, stats.stores), (1, 3));
        assert_eq!(units.held_bytes(), held);
        // The file holds the three units for a later process.
        let read = units_at(dir.path(), [3; 32]);
        assert_eq!(read.store().stats(0).entries, 3);
    }

    #[test]
    fn trim_drops_the_units_seen_least_recently() {
        let config = Config::default();
        let dir = tempfile::tempdir().unwrap();
        let units = units_at(dir.path(), [4; 32]);
        let sources = functions(8);
        for source in &sources {
            units.next_generation();
            let _ = chloro_core::format_source_with_unit_cache(source, &config, 1, units.store());
        }
        // The first source seen again: the most recent with the last one.
        units.next_generation();
        let _ = chloro_core::format_source_with_unit_cache(&sources[0], &config, 1, units.store());
        let per_entry = units.held_bytes() / 8;
        assert_eq!(units.trim(units.held_bytes()), 0);
        // A bound of four entries keeps three (three quarters of it).
        let dropped = units.trim(4 * per_entry);
        assert_eq!(dropped, 5);
        assert!(units.held_bytes() <= 3 * per_entry);
        let lookups = |source: &str| {
            let before = units.stats().unwrap();
            let _ = chloro_core::format_source_with_unit_cache(source, &config, 1, units.store());
            let after = units.stats().unwrap();
            after.hits - before.hits
        };
        // Kept: the one seen again and the two stored last; the others are formatted again.
        assert_eq!(lookups(&sources[0]), 1);
        assert_eq!(lookups(&sources[7]), 1);
        assert_eq!(lookups(&sources[6]), 1);
        assert_eq!(lookups(&sources[1]), 0);
    }

    #[test]
    fn digest_covers_context_and_text() {
        let digest = |context: &[u8], text: &str| unit_digest(&UnitKey::new(context, text));
        assert_eq!(digest(b"ab", "c"), digest(b"ab", "c"));
        // The context's length separates it from the text.
        assert_ne!(digest(b"ab", "c"), digest(b"a", "bc"));
        assert_ne!(digest(b"ab", "c"), digest(b"ab", "d"));
    }

    #[test]
    fn a_save_after_the_executable_changed_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let units = units_at(dir.path(), [5; 32]);
        let _ = chloro_core::format_source_with_unit_cache(
            SOURCE,
            &Config::default(),
            1,
            units.store(),
        );
        assert!(units.write(false).is_err());
        assert!(!dir.path().join(file_name_of(&[5; 32])).exists());
        // Nothing read, or nothing stored: nothing to write.
        assert!(!units_at(dir.path(), [5; 32]).write(true).unwrap());
    }
}
