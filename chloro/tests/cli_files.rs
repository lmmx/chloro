//! End-to-end tests of the CLI's file list.
#![cfg(feature = "cli")]

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

const UNFORMATTED: &str = "fn main(){let x=1;}\n";
const FORMATTED: &str = "fn main() {\n    let x = 1;\n}\n";

/// A file reachable twice through a symlink, and named twice on the command line, is
/// processed once: one report line, one write.
#[cfg(unix)]
#[test]
fn a_file_reachable_several_ways_is_processed_once() {
    let src = tempfile::tempdir().unwrap();
    let b = src.path().join("b.rs");
    let a = src.path().join("a.rs");
    fs::write(&b, UNFORMATTED).unwrap();
    std::os::unix::fs::symlink(&b, &a).unwrap();
    let dir = src.path().to_str().unwrap();
    let b_str = b.to_str().unwrap();
    let a_str = a.to_str().unwrap();
    let sources = [dir, b_str, b_str, a_str];

    let check = cargo_bin_cmd!("chloro")
        .arg("--check")
        .arg("--no-cache")
        .args(sources)
        .output()
        .unwrap();
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&check.stderr).trim(),
        "Error: 1 file(s) need formatting"
    );
    assert_eq!(fs::read_to_string(&b).unwrap(), UNFORMATTED);

    let write = cargo_bin_cmd!("chloro")
        .args(["-v", "--write", "--no-cache"])
        .args(sources)
        .output()
        .unwrap();
    let stderr = String::from_utf8(write.stderr).unwrap();
    assert_eq!(write.status.code(), Some(0), "{stderr}");
    let count = |prefix: &str| stderr.lines().filter(|l| l.starts_with(prefix)).count();
    assert_eq!(count("Processing: "), 1, "{stderr}");
    assert_eq!(count("  Wrote formatted output"), 1, "{stderr}");
    assert!(stderr.contains("Files changed: 1"), "{stderr}");
    assert_eq!(fs::read_to_string(&b).unwrap(), FORMATTED);
    assert!(
        fs::symlink_metadata(&a).unwrap().file_type().is_symlink(),
        "the symlink is left in place"
    );
}
