//! Persistent fixed-point cache for the CLI.
//!
//! The cache records one fact per entry: "this file content, formatted by this chloro binary
//! under this configuration, comes out unchanged". A later run that reads a file with the same
//! content hashes it, finds the hash in the cache, and reports the file as already formatted
//! without running the formatter. Nothing else is stored: no formatted output, no paths, no
//! timestamps of source files.
//!
//! # Key
//!
//! Three inputs determine the output of a format run, and each enters the key:
//!
//! - the content, as the BLAKE3 digest (256 bits) of the exact bytes handed to the formatter;
//! - the configuration, as a field-by-field serialisation of [`Config`] (see
//!   [`config_key_bytes`]); the destructuring pattern there fails to compile when a field is
//!   added, and the enum matches fail to compile when a variant is added;
//! - the binary, as the BLAKE3 digest of the running executable's bytes, together with the
//!   crate version, the cache format version, and the target OS and architecture.
//!
//! The binary and configuration keys select the cache *file*; the content digests are the
//! file's entries. Changing `--config` or rebuilding chloro therefore reads a different
//! (initially empty) file, and no entry recorded by one binary or configuration is ever
//! consulted by another.
//!
//! # On-disk format
//!
//! `<cache-dir>/<32 hex chars>.chloro-cache`, where the hex is the first 128 bits of the file
//! key `BLAKE3(tool key || config key)`. The file holds a 48-byte header (`CHLOROFP`, format
//! version `u32`, run generation `u32`, both little-endian, then the whole 32-byte file key)
//! followed by fixed 36-byte records: a 32-byte content digest and the `u32` generation in
//! which the digest was last seen. A file whose header or length does not fit this layout,
//! or whose key is not the one its name was derived from (a file copied under another name,
//! or a name collision), is ignored and overwritten.
//!
//! The set is read once at start-up and shared read-only across the worker threads. The
//! digests to record are collected from the workers' results and written once at the end,
//! to a temporary file in the same directory (`.<cache file name>.<pid>.<nanos>.tmp`,
//! created with `create_new`, so no two saves share one) that is then renamed over the
//! cache file.
//! Entries are bounded to [`MAX_ENTRIES`]; when the bound is exceeded, the entries seen
//! least recently (lowest generation) are dropped. Cache files in the directory that have not
//! been written for [`STALE_FILE_AGE`] (files of binaries or configurations no longer in use)
//! are removed, and so are temporary files of that age left behind by a crash.
//!
//! The executable's digest is taken when the cache is opened, which the CLI does before it
//! discovers files. On Linux the running image is read through `/proc/self/exe`; elsewhere
//! `current_exe` names a path, which a `cargo install` during the run can replace, so the
//! executable's size and modification time are recorded at open and a save whose values
//! differ is skipped: the entries belong to the binary that produced them.
//!
//! # Executable digest record
//!
//! Reading and hashing the executable (2 MB) costs more than the rest of a one-file run, so
//! its digest is recorded in the cache directory under the executable's identity and read
//! back by later runs, which then open the executable and take one `metadata` call on it
//! instead of reading it. The identity is the path `current_exe` names, the size and the
//! modification time and, on Unix, the device, the inode and the status-change time (`ctime`)
//! of the opened executable (`/proc/self/exe` on Linux, the running image). It is taken from
//! the open file, not by path, so it describes the bytes a miss reads even where
//! `/proc/self/exe` is emulated: under valgrind, `stat("/proc/self/exe")` names valgrind's
//! own binary while opening it gives the client's. The record is
//! `<cache-dir>/<32 hex chars>.chloro-exe`, the hex being the first 128 bits of
//! `BLAKE3("chloro executable identity\0" || identity)`, and holds `CHLOROEX`, the format
//! version, the identity (length-prefixed), the 32-byte BLAKE3 of the executable's bytes,
//! and a BLAKE3 of all of the above. A record whose identity is not the current one, or
//! whose layout or check does not hold, is ignored: the executable is read and hashed, and
//! the record is written again (atomically, as cache files are) if the identity did not
//! change while the executable was read. Records age out with the cache files
//! ([`STALE_FILE_AGE`]).
//!
//! What the record does not see: an executable whose bytes changed while its identity stayed
//! the same. On Unix any write to a file sets its `ctime` to the current time, and `ctime`
//! cannot be set from user space; a replacement by rename or copy gives a new inode or a new
//! `ctime`. The identity therefore stays the same only for a write that lands within the
//! file system's timestamp granularity of the previous change after a chloro run has
//! recorded it (a few milliseconds on Linux with coarse timestamps), for a clock set back to
//! the previous `ctime`, or for a write that bypasses the file system (the block device, or
//! another system mounting it). Elsewhere (Windows) the identity is path, size and
//! modification time, and a binary rewritten in place with the same size and its
//! modification time restored is not seen. In those cases the cache answers for the binary
//! the record was taken from, as it would if the cache file had been copied; anyone who can
//! write the cache directory can already add entries to it.

use chloro_core::Config;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A 256-bit BLAKE3 digest.
pub type Digest = [u8; 32];

/// Version of the on-disk layout. Part of the tool key, so a layout change reads a new file.
pub const FORMAT_VERSION: u32 = 2;

/// Suffix of every cache file written by chloro.
pub const FILE_SUFFIX: &str = ".chloro-cache";

/// Upper bound on the entries kept in one cache file (36 bytes each: 4.7 MB).
pub const MAX_ENTRIES: usize = 1 << 17;

/// Cache files not written for this long are removed when another cache file is saved.
pub const STALE_FILE_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

const MAGIC: &[u8; 8] = b"CHLOROFP";
/// Magic (8), format version (4), generation (4), file key (32).
const HEADER_LEN: usize = 48;
const KEY_OFFSET: usize = 16;
const RECORD_LEN: usize = 36;

/// Digest of a file's content as handed to the formatter.
pub fn content_digest(content: &[u8]) -> Digest {
    *blake3::hash(content).as_bytes()
}

/// Deterministic serialisation of every `Config` field, in declaration order
/// ([`Config::key_bytes`], whose destructuring pattern fails to compile when a field is
/// added to `Config`).
pub fn config_key_bytes(config: &Config) -> Vec<u8> {
    config.key_bytes()
}

/// Identity of the formatter: crate version, cache format version, target, and the digest
/// of the running executable. `None` when the executable cannot be read.
pub fn tool_key() -> Option<Digest> {
    Some(tool_key_of(&executable_digest()?))
}

/// [`tool_key`], taking the executable's digest from its record under `dir` when the record
/// names the running executable's identity, and recording it there otherwise (see the
/// module documentation).
pub fn tool_key_in(dir: &Path) -> Option<Digest> {
    let (file, path) = open_executable()?;
    // The identity is taken from the open file, the one whose bytes are read on a miss.
    let identity = |file: &fs::File| identity_bytes(path.as_deref(), &file.metadata().ok()?);
    let read = || read_to_end(&file).map(|bytes| *blake3::hash(&bytes).as_bytes());
    let digest = match identity(&file) {
        Some(id) => recorded_digest(dir, &id, || identity(&file), read)?,
        None => read()?,
    };
    Some(tool_key_of(&digest))
}

/// The running executable opened for reading (on Linux the running image, `/proc/self/exe`),
/// and the path `current_exe` names.
fn open_executable() -> Option<(fs::File, Option<PathBuf>)> {
    let path = std::env::current_exe().ok();
    if cfg!(target_os = "linux")
        && let Ok(file) = fs::File::open("/proc/self/exe")
    {
        return Some((file, path));
    }
    let file = fs::File::open(path.as_ref()?).ok()?;
    Some((file, path))
}

fn read_to_end(mut file: &fs::File) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    io::Read::read_to_end(&mut file, &mut bytes).ok()?;
    Some(bytes)
}

/// BLAKE3 of the running executable's bytes.
fn executable_digest() -> Option<Digest> {
    Some(*blake3::hash(&executable_bytes()?).as_bytes())
}

/// The tool key of an executable whose bytes have the digest `executable`.
fn tool_key_of(executable: &Digest) -> Digest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"chloro fixed-point cache\0");
    hasher.update(&FORMAT_VERSION.to_le_bytes());
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(b"\0");
    hasher.update(std::env::consts::OS.as_bytes());
    hasher.update(b"\0");
    hasher.update(std::env::consts::ARCH.as_bytes());
    hasher.update(b"\0");
    hasher.update(executable);
    *hasher.finalize().as_bytes()
}

/// The bytes of the running executable. On Linux `/proc/self/exe` is read directly: it is
/// the running image even after the file was replaced by a rename, where `current_exe`
/// names a path that no longer exists.
fn executable_bytes() -> Option<Vec<u8>> {
    if cfg!(target_os = "linux")
        && let Ok(bytes) = fs::read("/proc/self/exe")
    {
        return Some(bytes);
    }
    fs::read(std::env::current_exe().ok()?).ok()
}

/// Size and modification time of the file `current_exe` names, taken when the cache is
/// opened and again before it is saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExeStamp {
    len: u64,
    modified: Option<std::time::SystemTime>,
}

fn exe_stamp() -> Option<ExeStamp> {
    let meta = fs::metadata(std::env::current_exe().ok()?).ok()?;
    Some(ExeStamp {
        len: meta.len(),
        modified: meta.modified().ok(),
    })
}

/// Suffix of the files that record an executable's digest under its identity.
pub const EXE_FILE_SUFFIX: &str = ".chloro-exe";

const EXE_MAGIC: &[u8; 8] = b"CHLOROEX";

/// Serialises the identity of the executable at `path` with metadata `meta` (see the module
/// documentation). `None` when the platform gives no modification time; the executable is
/// then read on every run.
fn identity_bytes(path: Option<&Path>, meta: &fs::Metadata) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(128);
    let path = path.map_or(&[][..], |p| p.as_os_str().as_encoded_bytes());
    out.extend_from_slice(&(path.len() as u64).to_le_bytes());
    out.extend_from_slice(path);
    out.extend_from_slice(&meta.len().to_le_bytes());
    let (after_epoch, since) = match meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => (1u8, d),
        Err(e) => (0u8, e.duration()),
    };
    out.push(after_epoch);
    out.extend_from_slice(&since.as_secs().to_le_bytes());
    out.extend_from_slice(&since.subsec_nanos().to_le_bytes());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        out.extend_from_slice(&meta.dev().to_le_bytes());
        out.extend_from_slice(&meta.ino().to_le_bytes());
        out.extend_from_slice(&meta.ctime().to_le_bytes());
        out.extend_from_slice(&meta.ctime_nsec().to_le_bytes());
    }
    Some(out)
}

/// File name of the record for the executable identity `identity`.
fn exe_file_name(identity: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"chloro executable identity\0");
    hasher.update(identity);
    let key = hasher.finalize();
    let mut name = String::with_capacity(32 + EXE_FILE_SUFFIX.len());
    for byte in &key.as_bytes()[..16] {
        name.push_str(&format!("{byte:02x}"));
    }
    name.push_str(EXE_FILE_SUFFIX);
    name
}

/// The record of `digest` for `identity`.
fn exe_record(identity: &[u8], digest: &Digest) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(8 + 4 + 4 + identity.len() + 64);
    bytes.extend_from_slice(EXE_MAGIC);
    bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(
        &u32::try_from(identity.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    bytes.extend_from_slice(identity);
    bytes.extend_from_slice(digest);
    let check = blake3::hash(&bytes);
    bytes.extend_from_slice(check.as_bytes());
    bytes
}

/// The digest a record holds, if the record is well formed and was written for `identity`.
fn parse_exe_record(bytes: &[u8], identity: &[u8]) -> Option<Digest> {
    let (body, check) = bytes.split_at_checked(bytes.len().checked_sub(32)?)?;
    if blake3::hash(body).as_bytes()[..] != check[..] {
        return None;
    }
    let rest = body.strip_prefix(EXE_MAGIC)?;
    let (version, rest) = rest.split_at_checked(4)?;
    if version != FORMAT_VERSION.to_le_bytes() {
        return None;
    }
    let (len, rest) = rest.split_at_checked(4)?;
    let len = u32::from_le_bytes(len.try_into().ok()?) as usize;
    let (recorded, digest) = rest.split_at_checked(len)?;
    if recorded != identity {
        return None;
    }
    digest.try_into().ok()
}

/// The executable digest recorded under `dir` for `identity`; when there is none, `compute`
/// gives it, and it is recorded if `current` still returns `identity` afterwards (the file
/// did not change while it was read).
fn recorded_digest(
    dir: &Path,
    identity: &[u8],
    current: impl FnOnce() -> Option<Vec<u8>>,
    compute: impl FnOnce() -> Option<Digest>,
) -> Option<Digest> {
    let path = dir.join(exe_file_name(identity));
    if let Some(digest) = fs::read(&path)
        .ok()
        .and_then(|bytes| parse_exe_record(&bytes, identity))
    {
        return Some(digest);
    }
    let digest = compute()?;
    if current().as_deref() == Some(identity) {
        // A record that cannot be written costs the next run a read of the executable.
        let _ = write_atomically(dir, &path, &exe_record(identity, &digest));
    }
    Some(digest)
}

/// Writes `bytes` to `path` in `dir` through a fresh temporary file renamed over it.
fn write_atomically(dir: &Path, path: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let target = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let (tmp, mut file) = create_temp(dir, target)?;
    let written = file.write_all(bytes);
    drop(file);
    let written = written.and_then(|()| fs::rename(&tmp, path));
    if written.is_err() {
        // The temporary file was created by this call (`create_new`), so it is ours.
        let _ = fs::remove_file(&tmp);
    }
    written
}

/// Key of the cache file for one (tool, config) pair: `BLAKE3(tool || config)`. Its first
/// 128 bits name the file; the whole key is written into the file's header and checked when
/// the file is read.
pub fn file_key(tool: &Digest, config: &Config) -> Digest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(tool);
    hasher.update(&config_key_bytes(config));
    *hasher.finalize().as_bytes()
}

/// File name of the cache whose key is `key`.
pub fn file_name_of(key: &Digest) -> String {
    let mut name = String::with_capacity(32 + FILE_SUFFIX.len());
    for byte in &key[..16] {
        name.push_str(&format!("{byte:02x}"));
    }
    name.push_str(FILE_SUFFIX);
    name
}

/// File name of the cache for one (tool, config) pair.
pub fn file_name(tool: &Digest, config: &Config) -> String {
    file_name_of(&file_key(tool, config))
}

/// The cache directory, from the first of: the `--cache-dir` option, `CHLORO_CACHE_DIR`,
/// `$XDG_CACHE_HOME/chloro`, `%LOCALAPPDATA%\chloro\cache` (Windows),
/// `$HOME/Library/Caches/chloro` (macOS), `$HOME/.cache/chloro`. `None` when none is set.
pub fn resolve_dir(option: &str) -> Option<PathBuf> {
    let non_empty = |v: Option<std::ffi::OsString>| v.filter(|s| !s.is_empty()).map(PathBuf::from);
    if !option.is_empty() {
        return Some(PathBuf::from(option));
    }
    if let Some(dir) = non_empty(std::env::var_os("CHLORO_CACHE_DIR")) {
        return Some(dir);
    }
    if let Some(dir) = non_empty(std::env::var_os("XDG_CACHE_HOME")) {
        return Some(dir.join("chloro"));
    }
    if cfg!(windows)
        && let Some(dir) = non_empty(std::env::var_os("LOCALAPPDATA"))
    {
        return Some(dir.join("chloro").join("cache"));
    }
    let home = non_empty(std::env::var_os("HOME"))?;
    if cfg!(target_os = "macos") {
        return Some(home.join("Library").join("Caches").join("chloro"));
    }
    Some(home.join(".cache").join("chloro"))
}

/// The set of content digests known to be fixed points of one (tool, config) pair.
pub struct Cache {
    path: PathBuf,
    /// `BLAKE3(tool || config)`: written into the header, checked when the file is read.
    key: Digest,
    generation: u32,
    entries: HashMap<Digest, u32>,
    /// The executable's size and modification time when the cache was opened.
    exe_stamp: Option<ExeStamp>,
}

impl Cache {
    /// Read the cache file for `config` under `dir`. A missing, unreadable, or malformed file
    /// gives an empty cache at the same path.
    pub fn open(dir: &Path, config: &Config) -> Option<Self> {
        let tool = tool_key_in(dir)?;
        let key = file_key(&tool, config);
        let path = dir.join(file_name_of(&key));
        let (generation, entries) = fs::read(&path)
            .ok()
            .and_then(|bytes| parse(&bytes, &key))
            .unwrap_or_default();
        Some(Cache {
            path,
            key,
            generation,
            entries,
            exe_stamp: exe_stamp(),
        })
    }

    /// Path of the cache file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// `BLAKE3(tool || config)`, the key of this cache file and of the unit cache file next
    /// to it (`units.rs`).
    pub fn key(&self) -> &Digest {
        &self.key
    }

    /// Whether the executable's size and modification time are those taken when the cache
    /// was opened: entries produced by this run belong to the binary hashed then.
    pub fn executable_unchanged(&self) -> bool {
        exe_stamp() == self.exe_stamp
    }

    /// Number of entries read from disk.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache holds no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether `digest` is recorded as a fixed point.
    pub fn contains(&self, digest: &Digest) -> bool {
        self.entries.contains_key(digest)
    }

    /// Record `seen` as fixed points of this run and write the file atomically.
    ///
    /// # Errors
    ///
    /// Returns the first I/O error from creating the directory, writing the temporary file,
    /// or renaming it over the cache file, and an error of kind `Other` when the executable
    /// changed since the cache was opened (nothing is written then).
    pub fn save(mut self, seen: impl IntoIterator<Item = Digest>) -> io::Result<()> {
        if !self.executable_unchanged() {
            // The entries were produced by the binary hashed at `open`; a binary that
            // replaced it since must not be given them.
            return Err(io::Error::other(
                "the chloro executable changed during the run; the cache file was not written",
            ));
        }
        self.generation = self.generation.wrapping_add(1);
        for digest in seen {
            self.entries.insert(digest, self.generation);
        }
        let mut records: Vec<(Digest, u32)> = self.entries.into_iter().collect();
        if records.len() > MAX_ENTRIES {
            // Newest generation first; `wrapping_sub` orders correctly across a wrap.
            records.sort_by_key(|(_, seen)| self.generation.wrapping_sub(*seen));
            records.truncate(MAX_ENTRIES);
        }
        // A fixed order makes the file a function of its contents.
        records.sort_unstable();

        let mut bytes = Vec::with_capacity(HEADER_LEN + records.len() * RECORD_LEN);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        bytes.extend_from_slice(&self.generation.to_le_bytes());
        bytes.extend_from_slice(&self.key);
        for (digest, seen) in &records {
            bytes.extend_from_slice(digest);
            bytes.extend_from_slice(&seen.to_le_bytes());
        }

        let dir = self.path.parent().unwrap_or(Path::new("."));
        write_atomically(dir, &self.path, &bytes)?;
        remove_stale_files(dir);
        Ok(())
    }
}

/// Attempts at a fresh temporary file name before a save gives up.
const TEMP_ATTEMPTS: u128 = 16;

/// Name of a temporary file for the cache file `target`: `.<target>.<pid>.<nanos>.tmp`.
///
/// The target's name is part of it, so two saves of different cache files never share a
/// temporary file, whatever their process ids and clocks.
fn temp_name(target: &str, pid: u32, nanos: u128) -> String {
    format!(".{target}.{pid}.{nanos}.tmp")
}

/// Opens `path` for writing only if no file exists there (`create_new`): a file another
/// process is writing is never truncated or shared.
fn open_new(path: &Path) -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
}

/// Creates a new temporary file for `target` in `dir`. A name that already exists (another
/// process with the same id saving at the same clock reading) is retried with another
/// suffix; after [`TEMP_ATTEMPTS`] the `AlreadyExists` error is returned and the save is
/// skipped.
pub(crate) fn create_temp(dir: &Path, target: &str) -> io::Result<(PathBuf, fs::File)> {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut last = None;
    for attempt in 0..TEMP_ATTEMPTS {
        let tmp = dir.join(temp_name(target, pid, nanos.wrapping_add(attempt)));
        match open_new(&tmp) {
            Ok(file) => return Ok((tmp, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => last = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last.unwrap_or_else(|| io::Error::from(io::ErrorKind::AlreadyExists)))
}

/// Parse a cache file into its generation and entries. `None` for any layout mismatch, and
/// for a file whose header carries another key than `key` (a name collision, or a file
/// copied under another name).
fn parse(bytes: &[u8], key: &Digest) -> Option<(u32, HashMap<Digest, u32>)> {
    if bytes.len() < HEADER_LEN || &bytes[..8] != MAGIC {
        return None;
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().ok()?);
    if version != FORMAT_VERSION {
        return None;
    }
    let generation = u32::from_le_bytes(bytes[12..16].try_into().ok()?);
    if bytes[KEY_OFFSET..HEADER_LEN] != key[..] {
        return None;
    }
    let body = &bytes[HEADER_LEN..];
    if !body.len().is_multiple_of(RECORD_LEN) {
        return None;
    }
    let mut entries = HashMap::with_capacity(body.len() / RECORD_LEN);
    for record in body.chunks_exact(RECORD_LEN) {
        let digest: Digest = record[..32].try_into().ok()?;
        let seen = u32::from_le_bytes(record[32..].try_into().ok()?);
        entries.insert(digest, seen);
    }
    Some((generation, entries))
}

/// Remove the cache files, the unit cache files, the executable digest records, and the
/// temporary files of saves that did not finish, in `dir` not written for `STALE_FILE_AGE`.
/// Errors are ignored.
fn remove_stale_files(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let path = entry.path();
        // `<hex>.chloro-cache`, `<hex>.chloro-units` (`units.rs`), `<hex>.chloro-exe`, or
        // `.<any of these names>.<pid>.<nanos>.tmp` (`temp_name`).
        let ours = path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
            [FILE_SUFFIX, super::units::FILE_SUFFIX, EXE_FILE_SUFFIX]
                .iter()
                .any(|suffix| {
                    n.ends_with(suffix)
                        || (n.starts_with('.') && n.contains(suffix) && n.ends_with(".tmp"))
                })
        });
        if !ours {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age > STALE_FILE_AGE);
        if stale {
            let _ = fs::remove_file(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn default_config_key_is_pinned() {
        // Pins the serialisation: a change here changes every cache file name.
        let key = config_key_bytes(&Config::default());
        assert_eq!(key.len(), 49);
        assert_eq!(
            hex(&key),
            "6400000000000000000400000000000000000000000000000000000101010a000000000000000101000300010000010001"
        );
    }

    #[test]
    fn equal_configs_from_different_spellings_share_a_key() {
        let mut a = Config::default();
        a.set("fn_args_layout", "Vertical").unwrap();
        a.set("max_width", "80").unwrap();
        let mut b = Config::default();
        b.set("max_width", "\"80\"").unwrap();
        b.set("fn_params_layout", "Vertical").unwrap();
        assert_eq!(config_key_bytes(&a), config_key_bytes(&b));
    }

    #[test]
    fn every_option_changes_the_key() {
        let base = config_key_bytes(&Config::default());
        for (key, value) in [
            ("max_width", "80"),
            ("hard_tabs", "true"),
            ("tab_spaces", "2"),
            ("newline_style", "Windows"),
            ("use_small_heuristics", "Max"),
            ("fn_call_width", "60"),
            ("attr_fn_like_width", "70"),
            ("struct_lit_width", "18"),
            ("struct_variant_width", "35"),
            ("array_width", "60"),
            ("chain_width", "60"),
            ("single_line_if_else_max_width", "50"),
            ("single_line_let_else_max_width", "50"),
            ("reorder_imports", "false"),
            ("reorder_modules", "false"),
            ("remove_nested_parens", "false"),
            ("short_array_element_width_threshold", "5"),
            ("match_arm_leading_pipes", "Always"),
            ("fn_params_layout", "Compressed"),
            ("match_block_trailing_comma", "true"),
            ("edition", "2021"),
            ("style_edition", "2024"),
            ("merge_derives", "false"),
            ("use_try_shorthand", "true"),
            ("use_field_init_shorthand", "true"),
            ("force_explicit_abi", "false"),
            ("disable_all_formatting", "true"),
            ("rustfmt_version", "1.8.0"),
        ] {
            let mut config = Config::default();
            config.set(key, value).unwrap();
            assert_ne!(
                config_key_bytes(&config),
                base,
                "{key}={value} leaves the key unchanged"
            );
        }
    }

    #[test]
    fn explicit_default_heuristic_differs_from_unset() {
        // `fn_call_width=60` resolves to the same width as unset under `max_width=100`, and
        // the key still distinguishes them (the key records the configuration, not its
        // resolution).
        let mut config = Config::default();
        config.set("fn_call_width", "60").unwrap();
        assert_ne!(
            config_key_bytes(&config),
            config_key_bytes(&Config::default())
        );
    }

    #[test]
    fn parse_round_trip_and_rejects_corruption() {
        let d1 = content_digest(b"a");
        let d2 = content_digest(b"b");
        let key = [5u8; 32];
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        bytes.extend_from_slice(&7u32.to_le_bytes());
        bytes.extend_from_slice(&key);
        for (d, g) in [(d1, 3u32), (d2, 7u32)] {
            bytes.extend_from_slice(&d);
            bytes.extend_from_slice(&g.to_le_bytes());
        }
        let (generation, entries) = parse(&bytes, &key).unwrap();
        assert_eq!(generation, 7);
        assert_eq!(entries.get(&d1), Some(&3));
        assert_eq!(entries.get(&d2), Some(&7));

        assert!(
            parse(&bytes[..bytes.len() - 1], &key).is_none(),
            "truncated record"
        );
        assert!(parse(b"", &key).is_none(), "empty");
        assert!(parse(b"not a cache file at all", &key).is_none(), "foreign");
        let mut wrong_version = bytes.clone();
        wrong_version[8] ^= 1;
        assert!(parse(&wrong_version, &key).is_none(), "format version");
        let mut other_key = key;
        other_key[31] ^= 1;
        assert!(
            parse(&bytes, &other_key).is_none(),
            "a file copied under another name, or a name collision, is not read"
        );
    }

    #[test]
    fn save_then_open_keeps_entries_and_evicts_oldest() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let tool = [9u8; 32];
        let key = file_key(&tool, &config);
        let path = dir.path().join(file_name_of(&key));
        let cache = Cache {
            path: path.clone(),
            key,
            generation: 0,
            entries: HashMap::new(),
            exe_stamp: exe_stamp(),
        };
        let many: Vec<Digest> = (0..=MAX_ENTRIES as u32)
            .map(|i| content_digest(&i.to_le_bytes()))
            .collect();
        cache.save(many.iter().copied()).unwrap();
        let (generation, entries) = parse(&fs::read(&path).unwrap(), &key).unwrap();
        assert_eq!(generation, 1);
        assert_eq!(entries.len(), MAX_ENTRIES, "the bound applies on save");

        let cache = Cache {
            path: path.clone(),
            key,
            generation,
            entries,
            exe_stamp: exe_stamp(),
        };
        let newer = content_digest(b"newer");
        cache.save([newer]).unwrap();
        let (generation, entries) = parse(&fs::read(&path).unwrap(), &key).unwrap();
        assert_eq!(generation, 2);
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert!(entries.contains_key(&newer));
        let dropped = many.iter().filter(|d| !entries.contains_key(*d)).count();
        assert_eq!(dropped, 2, "one more generation-1 entry is dropped");
        assert!(
            dir.path().read_dir().unwrap().all(|e| {
                let name = e.unwrap().file_name();
                name.to_str().unwrap().ends_with(FILE_SUFFIX)
            }),
            "no temporary file is left behind"
        );
    }

    #[test]
    fn temporary_file_names_its_target_and_is_never_shared() {
        assert_eq!(
            temp_name("0123abcd.chloro-cache", 42, 1_700_000_000_123_456_789),
            ".0123abcd.chloro-cache.42.1700000000123456789.tmp"
        );
        // Two cache files never share a temporary name, whatever the pid and clock.
        assert_ne!(
            temp_name("a.chloro-cache", 1, 2),
            temp_name("b.chloro-cache", 1, 2)
        );

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(temp_name("a.chloro-cache", 1, 2));
        fs::write(&path, b"another process's entries").unwrap();
        let err = open_new(&path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            fs::read(&path).unwrap(),
            b"another process's entries",
            "an existing temporary file is neither truncated nor written"
        );

        // `create_temp` never returns an existing file.
        let (tmp, _file) = create_temp(dir.path(), "a.chloro-cache").unwrap();
        assert_ne!(tmp, path);
        let name = tmp.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with(".a.chloro-cache."), "{name}");
        assert!(name.ends_with(".tmp"), "{name}");
    }

    #[test]
    fn save_leaves_another_saves_temporary_file_alone() {
        let dir = tempfile::tempdir().unwrap();
        let key = file_key(&[3u8; 32], &Config::default());
        let path = dir.path().join(file_name_of(&key));
        let target = path.file_name().unwrap().to_str().unwrap();
        let foreign = dir
            .path()
            .join(temp_name(target, std::process::id().wrapping_add(1), 5));
        fs::write(&foreign, b"in progress").unwrap();
        let cache = Cache {
            path: path.clone(),
            key,
            generation: 0,
            entries: HashMap::new(),
            exe_stamp: exe_stamp(),
        };
        let d = content_digest(b"x");
        cache.save([d]).unwrap();
        assert!(
            parse(&fs::read(&path).unwrap(), &key)
                .unwrap()
                .1
                .contains_key(&d)
        );
        assert_eq!(fs::read(&foreign).unwrap(), b"in progress");
    }

    #[test]
    fn open_reads_the_file_its_key_names_and_rejects_a_copy() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let tool = tool_key().expect("the test binary can be read");
        let key = file_key(&tool, &config);
        let d = content_digest(b"fixed point");
        Cache::open(dir.path(), &config).unwrap().save([d]).unwrap();
        let cache = Cache::open(dir.path(), &config).unwrap();
        assert!(cache.contains(&d));
        assert_eq!(cache.path(), dir.path().join(file_name_of(&key)));

        // The file copied under another configuration's name is not read as that
        // configuration's cache: the header names the configuration it was written for.
        let mut other = Config::default();
        other.set("max_width", "80").unwrap();
        let copy = dir.path().join(file_name(&tool, &other));
        fs::copy(cache.path(), &copy).unwrap();
        let cache = Cache::open(dir.path(), &other).unwrap();
        assert_eq!(cache.path(), copy);
        assert!(cache.is_empty(), "a copied cache file is ignored");
    }

    #[test]
    fn save_is_skipped_when_the_executable_changed() {
        let dir = tempfile::tempdir().unwrap();
        let key = file_key(&[7u8; 32], &Config::default());
        let path = dir.path().join(file_name_of(&key));
        let cache = Cache {
            path: path.clone(),
            key,
            generation: 0,
            entries: HashMap::new(),
            exe_stamp: Some(ExeStamp {
                len: 0,
                modified: None,
            }),
        };
        let err = cache.save([content_digest(b"x")]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
        assert!(
            !path.exists(),
            "no cache file is written for a changed binary"
        );
        assert_eq!(
            dir.path().read_dir().unwrap().count(),
            0,
            "no temporary file either"
        );
    }

    #[test]
    fn stale_cache_and_temporary_files_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let now = std::time::SystemTime::now();
        let old = now - STALE_FILE_AGE - Duration::from_secs(60);
        let aged = |name: &str, time| {
            let p = dir.path().join(name);
            fs::write(&p, b"x").unwrap();
            fs::File::options()
                .write(true)
                .open(&p)
                .unwrap()
                .set_modified(time)
                .unwrap();
            p
        };
        let stale_cache = aged("00ff.chloro-cache", old);
        let stale_tmp = aged(".00ff.chloro-cache.12.34.tmp", old);
        let stale_exe = aged("00ee.chloro-exe", old);
        let stale_exe_tmp = aged(".00ee.chloro-exe.12.34.tmp", old);
        let fresh_exe = aged("00dd.chloro-exe", now);
        let fresh_tmp = aged(".00ff.chloro-cache.56.78.tmp", now);
        let foreign = aged(".notes.tmp", old);
        remove_stale_files(dir.path());
        assert!(!stale_cache.exists());
        assert!(
            !stale_tmp.exists(),
            "a temporary file left by a crash is removed"
        );
        assert!(fresh_tmp.exists(), "a save in progress is left alone");
        assert!(
            !stale_exe.exists(),
            "an executable record not written for long"
        );
        assert!(!stale_exe_tmp.exists());
        assert!(fresh_exe.exists());
        assert!(
            foreign.exists(),
            "a file that is not chloro's is left alone"
        );
    }

    #[test]
    fn tool_key_is_the_running_executable() {
        let key = tool_key().expect("the test binary can be read");
        assert_eq!(tool_key(), Some(key));
        assert_eq!(executable_bytes().map(|b| b.is_empty()), Some(false));
        assert!(exe_stamp().is_some_and(|s| s.len > 0));
    }

    /// `recorded_digest` with a computation that counts its calls.
    fn recorded(dir: &Path, identity: &[u8], digest: Digest, calls: &mut usize) -> Digest {
        recorded_digest(
            dir,
            identity,
            || Some(identity.to_vec()),
            || {
                *calls += 1;
                Some(digest)
            },
        )
        .unwrap()
    }

    #[test]
    fn exe_record_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let identity = b"an executable's identity".to_vec();
        let digest = content_digest(b"its bytes");
        let mut calls = 0;
        assert_eq!(recorded(dir.path(), &identity, digest, &mut calls), digest);
        assert_eq!(calls, 1, "computed on a miss");
        let path = dir.path().join(exe_file_name(&identity));
        assert!(path.exists(), "and recorded");
        assert_eq!(
            parse_exe_record(&fs::read(&path).unwrap(), &identity),
            Some(digest)
        );
        let other = content_digest(b"other bytes");
        assert_eq!(recorded(dir.path(), &identity, other, &mut calls), digest);
        assert_eq!(calls, 1, "read back, not computed, on a hit");
        assert!(
            dir.path().read_dir().unwrap().count() == 1,
            "no temporary file is left behind"
        );
    }

    #[test]
    fn exe_record_rejects_corruption_and_other_identities() {
        let identity = b"identity".to_vec();
        let digest = content_digest(b"bytes");
        let record = exe_record(&identity, &digest);
        assert_eq!(parse_exe_record(&record, &identity), Some(digest));
        assert_eq!(
            parse_exe_record(&record, b"identitx"),
            None,
            "another identity"
        );
        assert_eq!(
            parse_exe_record(&record[..record.len() - 1], &identity),
            None
        );
        assert_eq!(parse_exe_record(b"", &identity), None);
        for i in 0..record.len() {
            let mut flipped = record.clone();
            flipped[i] ^= 1;
            assert_eq!(parse_exe_record(&flipped, &identity), None, "byte {i}");
        }
    }

    #[test]
    fn a_changed_size_or_modification_time_recomputes() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("exe");
        fs::write(&exe, b"version one").unwrap();
        let identity = || identity_bytes(Some(&exe), &fs::metadata(&exe).unwrap()).unwrap();
        let cache = dir.path().join("cache");
        let mut calls = 0;
        let first = identity();
        recorded(&cache, &first, content_digest(b"1"), &mut calls);
        recorded(&cache, &identity(), content_digest(b"1"), &mut calls);
        assert_eq!(calls, 1, "an unchanged executable is a hit");

        let modified = fs::metadata(&exe).unwrap().modified().unwrap();
        // Same modification time, another size.
        fs::write(&exe, b"version two, longer").unwrap();
        fs::File::options()
            .write(true)
            .open(&exe)
            .unwrap()
            .set_modified(modified)
            .unwrap();
        let second = identity();
        assert_ne!(second, first);
        assert_eq!(
            recorded(&cache, &second, content_digest(b"2"), &mut calls),
            content_digest(b"2")
        );
        assert_eq!(calls, 2, "a changed size recomputes");

        // Same size, another modification time.
        fs::File::options()
            .write(true)
            .open(&exe)
            .unwrap()
            .set_modified(modified + Duration::from_secs(1))
            .unwrap();
        let third = identity();
        assert_ne!(third, second);
        recorded(&cache, &third, content_digest(b"3"), &mut calls);
        assert_eq!(calls, 3, "a changed modification time recomputes");
    }

    /// On Unix, a rewrite in place with the same size and the modification time put back
    /// changes the status-change time, which the identity holds.
    #[cfg(unix)]
    #[test]
    fn a_rewrite_with_its_modification_time_restored_recomputes() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("exe");
        fs::write(&exe, b"version one").unwrap();
        let identity = || identity_bytes(Some(&exe), &fs::metadata(&exe).unwrap()).unwrap();
        let before = identity();
        let modified = fs::metadata(&exe).unwrap().modified().unwrap();
        // Past the file system's timestamp granularity.
        std::thread::sleep(Duration::from_millis(50));
        fs::write(&exe, b"version two").unwrap();
        fs::File::options()
            .write(true)
            .open(&exe)
            .unwrap()
            .set_modified(modified)
            .unwrap();
        let meta = fs::metadata(&exe).unwrap();
        assert_eq!((meta.len(), meta.modified().unwrap()), (11, modified));
        assert_ne!(identity(), before);
    }

    #[test]
    fn a_stale_record_with_a_wrong_digest_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let identity = b"current identity".to_vec();
        let right = content_digest(b"right");
        let wrong = content_digest(b"wrong");
        let path = dir.path().join(exe_file_name(&identity));
        let mut calls = 0;
        // A record under this identity's name written for another identity (a copy, or a
        // name collision), then one with a corrupted digest.
        for stale in [exe_record(b"previous identity", &wrong), {
            let mut r = exe_record(&identity, &wrong);
            r[8 + 4 + 4 + identity.len()] ^= 1;
            r
        }] {
            fs::write(&path, stale).unwrap();
            assert_eq!(recorded(dir.path(), &identity, right, &mut calls), right);
            assert_eq!(
                parse_exe_record(&fs::read(&path).unwrap(), &identity),
                Some(right),
                "the record is replaced"
            );
        }
        assert_eq!(calls, 2);
        assert_eq!(recorded(dir.path(), &identity, wrong, &mut calls), right);
        assert_eq!(calls, 2);
    }

    #[test]
    fn no_record_is_written_when_the_executable_changes_while_read() {
        let dir = tempfile::tempdir().unwrap();
        let identity = b"identity at open".to_vec();
        let digest = content_digest(b"bytes");
        let got = recorded_digest(
            dir.path(),
            &identity,
            || Some(b"identity after the read".to_vec()),
            || Some(digest),
        );
        assert_eq!(got, Some(digest), "the run still uses the digest it read");
        assert_eq!(
            dir.path().read_dir().unwrap().count(),
            0,
            "nothing recorded"
        );
    }

    #[test]
    fn tool_key_in_is_tool_key() {
        let dir = tempfile::tempdir().unwrap();
        let key = tool_key().expect("the test binary can be read");
        assert_eq!(tool_key_in(dir.path()), Some(key), "computed and recorded");
        let records: Vec<_> = dir
            .path()
            .read_dir()
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(records.len(), 1, "{records:?}");
        assert!(records[0].ends_with(EXE_FILE_SUFFIX), "{records:?}");
        assert_eq!(tool_key_in(dir.path()), Some(key), "read from the record");
    }

    #[test]
    fn resolve_dir_prefers_option_then_env() {
        assert_eq!(resolve_dir("/x/y"), Some(PathBuf::from("/x/y")));
        // The environment is process-wide; the test only checks that an explicit option
        // wins and that a non-empty result is produced from whatever is set.
        assert!(resolve_dir("").is_none_or(|p| !p.as_os_str().is_empty()));
    }
}
