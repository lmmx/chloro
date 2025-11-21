use crate::helpers::compare_with_rustfmt;
use std::fs;
use std::path::PathBuf;

#[test]
fn compare_rust_analyzer_files() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("conformance")
        .join("fixtures")
        .join("rust-analyzer");

    if !fixtures_dir.exists() {
        eprintln!("Fixtures directory not found: {}", fixtures_dir.display());
        eprintln!("Skipping rust-analyzer comparison test");
        return;
    }

    let mut identical = 0;
    let mut different = 0;
    let mut total_bytes_original = 0;
    let mut total_bytes_chloro = 0;
    let mut total_bytes_rustfmt = 0;
    let mut files_by_status: Vec<(String, bool)> = Vec::new();

    for entry in walkdir::WalkDir::new(&fixtures_dir)
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
        eprintln!("Comparing: rust-analyzer/{}", name);

        let code = fs::read_to_string(path).unwrap();
        total_bytes_original += code.len();

        let result = compare_with_rustfmt(&code, &format!("rust-analyzer/{}", name));

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

    eprintln!();
    eprintln!("============================================================");
    eprintln!("RUST-ANALYZER COMPLETE SUMMARY");
    eprintln!("============================================================");
    eprintln!("Total files: {}", identical + different);
    eprintln!(
        "Identical to rustfmt: {} ({:.1}%)",
        identical,
        100.0 * identical as f64 / (identical + different) as f64
    );
    eprintln!(
        "Different from rustfmt: {} ({:.1}%)",
        different,
        100.0 * different as f64 / (identical + different) as f64
    );
    eprintln!();
    eprintln!("Total size (original): {} bytes", total_bytes_original);
    eprintln!("Total size (chloro):   {} bytes", total_bytes_chloro);
    eprintln!("Total size (rustfmt):  {} bytes", total_bytes_rustfmt);
    eprintln!();

    // Print files by status
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
