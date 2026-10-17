//! End-to-end tests of the CLI's file list.
// Every case makes symlinks, so the file is Unix-only.
#![cfg(all(feature = "cli", unix))]

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

const UNFORMATTED: &str = "fn main(){let x=1;}\n";
const FORMATTED: &str = "fn main() {\n    let x = 1;\n}\n";

/// A file reachable twice through a symlink, and named twice on the command line, is
/// processed once: one report line, one write.
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

/// The `Processing:` lines of a `-v` run, in order.
fn processed(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter_map(|l| l.strip_prefix("Processing: "))
        .map(str::to_owned)
        .collect()
}

/// A symlinked directory inside a walked tree is walked under its canonical path, and a file
/// reached both through it and through the real directory is processed once.
#[test]
fn a_symlinked_directory_is_walked_under_its_canonical_path() {
    let src = tempfile::tempdir().unwrap();
    let real = src.path().join("real");
    fs::create_dir(&real).unwrap();
    let x = real.join("x.rs");
    fs::write(&x, UNFORMATTED).unwrap();
    std::os::unix::fs::symlink(&real, src.path().join("link")).unwrap();
    // A directory outside the walked root, reached only through a relative symlink.
    let outside = tempfile::tempdir().unwrap();
    let y = outside.path().join("y.rs");
    fs::write(&y, UNFORMATTED).unwrap();
    std::os::unix::fs::symlink(outside.path(), src.path().join("ext")).unwrap();

    let write = cargo_bin_cmd!("chloro")
        .args(["-v", "--write", "--no-cache", "--threads", "1"])
        .arg(src.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8(write.stderr).unwrap();
    assert_eq!(write.status.code(), Some(0), "{stderr}");
    let mut expected = vec![
        x.canonicalize().unwrap().display().to_string(),
        y.canonicalize().unwrap().display().to_string(),
    ];
    expected.sort();
    let mut seen = processed(&stderr);
    seen.sort();
    assert_eq!(seen, expected, "{stderr}");
    assert_eq!(fs::read_to_string(&x).unwrap(), FORMATTED);
    assert_eq!(fs::read_to_string(&y).unwrap(), FORMATTED);
}

/// A symlinked `.rs` file inside a walked directory is processed under its target's
/// canonical path, which need not end in `.rs`; the symlink stays in place.
#[test]
fn a_symlinked_file_inside_a_walked_directory_is_its_target() {
    let src = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("target.txt");
    fs::write(&target, UNFORMATTED).unwrap();
    let dir = src.path().join("dir");
    fs::create_dir(&dir).unwrap();
    let link = dir.join("a.rs");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    // A symlink whose name does not end in `.rs` is not a Rust file, whatever its target.
    std::os::unix::fs::symlink(&target, dir.join("b.txt")).unwrap();

    let check = cargo_bin_cmd!("chloro")
        .args(["-v", "--check", "--no-cache"])
        .arg(src.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8(check.stderr).unwrap();
    assert_eq!(check.status.code(), Some(1), "{stderr}");
    assert_eq!(
        processed(&stderr),
        [target.canonicalize().unwrap().display().to_string()],
        "{stderr}"
    );

    let write = cargo_bin_cmd!("chloro")
        .args(["--write", "--no-cache"])
        .arg(src.path())
        .output()
        .unwrap();
    assert_eq!(write.status.code(), Some(0));
    assert_eq!(fs::read_to_string(&target).unwrap(), FORMATTED);
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

/// Symlinks back into the tree being walked (a cycle) end the walk with each file once.
#[test]
fn a_symlink_cycle_is_walked_once() {
    let src = tempfile::tempdir().unwrap();
    let x = src.path().join("x.rs");
    fs::write(&x, FORMATTED).unwrap();
    let sub = src.path().join("sub");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("y.rs"), FORMATTED).unwrap();
    std::os::unix::fs::symlink(".", src.path().join("self")).unwrap();
    std::os::unix::fs::symlink("..", sub.join("up")).unwrap();

    let check = cargo_bin_cmd!("chloro")
        .args(["-v", "--check", "--no-cache"])
        .arg(src.path())
        .output()
        .unwrap();
    let stderr = String::from_utf8(check.stderr).unwrap();
    assert_eq!(check.status.code(), Some(0), "{stderr}");
    assert_eq!(processed(&stderr).len(), 2, "{stderr}");
}

/// Files are processed in the order of their canonical paths when their sizes are equal
/// (`Path` order compares components: `a/b.rs` sorts before `a-b.rs`), wherever a symlink
/// takes the walk.
#[test]
fn files_of_equal_size_are_processed_in_path_order() {
    let src = tempfile::tempdir().unwrap();
    let root = src.path().canonicalize().unwrap();
    for name in [
        "a-b.rs", "a.rs", "B.rs", "a/b.rs", "a/c/d.rs", "a/c.rs", "z/e.rs",
    ] {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, FORMATTED).unwrap();
    }
    // Walked before `a` by name, but its files are under `z`.
    std::os::unix::fs::symlink(root.join("z"), root.join("0link")).unwrap();

    let check = cargo_bin_cmd!("chloro")
        .args(["-v", "--check", "--no-cache", "--threads", "1"])
        .arg(&root)
        .output()
        .unwrap();
    let stderr = String::from_utf8(check.stderr).unwrap();
    assert_eq!(check.status.code(), Some(0), "{stderr}");
    let expected: Vec<String> = [
        "B.rs", "a/b.rs", "a/c/d.rs", "a/c.rs", "a-b.rs", "a.rs", "z/e.rs",
    ]
    .iter()
    .map(|n| root.join(n).display().to_string())
    .collect();
    assert_eq!(processed(&stderr), expected, "{stderr}");
}
