//! End-to-end tests of the CLI's fixed-point cache.
//!
//! Every test points `CHLORO_CACHE_DIR` at its own temporary directory, so the user's cache
//! is neither read nor written.
#![cfg(feature = "cli")]

mod common;

use std::fs;
use std::path::Path;
use tempfile::TempDir;

const UNFORMATTED: &str = "fn main(){let x=1;}\n";
const FORMATTED: &str = "fn main() {\n    let x = 1;\n}\n";
const FORMATTED_TWO_SPACES: &str = "fn main() {\n  let x = 1;\n}\n";

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Run {
    /// Summary lines that must agree between a cold and a warm run.
    fn report(&self) -> Vec<&str> {
        self.stderr
            .lines()
            .filter(|l| {
                l.starts_with("Files ")
                    || l.starts_with("Error:")
                    || l.starts_with("Errors:")
                    || l.starts_with("  - ")
            })
            .collect()
    }

    fn cache_hits(&self) -> usize {
        self.stderr
            .lines()
            .find_map(|l| l.strip_prefix("Cache hits: "))
            .map(|n| n.trim().parse().unwrap())
            .unwrap_or_else(|| panic!("no `Cache hits:` line in\n{}", self.stderr))
    }

    fn files_changed(&self) -> usize {
        self.stderr
            .lines()
            .find_map(|l| l.strip_prefix("Files changed: "))
            .map(|n| n.trim().parse().unwrap())
            .unwrap_or_else(|| panic!("no `Files changed:` line in\n{}", self.stderr))
    }
}

/// Run `chloro -v <args>` with the cache under `cache_dir`.
fn chloro(cache_dir: &Path, args: &[&str]) -> Run {
    let output = common::chloro()
        .env("CHLORO_CACHE_DIR", cache_dir)
        .arg("-v")
        .args(args)
        .output()
        .unwrap();
    Run {
        code: output.status.code().unwrap(),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

/// A source tree of `formatted` already-formatted files and `unformatted` others.
fn tree(formatted: usize, unformatted: usize) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..formatted {
        fs::write(
            dir.path().join(format!("ok_{i}.rs")),
            format!("fn ok_{i}() {{\n    let x = {i};\n}}\n"),
        )
        .unwrap();
    }
    for i in 0..unformatted {
        fs::write(
            dir.path().join(format!("bad_{i}.rs")),
            format!("fn bad_{i}(){{let x={i};}}\n"),
        )
        .unwrap();
    }
    dir
}

/// Every file in the cache directory.
fn dir_entries(cache_dir: &Path) -> Vec<std::path::PathBuf> {
    match fs::read_dir(cache_dir) {
        Ok(entries) => entries.map(|e| e.unwrap().path()).collect(),
        Err(_) => Vec::new(),
    }
}

/// The files of the directory whose names end in `suffix`.
fn files_ending(cache_dir: &Path, suffix: &str) -> Vec<std::path::PathBuf> {
    dir_entries(cache_dir)
        .into_iter()
        .filter(|p| p.to_str().unwrap().ends_with(suffix))
        .collect()
}

/// The cache files (one per binary and configuration).
fn cache_files(cache_dir: &Path) -> Vec<std::path::PathBuf> {
    files_ending(cache_dir, ".chloro-cache")
}

/// The records of executable digests (one per executable identity).
fn exe_records(cache_dir: &Path) -> Vec<std::path::PathBuf> {
    files_ending(cache_dir, ".chloro-exe")
}

#[test]
fn cold_run_populates_and_warm_run_hits_with_the_same_report() {
    let cache = tempfile::tempdir().unwrap();
    let src = tree(5, 2);
    let path = src.path().to_str().unwrap();

    let cold = chloro(cache.path(), &["--check", path]);
    assert_eq!(cold.code, 1);
    assert_eq!(cold.cache_hits(), 0);
    assert_eq!(cold.files_changed(), 2);
    let files = cache_files(cache.path());
    assert_eq!(
        files.len(),
        1,
        "one cache file for the (binary, config) pair"
    );
    assert!(files[0].to_str().unwrap().ends_with(".chloro-cache"));
    assert_eq!(exe_records(cache.path()).len(), 1, "one executable record");
    assert_eq!(
        dir_entries(cache.path()).len(),
        2,
        "nothing else in the directory"
    );

    let warm = chloro(cache.path(), &["--check", path]);
    assert_eq!(warm.code, cold.code);
    assert_eq!(warm.report(), cold.report());
    assert_eq!(warm.cache_hits(), 5, "every formatted file is a hit");
    assert_eq!(warm.files_changed(), 2);
}

#[test]
fn editing_one_file_invalidates_only_that_file() {
    let cache = tempfile::tempdir().unwrap();
    let src = tree(4, 0);
    let path = src.path().to_str().unwrap();

    assert_eq!(chloro(cache.path(), &["--check", path]).code, 0);
    assert_eq!(chloro(cache.path(), &["--check", path]).cache_hits(), 4);

    // Still formatted, different content.
    fs::write(
        src.path().join("ok_0.rs"),
        "fn ok_0() {\n    let y = 0;\n}\n",
    )
    .unwrap();
    let run = chloro(cache.path(), &["--check", path]);
    assert_eq!(run.code, 0);
    assert_eq!(run.cache_hits(), 3);

    // The new content is recorded in turn.
    assert_eq!(chloro(cache.path(), &["--check", path]).cache_hits(), 4);
}

#[test]
fn changing_config_invalidates_everything() {
    let cache = tempfile::tempdir().unwrap();
    let src = tree(3, 0);
    let path = src.path().to_str().unwrap();

    assert_eq!(chloro(cache.path(), &["--check", path]).code, 0);
    assert_eq!(chloro(cache.path(), &["--check", path]).cache_hits(), 3);

    // Under tab_spaces=2 the files are not formatted, and nothing is a hit.
    let other = chloro(cache.path(), &["--check", "--config", "tab_spaces=2", path]);
    assert_eq!(other.code, 1);
    assert_eq!(other.cache_hits(), 0);
    assert_eq!(other.files_changed(), 3);
    assert_eq!(cache_files(cache.path()).len(), 2, "one file per config");

    // An option set to its default value reads the default cache.
    let same = chloro(cache.path(), &["--check", "--config", "tab_spaces=4", path]);
    assert_eq!(same.code, 0);
    assert_eq!(same.cache_hits(), 3);
    assert_eq!(cache_files(cache.path()).len(), 2);
}

#[test]
fn corrupt_cache_file_is_ignored_and_replaced() {
    let cache = tempfile::tempdir().unwrap();
    let src = tree(3, 1);
    let path = src.path().to_str().unwrap();

    let cold = chloro(cache.path(), &["--check", path]);
    let file = cache_files(cache.path()).remove(0);
    let valid = fs::read(&file).unwrap();

    for garbage in [
        b"".to_vec(),
        b"not a cache file".to_vec(),
        valid[..valid.len() - 1].to_vec(),
        {
            let mut wrong_magic = valid.clone();
            wrong_magic[0] ^= 0xff;
            wrong_magic
        },
    ] {
        fs::write(&file, &garbage).unwrap();
        let run = chloro(cache.path(), &["--check", path]);
        assert_eq!(run.code, cold.code);
        assert_eq!(run.report(), cold.report());
        assert_eq!(run.cache_hits(), 0, "a corrupt file gives no hits");
        assert_eq!(fs::read(&file).unwrap(), valid, "the file is rewritten");
    }
}

#[test]
fn no_cache_bypasses_reading_and_writing() {
    let cache = tempfile::tempdir().unwrap();
    let src = tree(3, 0);
    let path = src.path().to_str().unwrap();

    let run = chloro(cache.path(), &["--check", "--no-cache", path]);
    assert_eq!(run.code, 0);
    assert_eq!(run.cache_hits(), 0);
    assert!(run.stderr.contains("Cache: disabled (--no-cache)"));
    assert!(dir_entries(cache.path()).is_empty(), "nothing written");

    // Populate, then bypass: still no hits.
    assert_eq!(chloro(cache.path(), &["--check", path]).code, 0);
    assert_eq!(chloro(cache.path(), &["--check", path]).cache_hits(), 3);
    let run = chloro(cache.path(), &["--check", "--no-cache", path]);
    assert_eq!(run.cache_hits(), 0);
}

#[test]
fn cache_dir_option_overrides_the_environment() {
    let env_dir = tempfile::tempdir().unwrap();
    let opt_dir = tempfile::tempdir().unwrap();
    let src = tree(2, 0);
    let path = src.path().to_str().unwrap();

    let run = chloro(
        env_dir.path(),
        &[
            "--check",
            "--cache-dir",
            opt_dir.path().to_str().unwrap(),
            path,
        ],
    );
    assert_eq!(run.code, 0);
    assert_eq!(cache_files(opt_dir.path()).len(), 1);
    assert_eq!(exe_records(opt_dir.path()).len(), 1);
    assert!(dir_entries(env_dir.path()).is_empty());
}

#[test]
fn write_then_rerun_reports_nothing_to_do_from_the_cache() {
    let cache = tempfile::tempdir().unwrap();
    let src = tree(2, 3);
    let path = src.path().to_str().unwrap();

    let write = chloro(cache.path(), &["--write", path]);
    assert_eq!(write.code, 0);
    assert_eq!(write.files_changed(), 3);
    assert_eq!(write.cache_hits(), 0);
    for i in 0..3 {
        assert_eq!(
            fs::read_to_string(src.path().join(format!("bad_{i}.rs"))).unwrap(),
            format!("fn bad_{i}() {{\n    let x = {i};\n}}\n")
        );
    }

    // Rewritten files were verified as fixed points and recorded.
    let again = chloro(cache.path(), &["--write", path]);
    assert_eq!(again.code, 0);
    assert_eq!(again.files_changed(), 0);
    assert_eq!(again.cache_hits(), 5);

    let check = chloro(cache.path(), &["--check", path]);
    assert_eq!(check.code, 0);
    assert_eq!(check.files_changed(), 0);
    assert_eq!(check.cache_hits(), 5);
}

#[test]
fn a_file_that_would_change_is_never_a_hit() {
    let cache = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let file = src.path().join("lib.rs");
    fs::write(&file, UNFORMATTED).unwrap();
    let path = file.to_str().unwrap();

    for _ in 0..3 {
        let run = chloro(cache.path(), &["--check", path]);
        assert_eq!(run.code, 1);
        assert_eq!(run.files_changed(), 1);
        assert_eq!(run.cache_hits(), 0);
        assert_eq!(fs::read_to_string(&file).unwrap(), UNFORMATTED);
    }
    let cache_file = cache_files(cache.path()).remove(0);
    // Magic, format version, generation and the 32-byte file key.
    assert_eq!(
        fs::read(cache_file).unwrap().len(),
        48,
        "header only: no digest recorded"
    );

    // Print mode (no --check, no --write) records nothing for it either.
    let run = chloro(cache.path(), &[path]);
    assert_eq!(run.code, 0);
    assert_eq!(run.stdout, FORMATTED);
    assert_eq!(run.cache_hits(), 0);
    assert_eq!(chloro(cache.path(), &[path]).cache_hits(), 0);
}

#[test]
fn print_mode_output_is_identical_on_a_hit() {
    let cache = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let file = src.path().join("lib.rs");
    fs::write(&file, FORMATTED).unwrap();
    let path = file.to_str().unwrap();

    let cold = chloro(cache.path(), &[path]);
    assert_eq!(cold.code, 0);
    assert_eq!(cold.stdout, FORMATTED);
    assert_eq!(cold.cache_hits(), 0);

    let warm = chloro(cache.path(), &[path]);
    assert_eq!(warm.code, 0);
    assert_eq!(warm.stdout, FORMATTED);
    assert_eq!(warm.cache_hits(), 1);

    // The same content under another configuration is formatted, not replayed.
    let other = chloro(cache.path(), &["--config", "tab_spaces=2", path]);
    assert_eq!(other.stdout, FORMATTED_TWO_SPACES);
    assert_eq!(other.cache_hits(), 0);
}

#[test]
fn check_output_and_exit_code_are_unchanged_without_verbose() {
    let cache = tempfile::tempdir().unwrap();
    let src = tree(2, 1);
    let path = src.path().to_str().unwrap();
    let run = |args: &[&str]| {
        common::chloro()
            .env("CHLORO_CACHE_DIR", cache.path())
            .args(args)
            .output()
            .unwrap()
    };

    let cold = run(&["--check", path]);
    let warm = run(&["--check", path]);
    let bypass = run(&["--check", "--no-cache", path]);
    for out in [&warm, &bypass] {
        assert_eq!(out.status.code(), cold.status.code());
        assert_eq!(out.stdout, cold.stdout);
        assert_eq!(out.stderr, cold.stderr);
    }
    assert_eq!(cold.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&cold.stderr).trim(),
        "Error: 1 file(s) need formatting"
    );
}

/// Runs `exe -v <args>` with the cache under `cache_dir`, retrying while the kernel reports
/// the binary busy (a file descriptor this test wrote it through can be held for a moment by
/// a process another test thread is starting).
#[cfg(unix)]
fn run_exe(exe: &Path, cache_dir: &Path, args: &[&str]) -> Run {
    let mut attempts = 0;
    let output = loop {
        match std::process::Command::new(exe)
            .env("CHLORO_CACHE_DIR", cache_dir)
            .arg("-v")
            .args(args)
            .output()
        {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy && attempts < 50 => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            result => break result.unwrap(),
        }
    };
    Run {
        code: output.status.code().unwrap(),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

/// The executable's digest is recorded, and a binary changed on disk is not given the
/// entries of the binary it replaced: a longer file with its modification time put back,
/// then a rewrite in place of the same size with its modification time put back, each read
/// another cache file.
#[cfg(unix)]
#[test]
fn a_changed_binary_reads_another_cache_file() {
    let cache = tempfile::tempdir().unwrap();
    let bin = tempfile::tempdir().unwrap();
    let src = tree(2, 0);
    let path = src.path().to_str().unwrap();
    let exe = bin.path().join("chloro");
    fs::copy(env!("CARGO_BIN_EXE_chloro"), &exe).unwrap();

    let first = run_exe(&exe, cache.path(), &["--check", path]);
    assert_eq!(first.code, 0, "{}", first.stderr);
    assert_eq!(
        run_exe(&exe, cache.path(), &["--check", path]).cache_hits(),
        2
    );
    assert_eq!(cache_files(cache.path()).len(), 1);
    assert_eq!(exe_records(cache.path()).len(), 1, "recorded once");

    let rewrite = |bytes: &[u8]| {
        let modified = fs::metadata(&exe).unwrap().modified().unwrap();
        // Past the file system's timestamp granularity.
        std::thread::sleep(std::time::Duration::from_millis(50));
        let file = fs::File::options().write(true).open(&exe).unwrap();
        std::io::Write::write_all(&mut &file, bytes).unwrap();
        file.set_modified(modified).unwrap();
        drop(file);
    };
    let mut bytes = fs::read(&exe).unwrap();
    bytes.push(0);
    rewrite(&bytes);
    let longer = run_exe(&exe, cache.path(), &["--check", path]);
    assert_eq!(longer.code, 0, "{}", longer.stderr);
    assert_eq!(longer.cache_hits(), 0, "another binary, another cache file");
    assert_eq!(cache_files(cache.path()).len(), 2);

    *bytes.last_mut().unwrap() = 1;
    rewrite(&bytes);
    let same_size = run_exe(&exe, cache.path(), &["--check", path]);
    assert_eq!(same_size.code, 0, "{}", same_size.stderr);
    assert_eq!(same_size.cache_hits(), 0, "same size and modification time");
    assert_eq!(cache_files(cache.path()).len(), 3);
    assert_eq!(exe_records(cache.path()).len(), 3);
    assert_eq!(
        run_exe(&exe, cache.path(), &["--check", path]).cache_hits(),
        2
    );
}
