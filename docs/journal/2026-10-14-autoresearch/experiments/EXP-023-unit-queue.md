# EXP-023: a unit queue across files — the diagnostic first, and the stop rule

Ledger row E23 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md) (§3's rule for changes that add parallelism); the brief is
trick T3 of [../../2026-10-15-unmined-tricks.md](../../2026-10-15-unmined-tricks.md) §5 and
row E23 of its §7, lever L8 of
[../../2026-10-15-extensions-theory/incremental-and-process.md](../../2026-10-15-extensions-theory/incremental-and-process.md),
and the diagnostic-first and stop rule of §1.5 of
[../../2026-10-15-external-research-brief.md](../../2026-10-15-external-research-brief.md).
Labels: **measured** (a tool's output on the builds below), **reconstructed** (the
schedule rebuilt from measured per-thread CPU times, below), **read** (from source or from
an earlier record), **derived** (arithmetic on the above).

## Summary

- **The queue was not built: the stop rule fired.** Rebuilt from per-thread CPU clocks, the
  schedule the CLI runs today sits within 0.1–1.1% of the work-conserving line (total CPU /
  4) on eight of the nine crates, and a unit queue across files would take at most 0.1% off
  their four-core wall (and add 0.8% on `fx-hir`) (**reconstructed**). The tail is not where
  the gap between four cores and M4's 2.7–3.7 busy cores goes.
- **Where it goes (M4's quiet-host figures, decomposed):** the serial fixed cost of a run
  (process start-up and exit, about 2.2 ms, plus the directory walk, 0.1–0.7 ms) is 14–67% of
  the gap between the mean wall and CPU/4; the schedule is 1–5% of it on eight crates; the
  rest, 32–83%, is run-to-run spread: the fastest of M4's 30 runs is at the serial-adjusted
  line on every crate (0.92–1.04 of it) and the mean 5–24% above it (**derived**).
- **One exception, `fx-ide-db`:** its 625 KiB `lints.rs` is a critical path (parse plus a
  288 KB unit) about as long as a quarter of the crate's work. Today's schedule is 7.9% above
  CPU/4 there and its tail is 13% of the makespan; a unit queue would take 5.7% off (2.7% and
  4.1% in two earlier collections), which is the part of the tail that is not the chain
  itself (**reconstructed**): below the stop rule's 10%, and inside M4's run-to-run σ of 10%
  on that row.
- **Kept:** the CLI's `CHLORO_TRACE=<file>` timeline (phase marks, every file on its worker,
  every worker; wall-clock stamps only, cheap when off), a CLI test, and the diagnostic
  harness in `benchmarks/schedule/` (the patch for the CPU-clock build, the wall view, the
  reconstruction, the identity check). No formatter code changed; `chloro-core` is untouched.

## Identity

| field | value |
|---|---|
| branch | `20261015-unit-queue` from `9135c62` (integration branch `claude/chloro-perf-research-swarm-nrscj8`), worktree `/home/user/wt/queue` |
| base BKG | `05ab7b4` (232,047,700–232,050,590 `Ir`); `9135c62` adds only documentation after it |
| commits | `a1a7cf3` (the `CHLORO_TRACE` timeline and its test), `22a4b14` (`benchmarks/schedule/`), then this record and the ledger row |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0`, hyperfine 2.0.0, valgrind 3.22.0; release profile of the workspace (fat LTO, one codegen unit) |
| host | 4 vCPU "Intel(R) Xeon(R) Processor @ 2.80GHz" (KVM guest, 1 thread per core as the guest sees it, 33 MiB L3), 15.7 GiB, Linux 6.18.44-fc-v114. Other agents ran experiments throughout: the 1-minute load average read 3.5–26 during this work and 3.5–10.3 during the measurements below. Every multi-core wall figure measured here is **provisional** (protocol §6) |
| binaries | `chloro-base`: `9135c62`'s CLI, sha256 `e587ba96…3d12`, 2,167,656 bytes; `bench-base`: `f193784e…6198`; the CPU-clock diagnostic build `chloro-diag-base` (`9135c62` plus the kept trace plus `benchmarks/schedule/trace_patch.py`): `f058c9d2…ce4ce`; the kept head's CLI: `19073fff…1971`, 2,176,000 bytes |
| workloads | the harness's copies `/tmp/chloro-bench/formatted/<crate>/tree` (M4's), and `/tmp/chloro-bench/raw/<crate>/tree` for `--write` |

## Hypothesis, falsifiers, stop rule

Hypothesis (T3, L8): on multi-file workloads the idle tail — one worker finishing one large
file while the others wait — is the cause of the gap between four cores and the 2.7–3.7 busy
cores M4 measured, and a single queue of units (whole small files, items of large files) fed
by all files, with per-file stitching, removes most of it; bound 6.6–34.6% of four-core wall
on the nine crates (L8's CPU/4 line), zero on one core.

Falsifiers: the diagnostic shows the tail is not the cause (then the queue is not built and
this record says what is); four-core wall not below the base's range on the crates with the
largest tail; the pinned one-core wall or the instruction count regresses.

Stop rule (external brief §1.5, the brief of this experiment): if the tail is under 10% of
wall on every crate, or the gap is in serial phases, the queue is not built.

## Method

### Instrumentation

- **The kept timeline** (`chloro/src/cli/trace.rs`): with `CHLORO_TRACE=<file>` the CLI
  records, in nanoseconds since `main` was entered, the main thread's marks (`args`, `cache`,
  `walk`, `sort`, `spawn`, `joined`, `formatted`, `saved`, `finish`), each file as its worker
  handled it (worker, bytes, split or not, start, read, hashed, formatted, end) and each
  worker's span, and writes them when the run ends. Workers keep their lines in a
  thread-local buffer until they exit (a shared lock per file put one worker's preemption
  into the others' timelines: see *Rejected*). Without the variable every call returns on a
  cached `None`.
- **The CPU-clock build** (`benchmarks/schedule/trace_patch.py`, applied to a scratch tree,
  never committed): adds each file's and each worker's on-CPU time (the thread CPU clock,
  `CLOCK_THREAD_CPUTIME_ID`), and inside `parallel.rs` the parse, the units phase, the stitch,
  every item worker and every unit of a split file with its CPU time. These are what the
  reconstruction needs: the work each piece costs, independent of how long it waited for a
  core on a busy host.

### Two views

1. **Wall view (measured)**: from the timeline of each run, the work intervals (each file
   on its worker, less a split file's units phase, where its worker waits in `join`, plus
   each item worker), the number open at each moment, idle core time = 4 − that number over
   the format window, the tail = from the first worker's exit to the join, and who owns it.
   On this host the wall view is dominated by waiting for a core: the threads were on a CPU
   for 26–41% of their spans (the "on-CPU share" column), and the waiting shows up as tails,
   late worker starts and gaps between files. It is reported, labelled, and not used for the
   decision.
2. **Reconstruction (load-free)**: `benchmarks/schedule/sim.py` takes the measured on-CPU
   cost of every small file and, for every split file, of its sequential part (read, parse,
   unit extraction), of each unit and of the stitch, and replays three schedules on four idle
   cores:
   - **A, today's scheduler**: four file workers, largest file first; a split file's worker
     spawns min(4, units) item threads that take its units longest first while the worker
     waits; every runnable thread progresses at min(1, 4 / runnable) (processor sharing,
     the fair-share limit of the kernel's scheduler);
   - **B, the unit queue of the brief**: four workers; a split file's units go into the
     queue when its sequential part ends and are taken before any new file; the worker that
     finishes a file's last unit stitches it;
   - **C, the bound**: max(total / 4, the longest chain), the chain of a split file being its
     sequential part, its largest unit and its stitch.
   The costs are taken under the same load, so they carry its cache and memory-bandwidth
   inflation (the work totals are 1.0–1.5x M4's quiet CPU), but the shape of the schedule —
   which piece waits for which — does not depend on how long a thread waited for a core.

### Commands

```sh
git -C /home/user/chloro worktree add /home/user/wt/queue -b 20261015-unit-queue 9135c62
cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples
cp target/release/chloro chloro-base; cp target/release/examples/bench bench-base   # before any change
# the CPU-clock build, in a checkout of the kept head: apply, build, copy, drop the patch
python3 benchmarks/schedule/trace_patch.py "$SCRATCH" base
(cd "$SCRATCH" && cargo build --release -p chloro --features cli && git checkout chloro-core chloro/src)
cp "$SCRATCH"/target/release/chloro chloro-diag-base
# the collections (20 runs per crate at 4 threads unpinned, 10 at 1 thread on core 3,
# 5 each with a cold cache and with --write on raw copies)
benchmarks/schedule/collect.sh /home/user/wt/queue/chloro-diag-base $OUT 20 10
python3 benchmarks/schedule/report.py $OUT d1 d2 d4
python3 benchmarks/schedule/cpu_split.py /home/user/wt/queue/chloro-base 10 fx-hir fx-hir-ty fx-ide-db fx-syntax fx-parser reg-tokio reg-regex-syntax reg-clap-builder reg-winnow
benchmarks/schedule/identity.sh /home/user/wt/queue/chloro-base target/release/chloro $WORK
```

`diag.py` runs the binary with `--check --no-cache --threads N` (or the cold-cache and
`--write` modes) through `posix_spawn` and `wait4`, so its wall includes the spawn and the
child's exit, and its CPU is the child's `ru_utime + ru_stime`.

## The diagnostic

### D1. Wall view at four threads (measured, provisional: load 4.8–6.4)

Medians of 20 runs per crate, `--check --no-cache --threads 4`, unpinned, CPU-clock build.
Wall, in-process span (`main` to `finish`), the time outside `main` (exec, loader, exit, and
the harness's spawn), the main thread before the workers start and after they are joined,
the tail span and the idle core time in it, how many of the 20 runs ended on a split file,
worker utilisation (busy share of the format window: mean over workers, and the least busy),
the threads' on-CPU share of their spans, CPU (user+sys), load. ms unless marked.

| crate | wall | in-process | outside `main` | before workers | after join | tail span | tail / in-process | idle core-ms in tail | last file split (runs) | worker utilisation mean (min) | on-CPU share | CPU | load |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 137.6 | 131.5 | 6.2 | 0.16 | 0.01 | 11.4 | 10.1% | 18.9 | 9/20 | 0.85 (0.72) | 0.28 | 149.3 | 4.8 |
| fx-hir-ty | 296.8 | 289.9 | 6.9 | 0.95 | 0.06 | 2.4 | 0.7% | 7.3 | 0/20 | 0.87 (0.79) | 0.41 | 426.9 | 5.1 |
| fx-ide-db | 125.2 | 118.5 | 6.7 | 0.29 | 0.02 | 20.1 | 16.6% | 37.0 | 15/20 | 0.76 (0.56) | 0.31 | 135.5 | 5.5 |
| fx-syntax | 87.0 | 77.3 | 9.7 | 0.28 | 0.03 | 7.7 | 9.3% | 22.5 | 0/20 | 0.71 (0.49) | 0.31 | 74.0 | 5.5 |
| fx-parser | 59.3 | 54.6 | 4.7 | 0.25 | 0.02 | 10.1 | 17.0% | 29.1 | 0/20 | 0.67 (0.40) | 0.33 | 46.6 | 5.5 |
| reg-tokio | 33.9 | 29.1 | 4.8 | 0.46 | 0.03 | 10.4 | 35.9% | 27.6 | 0/20 | 0.52 (0.19) | 0.39 | 27.6 | 5.7 |
| reg-regex-syntax | 301.2 | 294.0 | 7.1 | 0.33 | 0.03 | 6.7 | 2.5% | 17.0 | 0/20 | 0.93 (0.87) | 0.26 | 381.5 | 5.7 |
| reg-clap-builder | 114.7 | 108.5 | 6.2 | 0.48 | 0.03 | 4.0 | 3.6% | 14.5 | 0/20 | 0.81 (0.69) | 0.33 | 140.1 | 6.0 |
| reg-winnow | 135.7 | 131.2 | 4.5 | 0.52 | 0.03 | 4.7 | 2.9% | 14.8 | 0/20 | 0.82 (0.72) | 0.33 | 135.6 | 6.4 |

- Read with the on-CPU share: our threads ran for 26–41% of the time they were runnable.
  The tail span exceeds 10% of the in-process time on `fx-hir`, `fx-ide-db`, `fx-parser` and
  `reg-tokio`, but on the last two (no split file) the last file to finish is always a whole
  small file and the "tail" is late starts and preemption: their least busy worker was busy
  19–40% of the window. On `fx-ide-db` (15 of 20 runs) and `fx-hir` (9 of 20) the last file
  is the split one: the tail there is that file.
- The main thread's serial phases inside `main` are small: 0.16–0.95 ms before the workers
  (the walk is nearly all of it; `fx-hir-ty`'s 122 files take the longest) and 0.01–0.06 ms
  after the join. Outside `main` (exec, dynamic loading, exit, plus the harness's own spawn
  and wait) the runs spend 4.5–9.7 ms here against M4's 1.7–2.4 ms for the whole of a
  one-file run on a quiet host.

### D2. The schedule rebuilt from CPU clocks (reconstructed)

Medians over the same 20 traces per crate. "Work" is the sum of on-CPU time of every file,
part and unit (no process start-up); A, B and C as in *Method*; "last to finish in A" is
what A ends on (a split file's stitch or a whole small file, majority of runs).

| crate | split files | work (CPU ms) | of it split files | work / 4 | longest chain | today (A) | A / (work/4) | A idle core-ms | A tail | last to finish in A | unit queue (B) | B / (work/4) | B gain over A | bound C |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 1 | 141.6 | 53.4 | 35.4 | 30.3 | 36.2 | 1.010 | 0.75 | 2.56 | split file (231 KiB) | 37.5 | 1.020 | -0.8% | 35.4 |
| fx-hir-ty | 1 | 417.9 | 34.0 | 104.5 | 32.5 | 104.6 | 1.001 | 0.10 | 0.05 | a whole small file | 104.6 | 1.001 | 0.0% | 104.5 |
| fx-ide-db | 1 | 127.8 | 47.7 | 31.9 | 32.4 | 34.6 | 1.079 | 11.80 | 4.45 | split file (625 KiB) | 33.2 | 1.009 | 5.7% | 32.7 |
| fx-syntax | 0 | 69.1 | 0.0 | 17.3 | 9.6 | 17.3 | 1.002 | 0.15 | 0.09 | a whole small file | 17.3 | 1.002 | 0.0% | 17.3 |
| fx-parser | 0 | 42.4 | 0.0 | 10.6 | 6.4 | 10.7 | 1.011 | 0.48 | 0.23 | a whole small file | 10.7 | 1.011 | 0.0% | 10.6 |
| reg-tokio | 0 | 22.4 | 0.0 | 5.6 | 3.8 | 5.6 | 1.003 | 0.08 | 0.04 | a whole small file | 5.6 | 1.003 | 0.0% | 5.6 |
| reg-regex-syntax | 4 | 366.6 | 187.3 | 91.7 | 41.9 | 92.1 | 1.005 | 0.91 | 0.33 | a whole small file | 92.1 | 1.004 | 0.1% | 91.7 |
| reg-clap-builder | 2 | 131.2 | 36.9 | 32.8 | 15.8 | 32.9 | 1.003 | 0.10 | 0.04 | a whole small file | 33.4 | 1.002 | 0.0% | 32.8 |
| reg-winnow | 0 | 130.0 | 0.0 | 32.5 | 16.3 | 32.6 | 1.002 | 0.20 | 0.09 | a whole small file | 32.6 | 1.002 | 0.0% | 32.5 |

- Today's schedule is within 0.1–1.1% of work/4 on eight crates; its idle core time there
  is 0.1–0.9 core-ms per run. The largest-first queue already does what the tail argument
  asks for: the split file starts first and its units are done while the small files keep
  the other workers busy; what ends the run is a whole small file.
- `fx-ide-db` is the exception: one split file of 625 KiB whose chain (32.4 ms) is longer
  than a quarter of the work (31.9 ms). A is 7.9% above work/4, idle 11.8 core-ms, tail 4.5
  ms, and the replay ends on that file in 18 of 20 runs. B takes 5.7% off A and lands 1.5% above
  the bound: under processor sharing the critical 288 KB unit gets 4/7 of a core while the
  three other file workers and three item threads run beside it; in the queue it has a core
  of its own. Two earlier collections with the same clock (15 runs with the shared-lock
  trace, 20 runs with the buffered one, before the `hashed` stamp; the reconstruction does
  not depend on either change; replayed before B charged each worker's visitor set-up, which
  moves this run's figure by 0.1 point) gave B gains of 4.1% and 2.7% there.
- `fx-hir` (one split file of 231 KiB, chain 30.3 of 35.4 ms) is the other crate whose
  replay ends on its split file (15 of 20); A is 1.0% above work/4, and the queue as specified
  (units before new files) comes out 0.8% worse than A, because it pulls all four workers onto
  the split file's units and leaves the long small files for the end.
- The queue's bound on the crates is therefore at most 0.1% of four-core wall on eight of
  them and about 6% on `fx-ide-db`, against L8's 6.6–34.6%: L8 took the whole distance
  between M3's mean wall and CPU/4 as queueing; D3 shows most of it is not.

### D3. Where M4's gap goes (derived from M4's quiet-host figures and D2)

M4's `head` rows (quiet host, 30 runs, `mc` unpinned and `1c` on core 3, `--check
--no-cache`). S is the serial part of a run: 2.2 ms for process start-up and exit (M4's
`tiny-one` `mc` CPU, one 13-byte file) plus the walk (the minimum of D1's "before workers"
over 20 runs, an over-estimate on a quiet host). W* = S + (CPU − S)/4 is the wall a
work-conserving scheduler reaches with the serial part on one core. The last three columns
split the distance from the mean wall to CPU/4 into the serial part (W* − CPU/4), the
schedule's part ((A/(work/4) − 1) × (CPU − S)/4, from D2) and the rest.

| crate | mc wall min / mean ± σ | mc CPU | busy cores (CPU / mean) | 1c wall | speed-up (1c / mc mean) | CPU mc / 1c | serial S | CPU/4 | W* = S + (CPU − S)/4 | min / W* | mean / W* | mean − CPU/4 | of it serial | of it schedule (A) | of it spread |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 27.5 / 32.1 ± 3.0 | 106.9 | 3.33 | 86.3 | 2.69 | 1.24 | 2.34 | 26.7 | 28.5 | 0.97 | 1.13 | 5.4 | 1.8 (33%) | 0.3 (5%) | 3.4 (62%) |
| fx-hir-ty | 75.0 / 84.5 ± 6.2 | 314.2 | 3.72 | 283.0 | 3.35 | 1.12 | 2.89 | 78.5 | 80.7 | 0.93 | 1.05 | 6.0 | 2.2 (36%) | 0.1 (1%) | 3.7 (62%) |
| fx-ide-db | 29.4 / 35.0 ± 3.6 | 105.9 | 3.03 | 85.3 | 2.44 | 1.25 | 2.43 | 26.5 | 28.3 | 1.04 | 1.24 | 8.5 | 1.8 (21%) | 2.0 (24%) | 4.7 (55%) |
| fx-syntax | 16.2 / 17.7 ± 1.0 | 59.3 | 3.35 | 52.5 | 2.97 | 1.15 | 2.44 | 14.8 | 16.7 | 0.97 | 1.06 | 2.9 | 1.8 (64%) | 0.0 (1%) | 1.0 (35%) |
| fx-parser | 10.7 / 12.9 ± 2.0 | 39.4 | 3.05 | 30.8 | 2.39 | 1.29 | 2.42 | 9.8 | 11.7 | 0.92 | 1.11 | 3.1 | 1.8 (59%) | 0.1 (3%) | 1.1 (37%) |
| reg-tokio | 7.4 / 8.4 ± 0.8 | 22.5 | 2.68 | 21.2 | 2.52 | 1.08 | 2.49 | 5.6 | 7.5 | 0.99 | 1.12 | 2.8 | 1.9 (67%) | 0.0 (1%) | 0.9 (32%) |
| reg-regex-syntax | 64.9 / 74.8 ± 6.6 | 248.4 | 3.32 | 223.4 | 2.99 | 1.12 | 2.45 | 62.1 | 63.9 | 1.02 | 1.17 | 12.7 | 1.8 (14%) | 0.3 (2%) | 10.6 (83%) |
| reg-clap-builder | 25.0 / 30.1 ± 3.4 | 95.3 | 3.17 | 80.5 | 2.67 | 1.19 | 2.52 | 23.8 | 25.7 | 0.97 | 1.17 | 6.3 | 1.9 (30%) | 0.1 (1%) | 4.3 (69%) |
| reg-winnow | 25.9 / 27.4 ± 1.3 | 93.9 | 3.43 | 89.1 | 3.25 | 1.07 | 2.55 | 23.5 | 25.4 | 1.02 | 1.08 | 3.9 | 1.9 (49%) | 0.0 (1%) | 2.0 (50%) |

- The fastest run of every crate is within −8% to +4% of W* (0.92–1.04): a run need lose
  nothing but the serial part against CPU/4 (a fastest run below W* also used less CPU than
  the mean that W* is computed from). The mean is 5–24% above W*: the spread between runs.
- The serial part is 1.8–2.2 ms on every crate, 14–67% of the gap (the larger share on the
  small crates: on `reg-tokio` W* is 7.5 ms against a CPU/4 of 5.6 ms).
- The schedule's part is 0.0–0.3 ms on eight crates (1–5% of the gap) and 2.0 ms on
  `fx-ide-db` (24%).
- The remainder, 32–83% of the gap, is the spread of the mean above the fastest run. This
  diagnostic cannot attribute it on this host (the load swamps it). It is largest on the
  crates with split files (mean/W* 1.13, 1.05, 1.24, 1.17, 1.17 with split files; 1.06, 1.11,
  1.12, 1.08 without), which is a hypothesis for M5, not a finding (*Follow-ups*).
- A separate term that the "busy cores" figure hides: the four-thread run costs 7–29% more
  CPU than the one-thread run (column "CPU mc / 1c"). It does not widen the gap to CPU/4 (it
  raises CPU/4 with the wall), but it is the largest single reason the four-core speed-up is
  2.4–3.4 and not 4: see D5.

### D4. Phases of a run (measured, provisional)

Medians: 10 runs (`--check --no-cache`, 1 thread, `taskset -c 3`), 20 runs (4 threads), 5
runs for the cold-cache `--check` (a fresh `--cache-dir` per run: the tool key, one digest per
file, the save) and for `--write --no-cache` on a fresh copy of the raw tree per run. "read",
"hash", "format" and "output" are sums over files of each file's spans (at 4 threads, of four
workers' spans: wall-clock, under load, so above the CPU). "cache open" is the executable's
hash for the tool key (E12's measurement, L3's lever); "output" is everything after the
format: the comparison with the input, the write of a changed file, freeing the file's
buffers. ms.

| crate | mode | threads | wall | outside `main` | args | cache open | walk | sort | read (sum) | hash (sum) | format (sum) | output (sum) | cache save | CPU | load |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | check, no cache | 1 | 177.7 | 5.0 | 0.03 | 0.00 | 0.16 | 0.01 | 1.02 | 0.00 | 170.3 | 0.20 | 0.00 | 161.9 | 6.4 |
| fx-hir | check, no cache | 4 | 137.6 | 6.2 | 0.03 | 0.00 | 0.13 | 0.00 | 1.24 | 0.00 | 415.8 | 0.18 | 0.01 | 149.3 | 4.8 |
| fx-hir | check, cold cache | 1 | 177.8 | 8.8 | 0.03 | 3.17 | 0.17 | 0.01 | 1.02 | 0.47 | 163.2 | 0.12 | 0.22 | 162.5 | 6.9 |
| fx-hir | check, cold cache | 4 | 155.0 | 8.7 | 0.03 | 6.25 | 0.17 | 0.01 | 1.54 | 0.49 | 386.9 | 0.12 | 0.19 | 149.5 | 6.6 |
| fx-hir | write raw, no cache | 1 | 154.8 | 10.7 | 0.02 | 0.00 | 0.15 | 0.01 | 0.83 | 0.00 | 139.7 | 3.12 | 0.00 | 117.1 | 10.0 |
| fx-hir | write raw, no cache | 4 | 170.5 | 4.7 | 0.03 | 0.00 | 0.15 | 0.00 | 1.06 | 0.00 | 461.4 | 13.94 | 0.01 | 137.4 | 7.3 |
| fx-hir-ty | check, no cache | 1 | 497.0 | 7.9 | 0.03 | 0.00 | 0.84 | 0.03 | 5.33 | 0.00 | 480.1 | 0.64 | 0.01 | 402.0 | 6.4 |
| fx-hir-ty | check, no cache | 4 | 296.8 | 6.9 | 0.03 | 0.00 | 0.86 | 0.03 | 7.76 | 0.00 | 981.2 | 0.67 | 0.02 | 426.9 | 5.1 |
| fx-hir-ty | check, cold cache | 1 | 390.9 | 9.6 | 0.02 | 2.69 | 0.75 | 0.02 | 4.52 | 2.52 | 362.7 | 0.45 | 0.20 | 346.0 | 6.9 |
| fx-hir-ty | check, cold cache | 4 | 357.6 | 6.7 | 0.03 | 3.26 | 0.70 | 0.02 | 24.77 | 2.93 | 1143.2 | 0.65 | 0.27 | 433.3 | 6.6 |
| fx-hir-ty | write raw, no cache | 1 | 550.6 | 5.0 | 0.03 | 0.00 | 0.71 | 0.03 | 6.64 | 0.00 | 516.4 | 18.36 | 0.01 | 442.9 | 10.0 |
| fx-hir-ty | write raw, no cache | 4 | 407.0 | 7.8 | 0.03 | 0.00 | 0.76 | 0.03 | 19.41 | 0.00 | 1337.6 | 44.76 | 0.04 | 385.8 | 7.6 |
| fx-ide-db | check, no cache | 1 | 169.3 | 8.3 | 0.03 | 0.00 | 0.28 | 0.01 | 2.19 | 0.00 | 156.3 | 0.33 | 0.00 | 138.1 | 6.5 |
| fx-ide-db | check, no cache | 4 | 125.2 | 6.7 | 0.03 | 0.00 | 0.25 | 0.01 | 5.07 | 0.00 | 359.8 | 0.34 | 0.01 | 135.5 | 5.5 |
| fx-ide-db | check, cold cache | 1 | 124.6 | 4.7 | 0.03 | 2.82 | 0.23 | 0.01 | 1.74 | 0.63 | 113.5 | 0.19 | 0.27 | 118.6 | 6.9 |
| fx-ide-db | check, cold cache | 4 | 125.5 | 4.0 | 0.03 | 2.73 | 0.21 | 0.01 | 2.31 | 0.70 | 325.8 | 0.21 | 0.23 | 132.0 | 6.6 |
| fx-ide-db | write raw, no cache | 1 | 137.3 | 5.2 | 0.02 | 0.00 | 0.23 | 0.01 | 1.50 | 0.00 | 125.9 | 4.39 | 0.00 | 108.7 | 10.2 |
| fx-ide-db | write raw, no cache | 4 | 170.9 | 6.0 | 0.03 | 0.00 | 0.26 | 0.01 | 6.27 | 0.00 | 418.8 | 13.99 | 0.01 | 130.7 | 8.3 |
| fx-syntax | check, no cache | 1 | 103.8 | 7.4 | 0.03 | 0.00 | 0.29 | 0.01 | 1.34 | 0.00 | 93.4 | 0.12 | 0.00 | 94.5 | 6.5 |
| fx-syntax | check, no cache | 4 | 87.0 | 9.7 | 0.03 | 0.00 | 0.24 | 0.01 | 1.37 | 0.00 | 220.7 | 0.09 | 0.01 | 74.0 | 5.5 |
| fx-syntax | check, cold cache | 1 | 85.6 | 7.7 | 0.03 | 2.81 | 0.20 | 0.01 | 1.08 | 0.41 | 67.2 | 0.09 | 0.21 | 69.8 | 6.9 |
| fx-syntax | check, cold cache | 4 | 113.4 | 6.2 | 0.04 | 3.52 | 0.32 | 0.01 | 3.72 | 0.58 | 282.5 | 0.14 | 0.22 | 108.4 | 6.6 |
| fx-syntax | write raw, no cache | 1 | 79.2 | 4.2 | 0.03 | 0.00 | 0.24 | 0.01 | 0.79 | 0.00 | 66.5 | 3.32 | 0.01 | 73.1 | 10.2 |
| fx-syntax | write raw, no cache | 4 | 146.0 | 7.1 | 0.03 | 0.00 | 0.33 | 0.01 | 3.08 | 0.00 | 357.1 | 27.13 | 0.01 | 83.6 | 8.3 |
| fx-parser | check, no cache | 1 | 48.4 | 5.2 | 0.03 | 0.00 | 0.26 | 0.01 | 0.83 | 0.00 | 41.6 | 0.06 | 0.00 | 46.5 | 6.5 |
| fx-parser | check, no cache | 4 | 59.3 | 4.7 | 0.03 | 0.00 | 0.20 | 0.01 | 1.00 | 0.00 | 129.8 | 0.07 | 0.01 | 46.6 | 5.5 |
| fx-parser | check, cold cache | 1 | 52.2 | 3.7 | 0.03 | 3.11 | 0.24 | 0.01 | 0.76 | 0.28 | 43.9 | 0.05 | 0.22 | 52.0 | 6.9 |
| fx-parser | check, cold cache | 4 | 67.7 | 5.7 | 0.03 | 6.94 | 0.23 | 0.01 | 0.89 | 0.31 | 143.7 | 0.07 | 0.20 | 58.3 | 6.6 |
| fx-parser | write raw, no cache | 1 | 47.8 | 3.7 | 0.03 | 0.00 | 0.18 | 0.01 | 0.64 | 0.00 | 40.4 | 2.38 | 0.00 | 40.4 | 10.2 |
| fx-parser | write raw, no cache | 4 | 102.0 | 3.7 | 0.03 | 0.00 | 0.25 | 0.01 | 1.42 | 0.00 | 171.3 | 3.04 | 0.01 | 60.0 | 8.3 |
| reg-tokio | check, no cache | 1 | 32.0 | 5.6 | 0.03 | 0.00 | 0.34 | 0.01 | 1.07 | 0.00 | 24.3 | 0.08 | 0.00 | 30.9 | 6.5 |
| reg-tokio | check, no cache | 4 | 33.9 | 4.8 | 0.03 | 0.00 | 0.40 | 0.01 | 1.02 | 0.00 | 56.1 | 0.06 | 0.01 | 27.6 | 5.7 |
| reg-tokio | check, cold cache | 1 | 37.8 | 4.8 | 0.03 | 3.42 | 0.39 | 0.01 | 0.92 | 0.25 | 27.0 | 0.07 | 0.23 | 37.4 | 6.9 |
| reg-tokio | check, cold cache | 4 | 54.3 | 8.3 | 0.04 | 10.81 | 0.34 | 0.01 | 1.28 | 0.24 | 67.3 | 0.07 | 0.28 | 33.4 | 6.7 |
| reg-tokio | write raw, no cache | 1 | 35.8 | 7.6 | 0.03 | 0.00 | 0.35 | 0.02 | 1.13 | 0.00 | 25.7 | 0.49 | 0.01 | 29.6 | 10.3 |
| reg-tokio | write raw, no cache | 4 | 44.6 | 6.8 | 0.02 | 0.00 | 0.30 | 0.01 | 1.08 | 0.00 | 77.6 | 0.44 | 0.01 | 25.6 | 8.3 |
| reg-regex-syntax | check, no cache | 1 | 353.0 | 5.9 | 0.03 | 0.00 | 0.25 | 0.01 | 3.02 | 0.00 | 343.0 | 0.38 | 0.00 | 339.4 | 6.7 |
| reg-regex-syntax | check, no cache | 4 | 301.2 | 7.1 | 0.03 | 0.00 | 0.28 | 0.01 | 4.97 | 0.00 | 1077.9 | 0.62 | 0.01 | 381.5 | 5.7 |
| reg-regex-syntax | check, cold cache | 1 | 285.3 | 5.2 | 0.03 | 2.79 | 0.26 | 0.01 | 2.64 | 1.09 | 269.0 | 0.25 | 0.18 | 264.0 | 7.3 |
| reg-regex-syntax | check, cold cache | 4 | 377.7 | 10.8 | 0.03 | 3.63 | 0.26 | 0.01 | 3.28 | 1.04 | 1262.5 | 0.33 | 0.18 | 374.8 | 6.7 |
| reg-regex-syntax | write raw, no cache | 1 | 406.1 | 9.5 | 0.03 | 0.00 | 0.26 | 0.01 | 3.55 | 0.00 | 384.5 | 5.15 | 0.00 | 313.2 | 10.3 |
| reg-regex-syntax | write raw, no cache | 4 | 401.7 | 17.0 | 0.03 | 0.00 | 0.21 | 0.01 | 4.54 | 0.00 | 1347.0 | 5.06 | 0.01 | 307.2 | 9.4 |
| reg-clap-builder | check, no cache | 1 | 128.3 | 5.1 | 0.03 | 0.00 | 0.44 | 0.02 | 2.20 | 0.00 | 119.3 | 0.26 | 0.01 | 121.1 | 6.7 |
| reg-clap-builder | check, no cache | 4 | 114.7 | 6.2 | 0.03 | 0.00 | 0.40 | 0.02 | 3.48 | 0.00 | 366.0 | 0.34 | 0.01 | 140.1 | 6.0 |
| reg-clap-builder | check, cold cache | 1 | 133.2 | 10.1 | 0.03 | 2.80 | 0.31 | 0.01 | 1.72 | 0.69 | 116.4 | 0.15 | 0.23 | 115.0 | 7.3 |
| reg-clap-builder | check, cold cache | 4 | 192.3 | 9.7 | 0.03 | 6.58 | 0.41 | 0.02 | 18.65 | 1.02 | 501.8 | 0.29 | 0.25 | 160.4 | 6.7 |
| reg-clap-builder | write raw, no cache | 1 | 144.6 | 5.8 | 0.03 | 0.00 | 0.33 | 0.02 | 1.93 | 0.00 | 135.0 | 0.23 | 0.01 | 117.7 | 10.3 |
| reg-clap-builder | write raw, no cache | 4 | 161.5 | 7.4 | 0.03 | 0.00 | 0.35 | 0.02 | 2.98 | 0.00 | 465.0 | 0.29 | 0.02 | 108.8 | 9.4 |
| reg-winnow | check, no cache | 1 | 159.2 | 4.7 | 0.03 | 0.00 | 0.46 | 0.02 | 2.13 | 0.00 | 150.6 | 0.23 | 0.01 | 157.1 | 6.7 |
| reg-winnow | check, no cache | 4 | 135.7 | 4.5 | 0.03 | 0.00 | 0.43 | 0.02 | 2.64 | 0.00 | 382.6 | 0.19 | 0.01 | 135.6 | 6.4 |
| reg-winnow | check, cold cache | 1 | 165.3 | 4.6 | 0.03 | 3.26 | 0.41 | 0.02 | 1.84 | 0.90 | 151.2 | 0.20 | 0.21 | 150.2 | 7.3 |
| reg-winnow | check, cold cache | 4 | 236.9 | 12.1 | 0.04 | 4.59 | 0.49 | 0.02 | 2.80 | 0.88 | 716.8 | 0.27 | 0.27 | 183.3 | 6.7 |
| reg-winnow | write raw, no cache | 1 | 154.6 | 8.2 | 0.03 | 0.00 | 0.36 | 0.02 | 1.81 | 0.00 | 124.3 | 4.96 | 0.01 | 106.1 | 10.4 |
| reg-winnow | write raw, no cache | 4 | 189.8 | 7.4 | 0.03 | 0.00 | 0.39 | 0.02 | 2.15 | 0.00 | 523.1 | 23.95 | 0.02 | 124.0 | 9.4 |

- On one core, formatting is 92–99% of a run's in-process time (`--check --no-cache`;
  82–97% with a cold cache, 85–97% for `--write`); reading the files 0.6–4%, the digests
  0.3–0.8% (cold cache), output 0.1–0.3% for `--check` and 0.2–5.4% for `--write` of a raw
  crate; the walk 0.15–0.84 ms per run, the sort under 0.03 ms. With a cold cache the cache
  open (the 2 MB executable's hash for the tool key) is 2.7–3.4 ms, more than the walk and the
  digests together on every crate: L3's lever, unchanged by this record.
- A one-file run (`tiny-one`, 30 runs, 1 thread, core 3, load 10.6) spends 0.46 ms inside
  `main` (median; 0.08 ms before the worker, 0.34 ms from its spawn to its join, of which
  0.08 ms formatting the file) of 10.6 ms wall
  here; M4 measured the whole run at 1.8 ms pinned on a quiet host. The process's fixed cost
  is outside `main`: exec, dynamic loading, runtime start-up and exit.
- Nothing in the run's own serial phases is a lever of the size L8 assumed; the fixed cost
  that matters at crate scale is the process's (D3's S).

### D5. CPU at one and four threads (measured, provisional: load 3.5–5.1)

`cpu_split.py`: medians of 10 interleaved rounds of `chloro-base --check --no-cache`:
1c1t = `taskset -c 3 --threads 1`, 1c4t = `taskset -c 3 --threads 4` (four threads sharing one
core: the threads' own cost, no parallel contention), 4c4t = `--threads 4` unpinned. Columns:
wall, user, sys, user+sys, peak RSS, voluntary and involuntary context switches, minor faults.

| crate | config | wall | user | sys | user+sys | RSS MiB | vol. csw | invol. csw | minor faults | load |
|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 1c1t | 113.7 | 95.8 | 8.5 | 104.3 | 11.3 | 2 | 8 | 2846 | 3.5 |
| fx-hir | 1c4t | 139.5 | 109.3 | 12.5 | 121.8 | 16.4 | 4 | 33 | 5112 | 3.5 |
| fx-hir | 4c4t | 89.1 | 112.2 | 14.4 | 126.6 | 15.0 | 7 | 35 | 4874 | 3.5 |
| fx-hir-ty | 1c1t | 340.1 | 316.6 | 8.0 | 324.6 | 11.3 | 2 | 9 | 1980 | 4.2 |
| fx-hir-ty | 1c4t | 404.2 | 364.9 | 22.3 | 387.1 | 18.4 | 4 | 103 | 5457 | 4.2 |
| fx-hir-ty | 4c4t | 226.0 | 337.6 | 23.6 | 361.2 | 17.9 | 12 | 94 | 4984 | 4.2 |
| fx-ide-db | 1c1t | 164.4 | 143.6 | 17.9 | 161.5 | 11.3 | 2 | 2 | 3128 | 4.2 |
| fx-ide-db | 1c4t | 152.3 | 116.7 | 23.9 | 140.5 | 18.4 | 6 | 40 | 5259 | 4.2 |
| fx-ide-db | 4c4t | 87.6 | 118.6 | 21.6 | 140.2 | 18.6 | 12 | 34 | 5314 | 4.2 |
| fx-syntax | 1c1t | 70.4 | 63.9 | 4.0 | 67.9 | 11.3 | 2 | 2 | 954 | 4.5 |
| fx-syntax | 1c4t | 82.0 | 65.5 | 7.6 | 73.1 | 11.3 | 3 | 20 | 1780 | 4.5 |
| fx-syntax | 4c4t | 53.0 | 72.4 | 6.4 | 78.7 | 11.3 | 6 | 20 | 1724 | 4.5 |
| fx-parser | 1c1t | 40.6 | 37.1 | 3.8 | 40.9 | 11.3 | 2 | 2 | 514 | 4.5 |
| fx-parser | 1c4t | 43.8 | 35.5 | 6.8 | 42.3 | 11.3 | 3 | 10 | 972 | 4.5 |
| fx-parser | 4c4t | 31.0 | 38.7 | 2.1 | 40.7 | 11.3 | 7 | 11 | 882 | 4.5 |
| reg-tokio | 1c1t | 32.2 | 21.5 | 2.2 | 23.6 | 11.3 | 2 | 4 | 410 | 4.5 |
| reg-tokio | 1c4t | 32.2 | 23.1 | 1.7 | 24.8 | 11.3 | 4 | 8 | 658 | 4.5 |
| reg-tokio | 4c4t | 21.5 | 23.4 | 2.1 | 25.5 | 11.3 | 8 | 6 | 600 | 4.5 |
| reg-regex-syntax | 1c1t | 287.2 | 256.1 | 17.8 | 273.9 | 13.8 | 2 | 4 | 4281 | 4.8 |
| reg-regex-syntax | 1c4t | 330.1 | 268.5 | 35.0 | 303.6 | 28.6 | 12 | 74 | 9830 | 4.8 |
| reg-regex-syntax | 4c4t | 186.9 | 291.9 | 33.3 | 325.1 | 29.0 | 46 | 80 | 9786 | 4.8 |
| reg-clap-builder | 1c1t | 105.1 | 97.9 | 7.8 | 105.7 | 11.4 | 2 | 3 | 1267 | 5.1 |
| reg-clap-builder | 1c4t | 105.1 | 93.4 | 8.2 | 101.5 | 11.7 | 7 | 26 | 2889 | 5.1 |
| reg-clap-builder | 4c4t | 72.8 | 111.5 | 14.5 | 126.0 | 11.4 | 24 | 30 | 2922 | 5.1 |
| reg-winnow | 1c1t | 119.1 | 104.0 | 5.9 | 109.8 | 11.4 | 2 | 2 | 1226 | 5.1 |
| reg-winnow | 1c4t | 149.1 | 126.7 | 11.1 | 137.8 | 11.4 | 4 | 36 | 2555 | 5.1 |
| reg-winnow | 4c4t | 82.0 | 124.3 | 8.7 | 133.0 | 11.4 | 5 | 33 | 2463 | 5.1 |

- Four threads take 1.5–2.8x the minor faults of one thread on every crate, the same whether
  they share one core (1c4t) or run on four (4c4t): it is the thread count, not the
  parallelism. Each worker's allocator arena faults in its own pages, and a split file's item
  threads add more for its duration. System time follows on the larger crates (`fx-hir-ty`
  8.0 → 22.3 and 23.6 ms, `reg-regex-syntax` 17.8 → 35.0 and 33.3 ms, peak RSS 13.8 → 28.6
  MiB) and not on the smallest (`reg-tokio` 2.2 → 1.7 and 2.1 ms). This is a part of M4's
  1.07–1.29x CPU inflation at four threads; how large a part, and how much is the hardware's
  (shared L3, memory bandwidth), needs the quiet host. On this host the 1c rows share core 3
  with other agents' pinned runs and the user times are noisy (`fx-ide-db` 1c1t above 1c4t).

## Decision

**The queue is not built.** The stop rule's first clause holds on eight of nine crates (the
reconstructed tail 0.05–2.1% of the makespan on seven of them and 7.1% on `fx-hir`; the
queue's gain at most 0.1%), and its second clause holds for the gap M4 measured: the
distance from the busy-core figure to four cores is the process's serial part and the
run-to-run spread, with the schedule 1–5% of it on eight crates. The one crate where the
tail is the schedule's (`fx-ide-db`, 13% of the makespan, 24% of its gap) is a single large
file's chain; the queue would take about 6% of its four-core wall: below the stop threshold,
inside M4's σ of 10% for the row, and unmeasurable on this host (on-CPU share 0.31). The
hypothesis is **refuted as stated** (the idle tail is not the cause of the gap); its second
half (the queue's four-core wall) is not tested, since nothing was built to test.

The bound of L8 and T3 (6.6–34.6% of four-core wall) is replaced by: at most 0.1% on eight
crates and about 6% on `fx-ide-db` (**reconstructed**), to be checked on M5's quiet host
with the kept timeline.

What was built and kept, with the gates below: the `CHLORO_TRACE` timeline (`trace.rs`, 188
lines with comments, and 41 lines in `cli.rs`, `orchestrate.rs` and `worker.rs`), its test
(`chloro/tests/cli_trace.rs`, three cases), and `benchmarks/schedule/`.

## Design that was not built

For the record, so a later attempt does not start from zero: a unit is a whole file below
128 KiB or one unit of `parallel.rs` (one top-level item or one reorderable group) of a larger
file; a worker that opens a large file parses it, extracts the units (`unit_ranges`) and
publishes them; workers take units of open files before new files (longest first), each
with a visitor per (worker, file) kept while it stays on that file (the line table and memo
amortise as in `format_units`); the worker finishing a file's last unit runs the existing
`stitchable`/`stitch` and reports the file. The library API stays as it is (an additive
`SplitFile` type holding the tree and the units behind an `Arc`, since a unit's visitor
borrows the tree). D2's B is this design under the replay; its weak point is the case of
`fx-hir`, where taking units before new files leaves long small files for the end.

## Gates (the kept commit)

All on the kept head (the timeline, its test, `benchmarks/schedule/`; `chloro-core`
untouched: the head's `bench` is byte-identical to `bench-base`, sha256 `f193784e…6198`).

| gate | command | result |
|---|---|---|
| format | `cargo fmt --all --check` | clean |
| lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| tests | `cargo test --workspace --all-features` | all pass: the CLI's 24 unit tests, `cli_cache` 11, `cli_files` 5, `cli_trace` 3 (new), `cli_unit_cache` 3; `chloro-core` 202 unit tests, conformance 19, `parallel_equivalence` 1, `unit_cache_equivalence` 1 and the rest |
| goldens, fixtures | `bench -n 1 --check scratchpad/golden/fixtures-e8d87c2.tsv --root chloro-core/tests/conformance/fixtures`, and with `--threads 4` | 0 of 1238 changed; at 4 threads also 0 of 1238 differ from `format_source` |
| goldens, registry | `bench -n 1 --check scratchpad/golden/registry-e8d87c2.tsv --root /root/.cargo/registry/src` | 0 changed of the 20,850 recorded files (25,024 present; the others not recorded, ignored) |
| conformance | `conform -i`; `conform -i -V 1.8.0` | 1220/1220 identical, 0 not idempotent, at 1.9.0 and at 1.8.0 |
| instructions | `benchmarks/profile.sh --core 3 --sample /tmp/chloro-bench/queuex49` | 232,049,681 `Ir`, inside the head's 232,047,700–232,050,590 (the formatter is untouched) |
| CLI identity | `benchmarks/schedule/identity.sh chloro-base target/release/chloro …` | 0 of 193 comparisons differ: `--check` on the nine formatted crates, the nine raw crates and the 1238-file fixtures tree; `--write` on copies of the nine raw crates and the fixtures tree (exit codes, stdout, stderr and the written trees); print mode (stdout exact at 1 thread, sorted at 2 and 4); at 1, 2 and 4 threads each; plus `--check` with a cold cache at 4 threads |
| cost when off | callgrind on the CLI process, and hyperfine (below) | instructions of a whole `--threads 1` run: `reg-tokio` 79,955,240 (base) and 79,951,698 (head), `fx-syntax` 264,828,900 and 264,828,564, each the same in two runs: no measurable cost; hyperfine under load 14–17: every range overlaps |

### The timeline's cost when off

Instructions (deterministic): `env -i PATH=/usr/bin:/bin setarch -R taskset -c 3 valgrind
--tool=callgrind ./chloro-{base,head} --check --no-cache --threads 1 <tree>`, the two binaries
at paths of the same length, two runs each: identical counts per binary, the head 3,542
(`reg-tokio`, 0.004%) and 336 (`fx-syntax`) instructions below the base: the checks the
timeline adds when off are lost in the two builds' layout differences.

Wall (measured, provisional, load 14.0–16.8, so σ of 15–50% and no comparison resolves):
hyperfine 2.0 `-N -i -w 2 -r 15`, two rounds in reversed order, `--check --no-cache`; `1c` =
`taskset -c 3 … --threads 1`, `4c` = unpinned, default threads.

| workload | command | n | wall min / mean ± σ ms | max | CPU (user+sys) ms | peak RSS MiB | load (start, end) |
|---|---|---|---|---|---|---|---|
| tiny-one | base-1c | 30 | 4.2 / 13.0 ± 6.0 | 26.6 | 4.6 | 3.6 | 14.03, 14.03 |
| tiny-one | head-1c | 30 | 4.0 / 13.5 ± 6.7 | 32.0 | 4.4 | 3.6 | 14.03, 14.03 |
| tiny-one | base-4c | 30 | 2.2 / 6.8 ± 3.8 | 17.0 | 2.8 | 3.7 | 14.03, 14.03 |
| tiny-one | head-4c | 30 | 2.4 / 8.2 ± 4.9 | 20.2 | 2.9 | 3.6 | 14.03, 14.03 |
|  | head/base 1c | | min 0.959, mean 1.032 | ranges overlap | CPU 0.940 | | |
|  | head/base 4c | | min 1.077, mean 1.197 | ranges overlap | CPU 1.013 | | |
| reg-tokio | base-1c | 30 | 29.3 / 68.5 ± 21.7 | 110.7 | 25.5 | 4.3 | 14.03, 14.71 |
| reg-tokio | head-1c | 30 | 31.7 / 68.9 ± 20.7 | 104.5 | 25.0 | 4.3 | 14.03, 14.71 |
| reg-tokio | base-4c | 30 | 35.5 / 59.1 ± 12.7 | 80.4 | 27.4 | 5.2 | 14.03, 14.71 |
| reg-tokio | head-4c | 30 | 36.8 / 54.7 ± 12.1 | 83.3 | 25.6 | 5.2 | 14.03, 14.71 |
|  | head/base 1c | | min 1.083, mean 1.006 | ranges overlap | CPU 0.977 | | |
|  | head/base 4c | | min 1.035, mean 0.925 | ranges overlap | CPU 0.933 | | |
| fx-hir-ty | base-1c | 30 | 563.2 / 842.5 ± 113.4 | 1020.8 | 388.2 | 8.1 | 14.71, 16.23 |
| fx-hir-ty | head-1c | 30 | 655.9 / 922.3 ± 148.5 | 1253.9 | 420.0 | 8.0 | 14.71, 16.23 |
| fx-hir-ty | base-4c | 30 | 426.6 / 631.3 ± 101.0 | 823.3 | 428.7 | 18.7 | 14.71, 16.23 |
| fx-hir-ty | head-4c | 30 | 477.5 / 585.4 ± 82.6 | 851.9 | 398.7 | 19.1 | 14.71, 16.23 |
|  | head/base 1c | | min 1.165, mean 1.095 | ranges overlap | CPU 1.082 | | |
|  | head/base 4c | | min 1.119, mean 0.927 | ranges overlap | CPU 0.930 | | |
| fx-ide-db | base-1c | 30 | 184.9 / 278.4 ± 38.0 | 375.0 | 133.2 | 10.1 | 16.23, 16.75 |
| fx-ide-db | head-1c | 30 | 178.4 / 291.7 ± 63.3 | 432.1 | 129.8 | 10.1 | 16.23, 16.75 |
| fx-ide-db | base-4c | 30 | 174.7 / 235.4 ± 35.5 | 316.8 | 138.1 | 19.4 | 16.23, 16.75 |
| fx-ide-db | head-4c | 30 | 143.6 / 229.4 ± 30.1 | 281.5 | 132.0 | 19.2 | 16.23, 16.75 |
|  | head/base 1c | | min 0.965, mean 1.048 | ranges overlap | CPU 0.974 | | |
|  | head/base 4c | | min 0.822, mean 0.975 | ranges overlap | CPU 0.956 | | |

## Rejected attempts

- **`/proc/thread-self/schedstat` as the thread clock.** Its run time is updated at
  scheduler events, not at the read: per-file deltas came out 0 for files of 1.4 ms and 3.5
  ms for a 1.0 ms file. Replaced by `CLOCK_THREAD_CPUTIME_ID`, which brings the running
  thread's time up to date. The first collection (15 runs per crate) used it and is not
  reported; every figure in D1, D2 and D4 is from the final collection.
- **One shared lock for the trace's lines.** Workers took a mutex per file; under load a
  worker preempted while holding it stalled the others, and gaps of up to 16 ms appeared
  between files. Replaced by thread-local buffers moved to the shared one when a worker exits.
- **The CPU clock in the kept trace.** It needs an `unsafe` libc call (or a dependency) for a
  diagnostic; the kept trace has wall stamps only and the CPU clock lives in
  `trace_patch.py`.

## Follow-ups

1. **M5 (quiet host): the wall view with the kept timeline**, 30 runs per crate at four
   threads, to confirm D2's reconstruction where it can be measured (idle core time and tail
   per run) and to attribute D3's spread: correlate each run's wall with its in-process format
   window, its worker start latency, and (with the CPU-clock build) its on-CPU total. If the
   slow runs are slow in CPU, the spread is the hardware's; if in idle cores, the scheduler's.
2. **Oversubscription by split files (a hypothesis from D3, not a finding):** the crates with
   split files have the larger spread. Test on M5 before any queue: the same crates with the
   split made non-oversubscribing by the smallest change — the split file's worker formats
   units itself instead of waiting in `join` (spawn `threads − 1`), and/or item threads
   capped at the idle worker count. If that closes the spread on the split crates, the queue's
   real argument is variance, not the tail, and E23 can be reopened with that hypothesis.
3. **The process's fixed cost (D3's S, 14–67% of the gap):** 2.2 ms of exec, loader, runtime
   and exit for a one-file run, nearly all outside `main` (D4: 0.46 ms of a `tiny-one` run is
   inside `main`). Levers L11 (static linking) and L4 (discovery) are the
   ledger's; measure exec-to-`main` and `finish`-to-exit separately first (`LD_DEBUG=statistics`,
   `strace -c -f`).
4. **CPU inflation at four threads (M4: 1.07–1.29x; D5: minor faults 1.5–2.8x, system time up
   to 3x on the larger crates):** split it on M5 into the thread count's share (1c4t against
   1c1t: arenas, faults, thread start-up) and the hardware's (4c4t against 1c4t: shared L3,
   memory bandwidth). The allocator side has been tried: E9's `glibc.malloc.arena_max=1` and
   E14's allocators under the CLI's four-thread pattern found no wall below glibc's defaults;
   a remaining lever would be less memory traffic per file, which is the formatter's (E20,
   E25), not the scheduler's.
5. **`fx-ide-db`'s chain:** below-unit parallelism inside the 288 KB `const` unit of
   `lints.rs` (EXP-001's open item) is the only lever on that crate's critical path; the queue
   would take about 6% and the chain would remain.
