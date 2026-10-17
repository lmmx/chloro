//! End-to-end test of `--version`.
#![cfg(feature = "cli")]

use assert_cmd::cargo::cargo_bin_cmd;

/// `--version` prints the crate version, plain for the release and otherwise `-dev` with
/// details: what `build.rs` found of the source (here, the test's own checkout).
#[test]
fn version_is_the_crate_version_plain_or_marked_dev() {
    let out = cargo_bin_cmd!("chloro").arg("--version").output().unwrap();
    assert!(out.status.success());
    let line = String::from_utf8(out.stdout).unwrap();
    let version = env!("CARGO_PKG_VERSION");
    let release = format!("chloro {version}\n");
    let dev = format!("chloro {version}-dev (");
    assert!(
        line == release || (line.starts_with(&dev) && line.ends_with(")\n")),
        "unexpected --version output: {line:?}"
    );
    assert_eq!(
        cargo_bin_cmd!("chloro").arg("-V").output().unwrap().stdout,
        line.as_bytes()
    );
}
