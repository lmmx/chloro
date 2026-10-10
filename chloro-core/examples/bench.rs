//! In-process throughput benchmark and output-equivalence check.
//!
//! ```text
//! cargo run --release -p chloro-core --example bench                      # conformance fixtures
//! cargo run --release -p chloro-core --example bench -- -n 10 -w 10       # 10 rounds, 10 slowest files
//! cargo run --release -p chloro-core --example bench -- --root ~/.cargo/registry/src
//! cargo run --release -p chloro-core --example bench -- --record out.tsv  # hash every output
//! cargo run --release -p chloro-core --example bench -- --check out.tsv   # compare with a record
//! ```
//!
//! Times `format_source` over every `.rs` file under the roots (default: the conformance
//! fixtures), excluding file I/O and process startup, and reports the best of `-n` rounds.
//!
//! `--record` writes a hash of the formatted output of every file; `--check` formats again and
//! lists the files whose output hash changed. A change meant to be behaviour-preserving (a
//! performance change) should leave every hash as recorded.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    files
}

fn output_hash(output: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    output.hash(&mut hasher);
    hasher.finish()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut rounds = 5;
    let mut worst = 0;
    let mut roots = Vec::new();
    let mut record = None;
    let mut check = None;
    while let Some(arg) = args.next() {
        let mut value = || args.next().expect("option takes a value");
        match arg.as_str() {
            "-n" => rounds = value().parse().expect("-n takes a number"),
            "-w" => worst = value().parse().expect("-w takes a number"),
            "--root" => roots.push(PathBuf::from(value())),
            "--record" => record = Some(PathBuf::from(value())),
            "--check" => check = Some(PathBuf::from(value())),
            _ => panic!("unknown argument {arg}"),
        }
    }
    if roots.is_empty() {
        roots.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures"));
    }

    let files: Vec<(PathBuf, String)> = roots
        .iter()
        .flat_map(|root| files_under(root))
        .filter_map(|path| std::fs::read_to_string(&path).ok().map(|src| (path, src)))
        .collect();
    let bytes: usize = files.iter().map(|(_, src)| src.len()).sum();

    if record.is_some() || check.is_some() {
        let hashes: Vec<u64> = files
            .iter()
            .map(|(_, src)| output_hash(&chloro_core::format_source(src)))
            .collect();
        if let Some(path) = record {
            let text: String = files
                .iter()
                .zip(&hashes)
                .map(|((p, _), h)| format!("{h:016x}\t{}\n", p.display()))
                .collect();
            std::fs::write(&path, text).expect("write record");
            println!("recorded {} outputs to {}", files.len(), path.display());
        }
        if let Some(path) = check {
            let recorded: std::collections::HashMap<String, String> =
                std::fs::read_to_string(&path)
                    .expect("read record")
                    .lines()
                    .filter_map(|l| l.split_once('\t'))
                    .map(|(h, p)| (p.to_owned(), h.to_owned()))
                    .collect();
            let mut changed = 0;
            for ((p, _), h) in files.iter().zip(&hashes) {
                let key = p.display().to_string();
                match recorded.get(&key) {
                    Some(old) if *old == format!("{h:016x}") => {}
                    Some(_) => {
                        changed += 1;
                        println!("changed: {key}");
                    }
                    None => println!("not recorded: {key}"),
                }
            }
            println!("{changed} of {} outputs changed", files.len());
            if changed > 0 {
                std::process::exit(1);
            }
        }
        return;
    }

    let mut per_file = vec![f64::MAX; files.len()];
    let mut best = f64::MAX;
    for _ in 0..rounds {
        let start = Instant::now();
        for ((_, src), slot) in files.iter().zip(per_file.iter_mut()) {
            let t = Instant::now();
            std::hint::black_box(chloro_core::format_source(src));
            *slot = slot.min(t.elapsed().as_secs_f64());
        }
        best = best.min(start.elapsed().as_secs_f64());
    }
    let mb = bytes as f64 / 1e6;
    println!(
        "{} files, {mb:.1} MB: {best:.3}s, {:.2} MB/s (best of {rounds})",
        files.len(),
        mb / best
    );
    let mut order: Vec<usize> = (0..files.len()).collect();
    order.sort_by(|&a, &b| per_file[b].total_cmp(&per_file[a]));
    for &i in order.iter().take(worst) {
        let (path, src) = &files[i];
        println!(
            "{:8.2} ms {:6.2} MB/s  {}",
            per_file[i] * 1e3,
            src.len() as f64 / 1e6 / per_file[i],
            path.display()
        );
    }
}
