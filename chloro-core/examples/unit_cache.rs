//! Output identity and timing of `format_source_with_unit_cache` (experiment E15).
//!
//! ```text
//! cargo run --release -p chloro-core --example unit_cache                         # fixtures
//! cargo run --release -p chloro-core --example unit_cache -- --root ~/.cargo/registry/src
//! cargo run --release -p chloro-core --example unit_cache -- --threads 4 --shift --blank
//! cargo run --release -p chloro-core --example unit_cache -- --edit ORIG EDITED -n 20
//! ```
//!
//! For every `.rs` file under the roots (default: the conformance fixtures), formats the file
//! with `format_source_with_config` (the reference), then twice with
//! `format_source_with_unit_cache`: a cold pass (which stores every unit) and a warm pass
//! (which finds every unit), and compares both with the reference byte for byte. The cache is
//! exact: keyed by the whole key (context bytes and unit text), no hash. It is shared by all
//! files (cleared when it holds more than `--max-mb` megabytes), so a cold pass also finds the
//! units an earlier file had.
//!
//! - `--shift`: also formats the file with a comment line inserted at its start (every unit
//!   moves by the same bytes and one line) with the warm cache, and compares with the
//!   reference for that text: every position a hit replays is rebased.
//! - `--blank`: for every unit stored by the cold pass, formats the file with every byte
//!   outside the unit replaced by a space (newlines kept, so positions and lines are
//!   unchanged), with the warm cache (the unit hits) and without (the reference): a unit
//!   whose output read the source outside its own text would differ.
//! - `--every N`: only every `N`th file of the sorted list.
//! - `--edit ORIG EDITED`: warms a cache on ORIG, then times EDITED (best of `-n`) with a
//!   fresh copy of that cache per round against `format_source_with_config`, and checks
//!   the two are equal.
//!
//! Exits with status 1 on any difference.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use chloro_core::{CachedUnit, Config, UnitCache, UnitKey};

/// An exact in-memory unit cache: the key is the whole context and text.
#[derive(Default, Clone)]
struct Exact {
    map: RefCell<HashMap<(Vec<u8>, String), CachedUnit>>,
    /// Lookups and hits since the last `take_counts`.
    lookups: Cell<usize>,
    hits: Cell<usize>,
    inserts: Cell<usize>,
    /// Records found but not used, over the run.
    rejected: Cell<usize>,
    /// The texts stored since the last `take_inserted`.
    inserted: RefCell<Vec<String>>,
    bytes: Cell<usize>,
}

impl UnitCache for Exact {
    fn get(&self, key: &UnitKey<'_>) -> Option<CachedUnit> {
        self.lookups.set(self.lookups.get() + 1);
        let map = self.map.borrow();
        // Borrowed lookup without building an owned key: one context per run.
        let found = map
            .get(&(key.context().to_vec(), key.text().to_owned()))
            .cloned();
        if found.is_some() {
            self.hits.set(self.hits.get() + 1);
        }
        found
    }

    fn insert(&self, key: &UnitKey<'_>, unit: CachedUnit) {
        self.inserts.set(self.inserts.get() + 1);
        self.bytes
            .set(self.bytes.get() + key.text().len() + key.context().len() + unit.heap_size());
        self.inserted.borrow_mut().push(key.text().to_owned());
        self.map
            .borrow_mut()
            .insert((key.context().to_vec(), key.text().to_owned()), unit);
    }
}

impl Exact {
    /// Lookups, units used from the cache (lookups not followed by a store: every unit
    /// formatted is stored), and stores, since the last call. A record found but not used
    /// (a skipped-range record on another line, a tail that differs) counts as formatted.
    fn take_counts(&self) -> (usize, usize, usize) {
        let lookups = self.lookups.replace(0);
        let inserts = self.inserts.replace(0);
        let found = self.hits.replace(0);
        self.rejected
            .set(self.rejected.get() + found - (lookups - inserts));
        (lookups, lookups - inserts, inserts)
    }

    fn deep_clone(&self) -> Exact {
        Exact {
            map: RefCell::new(self.map.borrow().clone()),
            ..Exact::default()
        }
    }
}

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

/// `src` with a comment line inserted at its start, or `None` when the start is not a
/// line where a comment may go (a byte order mark, a `#!` line, front matter).
fn shifted(src: &str) -> Option<String> {
    if src.starts_with('\u{feff}') || src.starts_with("#!") || src.starts_with("---") {
        return None;
    }
    Some(format!("// unit cache probe\n{src}"))
}

/// `src` with every byte outside `lo..hi` replaced by a space, newlines kept.
fn blanked(src: &str, lo: usize, hi: usize) -> String {
    let mut out = String::with_capacity(src.len());
    for (i, c) in src.char_indices() {
        if (lo..hi).contains(&i) || c == '\n' {
            out.push(c);
        } else {
            // One space per byte keeps every position.
            for _ in 0..c.len_utf8() {
                out.push(' ');
            }
        }
    }
    out
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut roots = Vec::new();
    let mut threads = 1;
    let mut shift = false;
    let mut blank = false;
    let mut max_mb = 1024usize;
    let mut edit = None;
    let mut rounds = 10;
    let mut times_over = 128 * 1024;
    let mut every = 1;
    while let Some(arg) = args.next() {
        let mut value = || args.next().expect("option takes a value");
        match arg.as_str() {
            "--root" => roots.push(PathBuf::from(value())),
            "--threads" => threads = value().parse().expect("--threads takes a number"),
            "--shift" => shift = true,
            "--blank" => blank = true,
            "--max-mb" => max_mb = value().parse().expect("--max-mb takes a number"),
            "--times-over" => times_over = value().parse().expect("--times-over takes bytes"),
            "--every" => every = value().parse().expect("--every takes a number"),
            "-n" => rounds = value().parse().expect("-n takes a number"),
            "--edit" => edit = Some((PathBuf::from(value()), PathBuf::from(value()))),
            _ => panic!("unknown argument {arg}"),
        }
    }
    let config = Config::default();

    if let Some((orig, edited)) = edit {
        let orig = std::fs::read_to_string(orig).expect("read ORIG");
        let edited = std::fs::read_to_string(edited).expect("read EDITED");
        let warm = Exact::default();
        let _ = chloro_core::format_source_with_unit_cache(&orig, &config, threads, &warm);
        let (units, _, _) = warm.take_counts();
        let reference = chloro_core::format_source_with_config(&edited, &config);
        let (mut best_ref, mut best_cached, mut best_parallel) = (f64::MAX, f64::MAX, f64::MAX);
        let mut hits = 0;
        for _ in 0..rounds {
            let t = Instant::now();
            std::hint::black_box(chloro_core::format_source_with_config(&edited, &config));
            best_ref = best_ref.min(t.elapsed().as_secs_f64());
            let t = Instant::now();
            std::hint::black_box(chloro_core::format_source_with_config_parallel(
                &edited, &config, threads,
            ));
            best_parallel = best_parallel.min(t.elapsed().as_secs_f64());
            let cache = warm.deep_clone();
            let t = Instant::now();
            let out = chloro_core::format_source_with_unit_cache(&edited, &config, threads, &cache);
            best_cached = best_cached.min(t.elapsed().as_secs_f64());
            assert!(out == reference, "cached output differs from the reference");
            hits = cache.take_counts().1;
        }
        println!(
            "edit: {} bytes, {units} units, {hits} hits; best of {rounds}: \
             format_source_with_config {:.2} ms, parallel({threads}) {:.2} ms, \
             unit cache {:.2} ms ({:.3}x)",
            edited.len(),
            best_ref * 1e3,
            best_parallel * 1e3,
            best_cached * 1e3,
            best_cached / best_ref
        );
        return;
    }

    if roots.is_empty() {
        roots.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures"));
    }
    let files: Vec<(PathBuf, String)> = roots
        .iter()
        .flat_map(|root| files_under(root))
        .step_by(every)
        .filter_map(|path| std::fs::read_to_string(&path).ok().map(|src| (path, src)))
        .collect();

    let cache = Exact::default();
    let mut differ = 0;
    let (mut units, mut cold_hits, mut warm_hits, mut stored) = (0, 0, 0, 0);
    let (mut shift_files, mut shift_units, mut shift_hits) = (0, 0, 0);
    let (mut blank_units, mut blank_hits) = (0, 0);
    let mut warm_misses_files = 0;
    let mut clears = 0;
    let mut peak_bytes = 0;
    let started = Instant::now();
    for (path, src) in &files {
        if cache.bytes.get() > max_mb << 20 {
            cache.map.borrow_mut().clear();
            cache.bytes.set(0);
            clears += 1;
        }
        let t = Instant::now();
        let reference = chloro_core::format_source_with_config(src, &config);
        let t_ref = t.elapsed().as_secs_f64();
        cache.inserted.borrow_mut().clear();
        let t = Instant::now();
        let cold = chloro_core::format_source_with_unit_cache(src, &config, threads, &cache);
        let t_cold = t.elapsed().as_secs_f64();
        let (lookups, hits, inserts) = cache.take_counts();
        units += lookups;
        cold_hits += hits;
        stored += inserts;
        let t = Instant::now();
        let warm = chloro_core::format_source_with_unit_cache(src, &config, threads, &cache);
        let t_warm = t.elapsed().as_secs_f64();
        let (wl, wh, _) = cache.take_counts();
        warm_hits += wh;
        if wh != wl {
            warm_misses_files += 1;
            println!(
                "warm pass formatted {} of {wl} units: {}",
                wl - wh,
                path.display()
            );
        }
        if cold != reference {
            differ += 1;
            println!("cold pass differs: {}", path.display());
        }
        if warm != reference {
            differ += 1;
            println!("warm pass differs: {}", path.display());
        }
        if src.len() >= times_over {
            println!(
                "{:>8} bytes {lookups:>5} units: reference {:7.2} ms, cold {:7.2} ms, warm {:7.2} ms ({:.3}x)  {}",
                src.len(),
                t_ref * 1e3,
                t_cold * 1e3,
                t_warm * 1e3,
                t_warm / t_ref,
                path.display()
            );
        }
        if shift && let Some(variant) = shifted(src) {
            let reference = chloro_core::format_source_with_config(&variant, &config);
            let out =
                chloro_core::format_source_with_unit_cache(&variant, &config, threads, &cache);
            let (l, h, _) = cache.take_counts();
            shift_files += 1;
            shift_units += l;
            shift_hits += h;
            if out != reference {
                differ += 1;
                println!("shifted file differs: {}", path.display());
            }
        }
        if blank {
            let inserted = std::mem::take(&mut *cache.inserted.borrow_mut());
            for text in inserted {
                // The normalised source is what the units index; a file with `\r\n` or a
                // byte order mark is left out.
                if src.contains('\r') || src.starts_with('\u{feff}') {
                    break;
                }
                let Some(lo) = src.find(&text) else { continue };
                let variant = blanked(src, lo, lo + text.len());
                let reference = chloro_core::format_source_with_config(&variant, &config);
                let out =
                    chloro_core::format_source_with_unit_cache(&variant, &config, threads, &cache);
                let (_, h, _) = cache.take_counts();
                blank_units += 1;
                blank_hits += h;
                if out != reference {
                    differ += 1;
                    println!("blanked unit at byte {lo} differs: {}", path.display());
                }
            }
        }
        peak_bytes = peak_bytes.max(cache.bytes.get());
    }
    let entries = cache.map.borrow().len();
    println!(
        "{} files, threads {threads}: {units} units looked up cold, {cold_hits} found cold, \
         {stored} stored, {warm_hits} found warm ({warm_misses_files} files with a warm miss); \
         {} records found but not used; \
         {entries} entries at the end, peak {:.1} MB held (keys and records), {clears} clears; {:.1} s",
        files.len(),
        cache.rejected.get(),
        peak_bytes as f64 / 1e6,
        started.elapsed().as_secs_f64()
    );
    if shift {
        println!("shift: {shift_files} files, {shift_units} units, {shift_hits} found");
    }
    if blank {
        println!("blank: {blank_units} units checked, {blank_hits} found");
    }
    println!(
        "{differ} differences against format_source_with_config (cold, warm{}{})",
        if shift { ", shifted" } else { "" },
        if blank { ", blanked" } else { "" }
    );
    if differ > 0 {
        std::process::exit(1);
    }
}
