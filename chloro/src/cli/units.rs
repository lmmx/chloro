//! Persistent unit cache for the CLI: formatted top-level units, for files the fixed-point
//! cache does not answer.
//!
//! The fixed-point cache (`cache.rs`) answers a file whose content was seen formatted. A
//! file that changed misses it; this cache then answers every top-level unit of the file
//! that did not change (`chloro_core::format_source_with_unit_cache`), so only the edited
//! units are formatted. The output is the same with and without it.
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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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

/// The entries of one unit cache file, shared by the worker threads.
pub struct UnitStore {
    generation: u32,
    entries: Mutex<HashMap<Digest, (u32, CachedUnit)>>,
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
        *seen = self.generation;
        self.hits.fetch_add(1, Ordering::Relaxed);
        Some(unit.clone())
    }

    fn insert(&self, key: &UnitKey<'_>, unit: CachedUnit) {
        let digest = unit_digest(key);
        self.stores.fetch_add(1, Ordering::Relaxed);
        self.dirty.store(true, Ordering::Relaxed);
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(digest, (self.generation, unit));
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

impl UnitStore {
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
                let store = UnitStore {
                    generation: generation.wrapping_add(1),
                    entries: Mutex::new(entries),
                    file_len,
                    lookups: AtomicUsize::new(0),
                    hits: AtomicUsize::new(0),
                    stores: AtomicUsize::new(0),
                    dirty: AtomicBool::new(false),
                };
                (store, read)
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
        let Some((store, _)) = self.store.into_inner() else {
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
        let generation = store.generation;
        let entries = store
            .entries
            .into_inner()
            .unwrap_or_else(|e| e.into_inner());
        let mut records: Vec<(Digest, u32, Vec<u8>)> = entries
            .into_iter()
            .map(|(digest, (seen, unit))| (digest, seen, unit.to_bytes()))
            .collect();
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
