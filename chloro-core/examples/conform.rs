//! Offline conformance report.
//!
//! Formats every fixture under `tests/conformance/fixtures/rust-analyzer` and compares the
//! result with the rustfmt reference output checked in under
//! `tests/conformance/snapshots/ra`. No rustfmt subprocess is spawned, so the report is
//! deterministic and takes well under a second in release mode.
//!
//! ```text
//! cargo run --release -p chloro-core --example conform             # summary
//! cargo run --release -p chloro-core --example conform -- -w 20    # 20 worst files
//! cargo run --release -p chloro-core --example conform -- -d hir/src_lib   # diff one file
//! cargo run --release -p chloro-core --example conform -- -i       # idempotence check
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chloro_core::format_source;
use imara_diff::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};

struct Case {
    /// Snapshot key, e.g. `hir/src_lib`.
    key: String,
    source: String,
    expected: String,
}

fn snapshot_key(crate_name: &str, rel: &Path) -> String {
    let rel = rel.to_string_lossy();
    let stem = rel.strip_suffix(".rs").unwrap_or(&rel);
    format!(
        "{}/{}",
        crate_name.replace('-', "_"),
        stem.replace('/', "_")
    )
}

fn load_cases(root: &Path) -> Vec<Case> {
    let fixtures = root.join("fixtures").join("rust-analyzer");
    let snapshots = root.join("snapshots").join("ra");
    let mut cases = Vec::new();
    let mut crates: Vec<_> = fs::read_dir(&fixtures)
        .expect("fixtures directory")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    crates.sort();
    for crate_dir in crates {
        let crate_name = crate_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let mut files: Vec<PathBuf> = walkdir::WalkDir::new(&crate_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
            .collect();
        files.sort();
        for path in files {
            let key = snapshot_key(&crate_name, path.strip_prefix(&crate_dir).unwrap());
            let expected_path = snapshots.join(format!("{key}.rustfmt.rs"));
            let Ok(expected) = fs::read_to_string(&expected_path) else {
                continue;
            };
            let source = fs::read_to_string(&path).expect("fixture readable");
            cases.push(Case {
                key,
                source,
                expected,
            });
        }
    }
    cases
}

fn diff_text(expected: &str, actual: &str) -> (u32, u32, String) {
    let input = InternedInput::new(expected, actual);
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_lines(&input);
    let (add, rem) = (diff.count_additions(), diff.count_removals());
    let text = if add + rem == 0 {
        String::new()
    } else {
        let printer = BasicLineDiffPrinter(&input.interner);
        diff.unified_diff(&printer, UnifiedDiffConfig::default(), &input)
            .to_string()
    };
    (add, rem, text)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut worst = 0usize;
    let mut show: Vec<String> = Vec::new();
    let mut idempotence = false;
    let mut filter: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-w" => worst = it.next().and_then(|n| n.parse().ok()).unwrap_or(20),
            "-d" => show.push(it.next().cloned().unwrap_or_default()),
            "-i" => idempotence = true,
            "-f" => filter = it.next().cloned(),
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("conformance");
    let mut cases = load_cases(&root);
    if let Some(f) = &filter {
        cases.retain(|c| c.key.contains(f.as_str()));
    }

    let mut elapsed = Duration::ZERO;
    let mut bytes = 0usize;
    let mut identical = 0usize;
    let mut total_add = 0u64;
    let mut total_rem = 0u64;
    let mut per_file: Vec<(u32, &str)> = Vec::new();
    let mut not_idempotent: Vec<&str> = Vec::new();

    for case in &cases {
        bytes += case.source.len();
        let start = Instant::now();
        let out = format_source(&case.source);
        elapsed += start.elapsed();
        let (add, rem, text) = diff_text(&case.expected, &out);
        if add + rem == 0 {
            identical += 1;
        }
        total_add += u64::from(add);
        total_rem += u64::from(rem);
        per_file.push((add + rem, &case.key));
        if show.contains(&case.key) {
            println!("=== {} (- rustfmt, + chloro) ===\n{text}", case.key);
        }
        if idempotence && format_source(&out) != out {
            not_idempotent.push(&case.key);
        }
    }

    per_file.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    for (n, key) in per_file.iter().take(worst) {
        println!("{n:6}  {key}");
    }
    if idempotence {
        println!("not idempotent: {}", not_idempotent.len());
        for key in &not_idempotent {
            println!("  {key}");
        }
    }
    let secs = elapsed.as_secs_f64();
    println!(
        "identical: {identical}/{} | diff lines: +{total_add} -{total_rem} | {:.1} MB in {:.3}s ({:.1} MB/s)",
        cases.len(),
        bytes as f64 / 1e6,
        secs,
        bytes as f64 / 1e6 / secs.max(1e-9),
    );
}
