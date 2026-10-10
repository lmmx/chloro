//! Counts heap allocations made by `format_source` over every `.rs` file under ROOT
//! (default: the conformance fixtures), formatted once, with a histogram of request sizes.
//!
//! ```text
//! cargo run --release -p chloro-core --example alloc_count -- [ROOT]
//! ```
//!
//! Uses only `chloro_core::format_source`, so the file can be copied into a checkout of an
//! older commit to count that version's allocations. `realloc` counts as an allocation.

use std::alloc::{GlobalAlloc, Layout, System};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

static COUNT: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
/// Allocations by request size: bucket `i` holds sizes below `2^i`.
static BUCKETS: [AtomicU64; 32] = [const { AtomicU64::new(0) }; 32];

fn record(size: usize) {
    COUNT.fetch_add(1, Relaxed);
    BYTES.fetch_add(size as u64, Relaxed);
    BUCKETS[((usize::BITS - size.leading_zeros()) as usize).min(31)].fetch_add(1, Relaxed);
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn main() {
    let root = std::env::args().nth(1).map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures"),
        Into::into,
    );
    let sources: Vec<String> = walkdir::WalkDir::new(&root)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .collect();
    let bytes: usize = sources.iter().map(String::len).sum();

    let buckets: Vec<u64> = BUCKETS.iter().map(|b| b.load(Relaxed)).collect();
    let (count, allocated) = (COUNT.load(Relaxed), BYTES.load(Relaxed));
    for src in &sources {
        std::hint::black_box(chloro_core::format_source(src));
    }
    let count = COUNT.load(Relaxed) - count;
    let allocated = BYTES.load(Relaxed) - allocated;
    println!(
        "{} files, {bytes} bytes: {count} allocations ({:.3} per input byte), {allocated} bytes requested ({:.1}x input)",
        sources.len(),
        count as f64 / bytes as f64,
        allocated as f64 / bytes as f64
    );
    for (i, before) in buckets.iter().enumerate() {
        let n = BUCKETS[i].load(Relaxed) - before;
        if n > 0 {
            println!("  size < 2^{i:<2} {n:>10}");
        }
    }
}
