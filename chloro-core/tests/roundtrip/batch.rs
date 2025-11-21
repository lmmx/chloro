use crate::helpers::run_roundtrip;
use std::fs;
use std::path::{Path, PathBuf};

fn visit_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            visit_rs_files(&path, files);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn all_fixtures_are_idempotent() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    let mut failures = Vec::new();
    let mut rs_files = Vec::new();
    visit_rs_files(&fixtures_dir, &mut rs_files);

    for path in rs_files {
        let code = fs::read_to_string(&path).unwrap();
        let result = run_roundtrip(&code);

        if !result.is_idempotent {
            failures.push(path);
        }
    }

    assert!(
        failures.is_empty(),
        "Non-idempotent formatting in {} files:\n{}",
        failures.len(),
        failures
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
