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
//! `<cache-dir>/<32 hex chars>.chloro-cache`, where the hex is the first 128 bits of
//! `BLAKE3(tool key || config key)`. The file holds a 16-byte header (`CHLOROFP`, format
//! version `u32`, run generation `u32`, both little-endian) followed by fixed 36-byte records:
//! a 32-byte content digest and the `u32` generation in which the digest was last seen. A
//! file whose header or length does not fit this layout is ignored and overwritten.
//!
//! The set is read once at start-up and shared read-only across the worker threads. The
//! digests to record are collected from the workers' results and written once at the end,
//! to a temporary file in the same directory that is then renamed over the cache file.
//! Entries are bounded to [`MAX_ENTRIES`]; when the bound is exceeded, the entries seen
//! least recently (lowest generation) are dropped. Cache files in the directory that have not
//! been written for [`STALE_FILE_AGE`] (files of binaries or configurations no longer in use)
//! are removed.

use chloro_core::Config;
use chloro_core::formatter::config::{
    Density, Edition, Heuristics, MatchArmLeadingPipe, NewlineStyle, StyleEdition,
};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A 256-bit BLAKE3 digest.
pub type Digest = [u8; 32];

/// Version of the on-disk layout. Part of the tool key, so a layout change reads a new file.
pub const FORMAT_VERSION: u32 = 1;

/// Suffix of every cache file written by chloro.
pub const FILE_SUFFIX: &str = ".chloro-cache";

/// Upper bound on the entries kept in one cache file (36 bytes each: 4.7 MB).
pub const MAX_ENTRIES: usize = 1 << 17;

/// Cache files not written for this long are removed when another cache file is saved.
pub const STALE_FILE_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

const MAGIC: &[u8; 8] = b"CHLOROFP";
const HEADER_LEN: usize = 16;
const RECORD_LEN: usize = 36;

/// Digest of a file's content as handed to the formatter.
pub fn content_digest(content: &[u8]) -> Digest {
    *blake3::hash(content).as_bytes()
}

/// Deterministic serialisation of every `Config` field, in declaration order.
///
/// The destructuring pattern names every field without `..`, so adding a field to `Config`
/// fails to compile here until the field is added to the key.
pub fn config_key_bytes(config: &Config) -> Vec<u8> {
    let Config {
        max_width,
        hard_tabs,
        tab_spaces,
        newline_style,
        use_small_heuristics,
        fn_call_width,
        attr_fn_like_width,
        struct_lit_width,
        struct_variant_width,
        array_width,
        chain_width,
        single_line_if_else_max_width,
        single_line_let_else_max_width,
        reorder_imports,
        reorder_modules,
        remove_nested_parens,
        short_array_element_width_threshold,
        match_arm_leading_pipes,
        fn_params_layout,
        match_block_trailing_comma,
        edition,
        style_edition,
        merge_derives,
        use_try_shorthand,
        use_field_init_shorthand,
        force_explicit_abi,
        disable_all_formatting,
    } = config;

    let mut out = Vec::with_capacity(128);
    let usize_ = |out: &mut Vec<u8>, v: usize| out.extend_from_slice(&(v as u64).to_le_bytes());
    let bool_ = |out: &mut Vec<u8>, v: bool| out.push(u8::from(v));
    let opt = |out: &mut Vec<u8>, v: Option<usize>| match v {
        None => out.push(0),
        Some(v) => {
            out.push(1);
            out.extend_from_slice(&(v as u64).to_le_bytes());
        }
    };
    let newline = |v: NewlineStyle| match v {
        NewlineStyle::Auto => 0u8,
        NewlineStyle::Native => 1,
        NewlineStyle::Unix => 2,
        NewlineStyle::Windows => 3,
    };
    let heuristics = |v: Heuristics| match v {
        Heuristics::Default => 0u8,
        Heuristics::Max => 1,
        Heuristics::Off => 2,
    };
    let pipes = |v: MatchArmLeadingPipe| match v {
        MatchArmLeadingPipe::Always => 0u8,
        MatchArmLeadingPipe::Never => 1,
        MatchArmLeadingPipe::Preserve => 2,
    };
    let density = |v: Density| match v {
        Density::Compressed => 0u8,
        Density::Tall => 1,
        Density::Vertical => 2,
    };
    let edition_ = |v: Edition| match v {
        Edition::Edition2015 => 0u8,
        Edition::Edition2018 => 1,
        Edition::Edition2021 => 2,
        Edition::Edition2024 => 3,
    };
    let style = |v: Option<StyleEdition>| match v {
        None => 0u8,
        Some(StyleEdition::Edition2015) => 1,
        Some(StyleEdition::Edition2018) => 2,
        Some(StyleEdition::Edition2021) => 3,
        Some(StyleEdition::Edition2024) => 4,
    };

    usize_(&mut out, *max_width);
    bool_(&mut out, *hard_tabs);
    usize_(&mut out, *tab_spaces);
    out.push(newline(*newline_style));
    out.push(heuristics(*use_small_heuristics));
    opt(&mut out, *fn_call_width);
    opt(&mut out, *attr_fn_like_width);
    opt(&mut out, *struct_lit_width);
    opt(&mut out, *struct_variant_width);
    opt(&mut out, *array_width);
    opt(&mut out, *chain_width);
    opt(&mut out, *single_line_if_else_max_width);
    opt(&mut out, *single_line_let_else_max_width);
    bool_(&mut out, *reorder_imports);
    bool_(&mut out, *reorder_modules);
    bool_(&mut out, *remove_nested_parens);
    usize_(&mut out, *short_array_element_width_threshold);
    out.push(pipes(*match_arm_leading_pipes));
    out.push(density(*fn_params_layout));
    bool_(&mut out, *match_block_trailing_comma);
    out.push(edition_(*edition));
    out.push(style(*style_edition));
    bool_(&mut out, *merge_derives);
    bool_(&mut out, *use_try_shorthand);
    bool_(&mut out, *use_field_init_shorthand);
    bool_(&mut out, *force_explicit_abi);
    bool_(&mut out, *disable_all_formatting);
    out
}

/// Identity of the formatter: crate version, cache format version, target, and the digest
/// of the running executable. `None` when the executable cannot be read.
pub fn tool_key() -> Option<Digest> {
    let exe = std::env::current_exe().ok()?;
    let bytes = fs::read(exe).ok()?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"chloro fixed-point cache\0");
    hasher.update(&FORMAT_VERSION.to_le_bytes());
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(b"\0");
    hasher.update(std::env::consts::OS.as_bytes());
    hasher.update(b"\0");
    hasher.update(std::env::consts::ARCH.as_bytes());
    hasher.update(b"\0");
    hasher.update(blake3::hash(&bytes).as_bytes());
    Some(*hasher.finalize().as_bytes())
}

/// File name of the cache for one (tool, config) pair.
pub fn file_name(tool: &Digest, config: &Config) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(tool);
    hasher.update(&config_key_bytes(config));
    let digest = hasher.finalize();
    let mut name = String::with_capacity(32 + FILE_SUFFIX.len());
    for byte in &digest.as_bytes()[..16] {
        name.push_str(&format!("{byte:02x}"));
    }
    name.push_str(FILE_SUFFIX);
    name
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
    generation: u32,
    entries: HashMap<Digest, u32>,
}

impl Cache {
    /// Read the cache file for `config` under `dir`. A missing, unreadable, or malformed file
    /// gives an empty cache at the same path.
    pub fn open(dir: &Path, config: &Config) -> Option<Self> {
        let tool = tool_key()?;
        let path = dir.join(file_name(&tool, config));
        let (generation, entries) = fs::read(&path)
            .ok()
            .and_then(|bytes| parse(&bytes))
            .unwrap_or_default();
        Some(Cache {
            path,
            generation,
            entries,
        })
    }

    /// Path of the cache file.
    pub fn path(&self) -> &Path {
        &self.path
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
    /// or renaming it over the cache file.
    pub fn save(mut self, seen: impl IntoIterator<Item = Digest>) -> io::Result<()> {
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
        for (digest, seen) in &records {
            bytes.extend_from_slice(digest);
            bytes.extend_from_slice(&seen.to_le_bytes());
        }

        let dir = self.path.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(dir)?;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        let tmp = dir.join(format!(".{}.{}.tmp", std::process::id(), nanos));
        let written = fs::write(&tmp, &bytes).and_then(|()| fs::rename(&tmp, &self.path));
        if written.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        written?;
        remove_stale_files(dir);
        Ok(())
    }
}

/// Parse a cache file into its generation and entries. `None` for any layout mismatch.
fn parse(bytes: &[u8]) -> Option<(u32, HashMap<Digest, u32>)> {
    if bytes.len() < HEADER_LEN || &bytes[..8] != MAGIC {
        return None;
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().ok()?);
    if version != FORMAT_VERSION {
        return None;
    }
    let generation = u32::from_le_bytes(bytes[12..16].try_into().ok()?);
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

/// Remove cache files in `dir` not written for `STALE_FILE_AGE`. Errors are ignored.
fn remove_stale_files(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_cache_file = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(FILE_SUFFIX));
        if !is_cache_file {
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
        assert_eq!(key.len(), 48);
        assert_eq!(
            hex(&key),
            "6400000000000000000400000000000000000000000000000000000101010a0000000000000001010003000100000100"
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
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        bytes.extend_from_slice(&7u32.to_le_bytes());
        for (d, g) in [(d1, 3u32), (d2, 7u32)] {
            bytes.extend_from_slice(&d);
            bytes.extend_from_slice(&g.to_le_bytes());
        }
        let (generation, entries) = parse(&bytes).unwrap();
        assert_eq!(generation, 7);
        assert_eq!(entries.get(&d1), Some(&3));
        assert_eq!(entries.get(&d2), Some(&7));

        assert!(
            parse(&bytes[..bytes.len() - 1]).is_none(),
            "truncated record"
        );
        assert!(parse(b"").is_none(), "empty");
        assert!(parse(b"not a cache file at all").is_none(), "foreign");
        let mut wrong_version = bytes.clone();
        wrong_version[8] ^= 1;
        assert!(parse(&wrong_version).is_none(), "format version");
    }

    #[test]
    fn save_then_open_keeps_entries_and_evicts_oldest() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::default();
        let tool = [9u8; 32];
        let path = dir.path().join(file_name(&tool, &config));
        let cache = Cache {
            path: path.clone(),
            generation: 0,
            entries: HashMap::new(),
        };
        let many: Vec<Digest> = (0..=MAX_ENTRIES as u32)
            .map(|i| content_digest(&i.to_le_bytes()))
            .collect();
        cache.save(many.iter().copied()).unwrap();
        let (generation, entries) = parse(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(generation, 1);
        assert_eq!(entries.len(), MAX_ENTRIES, "the bound applies on save");

        let cache = Cache {
            path: path.clone(),
            generation,
            entries,
        };
        let newer = content_digest(b"newer");
        cache.save([newer]).unwrap();
        let (generation, entries) = parse(&fs::read(&path).unwrap()).unwrap();
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
    fn resolve_dir_prefers_option_then_env() {
        assert_eq!(resolve_dir("/x/y"), Some(PathBuf::from("/x/y")));
        // The environment is process-wide; the test only checks that an explicit option
        // wins and that a non-empty result is produced from whatever is set.
        assert!(resolve_dir("").is_none_or(|p| !p.as_os_str().is_empty()));
    }
}
