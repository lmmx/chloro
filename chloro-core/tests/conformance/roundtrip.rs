use super::helpers::roundtrip::{compare_roundtrip, write_snapshots};
use insta::assert_snapshot;
use std::fs;
use std::path::PathBuf;

/// Load all `.rs` files from the given crate into a single string
fn load_rust_analyzer_crate(crate_name: &str) -> String {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("conformance")
        .join("fixtures")
        .join("rust-analyzer")
        .join(crate_name);

    let mut all_code = String::new();

    for entry in walkdir::WalkDir::new(&crate_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
    {
        let code = fs::read_to_string(entry.path()).expect("Failed to read source file");
        all_code.push_str(&code);
        all_code.push('\n');
    }

    all_code
}

#[test]
fn snapshot_parser_crate_roundtrip_diff() {
    let code = load_rust_analyzer_crate("parser");
    let result = compare_roundtrip(&code);

    // Optionally write full chloro/rustfmt outputs
    write_snapshots(&result, "parser_crate");

    // Produce diff text using your existing roundtrip diff utility
    use imara_diff::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};

    let input = InternedInput::new(result.rustfmt.as_str(), result.chloro.as_str());
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_lines(&input);

    let config = UnifiedDiffConfig::default();
    let printer = BasicLineDiffPrinter(&input.interner);
    let unified_diff = diff.unified_diff(&printer, config, &input);
    let cleaned_diff = crate::helpers::strip_hunk_headers(&unified_diff.to_string());

    // Snapshot just the diff
    assert_snapshot!("parser_crate_roundtrip_diff", cleaned_diff);
}
