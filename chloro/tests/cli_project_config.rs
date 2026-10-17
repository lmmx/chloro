//! End-to-end tests of the configuration of each file: its `rustfmt.toml` as rustfmt finds
//! it, its crate's edition as `cargo fmt` passes it, and `--config` over both. The expected
//! outputs of the workspace below are those of `cargo fmt` with rustfmt 1.9.0.
#![cfg(feature = "cli")]

mod common;

use std::fs;
use std::path::Path;
use tempfile::TempDir;

const SOURCE: &str = "use crate::{a10, a2};\nfn f() { let x = some_function(arg_one, arg_two); }\n";
/// Style edition before 2024 (`a10` before `a2`), at `max_width = 40`.
const OLD_NARROW: &str =
    "use crate::{a10, a2};\nfn f() {\n    let x =\n        some_function(arg_one, arg_two);\n}\n";
/// Style edition before 2024, at the default width.
const OLD_WIDE: &str =
    "use crate::{a10, a2};\nfn f() {\n    let x = some_function(arg_one, arg_two);\n}\n";
/// The defaults: style edition 2024 (version sorting), at the default width.
const DEFAULT: &str =
    "use crate::{a2, a10};\nfn f() {\n    let x = some_function(arg_one, arg_two);\n}\n";

/// A workspace whose packages take their edition three ways, with a `rustfmt.toml` at the
/// root (an unstable and an unknown option among its options) and a `.rustfmt.toml` in `b`.
fn workspace() -> TempDir {
    let dir = TempDir::new().unwrap();
    let files = [
        (
            "Cargo.toml",
            "[workspace]\nmembers = [\"a\", \"b\", \"c\"]\n\n[workspace.package]\nedition = \"2018\"\n",
        ),
        (
            "rustfmt.toml",
            "max_width = 40\nimports_granularity = \"Crate\"\nbogus = 1\n",
        ),
        (
            "a/Cargo.toml",
            "[package]\nname = \"a\"\nedition = \"2021\"\n",
        ),
        (
            "b/Cargo.toml",
            "[package]\nname = \"b\"\nedition.workspace = true\n",
        ),
        ("b/.rustfmt.toml", "max_width = 100\n"),
        ("c/Cargo.toml", "[package]\nname = \"c\"\n"),
        ("a/src/lib.rs", SOURCE),
        ("b/src/lib.rs", SOURCE),
        ("c/src/lib.rs", SOURCE),
    ];
    for (path, content) in files {
        let path = dir.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    dir
}

/// `chloro <args>` in `cwd`: the exit status and standard error.
fn chloro(cwd: &Path, args: &[&str]) -> (i32, String) {
    let output = common::chloro()
        .current_dir(cwd)
        .env("CHLORO_CACHE_DIR", cwd.join(".cache"))
        .args(args)
        .output()
        .unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    (output.status.code().unwrap(), stderr)
}

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap()
}

#[test]
fn a_workspace_is_formatted_as_cargo_fmt_formats_it() {
    let ws = workspace();
    let (code, stderr) = chloro(ws.path(), &["--write", "--no-cache"]);
    assert_eq!(code, 0, "{stderr}");
    // `a`: its own edition 2021, the root's width. `b`: the workspace's edition 2018, its
    // own `.rustfmt.toml` (preferred to a `rustfmt.toml`, and nearer than the root's). `c`:
    // no edition, so 2015, and the root's width.
    assert_eq!(read(ws.path(), "a/src/lib.rs"), OLD_NARROW);
    assert_eq!(read(ws.path(), "b/src/lib.rs"), OLD_WIDE);
    assert_eq!(read(ws.path(), "c/src/lib.rs"), OLD_NARROW);
    // The root file's warnings, once although two packages use it.
    let unknown = "Warning: Unknown configuration option `bogus`\n";
    let unstable = "Warning: can't set `imports_granularity = \"Crate\"`, unstable features are only available in nightly channel.\n";
    assert_eq!(stderr.matches(unknown).count(), 1, "{stderr}");
    assert_eq!(stderr.matches(unstable).count(), 1, "{stderr}");
}

#[test]
fn config_overrides_the_files_and_isolated_reads_neither() {
    let ws = workspace();
    let a = ws.path().join("a/src/lib.rs");
    let a = a.to_str().unwrap();
    for (args, expected) in [
        (vec!["--config", "max_width=100", a], OLD_WIDE),
        // The edition of `--config` over the package's.
        (
            vec!["--config", "edition=2024", a],
            "use crate::{a2, a10};\nfn f() {\n    let x =\n        some_function(arg_one, arg_two);\n}\n",
        ),
        (vec!["--isolated", a], DEFAULT),
    ] {
        let output = common::chloro()
            .current_dir(ws.path())
            .arg("--no-cache")
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected,
            "{args:?}"
        );
    }
}

#[test]
fn a_crates_edition_overrides_the_files_and_style_edition_does_not() {
    // As `cargo fmt`, whose `--edition` overrides the file's `edition` but not its
    // `style_edition`.
    let ws = workspace();
    let a = ws.path().join("a/src/lib.rs");
    for (toml, expected) in [
        ("edition = \"2024\"\n", OLD_WIDE),
        ("style_edition = \"2024\"\n", DEFAULT),
    ] {
        fs::write(ws.path().join("a/rustfmt.toml"), toml).unwrap();
        let output = common::chloro()
            .args(["--no-cache"])
            .arg(&a)
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected,
            "{toml}"
        );
    }
}

#[test]
fn config_path_names_the_file_for_every_file() {
    let ws = workspace();
    for path in ["b/.rustfmt.toml", "b"] {
        let (code, stderr) = chloro(ws.path(), &["--check", "--no-cache", "--config-path", path]);
        assert_eq!(code, 1, "{stderr}");
        let output = common::chloro()
            .current_dir(ws.path())
            .args(["--no-cache", "--config-path", path, "a/src/lib.rs"])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            OLD_WIDE,
            "{path}"
        );
    }
    let (code, stderr) = chloro(ws.path(), &["--config-path", "missing.toml"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("unable to find a config file"), "{stderr}");
    let (code, stderr) = chloro(ws.path(), &["--config-path", "b", "--isolated"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("cannot be used together"), "{stderr}");
}

#[test]
fn a_file_rustfmt_refuses_stops_the_run_before_any_file_is_written() {
    for (toml, detail) in [
        (
            "max_width = \"wide\"\n",
            "invalid type: string \"wide\", expected usize in `max_width`",
        ),
        ("max_width = \n", "TOML parse error"),
    ] {
        let ws = workspace();
        fs::write(ws.path().join("rustfmt.toml"), toml).unwrap();
        let (code, stderr) = chloro(ws.path(), &["--write", "--no-cache"]);
        assert_eq!(code, 1, "{stderr}");
        assert!(
            stderr.contains("failed to parse.\nError details: "),
            "{stderr}"
        );
        assert!(stderr.contains(detail), "{stderr}");
        for package in ["a", "b", "c"] {
            assert_eq!(read(ws.path(), &format!("{package}/src/lib.rs")), SOURCE);
        }
    }
}

#[test]
fn the_home_directorys_file_applies_where_no_project_has_one() {
    let home = TempDir::new().unwrap();
    fs::write(home.path().join("rustfmt.toml"), "max_width = 40\n").unwrap();
    let src = TempDir::new().unwrap();
    let file = src.path().join("lib.rs");
    fs::write(&file, SOURCE).unwrap();
    let output = common::chloro()
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .args(["--no-cache"])
        .arg(&file)
        .output()
        .unwrap();
    let narrow = "use crate::{a2, a10};\nfn f() {\n    let x =\n        some_function(arg_one, arg_two);\n}\n";
    assert_eq!(String::from_utf8(output.stdout).unwrap(), narrow);
}

#[test]
fn the_cache_follows_the_configuration_file() {
    let ws = workspace();
    let a = "a/src/lib.rs";
    fs::write(ws.path().join(a), OLD_NARROW).unwrap();
    // Formatted under the root's width: a fixed point, recorded in the cache.
    assert_eq!(chloro(ws.path(), &["--check", a]).0, 0);
    assert_eq!(chloro(ws.path(), &["--check", a]).0, 0);
    // Under another width the same content needs formatting: the cache does not answer.
    fs::write(ws.path().join("rustfmt.toml"), "max_width = 100\n").unwrap();
    assert_eq!(chloro(ws.path(), &["--check", a]).0, 1);
}

#[test]
fn the_release_is_read_before_the_options_it_makes_stable() {
    // `hex_literal_case` is stable from rustfmt 1.10.0, so it applies when the release is
    // 1.10.0, named after it in the file or on the command line, and is a warning before.
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("lib.rs");
    fs::write(&file, "const A: u8 = 0xab;\n").unwrap();
    for (toml, args, expected) in [
        (
            "hex_literal_case = \"Upper\"\nrustfmt_version = \"1.10.0\"\n",
            &[][..],
            "const A: u8 = 0xAB;\n",
        ),
        (
            "hex_literal_case = \"Upper\"\n",
            &["--config", "rustfmt_version=1.10.0"],
            "const A: u8 = 0xAB;\n",
        ),
        (
            "hex_literal_case = \"Upper\"\nrustfmt_version = \"1.10.0\"\n",
            &["--config", "rustfmt_version=1.9.0"],
            "const A: u8 = 0xab;\n",
        ),
        (
            "",
            &["--config", "hex_literal_case=upper,rustfmt_version=1.10.0"],
            "const A: u8 = 0xAB;\n",
        ),
    ] {
        fs::write(dir.path().join("rustfmt.toml"), toml).unwrap();
        let output = common::chloro()
            .args(["--no-cache"])
            .args(args)
            .arg(&file)
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected,
            "{toml} {args:?}"
        );
        let warned = String::from_utf8(output.stderr)
            .unwrap()
            .contains("can't set `hex_literal_case");
        assert_eq!(
            warned,
            args.contains(&"rustfmt_version=1.9.0"),
            "{toml} {args:?}"
        );
    }
}
