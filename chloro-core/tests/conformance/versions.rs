// tests/conformance/versions.rs
//
// Reproducers of the difference classes between rustfmt releases
// (`docs/journal/2026-10-14-autoresearch/rustfmt-versions.md`): every `.rs` file under
// `tests/conformance/rustfmt-versions` is formatted with `rustfmt_version` set to each
// release at editions 2024 and 2021 and compared with the release's own output, stored
// as `snapshots/rustfmt-versions/<case>.<version>.e<edition>.rustfmt.rs` (the input where
// that rustfmt rejects the file or panics on it).
//
// The snapshots are regenerated when `CHLORO_RUSTFMT_VERSIONS` names the toolchains of
// the releases other than 1.9.0 (e.g. `1.8.0=1.88.0`); 1.9.0 then comes from the `rustfmt`
// on `PATH`. The cases under `rustfmt-versions/partial/` are the documented partial cases:
// chloro's output is stored next to rustfmt's as `<case>.<version>.e<edition>.chloro.rs`
// and the test checks chloro against that file, so a change in the divergence is seen.

use crate::helpers::rustfmt::{format_with_rustfmt_command, rustfmt_versions};
use chloro_core::formatter::config::Config;
use chloro_core::formatter::format_source_with_config;
use std::fs;
use std::path::{Path, PathBuf};

const VERSIONS: &[&str] = &["1.9.0", "1.8.0"];
const EDITIONS: &[&str] = &["2024", "2021"];

fn cases(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

fn regenerate(code: &str, version: &str, edition: &str) -> Option<String> {
    let versions = rustfmt_versions();
    if versions.is_empty() {
        return None;
    }
    let command = if version == "1.9.0" {
        vec!["rustfmt".to_owned()]
    } else {
        versions.iter().find(|(v, _)| v == version)?.1.clone()
    };
    Some(
        format_with_rustfmt_command(code, &command, edition).unwrap_or_else(|e| {
            eprintln!("rustfmt {version} --edition {edition} rejected the input: {e}");
            code.to_owned()
        }),
    )
}

fn run(partial: bool) -> Vec<String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("conformance");
    let inputs = root.join("rustfmt-versions");
    let inputs = if partial {
        inputs.join("partial")
    } else {
        inputs
    };
    let snapshots = root.join("snapshots").join("rustfmt-versions");
    fs::create_dir_all(&snapshots).unwrap();
    let mut failures = Vec::new();
    let mut checked = 0;
    for path in cases(&inputs) {
        let case = path.file_stem().unwrap().to_string_lossy().into_owned();
        let code = fs::read_to_string(&path).unwrap();
        for version in VERSIONS {
            for edition in EDITIONS {
                let stem = format!("{case}.{version}.e{edition}");
                let rustfmt_path = snapshots.join(format!("{stem}.rustfmt.rs"));
                let chloro_path = snapshots.join(format!("{stem}.chloro.rs"));
                let mut config = Config::default();
                config.set("rustfmt_version", version).unwrap();
                config.set("edition", edition).unwrap();
                let chloro = format_source_with_config(&code, &config);
                let expected = match regenerate(&code, version, edition) {
                    Some(text) => {
                        fs::write(&rustfmt_path, &text).unwrap();
                        eprintln!("Wrote: {}", rustfmt_path.display());
                        if partial {
                            fs::write(&chloro_path, &chloro).unwrap();
                            eprintln!("Wrote: {}", chloro_path.display());
                        }
                        text
                    }
                    None => match fs::read_to_string(&rustfmt_path) {
                        Ok(text) => text,
                        Err(_) => {
                            eprintln!("No snapshot {}; skipped", rustfmt_path.display());
                            continue;
                        }
                    },
                };
                checked += 1;
                let target = if partial {
                    fs::read_to_string(&chloro_path).unwrap_or_else(|_| expected.clone())
                } else {
                    expected.clone()
                };
                if chloro == target {
                    eprintln!("✓ {stem}");
                } else {
                    eprintln!("✗ {stem}\n--- expected\n{target}\n--- chloro\n{chloro}");
                    failures.push(stem);
                }
                if partial && chloro == expected {
                    eprintln!("  (identical to rustfmt {version}: no longer partial)");
                }
            }
        }
    }
    eprintln!(
        "checked {checked} (case, version, edition) triples; {} failed",
        failures.len()
    );
    failures
}

#[test]
fn rustfmt_versions_reproducers() {
    let failures = run(false);
    assert!(failures.is_empty(), "differ from rustfmt: {failures:?}");
}

#[test]
fn rustfmt_versions_partial_cases() {
    let failures = run(true);
    assert!(
        failures.is_empty(),
        "differ from the recorded output: {failures:?}"
    );
}
