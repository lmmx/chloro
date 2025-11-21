use crate::helpers::rustfmt;
use chloro_core::format_source;
use std::fs;
use std::path::PathBuf;

/// Result of comparing Chloro vs rustfmt
pub struct RoundTripResult {
    pub chloro: String,
    pub rustfmt: String,
}

fn normalize_code(code: &str) -> String {
    code.trim()
        .replace("\r\n", "\n")
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Compare Chloro output against rustfmt normalized output
pub fn compare_roundtrip(code: &str) -> RoundTripResult {
    let chloro_raw = format_source(code);
    let rustfmt_raw = rustfmt::format_with_rustfmt(code).unwrap_or_else(|_| code.to_string());

    let chloro = normalize_code(&chloro_raw);
    let rustfmt = normalize_code(&rustfmt_raw);

    RoundTripResult { chloro, rustfmt }
}

/// Optionally write roundtrip outputs to snapshots directory
pub fn write_snapshots(result: &RoundTripResult, name: &str) {
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("conformance")
        .join("snapshots");
    fs::create_dir_all(&output_dir).unwrap();

    let safe_name = name.replace('/', "_");

    fs::write(
        output_dir.join(format!("{}_chloro.rs", safe_name)),
        format!("{}\n", result.chloro),
    )
    .unwrap();

    fs::write(
        output_dir.join(format!("{}_rustfmt.rs", safe_name)),
        format!("{}\n", result.rustfmt),
    )
    .unwrap();
}
