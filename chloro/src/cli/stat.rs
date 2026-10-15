//! Per-path file stamps for a resident server: a file whose metadata is unchanged since its
//! content was read and hashed is answered from the fixed-point cache without reading it.
//!
//! A stamp is the file's size, modification time and, on Unix, device, inode and
//! status-change time (`ctime`). Any write to a file sets its `ctime` to the current time,
//! and a replacement by rename gives a new inode, so an equal stamp means equal content
//! except for a write that lands in the same timestamp tick as the one before it. That case
//! is closed as git closes its "racily clean" entries: a stamp is recorded only when the
//! file's modification and status-change times are at least [`SETTLED`] older than the
//! moment the stamp was taken (before the read). A file written more recently is read and
//! hashed on every request until it settles.

use super::cache::Digest;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

/// How much older than the moment of its stamp a file's times must be for the stamp to be
/// trusted: longer than any file system's timestamp granularity and than the clock's steps
/// between two reads.
pub const SETTLED: Duration = Duration::from_secs(1);

/// Upper bound on the paths remembered; past it the table is cleared.
pub const MAX_PATHS: usize = 1 << 17;

/// The metadata of a file that identifies its content (see the module documentation).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub len: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    unix: (u64, u64, i64, i64),
}

/// When a stamp was taken and what it found.
#[derive(Clone, Debug)]
pub struct Probe {
    pub stamp: Stamp,
    taken: SystemTime,
}

impl Probe {
    /// Whether the file's times are at least [`SETTLED`] before the moment of the stamp.
    fn settled(&self) -> bool {
        let Some(limit) = self.taken.checked_sub(SETTLED) else {
            return false;
        };
        let modified = self.stamp.modified.is_some_and(|m| m <= limit);
        #[cfg(unix)]
        {
            let (_, _, secs, nanos) = self.stamp.unix;
            let changed = SystemTime::UNIX_EPOCH.checked_add(Duration::new(
                secs.max(0) as u64,
                nanos.clamp(0, 999_999_999) as u32,
            ));
            modified && secs >= 0 && changed.is_some_and(|c| c <= limit)
        }
        #[cfg(not(unix))]
        modified
    }
}

/// The stamp of the file at `path`, taken now. `None` when it has no metadata.
pub fn probe(path: &Path) -> Option<Probe> {
    let taken = SystemTime::now();
    let meta = fs::metadata(path).ok()?;
    Some(Probe {
        stamp: Stamp {
            len: meta.len(),
            modified: meta.modified().ok(),
            #[cfg(unix)]
            unix: {
                use std::os::unix::fs::MetadataExt;
                (meta.dev(), meta.ino(), meta.ctime(), meta.ctime_nsec())
            },
        },
        taken,
    })
}

/// The content digest last read from each path, under its stamp.
#[derive(Default)]
pub struct StatTable {
    entries: Mutex<HashMap<PathBuf, (Stamp, Digest)>>,
}

impl StatTable {
    /// The digest recorded for `path` under a stamp equal to `probe`'s.
    pub fn digest(&self, path: &Path, probe: &Probe) -> Option<Digest> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let (stamp, digest) = entries.get(path)?;
        (*stamp == probe.stamp).then_some(*digest)
    }

    /// Records that the content read from `path` after `probe` has `digest`, if the file had
    /// settled; otherwise forgets the path.
    pub fn record(&self, path: &Path, probe: Probe, digest: Digest) {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if !probe.settled() {
            entries.remove(path);
            return;
        }
        if entries.len() >= MAX_PATHS && !entries.contains_key(path) {
            entries.clear();
        }
        entries.insert(path.to_path_buf(), (probe.stamp, digest));
    }

    /// Forgets `path` (its content was just written).
    pub fn forget(&self, path: &Path) {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(path);
    }

    /// Paths remembered.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Whether no path is remembered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(path: &Path) {
        let old = SystemTime::now() - Duration::from_secs(10);
        let file = fs::File::options().write(true).open(path).unwrap();
        file.set_modified(old).unwrap();
    }

    #[test]
    fn a_fresh_file_is_not_trusted_and_a_settled_one_is() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.rs");
        fs::write(&path, "fn a() {}\n").unwrap();
        let table = StatTable::default();
        // Just written: its times are not settled.
        table.record(&path, probe(&path).unwrap(), [1; 32]);
        assert!(table.is_empty());

        settle(&path);
        // The `ctime` of the `set_modified` call is now: still not settled on Unix.
        table.record(&path, probe(&path).unwrap(), [1; 32]);
        if cfg!(unix) {
            assert!(table.is_empty());
        }
        // A probe taken later than `SETTLED` after the last change trusts it.
        let mut late = probe(&path).unwrap();
        late.taken += SETTLED + Duration::from_secs(10);
        table.record(&path, late.clone(), [1; 32]);
        assert_eq!(table.digest(&path, &late), Some([1; 32]));
        assert_eq!(table.digest(&path, &probe(&path).unwrap()), Some([1; 32]));

        // A write changes the stamp (size here; `ctime` in any case on Unix).
        fs::write(&path, "fn b() {}\n").unwrap();
        assert_eq!(table.digest(&path, &probe(&path).unwrap()), None);
        // A failed settle forgets the path.
        table.record(&path, probe(&path).unwrap(), [2; 32]);
        assert!(table.is_empty());
    }

    #[test]
    fn a_same_size_rewrite_with_the_old_modification_time_is_seen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.rs");
        fs::write(&path, "fn a() {}\n").unwrap();
        settle(&path);
        let mut first = probe(&path).unwrap();
        first.taken += SETTLED + Duration::from_secs(10);
        let table = StatTable::default();
        table.record(&path, first.clone(), [1; 32]);
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        fs::write(&path, "fn b() {}\n").unwrap();
        let file = fs::File::options().write(true).open(&path).unwrap();
        file.set_modified(modified).unwrap();
        let second = probe(&path).unwrap();
        if cfg!(unix) {
            // Same size and modification time; the status-change time differs.
            assert_eq!(second.stamp.len, first.stamp.len);
            assert_eq!(second.stamp.modified, first.stamp.modified);
            assert_eq!(table.digest(&path, &second), None);
        }
        table.forget(&path);
        assert!(table.is_empty());
    }
}
