//! End-to-end tests of which files a directory contributes: the walk skips hidden entries
//! and cache directories (a `CACHEDIR.TAG` file, as Cargo writes in `target/`), and a path
//! named on the command line is always used.
#![cfg(feature = "cli")]

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// The tag Cargo writes, as the Cache Directory Tagging Specification requires it to start.
const TAG: &str =
    "Signature: 8a477f597d28d172789f06886806bc55\n# This file is a cache directory tag.\n";

/// A project: sources, a Cargo `target/` with a generated source, hidden entries, and a
/// directory whose `CACHEDIR.TAG` lacks the signature.
fn project() -> TempDir {
    let dir = TempDir::new().unwrap();
    let files = [
        ("src/lib.rs", "fn a() {}\n"),
        ("src/inner/mod.rs", "fn b() {}\n"),
        ("src/.hidden.rs", "fn c() {}\n"),
        ("src/.cache/d.rs", "fn d() {}\n"),
        (".git/e.rs", "fn e() {}\n"),
        ("target/CACHEDIR.TAG", TAG),
        ("target/debug/build/out/f.rs", "fn f() {}\n"),
        ("notes/CACHEDIR.TAG", "not a tag\n"),
        ("notes/g.rs", "fn g() {}\n"),
    ];
    for (path, content) in files {
        let path = dir.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    dir
}

/// The files `chloro -v --check <args>` processes in `cwd`, relative to `root`, sorted.
fn processed(cwd: &Path, root: &Path, args: &[&str]) -> Vec<String> {
    let output = cargo_bin_cmd!("chloro")
        .current_dir(cwd)
        .args(["-v", "--check", "--no-cache"])
        .args(args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let root = fs::canonicalize(root).unwrap();
    let mut files: Vec<String> = String::from_utf8(output.stderr)
        .unwrap()
        .lines()
        .filter_map(|l| l.strip_prefix("Processing: "))
        .map(|p| {
            let path = Path::new(p).strip_prefix(&root).unwrap();
            let parts: Vec<_> = path.iter().map(|c| c.to_string_lossy()).collect();
            parts.join("/")
        })
        .collect();
    files.sort();
    files
}

#[test]
fn the_walk_skips_hidden_entries_and_cache_directories() {
    let dir = project();
    // With no source, the current directory.
    let expected = ["notes/g.rs", "src/inner/mod.rs", "src/lib.rs"];
    assert_eq!(processed(dir.path(), dir.path(), &[]), expected);
    assert_eq!(processed(dir.path(), dir.path(), &["."]), expected);
}

#[test]
fn a_source_named_on_the_command_line_is_always_used() {
    let dir = project();
    let args = ["target", "src/.hidden.rs", "src/.cache"];
    let expected = [
        "src/.cache/d.rs",
        "src/.hidden.rs",
        "target/debug/build/out/f.rs",
    ];
    assert_eq!(processed(dir.path(), dir.path(), &args), expected);
}

#[test]
fn help_gives_the_current_directory_as_the_default_source() {
    let output = cargo_bin_cmd!("chloro").arg("--help").output().unwrap();
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("(default: '.', the current"), "{help}");
}
