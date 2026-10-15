//! End-to-end tests of the CLI's fixed-point cache.
//!
//! Every test points `CHLORO_CACHE_DIR` at its own temporary directory, so the user's cache
//! is neither read nor written.
#![cfg(feature = "cli")]

use assert_cmd::cargo::cargo_bin_cmd;
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
    let output = cargo_bin_cmd!("chloro")
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

/// The fixed-point cache files in `cache_dir` (the unit cache's `.chloro-units` files next
/// to them are tested in `cli_unit_cache.rs`).
fn cache_files(cache_dir: &Path) -> Vec<std::path::PathBuf> {
    match fs::read_dir(cache_dir) {
        Ok(entries) => entries
            .map(|e| e.unwrap().path())
            .filter(|p| !p.to_string_lossy().ends_with(".chloro-units"))
            .collect(),
        Err(_) => Vec::new(),
    }
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
    assert!(cache_files(cache.path()).is_empty(), "nothing written");

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
    assert!(cache_files(env_dir.path()).is_empty());
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
        cargo_bin_cmd!("chloro")
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
