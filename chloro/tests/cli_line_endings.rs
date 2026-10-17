//! End-to-end tests of line endings and byte order marks: whether `--check` reports a file
//! and what `--write` leaves on disk, as rustfmt 1.9.0 decides (each expected result below
//! was observed with rustfmt). Under `newline_style = Auto`, a file whose only difference from
//! its formatted output is `\r\n` line endings or a byte order mark is formatted and left as
//! it is; under `Unix` and `Windows` it is rewritten.
#![cfg(feature = "cli")]

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const LF: &str = "fn a() {}\nfn b() {}\n";
const CRLF: &str = "fn a() {}\r\nfn b() {}\r\n";

/// `chloro <args>` under `dir`: the exit status and standard output.
fn chloro(dir: &Path, args: &[&str]) -> (i32, String) {
    let output = cargo_bin_cmd!("chloro")
        .env("CHLORO_CACHE_DIR", dir.join("cache"))
        .args(args)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    (output.status.code().unwrap(), stdout)
}

/// The `--check` status for `content` under `style`, and the file after `--write`.
fn check_and_write(content: &str, style: &str) -> (i32, String) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("f.rs");
    fs::write(&path, content).unwrap();
    let config = format!("newline_style={style}");
    let file = path.to_str().unwrap();
    let (check, _) = chloro(
        dir.path(),
        &["--no-cache", "--config", &config, "--check", file],
    );
    let (write, _) = chloro(
        dir.path(),
        &["--no-cache", "--config", &config, "--write", file],
    );
    assert_eq!(write, 0);
    (check, fs::read_to_string(&path).unwrap())
}

#[test]
fn auto_leaves_a_file_that_differs_only_in_line_endings_or_a_byte_order_mark() {
    for content in [
        LF,
        CRLF,
        "fn a() {}\nfn b() {}\r\n",
        "\u{feff}fn a() {}\nfn b() {}\n",
    ] {
        assert_eq!(
            check_and_write(content, "Auto"),
            (0, content.to_owned()),
            "{content:?}"
        );
    }
}

#[test]
fn auto_writes_lf_when_the_file_needs_formatting() {
    assert_eq!(
        check_and_write("fn a(){}\r\nfn b() {}\r\n", "Auto"),
        (1, LF.to_owned())
    );
}

#[test]
fn unix_and_windows_rewrite_every_other_line_ending() {
    let mixed = "fn a() {}\nfn b() {}\r\n";
    let bom = "\u{feff}fn a() {}\nfn b() {}\n";
    assert_eq!(check_and_write(LF, "Unix"), (0, LF.to_owned()));
    assert_eq!(check_and_write(CRLF, "Unix"), (1, LF.to_owned()));
    assert_eq!(check_and_write(mixed, "Unix"), (1, LF.to_owned()));
    assert_eq!(check_and_write(bom, "Unix"), (1, LF.to_owned()));
    assert_eq!(check_and_write(CRLF, "Windows"), (0, CRLF.to_owned()));
    assert_eq!(check_and_write(LF, "Windows"), (1, CRLF.to_owned()));
    assert_eq!(check_and_write(mixed, "Windows"), (1, CRLF.to_owned()));
    assert_eq!(check_and_write(bom, "Windows"), (1, CRLF.to_owned()));
}

#[test]
fn print_mode_prints_the_formatted_text_cold_and_warm() {
    // The `\r\n` file is formatted under `Auto`, but its formatted text has `\n` line
    // endings: the cache, which prints a hit's content as its output, does not record it.
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("f.rs");
    fs::write(&path, CRLF).unwrap();
    let file = path.to_str().unwrap();
    for _ in 0..2 {
        assert_eq!(chloro(dir.path(), &[file]), (0, LF.to_owned()));
    }
    assert_eq!(fs::read_to_string(&path).unwrap(), CRLF);
}
