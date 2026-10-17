#!/usr/bin/env python3
"""EXP-023: the diagnostic build's patch, applied to a scratch worktree and never committed.

usage: trace_patch.py WORKTREE base|owner

Adds to the CHLORO_TRACE timeline (chloro/src/cli/trace.rs) what it cannot record without
touching chloro-core or calling the thread CPU clock: each file's and worker's on-CPU time
(CLOCK_THREAD_CPUTIME_ID, Linux x86-64, through an `unsafe` libc call), and for a file
formatted on several threads (parallel.rs) its parse, units phase, stitch, every item worker
and every unit, as `item` lines (kind 1 parse, 2 units phase, 3 stitch, 0 item worker, 4 unit;
`n` encodes the file's length times 1e6 plus the unit length or count). Build with
`cargo build --release -p chloro --features cli` after applying, copy the binary out, then
`git checkout chloro-core chloro/src` to drop the patch. sim.py reads these traces.

`owner` is for a checkout of 68889e1 (was 0a6080c) (EXP-029's lever (a), reverted), whose split-file worker
formats units itself beside `threads - 1` spawned ones: the same records, plus an `item 5` line
with that worker's share of the units (its CPU is also in the file's; split_sim.py takes it out
of the file's sequential part)."""
import sys
root, variant = sys.argv[1], sys.argv[2]

def edit(path, pairs):
    p = f'{root}/{path}'
    s = open(p).read()
    for a, b in pairs:
        assert s.count(a) == 1, (path, a, s.count(a))
        s = s.replace(a, b)
    open(p, 'w').write(s)

CPU = '''
/// DIAGNOSTIC (EXP-023, temporary): (kind, start, end, on-CPU ns, n).
/// kind 0: an item worker; 1: the file's parse; 2: the units phase; 3: the stitch; 4: a unit.
pub static DIAG: std::sync::Mutex<Vec<(u8, std::time::Instant, std::time::Instant, u64, usize)>> =
    std::sync::Mutex::new(Vec::new());
thread_local! {
    static LOCAL_DIAG: std::cell::RefCell<Vec<(u8, std::time::Instant, std::time::Instant, u64, usize)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}
/// DIAGNOSTIC: records into the thread's buffer.
pub fn diag_push(r: (u8, std::time::Instant, std::time::Instant, u64, usize)) {
    LOCAL_DIAG.with(|l| l.borrow_mut().push(r));
}
/// DIAGNOSTIC: moves the thread's records to the shared list.
pub fn diag_flush() {
    let v = LOCAL_DIAG.with(|l| std::mem::take(&mut *l.borrow_mut()));
    DIAG.lock().unwrap().extend(v);
}

#[repr(C)]
struct Timespec {
    sec: i64,
    nsec: i64,
}
unsafe extern "C" {
    fn clock_gettime(clock: i32, ts: *mut Timespec) -> i32;
}

/// DIAGNOSTIC: the calling thread's CPU clock (CLOCK_THREAD_CPUTIME_ID), nanoseconds.
pub fn diag_cpu() -> u64 {
    let mut ts = Timespec { sec: 0, nsec: 0 };
    // SAFETY: diagnostic only; `ts` is a valid out-pointer.
    unsafe { clock_gettime(3, &mut ts) };
    (ts.sec as u64) * 1_000_000_000 + ts.nsec as u64
}

struct DiagPhase(u8, std::time::Instant, usize, u64);
impl Drop for DiagPhase {
    fn drop(&mut self) {
        diag_push((self.0, self.1, std::time::Instant::now(), diag_cpu() - self.3, self.2));
    }
}
'''

edit('chloro-core/src/formatter.rs', [('\nmod parallel;', '\npub mod parallel;')])
edit('chloro-core/src/formatter/parallel.rs', [
    ('/// The first `format_missing*` call a unit makes', CPU + '\n/// The first `format_missing*` call a unit makes'),
    ('''    let work = || {
        let arena = TreeArena::new();''', '''    let work = || {
        let (d0, c0) = (std::time::Instant::now(), diag_cpu());
        let arena = TreeArena::new();'''),
    ('''            let out = format_unit(&mut visitor, &run, &items[first..first + len]);
            done.push((u, out));
        }
        done
    };''', '''            let (u0, uc) = (std::time::Instant::now(), diag_cpu());
            let out = format_unit(&mut visitor, &run, &items[first..first + len]);
            diag_push((4, u0, std::time::Instant::now(), diag_cpu() - uc, source.len() * 1_000_000 + unit_len(&units[u]) as usize));
            done.push((u, out));
        }
        let n = done.len();
        diag_push((0, d0, std::time::Instant::now(), diag_cpu() - c0, source.len() * 1_000_000 + n));
        diag_flush();
        done
    };'''),
    ('''    std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads).map(|_| s.spawn(work)).collect();''', '''    let _g = DiagPhase(2, std::time::Instant::now(), source.len() * 1_000_000 + order.len(), diag_cpu());
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads).map(|_| s.spawn(work)).collect();''') if variant == 'base' else
    ('''    std::thread::scope(|s| {
        let handles: Vec<_> = (1..threads).map(|_| s.spawn(work)).collect();
        let mut done = work();''', '''    let _g = DiagPhase(2, std::time::Instant::now(), source.len() * 1_000_000 + order.len(), diag_cpu());
    std::thread::scope(|s| {
        let handles: Vec<_> = (1..threads).map(|_| s.spawn(work)).collect();
        let (o0, oc) = (std::time::Instant::now(), diag_cpu());
        let mut done = work();
        diag_push((5, o0, std::time::Instant::now(), diag_cpu() - oc, source.len() * 1_000_000 + done.len()));'''),
    ('''            if ok {
                stitch(&mut main, &run, &outputs);''', '''            if ok {
                let _g = DiagPhase(3, std::time::Instant::now(), source.len() * 1_000_000 + outputs.len(), diag_cpu());
                stitch(&mut main, &run, &outputs);'''),
])
edit('chloro-core/src/formatter/formatting.rs', [
    ('''    let tree = parse(source, config.edition().to_ra(), config.rustfmt_version())?;
    let file = ast::SourceFile::cast(tree.root()).expect("the root is a source file");
    if contains_skip(&inner_attributes(file.syntax())) {
        return Some(Formatted {
            text: source.to_owned(),
            echoed: true,''', '''    let (d0, dc) = (std::time::Instant::now(), super::parallel::diag_cpu());
    let tree = parse(source, config.edition().to_ra(), config.rustfmt_version())?;
    super::parallel::diag_push((1, d0, std::time::Instant::now(), super::parallel::diag_cpu() - dc, source.len()));
    let file = ast::SourceFile::cast(tree.root()).expect("the root is a source file");
    if contains_skip(&inner_attributes(file.syntax())) {
        return Some(Formatted {
            text: source.to_owned(),
            echoed: true,'''),
])
edit('chloro/src/cli/trace.rs', [
    ('''        mark("finish");''', '''        mark("finish");
        for (kind, a, b, cpu, n) in chloro_core::formatter::parallel::DIAG.lock().unwrap().drain(..) {
            let ns = |i: Instant| i.saturating_duration_since(t.origin).as_nanos();
            push(format_args!("item\\t{kind}\\t{}\\t{}\\t{cpu}\\t{n}", ns(a), ns(b)));
        }'''),
    ('''pub fn worker(id: usize, start: u64, end: u64) {
    if enabled() {
        push_local(format_args!("worker\\t{id}\\t{start}\\t{end}"));''', '''pub fn worker(id: usize, start: u64, end: u64) {
    if enabled() {
        let cpu = chloro_core::formatter::parallel::diag_cpu();
        chloro_core::formatter::parallel::diag_flush();
        push_local(format_args!("worker\\t{id}\\t{start}\\t{end}\\t{cpu}"));'''),
    ('''            f.end
        ));''', '''            f.end
        ));
        push_local(format_args!("cpu\\t{}\\t{}", f.start, CPU0.with(|c| chloro_core::formatter::parallel::diag_cpu() - c.get())));'''),
    ('''/// Stamps the end of the current file's read.''', '''thread_local! {
    pub static CPU0: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
/// DIAGNOSTIC: marks the start of a file's CPU span.
pub fn cpu_start() {
    CPU0.with(|c| c.set(chloro_core::formatter::parallel::diag_cpu()));
}

/// Stamps the end of the current file's read.'''),
])
if variant in ('base', 'owner'):
    edit('chloro/src/cli/orchestrate.rs', [
        ('''                        let start = trace::now();''', '''                        let start = trace::now();
                        trace::cpu_start();'''),
    ])
print('applied', variant)
