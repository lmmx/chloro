//! Records at compile time whether this build is a tagged release, for `chloro --version`.
//!
//! The version is the plain package version only when the source is the release: a git
//! checkout whose HEAD is the commit of the tag `chloro-v<version>` with no uncommitted
//! changes to the files that go into the binary, or a packaged crate (crates.io) whose
//! `.cargo_vcs_info.json` records a clean source. Anything else is a development build,
//! `<version>-dev`, with the abbreviated commit and `dirty` where known. Without either
//! kind of metadata the source is reported as unknown rather than as the release.
//!
//! Sets `CHLORO_VERSION` (`0.8.0` or `0.8.0-dev`) and `CHLORO_VERSION_DETAILS` (empty, or
//! e.g. `2e1bc8f, dirty`). Only local files and the local `git` are read; a build never
//! fails because of them.

use std::path::Path;
use std::process::Command;

/// The files that go into the binary, relative to this crate (git pathspecs).
const INPUTS: &[&str] = &[
    "build.rs",
    "src",
    "Cargo.toml",
    "../chloro-core/src",
    "../chloro-core/Cargo.toml",
    "../Cargo.toml",
    "../Cargo.lock",
];

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

fn main() {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let (release, details) = classify(&version);
    let suffix = if release { "" } else { "-dev" };
    println!("cargo:rustc-env=CHLORO_VERSION={version}{suffix}");
    println!(
        "cargo:rustc-env=CHLORO_VERSION_DETAILS={}",
        details.join(", ")
    );
}

/// Whether the source is the tagged release, and the details to show beside the version.
fn classify(version: &str) -> (bool, Vec<String>) {
    // A packaged crate (crates.io, `cargo package`): Cargo records the commit it was
    // packaged from, and `"dirty": true` when the tree had uncommitted changes. Checked
    // first, so that a crate unpacked inside some other git repository (vendored) is not
    // described by that repository's HEAD.
    let vcs_info = Path::new(".cargo_vcs_info.json");
    if vcs_info.exists() {
        println!("cargo:rerun-if-changed=.cargo_vcs_info.json");
        let info = std::fs::read_to_string(vcs_info).unwrap_or_default();
        let dirty = info.contains("\"dirty\": true") || info.contains("\"dirty\":true");
        if !dirty {
            return (true, vec![]);
        }
        let sha = info
            .split("\"sha1\"")
            .nth(1)
            .and_then(|rest| rest.split('"').nth(1))
            .map(|sha| sha.chars().take(7).collect::<String>());
        return (false, sha.into_iter().chain(["dirty".to_owned()]).collect());
    }

    let Some(head) = git(&["rev-parse", "HEAD"]) else {
        return (false, vec!["unknown source".to_owned()]);
    };
    rerun_on_git_changes();

    let tag = format!(
        "refs/tags/{}-v{version}",
        std::env::var("CARGO_PKG_NAME").unwrap()
    );
    let tagged = git(&[
        "rev-parse",
        "--verify",
        "--quiet",
        &format!("{tag}^{{commit}}"),
    ])
    .is_some_and(|commit| commit == head);
    let mut status = vec!["status", "--porcelain", "--untracked-files=normal", "--"];
    status.extend(INPUTS);
    // Unknown status (git failed) counts as dirty: never report an unverified release.
    let dirty = git(&status).is_none_or(|out| !out.is_empty());

    if tagged && !dirty {
        return (true, vec![]);
    }
    let mut details = vec![head.chars().take(7).collect()];
    if dirty {
        details.push("dirty".to_owned());
    }
    (false, details)
}

/// Rerun when HEAD moves, a tag is added or moved, or an input file changes. Only existing
/// paths are listed (Cargo reruns on every build for a missing one); `logs/HEAD` changes on
/// every commit and checkout, also when the branch's ref is packed and has no file of its own.
fn rerun_on_git_changes() {
    for input in INPUTS {
        println!("cargo:rerun-if-changed={input}");
    }
    let mut paths = ["HEAD", "logs/HEAD", "packed-refs", "refs/tags"]
        .map(String::from)
        .to_vec();
    if let Some(branch) = git(&["symbolic-ref", "--quiet", "HEAD"]) {
        paths.push(branch);
    }
    for path in paths {
        if let Some(file) = git(&["rev-parse", "--git-path", &path])
            && Path::new(&file).exists()
        {
            println!("cargo:rerun-if-changed={file}");
        }
    }
}
