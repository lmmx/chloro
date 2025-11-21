use crate::helpers::compare_with_rustfmt;
use std::fs;
use std::path::PathBuf;

#[test]
fn compare_all_fixtures() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("conformance")
        .join("fixtures");

    if !fixtures_dir.exists() {
        eprintln!("Fixtures directory not found: {}", fixtures_dir.display());
        eprintln!("Skipping batch comparison test");
        return;
    }

    let mut identical = 0;
    let mut different = 0;

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
            .to_string();

        eprintln!();
        eprintln!("============================================================");
        eprintln!("Comparing: {}", name);

        let code = fs::read_to_string(path).unwrap();
        let result = compare_with_rustfmt(&code, &name);

        if result.chloro == result.rustfmt {
            identical += 1;
            eprintln!("✓ Identical to rustfmt");
        } else {
            different += 1;
            eprintln!("✗ Different from rustfmt");
        }
    }

    eprintln!();
    eprintln!("============================================================");
    eprintln!("SUMMARY");
    eprintln!("============================================================");
    eprintln!("Total fixtures: {}", identical + different);
    eprintln!("Identical to rustfmt: {}", identical);
    eprintln!("Different from rustfmt: {}", different);
    eprintln!();
}
