//! End-to-end tests of the CLI's unit cache: a large file edited in one item is formatted
//! from the cached units of the others, with the output of the sequential formatter.
//!
//! The unit cache is opt-in (`--unit-cache`). Every test points `CHLORO_CACHE_DIR` at its own
//! temporary directory.
#![cfg(feature = "cli")]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

/// A fixture over 128 KiB (so the CLI gives it several item threads) with 104 units.
fn fixture() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../chloro-core/tests/conformance/fixtures/rust-analyzer/rust-analyzer/src/config.rs",
    );
    fs::read_to_string(path).unwrap()
}

struct Run {
    code: i32,
    stderr: String,
}

impl Run {
    /// `(looked up, found, stored)` from the verbose unit cache line.
    fn units(&self) -> (usize, usize, usize) {
        let line = self
            .stderr
            .lines()
            .find(|l| l.starts_with("Unit cache: ") && l.contains("units looked up"))
            .unwrap_or_else(|| panic!("no unit cache line in\n{}", self.stderr));
        let number = |label: &str| -> usize {
            let rest = &line[line.find(label).unwrap() + label.len()..];
            rest.split(|c: char| !c.is_ascii_digit())
                .next()
                .unwrap()
                .parse()
                .unwrap()
        };
        (
            number("units looked up: "),
            number("found: "),
            number("stored: "),
        )
    }
}

fn chloro(cache_dir: &Path, args: &[&str], file: &Path) -> Run {
    let output = common::chloro()
        .env("CHLORO_CACHE_DIR", cache_dir)
        .arg("-v")
        .args(args)
        .arg(file)
        .output()
        .unwrap();
    Run {
        code: output.status.code().unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path
}

/// `source` with one letter of the last string literal that starts with a lowercase letter
/// changed: an edit inside one item that keeps the file formatted.
fn flip_literal(source: &str) -> String {
    let at = source
        .match_indices('"')
        .map(|(i, _)| i + 1)
        .rfind(|&i| source.as_bytes().get(i).is_some_and(u8::is_ascii_lowercase))
        .unwrap();
    let mut edited = source.to_owned();
    let flipped = if &source[at..=at] == "z" { "a" } else { "z" };
    edited.replace_range(at..=at, flipped);
    edited
}

#[test]
fn an_edit_to_one_item_formats_only_that_item() {
    let cache = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let original = fixture();
    let config = chloro_core::Config::default();
    assert!(original.len() >= 128 * 1024);

    // Cold: every unit formatted and stored.
    let file = write(work.path(), "config.rs", &original);
    let cold = chloro(cache.path(), &["--check", "--unit-cache"], &file);
    let (units, found, stored) = cold.units();
    assert!(units > 100, "{units} units");
    assert_eq!((found, stored), (0, units));
    let reference_code = chloro(cache.path(), &["--check", "--no-cache"], &file).code;
    assert_eq!(cold.code, reference_code);

    // One literal changed: the fixed-point cache misses, every other unit is found.
    let edited = flip_literal(&original);
    assert_ne!(edited, original);
    write(work.path(), "config.rs", &edited);
    let warm = chloro(cache.path(), &["--check", "--unit-cache"], &file);
    assert_eq!(warm.units(), (units, units - 1, 1));
    let expected_code =
        i32::from(chloro_core::format_source_with_config(&edited, &config) != edited);
    assert_eq!(warm.code, expected_code);
    assert_eq!(
        warm.code,
        chloro(cache.path(), &["--check", "--no-cache"], &file).code
    );

    // An edit that unformats one item: `--write` gives the sequential formatter's output.
    let unformatted = {
        let at = edited.rfind("pub fn ").unwrap();
        let mut text = edited.clone();
        text.replace_range(at..at + 7, "pub  fn  ");
        text
    };
    write(work.path(), "config.rs", &unformatted);
    let check = chloro(cache.path(), &["--check", "--unit-cache"], &file);
    assert_eq!(check.code, 1);
    assert_eq!(check.units(), (units, units - 1, 1));
    let written = chloro(cache.path(), &["--write", "--unit-cache"], &file);
    assert_eq!(written.code, 0);
    // The check run stored the edited unit: the write run finds every unit.
    assert_eq!(written.units(), (units, units, 0));
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        chloro_core::format_source_with_config(&unformatted, &config)
    );
}

#[test]
fn the_unit_cache_is_opt_in_and_no_cache_turns_it_off() {
    let cache = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let file = write(work.path(), "a.rs", "fn a(){}\n\nfn b() {}\n");
    for flags in [
        &["--check"][..],
        &["--check", "--no-cache"],
        &["--check", "--no-cache", "--unit-cache"],
    ] {
        let run = chloro(cache.path(), flags, &file);
        assert_eq!(run.code, 1);
        assert!(!run.stderr.contains("units looked up"), "{}", run.stderr);
    }
    let units_files = |dir: &Path| {
        fs::read_dir(dir).map_or(0, |entries| {
            entries
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().ends_with(".chloro-units"))
                .count()
        })
    };
    assert_eq!(units_files(cache.path()), 0);
    let run = chloro(cache.path(), &["--check", "--unit-cache"], &file);
    assert_eq!(run.code, 1);
    assert_eq!(run.units(), (2, 0, 2));
    assert_eq!(units_files(cache.path()), 1);
    // A second run finds both units; nothing new to write.
    let run = chloro(cache.path(), &["--check", "--unit-cache"], &file);
    assert_eq!(run.units(), (2, 2, 0));
    assert!(!run.stderr.contains("Unit cache written"), "{}", run.stderr);
    // Without the flag the file is neither read nor written.
    let run = chloro(cache.path(), &["--check"], &file);
    assert!(!run.stderr.contains("units looked up"), "{}", run.stderr);
}

#[test]
fn an_unchanged_tree_never_reads_the_unit_cache() {
    let cache = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let file = write(work.path(), "a.rs", "fn a() {}\n");
    let first = chloro(cache.path(), &["--check", "--unit-cache"], &file);
    assert_eq!(first.code, 0);
    // The second run answers the file from the fixed-point cache: no unit is looked up.
    let second = chloro(cache.path(), &["--check", "--unit-cache"], &file);
    assert_eq!(second.code, 0);
    assert!(
        !second.stderr.contains("units looked up"),
        "{}",
        second.stderr
    );
}
