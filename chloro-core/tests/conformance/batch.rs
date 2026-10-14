// tests/conformance/batch.rs

use crate::helpers::compare_with_rustfmt;
use crate::helpers::rustfmt::{format_with_rustfmt_command, rustfmt_versions};
use chloro_core::formatter::config::Config;
use chloro_core::formatter::format_source_with_config;
use std::fs;
use std::path::PathBuf;

/// Reference output of another rustfmt release for one fixture: the snapshot under
/// `snapshots/ra-<version>/` holds it only where it differs from the 1.9.0 snapshot, so
/// the file is written when the outputs differ and removed when they agree.
fn update_version_snapshot(
    version: &str,
    command: &[String],
    key: &str,
    code: &str,
    rustfmt_1_9_0: &str,
) -> (String, bool) {
    let expected = format_with_rustfmt_command(code, command, "2024").unwrap_or_else(|e| {
        eprintln!("rustfmt {version} rejected the input: {e}");
        code.to_owned()
    });
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("conformance")
        .join("snapshots")
        .join(format!("ra-{version}"))
        .join(format!("{key}.rustfmt.rs"));
    if expected == rustfmt_1_9_0 {
        if path.exists() {
            fs::remove_file(&path).unwrap();
            eprintln!("Removed: {}", path.display());
        }
    } else {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &expected).unwrap();
        eprintln!("Wrote: {}", path.display());
    }
    let mut config = Config::default();
    config.set("rustfmt_version", version).unwrap();
    let chloro = format_source_with_config(code, &config);
    let identical = chloro == expected;
    eprintln!(
        "rustfmt_version={version}: {}",
        if identical {
            "✓ identical"
        } else {
            "✗ different"
        }
    );
    (expected, identical)
}

/// Helper function to test all files in a rust-analyzer crate
fn compare_rust_analyzer_crate(crate_name: &str) {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("conformance")
        .join("fixtures")
        .join("rust-analyzer")
        .join(crate_name);

    if !fixtures_dir.exists() {
        eprintln!("Fixtures directory not found: {}", fixtures_dir.display());
        eprintln!("Skipping rust-analyzer/{} comparison test", crate_name);
        return;
    }

    let mut identical = 0;
    let mut different = 0;
    let mut total_bytes_original = 0;
    let mut total_bytes_chloro = 0;
    let mut total_bytes_rustfmt = 0;
    let mut files_by_status: Vec<(String, bool)> = Vec::new();
    let versions = rustfmt_versions();
    let mut identical_by_version = vec![0usize; versions.len()];
    let mut snapshots_by_version = vec![0usize; versions.len()];

    for entry in walkdir::WalkDir::new(&fixtures_dir)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
    {
        let path = entry.path();
        let name = path
            .strip_prefix(&fixtures_dir)
            .unwrap()
            .to_string_lossy()
            .strip_suffix(".rs")
            .unwrap_or_default()
            .to_string();

        eprintln!();
        eprintln!("============================================================");
        eprintln!("Comparing: {}/{}", crate_name, name);

        let code = fs::read_to_string(path).unwrap();
        total_bytes_original += code.len();

        let key = format!(
            "{}/{}",
            crate_name.replace('-', "_"),
            name.replace('/', "_")
        );
        let result = compare_with_rustfmt(&code, &format!("ra/{key}"));
        for (i, (version, command)) in versions.iter().enumerate() {
            let (expected, identical) =
                update_version_snapshot(version, command, &key, &code, &result.rustfmt);
            identical_by_version[i] += usize::from(identical);
            snapshots_by_version[i] += usize::from(expected != result.rustfmt);
        }

        total_bytes_chloro += result.chloro.len();
        total_bytes_rustfmt += result.rustfmt.len();

        let is_identical = result.chloro == result.rustfmt;
        files_by_status.push((name.clone(), is_identical));

        if is_identical {
            identical += 1;
            eprintln!("✓ Identical to rustfmt");
        } else {
            different += 1;
            eprintln!("✗ Different from rustfmt");
        }
    }

    let total = identical + different;
    if total == 0 {
        eprintln!("No files found in rust-analyzer/{}", crate_name);
        return;
    }

    eprintln!();
    eprintln!("============================================================");
    eprintln!("RUST-ANALYZER/{} SUMMARY", crate_name.to_uppercase());
    eprintln!("============================================================");
    eprintln!("Total files: {}", total);
    eprintln!(
        "Identical to rustfmt: {} ({:.1}%)",
        identical,
        100.0 * identical as f64 / total as f64
    );
    eprintln!(
        "Different from rustfmt: {} ({:.1}%)",
        different,
        100.0 * different as f64 / total as f64
    );
    for (i, (version, _)) in versions.iter().enumerate() {
        eprintln!(
            "rustfmt_version={version}: identical {} ({:.1}%), {} snapshots differ from 1.9.0",
            identical_by_version[i],
            100.0 * identical_by_version[i] as f64 / total as f64,
            snapshots_by_version[i]
        );
    }
    eprintln!();
    eprintln!("Total size (original): {} bytes", total_bytes_original);
    eprintln!("Total size (chloro):   {} bytes", total_bytes_chloro);
    eprintln!("Total size (rustfmt):  {} bytes", total_bytes_rustfmt);
    eprintln!();

    eprintln!("IDENTICAL FILES:");
    for (name, is_identical) in &files_by_status {
        if *is_identical {
            eprintln!("  ✓ {}", name);
        }
    }

    eprintln!();
    eprintln!("DIFFERENT FILES:");
    for (name, is_identical) in &files_by_status {
        if !*is_identical {
            eprintln!("  ✗ {}", name);
        }
    }
    eprintln!();
}

#[test]
fn compare_rust_analyzer_hir() {
    compare_rust_analyzer_crate("hir");
}

#[test]
fn compare_rust_analyzer_hir_def() {
    compare_rust_analyzer_crate("hir-def");
}

#[test]
fn compare_rust_analyzer_hir_expand() {
    compare_rust_analyzer_crate("hir-expand");
}

#[test]
fn compare_rust_analyzer_hir_ty() {
    compare_rust_analyzer_crate("hir-ty");
}

#[test]
fn compare_rust_analyzer_ide() {
    compare_rust_analyzer_crate("ide");
}

#[test]
fn compare_rust_analyzer_ide_assists() {
    compare_rust_analyzer_crate("ide-assists");
}

#[test]
fn compare_rust_analyzer_ide_completion() {
    compare_rust_analyzer_crate("ide-completion");
}

#[test]
fn compare_rust_analyzer_ide_db() {
    compare_rust_analyzer_crate("ide-db");
}

#[test]
fn compare_rust_analyzer_ide_diagnostics() {
    compare_rust_analyzer_crate("ide-diagnostics");
}

#[test]
fn compare_rust_analyzer_parser() {
    compare_rust_analyzer_crate("parser");
}

#[test]
fn compare_rust_analyzer_rust_analyzer() {
    compare_rust_analyzer_crate("rust-analyzer");
}

#[test]
fn compare_rust_analyzer_syntax() {
    compare_rust_analyzer_crate("syntax");
}
