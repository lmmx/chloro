use std::fs;
use std::path::PathBuf;

pub fn load_fixture(name: &str) -> String {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("conformance")
        .join("fixtures")
        .join(format!("{}.rs", name));

    eprintln!("Loading fixture from: {}", fixture_path.display());

    fs::read_to_string(&fixture_path).unwrap_or_else(|e| {
        panic!(
            "Fixture not found: {} (error: {})",
            fixture_path.display(),
            e
        )
    })
}
