use chloro_core::format_source;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[ctor::ctor]
fn init_debug() {
    chloro_core::debug::set_debug(true);
}

pub struct RoundtripResult {
    pub original: String,
    pub formatted_once: String,
    pub formatted_twice: String,
    pub is_idempotent: bool,
}

pub fn run_roundtrip(code: &str) -> RoundtripResult {
    let formatted_once = format_source(code);
    let formatted_twice = format_source(&formatted_once);

    RoundtripResult {
        original: code.to_string(),
        formatted_once: formatted_once.clone(),
        formatted_twice: formatted_twice.clone(),
        is_idempotent: formatted_once == formatted_twice,
    }
}

pub fn load_fixture(name: &str) -> String {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("roundtrip")
        .join("fixtures")
        .join(format!("{}.rs", name));

    fs::read_to_string(fixture_path).unwrap_or_else(|_| panic!("Fixture not found: {}", name))
}

pub fn assert_idempotent(code: &str) {
    let result = run_roundtrip(code);
    assert!(
        result.is_idempotent,
        "Formatting not idempotent!\nOriginal:\n{}\n\nFirst:\n{}\n\nSecond:\n{}",
        result.original, result.formatted_once, result.formatted_twice
    );
}
