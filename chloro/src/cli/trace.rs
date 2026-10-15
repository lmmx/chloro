//! An optional timeline of one run, for the scheduling diagnostic of
//! `docs/journal/2026-10-14-autoresearch/experiments/EXP-023-unit-queue.md`.
//!
//! With `CHLORO_TRACE=<path>` in the environment the run records, in nanoseconds since
//! `main` was entered, the phase marks of the main thread, every file a worker handled
//! (start, read, hashed, formatted, end, bytes, whether its items were split) and every
//! worker's span, and writes them to `<path>` as tab-separated lines when the run reaches
//! its summary (`benchmarks/schedule/README.md` has the format). Without the variable every
//! call returns at once on a cached `None`.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

struct Trace {
    origin: Instant,
    path: PathBuf,
    lines: Mutex<String>,
}

static TRACE: OnceLock<Option<Trace>> = OnceLock::new();

fn get() -> Option<&'static Trace> {
    TRACE.get().and_then(Option::as_ref)
}

/// Starts the trace when `CHLORO_TRACE` names a file; the first call fixes the origin.
pub fn init() {
    TRACE.get_or_init(|| {
        let path = std::env::var_os("CHLORO_TRACE").filter(|p| !p.is_empty())?;
        Some(Trace {
            origin: Instant::now(),
            path: PathBuf::from(path),
            lines: Mutex::new(String::new()),
        })
    });
}

/// Whether the run is traced.
pub fn enabled() -> bool {
    get().is_some()
}

/// Nanoseconds since the origin, or 0 when the run is not traced.
pub fn now() -> u64 {
    get().map_or(0, |t| {
        u64::try_from(t.origin.elapsed().as_nanos()).unwrap_or(u64::MAX)
    })
}

fn push(line: std::fmt::Arguments<'_>) {
    if let Some(t) = get() {
        let mut lines = t
            .lines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ = lines.write_fmt(line);
        lines.push('\n');
    }
}

thread_local! {
    /// A worker's lines, moved to the shared buffer when the worker exits: a shared lock
    /// taken per file would put a preempted worker's lock wait into the others' timelines.
    static LOCAL: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

fn push_local(line: std::fmt::Arguments<'_>) {
    LOCAL.with(|l| {
        let mut l = l.borrow_mut();
        let _ = l.write_fmt(line);
        l.push('\n');
    });
}

/// Moves the calling thread's lines to the shared buffer.
fn flush_local() {
    if let Some(t) = get() {
        let local = LOCAL.with(|l| std::mem::take(&mut *l.borrow_mut()));
        let mut lines = t
            .lines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        lines.push_str(&local);
    }
}

/// A phase mark of the main thread, at the current time.
pub fn mark(name: &str) {
    if enabled() {
        push(format_args!("mark\t{name}\t{}", now()));
    }
}

/// One file as a worker handled it.
#[derive(Default, Clone, Copy)]
pub struct FileSpan {
    pub worker: usize,
    pub bytes: u64,
    /// The items were formatted by several workers (or threads).
    pub split: bool,
    pub start: u64,
    pub read: u64,
    pub hashed: u64,
    pub formatted: u64,
    pub end: u64,
}

thread_local! {
    /// The read, hashed and formatted stamps of the file the thread is on, and whether its
    /// items were split.
    static STAMPS: std::cell::Cell<(u64, u64, u64, bool)> =
        const { std::cell::Cell::new((0, 0, 0, false)) };
}

/// Stamps the end of the current file's read.
pub fn stamp_read() {
    if enabled() {
        STAMPS.with(|s| s.set((now(), 0, 0, false)));
    }
}

/// Stamps the end of the current file's content digest (the cache is on).
pub fn stamp_hashed() {
    if enabled() {
        STAMPS.with(|s| {
            let (read, _, _, _) = s.get();
            s.set((read, now(), 0, false));
        });
    }
}

/// Stamps the end of the current file's formatting; `split` when its items were split.
pub fn stamp_formatted(split: bool) {
    if enabled() {
        STAMPS.with(|s| {
            let (read, hashed, _, _) = s.get();
            s.set((read, hashed, now(), split));
        });
    }
}

/// Takes the stamps of the current file: (read, hashed, formatted, split); 0 for a stamp
/// the file did not reach (no cache: no digest; a cache hit: not formatted).
pub fn take_stamps() -> (u64, u64, u64, bool) {
    STAMPS.with(|s| s.replace((0, 0, 0, false)))
}

/// Records one file (in the worker's own buffer, see [`worker`]).
pub fn file(f: &FileSpan) {
    if enabled() {
        push_local(format_args!(
            "file\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            f.worker,
            f.bytes,
            u8::from(f.split),
            f.start,
            f.read,
            f.hashed,
            f.formatted,
            f.end
        ));
    }
}

/// Records one worker's span from its first instruction to its exit, and moves the lines
/// the worker recorded to the shared buffer. A worker's last call.
pub fn worker(id: usize, start: u64, end: u64) {
    if enabled() {
        push_local(format_args!("worker\t{id}\t{start}\t{end}"));
        flush_local();
    }
}

/// Writes the trace file. Called once, at the end of the run.
pub fn finish() {
    if let Some(t) = get() {
        mark("finish");
        let lines = t
            .lines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Err(e) = std::fs::write(&t.path, lines.as_bytes()) {
            eprintln!("Trace not written: {}: {e}", t.path.display());
        }
    }
}
