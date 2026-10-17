//! End-to-end tests of a file that does not parse: rustfmt leaves it as it is, prints the
//! parse error and exits with status 1, and so does chloro, whatever the mode; the other
//! files are formatted as usual.
#![cfg(feature = "cli")]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const BROKEN: &str = "fn main() {\n    let x = ;\n}\n";
const UNFORMATTED: &str = "fn main(){let x=1;}\n";
const FORMATTED: &str = "fn main() {\n    let x = 1;\n}\n";

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

/// Run `chloro <args>` with its cache under `dir`.
fn chloro(dir: &Path, args: &[&str]) -> Run {
    let output = common::chloro()
        .env("CHLORO_CACHE_DIR", dir.join("cache"))
        .args(args)
        .output()
        .unwrap();
    Run {
        code: output.status.code().unwrap(),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

/// Write `content` to `dir/name` and return its path, as the CLI names it (canonical).
fn file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    fs::canonicalize(path).unwrap()
}

/// The error rustfmt prints for `BROKEN` at `path`, with rustc's message in place of
/// rust-analyzer's.
fn broken_error(path: &Path) -> String {
    format!(
        "error: expected expression, found `;`\n --> {}:2:13\n  |\n2 |     let x = ;\n  |             ^\n\n",
        path.display()
    )
}

#[test]
fn check_prints_the_error_and_exits_with_1() {
    let dir = TempDir::new().unwrap();
    let broken = file(dir.path(), "broken.rs", BROKEN);
    let run = chloro(dir.path(), &["--check", broken.to_str().unwrap()]);
    assert_eq!(run.code, 1);
    assert_eq!(run.stderr, broken_error(&broken));
    assert_eq!(run.stdout, "");
}

#[test]
fn write_formats_the_other_files_and_leaves_the_broken_one() {
    let dir = TempDir::new().unwrap();
    let broken = file(dir.path(), "broken.rs", BROKEN);
    let other = file(dir.path(), "other.rs", UNFORMATTED);
    let run = chloro(dir.path(), &["--write", dir.path().to_str().unwrap()]);
    assert_eq!(run.code, 1);
    assert_eq!(run.stderr, broken_error(&broken));
    assert_eq!(fs::read_to_string(&broken).unwrap(), BROKEN);
    assert_eq!(fs::read_to_string(&other).unwrap(), FORMATTED);
}

#[test]
fn print_mode_prints_only_the_files_that_parse() {
    let dir = TempDir::new().unwrap();
    let broken = file(dir.path(), "broken.rs", BROKEN);
    let other = file(dir.path(), "other.rs", UNFORMATTED);
    let run = chloro(
        dir.path(),
        &[broken.to_str().unwrap(), other.to_str().unwrap()],
    );
    assert_eq!(run.code, 1);
    assert_eq!(run.stdout, FORMATTED);
    assert_eq!(run.stderr, broken_error(&broken));
}

#[test]
fn every_error_is_printed_in_path_order() {
    let dir = TempDir::new().unwrap();
    let a = file(dir.path(), "a.rs", BROKEN);
    let b = file(dir.path(), "b.rs", "fn f(\n");
    file(dir.path(), "c.rs", FORMATTED);
    let run = chloro(dir.path(), &["--check", dir.path().to_str().unwrap()]);
    assert_eq!(run.code, 1);
    let b_error = format!(
        "error: expected `)`, found `<eof>`\n --> {}:1:6\n  |\n1 | fn f(\n  |      ^\n\n",
        b.display()
    );
    assert_eq!(run.stderr, broken_error(&a) + &b_error);
}

#[test]
fn a_file_that_does_not_parse_is_never_cached() {
    let dir = TempDir::new().unwrap();
    let broken = file(dir.path(), "broken.rs", BROKEN);
    let good = file(dir.path(), "good.rs", FORMATTED);
    let args = [
        "-v",
        "--check",
        broken.to_str().unwrap(),
        good.to_str().unwrap(),
    ];
    let cold = chloro(dir.path(), &args);
    let warm = chloro(dir.path(), &args);
    for run in [&cold, &warm] {
        assert_eq!(run.code, 1);
        assert!(
            run.stderr.contains(&broken_error(&broken)),
            "{}",
            run.stderr
        );
        assert!(
            run.stderr.contains("Files that do not parse: 1\n"),
            "{}",
            run.stderr
        );
    }
    // The formatted file is answered from the cache; the broken one is formatted again.
    assert!(warm.stderr.contains("Cache hits: 1\n"), "{}", warm.stderr);
}

#[test]
fn files_that_parse_report_nothing() {
    let dir = TempDir::new().unwrap();
    let good = file(dir.path(), "good.rs", FORMATTED);
    let run = chloro(dir.path(), &["--check", good.to_str().unwrap()]);
    assert_eq!((run.code, run.stderr.as_str()), (0, ""));
}
