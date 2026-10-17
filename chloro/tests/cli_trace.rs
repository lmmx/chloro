//! The run timeline written under `CHLORO_TRACE` (EXP-023's diagnostic): one line per phase
//! mark, file and worker, in order, and no change to the run's output or exit code.
#![cfg(feature = "cli")]

mod common;

use std::fs;
use std::path::Path;

const UNFORMATTED: &str = "fn main(){let x=1;}\n";
const FORMATTED: &str = "fn main() {\n    let x = 1;\n}\n";

/// The trace's lines of one kind, split into fields (the kind dropped).
fn lines<'a>(trace: &'a str, kind: &str) -> Vec<Vec<&'a str>> {
    trace
        .lines()
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .filter(|f| f[0] == kind)
        .map(|f| f[1..].to_vec())
        .collect()
}

fn num(field: &str) -> u64 {
    field.parse().unwrap()
}

/// A tree of three small files and one of at least 128 KiB (formatted on several workers).
fn tree(dir: &Path) -> Vec<(String, u64)> {
    fs::write(dir.join("a.rs"), FORMATTED).unwrap();
    fs::write(dir.join("b.rs"), UNFORMATTED).unwrap();
    fs::create_dir(dir.join("m")).unwrap();
    fs::write(dir.join("m/c.rs"), FORMATTED).unwrap();
    let big: String = (0..4000)
        .map(|i| format!("fn f{i}() {{\n    let x = {i};\n}}\n"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(big.len() >= 128 * 1024);
    fs::write(dir.join("big.rs"), &big).unwrap();
    ["a.rs", "b.rs", "m/c.rs", "big.rs"]
        .iter()
        .map(|p| {
            let len = fs::metadata(dir.join(p)).unwrap().len();
            (p.to_string(), len)
        })
        .collect()
}

#[test]
fn a_traced_run_records_its_marks_files_and_workers() {
    let src = tempfile::tempdir().unwrap();
    let files = tree(src.path());
    let out = tempfile::tempdir().unwrap();
    let trace_path = out.path().join("trace.tsv");

    let plain = common::chloro()
        .args(["--check", "--no-cache", "--threads", "2"])
        .arg(src.path())
        .output()
        .unwrap();
    let traced = common::chloro()
        .env("CHLORO_TRACE", &trace_path)
        .args(["--check", "--no-cache", "--threads", "2"])
        .arg(src.path())
        .output()
        .unwrap();
    assert_eq!(traced.status.code(), Some(1));
    assert_eq!(traced.status.code(), plain.status.code());
    assert_eq!(traced.stdout, plain.stdout);
    assert_eq!(traced.stderr, plain.stderr);

    let trace = fs::read_to_string(&trace_path).unwrap();
    let marks: Vec<(&str, u64)> = lines(&trace, "mark")
        .iter()
        .map(|f| (f[0], num(f[1])))
        .collect();
    let names: Vec<&str> = marks.iter().map(|m| m.0).collect();
    assert_eq!(
        names,
        [
            "args",
            "cache",
            "walk",
            "sort",
            "config",
            "spawn",
            "joined",
            "formatted",
            "saved",
            "finish"
        ]
    );
    assert!(marks.windows(2).all(|w| w[0].1 <= w[1].1), "{trace}");
    let at = |name: &str| marks.iter().find(|m| m.0 == name).unwrap().1;

    // file: worker, bytes, split, start, read, hashed, formatted, end
    let file_lines = lines(&trace, "file");
    let mut sizes: Vec<u64> = file_lines.iter().map(|f| num(f[1])).collect();
    sizes.sort_unstable();
    let mut expected: Vec<u64> = files.iter().map(|f| f.1).collect();
    expected.sort_unstable();
    assert_eq!(sizes, expected);
    for f in &file_lines {
        let [start, read, hashed, formatted, end] = [3, 4, 5, 6, 7].map(|i| num(f[i]));
        assert!(
            at("spawn") <= start && start <= read && read <= formatted,
            "{f:?}"
        );
        assert!(formatted <= end && end <= at("joined"), "{f:?}");
        assert_eq!(hashed, 0, "no digest without the cache: {f:?}");
        let big = num(f[1]) >= 128 * 1024;
        assert_eq!(f[2] == "1", big, "only the large file is split: {f:?}");
    }

    // worker: id, start, end; each file inside its worker's span
    let workers = lines(&trace, "worker");
    assert!(!workers.is_empty() && workers.len() <= 2, "{trace}");
    for f in &file_lines {
        let w = workers.iter().find(|w| w[0] == f[0]).unwrap();
        assert!(
            num(w[1]) <= num(f[3]) && num(f[7]) <= num(w[2]),
            "{f:?} {w:?}"
        );
    }
}

#[test]
fn a_traced_run_with_the_cache_stamps_the_digest() {
    let src = tempfile::tempdir().unwrap();
    tree(src.path());
    let out = tempfile::tempdir().unwrap();
    let trace_path = out.path().join("trace.tsv");
    let cache = out.path().join("cache");

    let traced = common::chloro()
        .env("CHLORO_TRACE", &trace_path)
        .args(["--check", "--threads", "1", "--cache-dir"])
        .arg(&cache)
        .arg(src.path())
        .output()
        .unwrap();
    assert_eq!(traced.status.code(), Some(1));
    let trace = fs::read_to_string(&trace_path).unwrap();
    let file_lines = lines(&trace, "file");
    assert_eq!(file_lines.len(), 4);
    for f in &file_lines {
        let [read, hashed, formatted] = [4, 5, 6].map(|i| num(f[i]));
        assert!(read <= hashed && hashed <= formatted && hashed > 0, "{f:?}");
        assert_eq!(f[2], "0", "one thread splits nothing: {f:?}");
    }
}

#[test]
fn an_untraced_run_writes_no_trace() {
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("a.rs"), FORMATTED).unwrap();
    let out = tempfile::tempdir().unwrap();
    let run = common::chloro()
        .env_remove("CHLORO_TRACE")
        .current_dir(out.path())
        .args(["--check", "--no-cache"])
        .arg(src.path())
        .output()
        .unwrap();
    assert_eq!(run.status.code(), Some(0));
    assert_eq!(fs::read_dir(out.path()).unwrap().count(), 0);
}
