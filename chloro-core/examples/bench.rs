//! In-process throughput benchmark and output-equivalence check.
//!
//! ```text
//! cargo run --release -p chloro-core --example bench                      # conformance fixtures
//! cargo run --release -p chloro-core --example bench -- -n 10 -w 10       # 10 rounds, 10 slowest files
//! cargo run --release -p chloro-core --example bench -- --root ~/.cargo/registry/src
//! cargo run --release -p chloro-core --example bench -- --record out.tsv  # hash every output
//! cargo run --release -p chloro-core --example bench -- --check out.tsv   # compare with a record
//! cargo run --release -p chloro-core --example bench -- --threads 4       # the parallel API
//! ```
//!
//! Times `format_source` over every `.rs` file under the roots (default: the conformance
//! fixtures), excluding file I/O and process startup, and reports the best of `-n` rounds.
//!
//! `--record` writes a hash of the formatted output of every file; `--check` formats again and
//! lists the files whose output hash changed. A change meant to be behaviour-preserving (a
//! performance change) should leave every hash as recorded.
//!
//! `--threads N` formats every file with `format_source_with_config_parallel(src, .., N)`
//! instead of `format_source`, for the timings and the hashes alike, and additionally
//! compares each output with `format_source`'s: `--threads 4 --check out.tsv` checks the
//! parallel path against hashes recorded from the sequential one.
//!
//! Allocator features (off by default; EXP-014): `alloc-mimalloc`, `alloc-jemalloc` and
//! `alloc-snmalloc` make that allocator the example's global allocator (the first enabled in
//! that order wins; glibc's `malloc` otherwise). `alloc-timing` wraps the global allocator,
//! counts calls and requested bytes and accumulates `rdtsc` cycles inside every `alloc`,
//! `dealloc`, `realloc` and `alloc_zeroed`, and prints the totals for the timed rounds and
//! their share of the rounds' cycles at exit. `CHLORO_ALLOC_TIMING=count` keeps the counting
//! and skips the `rdtsc` pair (the wrapper's own overhead), `=off` passes calls straight through.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// The inner global allocator chosen by the `alloc-*` features.
#[cfg(feature = "alloc-mimalloc")]
type Inner = mimalloc::MiMalloc;
#[cfg(feature = "alloc-mimalloc")]
const INNER: Inner = mimalloc::MiMalloc;
#[cfg(all(feature = "alloc-jemalloc", not(feature = "alloc-mimalloc")))]
type Inner = tikv_jemallocator::Jemalloc;
#[cfg(all(feature = "alloc-jemalloc", not(feature = "alloc-mimalloc")))]
const INNER: Inner = tikv_jemallocator::Jemalloc;
#[cfg(all(
    feature = "alloc-snmalloc",
    not(any(feature = "alloc-mimalloc", feature = "alloc-jemalloc"))
))]
type Inner = snmalloc_rs::SnMalloc;
#[cfg(all(
    feature = "alloc-snmalloc",
    not(any(feature = "alloc-mimalloc", feature = "alloc-jemalloc"))
))]
const INNER: Inner = snmalloc_rs::SnMalloc;
#[cfg(all(
    feature = "alloc-timing",
    not(any(
        feature = "alloc-mimalloc",
        feature = "alloc-jemalloc",
        feature = "alloc-snmalloc"
    ))
))]
type Inner = std::alloc::System;
#[cfg(all(
    feature = "alloc-timing",
    not(any(
        feature = "alloc-mimalloc",
        feature = "alloc-jemalloc",
        feature = "alloc-snmalloc"
    ))
))]
const INNER: Inner = std::alloc::System;

/// Name of the global allocator, for the report.
#[cfg(any(
    feature = "alloc-timing",
    feature = "alloc-mimalloc",
    feature = "alloc-jemalloc",
    feature = "alloc-snmalloc"
))]
const INNER_NAME: &str = if cfg!(feature = "alloc-mimalloc") {
    "mimalloc"
} else if cfg!(feature = "alloc-jemalloc") {
    "jemalloc"
} else if cfg!(feature = "alloc-snmalloc") {
    "snmalloc"
} else {
    "system (glibc malloc)"
};

// No `#[global_allocator]` at all without the features: the default build is the plain one.
#[cfg(all(
    not(feature = "alloc-timing"),
    any(
        feature = "alloc-mimalloc",
        feature = "alloc-jemalloc",
        feature = "alloc-snmalloc"
    )
))]
#[global_allocator]
static GLOBAL: Inner = INNER;

#[cfg(feature = "alloc-timing")]
#[global_allocator]
static GLOBAL: alloc_timing::Timed<Inner> = alloc_timing::Timed(INNER);

/// Call, byte and cycle counters around the global allocator (`alloc-timing`).
///
/// Every counter is a relaxed atomic updated after the timed window closes, so the cycles
/// attributed to the allocator are those between the two `rdtsc` reads around the inner call:
/// the inner allocator's own work plus one `rdtsc` latency, which `calibrate` measures.
#[cfg(feature = "alloc-timing")]
mod alloc_timing {
    use std::alloc::{GlobalAlloc, Layout};
    use std::sync::atomic::{AtomicU8, AtomicU64, Ordering::Relaxed};

    /// 0 = not read yet, 1 = off, 2 = count only, 3 = count and time.
    static MODE: AtomicU8 = AtomicU8::new(0);
    const OPS: usize = 4;
    pub const NAMES: [&str; OPS] = ["alloc", "dealloc", "realloc", "alloc_zeroed"];
    static CALLS: [AtomicU64; OPS] = [const { AtomicU64::new(0) }; OPS];
    static CYCLES: [AtomicU64; OPS] = [const { AtomicU64::new(0) }; OPS];
    /// Requested bytes: `alloc`/`alloc_zeroed` sizes, `realloc` new sizes, `dealloc` sizes.
    static BYTES: [AtomicU64; OPS] = [const { AtomicU64::new(0) }; OPS];

    /// Reads `CHLORO_ALLOC_TIMING` once; called from `main` before anything is measured.
    pub fn init() -> &'static str {
        let (mode, name) = match std::env::var("CHLORO_ALLOC_TIMING").as_deref() {
            Ok("off") => (1, "off"),
            Ok("count") => (2, "count"),
            _ => (3, "time"),
        };
        MODE.store(mode, Relaxed);
        name
    }

    #[inline(always)]
    pub fn now() -> u64 {
        // SAFETY: `rdtsc` reads the time-stamp counter; it has no memory effects and is
        // available on every x86-64 CPU.
        unsafe { core::arch::x86_64::_rdtsc() }
    }

    #[inline(always)]
    fn measure<T>(op: usize, bytes: usize, f: impl FnOnce() -> T) -> T {
        match MODE.load(Relaxed) {
            3 => {
                let t0 = now();
                let r = f();
                let dt = now().wrapping_sub(t0);
                CALLS[op].fetch_add(1, Relaxed);
                CYCLES[op].fetch_add(dt, Relaxed);
                BYTES[op].fetch_add(bytes as u64, Relaxed);
                r
            }
            2 => {
                let r = f();
                CALLS[op].fetch_add(1, Relaxed);
                BYTES[op].fetch_add(bytes as u64, Relaxed);
                r
            }
            _ => f(),
        }
    }

    pub struct Timed<A>(pub A);

    // SAFETY: every method forwards its arguments unchanged to the inner allocator and
    // returns its result unchanged; the counters are atomics and allocate nothing.
    unsafe impl<A: GlobalAlloc> GlobalAlloc for Timed<A> {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            measure(0, layout.size(), || unsafe { self.0.alloc(layout) })
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            measure(1, layout.size(), || unsafe { self.0.dealloc(ptr, layout) })
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            measure(2, new_size, || unsafe {
                self.0.realloc(ptr, layout, new_size)
            })
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            measure(3, layout.size(), || unsafe { self.0.alloc_zeroed(layout) })
        }
    }

    /// Counters at one instant: (calls, cycles, bytes) per operation.
    #[derive(Clone, Copy)]
    pub struct Snapshot([(u64, u64, u64); OPS]);

    pub fn snapshot() -> Snapshot {
        Snapshot(std::array::from_fn(|i| {
            (
                CALLS[i].load(Relaxed),
                CYCLES[i].load(Relaxed),
                BYTES[i].load(Relaxed),
            )
        }))
    }

    /// Median cycles of an empty `rdtsc` pair: the floor each timed call carries.
    pub fn calibrate() -> u64 {
        let mut d: Vec<u64> = (0..10_001)
            .map(|_| {
                let t0 = now();
                let t1 = now();
                t1.wrapping_sub(t0)
            })
            .collect();
        d.sort_unstable();
        d[d.len() / 2]
    }

    /// Prints the counters accumulated between `before` and `after` against `total` cycles.
    pub fn report(before: Snapshot, after: Snapshot, total: u64, mode: &str, inner: &str) {
        let floor = calibrate();
        println!("alloc-timing ({mode}), allocator {inner}: {total} cycles in the timed rounds");
        let (mut calls, mut cycles) = (0, 0);
        for (i, name) in NAMES.iter().enumerate() {
            let (c, y, b) = (
                after.0[i].0 - before.0[i].0,
                after.0[i].1 - before.0[i].1,
                after.0[i].2 - before.0[i].2,
            );
            calls += c;
            cycles += y;
            let per = if c > 0 { y as f64 / c as f64 } else { 0.0 };
            println!("  {name:<12} {c:>12} calls {b:>14} bytes {y:>14} cycles ({per:.1} per call)");
        }
        let net = cycles.saturating_sub(calls * floor);
        println!(
            "  total        {calls:>12} calls {cycles:>14} cycles = {:.2}% of the rounds; rdtsc pair floor {floor} cycles, minus calls x floor: {net} = {:.2}%",
            100.0 * cycles as f64 / total as f64,
            100.0 * net as f64 / total as f64
        );
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

fn output_hash(output: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    output.hash(&mut hasher);
    hasher.finish()
}

fn main() {
    #[cfg(feature = "alloc-timing")]
    let timing_mode = alloc_timing::init();
    let mut args = std::env::args().skip(1);
    let mut rounds = 5;
    let mut worst = 0;
    let mut roots = Vec::new();
    let mut record = None;
    let mut check = None;
    let mut threads = None;
    while let Some(arg) = args.next() {
        let mut value = || args.next().expect("option takes a value");
        match arg.as_str() {
            "-n" => rounds = value().parse().expect("-n takes a number"),
            "-w" => worst = value().parse().expect("-w takes a number"),
            "--root" => roots.push(PathBuf::from(value())),
            "--record" => record = Some(PathBuf::from(value())),
            "--check" => check = Some(PathBuf::from(value())),
            "--threads" => threads = Some(value().parse().expect("--threads takes a number")),
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

    let config = chloro_core::Config::default();
    let format = |src: &str| match threads {
        Some(n) => chloro_core::format_source_with_config_parallel(src, &config, n),
        None => chloro_core::format_source(src),
    };

    if let Some(n) = threads {
        let mut differ = 0;
        for (p, src) in &files {
            if format(src) != chloro_core::format_source(src) {
                differ += 1;
                println!("parallel output differs: {}", p.display());
            }
        }
        println!(
            "{differ} of {} outputs differ between {n} threads and format_source",
            files.len()
        );
        if differ > 0 {
            std::process::exit(1);
        }
    }

    if record.is_some() || check.is_some() {
        let hashes: Vec<u64> = files
            .iter()
            .map(|(_, src)| output_hash(&format(src)))
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
    #[cfg(feature = "alloc-timing")]
    let (timing_before, cycles_before) = (alloc_timing::snapshot(), alloc_timing::now());
    for _ in 0..rounds {
        let start = Instant::now();
        for ((_, src), slot) in files.iter().zip(per_file.iter_mut()) {
            let t = Instant::now();
            std::hint::black_box(format(src));
            *slot = slot.min(t.elapsed().as_secs_f64());
        }
        best = best.min(start.elapsed().as_secs_f64());
    }
    #[cfg(feature = "alloc-timing")]
    let (timing_after, cycles_after) = (alloc_timing::snapshot(), alloc_timing::now());
    let mb = bytes as f64 / 1e6;
    println!(
        "{} files, {mb:.1} MB: {best:.3}s, {:.2} MB/s (best of {rounds})",
        files.len(),
        mb / best
    );
    #[cfg(any(
        feature = "alloc-timing",
        feature = "alloc-mimalloc",
        feature = "alloc-jemalloc",
        feature = "alloc-snmalloc"
    ))]
    println!("global allocator: {INNER_NAME}");
    #[cfg(feature = "alloc-timing")]
    alloc_timing::report(
        timing_before,
        timing_after,
        cycles_after.wrapping_sub(cycles_before),
        timing_mode,
        INNER_NAME,
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
