//! End-to-end tests of `--stdin`: standard input formatted to standard output, as `rustfmt`
//! formats it with no file, under the configuration of `--stdin-filepath` or of the current
//! directory.
#![cfg(feature = "cli")]

mod common;

use std::fs;
use std::path::Path;
use tempfile::TempDir;

const SOURCE: &str = "fn f() { let x = some_function(arg_one, arg_two); }\n";
const FORMATTED: &str = "fn f() {\n    let x = some_function(arg_one, arg_two);\n}\n";
const NARROW: &str = "fn f() {\n    let x =\n        some_function(arg_one, arg_two);\n}\n";

/// `chloro <args>` in `cwd` with `input` on standard input: the exit status, standard
/// output and standard error.
fn chloro(cwd: &Path, args: &[&str], input: &str) -> (i32, String, String) {
    let output = common::chloro()
        .current_dir(cwd)
        .env("CHLORO_CACHE_DIR", cwd.join(".cache"))
        .args(args)
        .write_stdin(input)
        .output()
        .unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

/// A project whose `rustfmt.toml` sets `max_width = 40`, with a source directory.
fn project() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("rustfmt.toml"), "max_width = 40\n").unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    dir
}

#[test]
fn standard_input_is_formatted_to_standard_output() {
    let dir = TempDir::new().unwrap();
    assert_eq!(
        chloro(dir.path(), &["--stdin"], SOURCE),
        (0, FORMATTED.to_owned(), String::new())
    );
    // `\r\n` reads as `\n`, as in a file.
    let crlf = SOURCE.replace('\n', "\r\n");
    assert_eq!(chloro(dir.path(), &["--stdin"], &crlf).1, FORMATTED);
    // No cache is written.
    assert!(!dir.path().join(".cache").exists());
}

#[test]
fn input_that_does_not_parse_writes_nothing_and_exits_with_1() {
    let dir = TempDir::new().unwrap();
    let (code, stdout, stderr) =
        chloro(dir.path(), &["--stdin"], "fn main() {\n    let x = ;\n}\n");
    assert_eq!((code, stdout.as_str()), (1, ""));
    assert_eq!(
        stderr,
        "error: expected expression, found `;`\n --> <stdin>:2:13\n  |\n2 |     let x = ;\n  |             ^\n\n"
    );
}

#[test]
fn the_configuration_is_the_files_or_the_current_directorys() {
    let dir = project();
    // `--stdin-filepath`: the file's configuration, and its path in messages; the file need
    // not exist.
    let elsewhere = TempDir::new().unwrap();
    let file = dir.path().join("src/new.rs");
    let args = ["--stdin", "--stdin-filepath", file.to_str().unwrap()];
    let (code, stdout, _) = chloro(elsewhere.path(), &args, SOURCE);
    assert_eq!((code, stdout.as_str()), (0, NARROW));
    let (_, _, stderr) = chloro(
        dir.path(),
        &["--stdin", "--stdin-filepath", "src/new.rs"],
        "fn main( {\n",
    );
    assert!(stderr.contains(" --> src/new.rs:1:10\n"), "{stderr}");
    // Without it, the current directory's, as rustfmt reads standard input.
    assert_eq!(
        chloro(&dir.path().join("src"), &["--stdin"], SOURCE).1,
        NARROW
    );
    assert_eq!(
        chloro(dir.path(), &["--stdin", "--isolated"], SOURCE).1,
        FORMATTED
    );
}

#[test]
fn check_exits_with_1_when_the_input_needs_formatting() {
    let dir = TempDir::new().unwrap();
    let (code, stdout, stderr) = chloro(dir.path(), &["--stdin", "--check"], SOURCE);
    assert_eq!((code, stdout.as_str()), (1, ""));
    assert_eq!(stderr, "Error: <stdin> needs formatting\n");
    assert_eq!(chloro(dir.path(), &["--stdin", "--check"], FORMATTED).0, 0);
    // Formatted but for its `\r\n` line endings: formatted, as rustfmt decides.
    let crlf = FORMATTED.replace('\n', "\r\n");
    assert_eq!(chloro(dir.path(), &["--stdin", "--check"], &crlf).0, 0);
}

#[test]
fn write_sources_and_a_lone_filepath_are_refused() {
    let dir = TempDir::new().unwrap();
    for args in [
        &["--stdin", "--write"][..],
        &["--stdin", "src"],
        &["--stdin-filepath", "a.rs"],
    ] {
        let (code, stdout, stderr) = chloro(dir.path(), args, SOURCE);
        assert_eq!((code, stdout.as_str()), (1, ""), "{args:?}");
        assert!(stderr.starts_with("Error: "), "{args:?}: {stderr}");
    }
}
