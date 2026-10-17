# EXP-029: the split file's own schedule — M5's tail, rebuilt and bounded

Ledger row E29 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md) (§3 as amended: both samples, a kept lever regresses
neither; §6 for wall clock on a shared host). The starting point is measurement T of
[../measurements/m5-quiet-host.md](../measurements/m5-quiet-host.md) (two crates whose tail
is owned by their one split file), the reconstruction and Follow-ups 2 and 5 of
[EXP-023](EXP-023-unit-queue.md), and the open item of [EXP-001](EXP-001-parallel-items.md)
(parallelism below the unit inside one large unit). Labels: **measured** (a tool's output on
the builds below), **reconstructed** (a schedule replayed from measured per-thread CPU times),
**read** (from source or an earlier record), **derived** (arithmetic on the above). Every wall
figure is **provisional** (protocol §6): other agents shared the host in every collection.
The quietest, Q2, ran `fx-hir` and `fx-ide-db` at a 1-minute load of 0.88–1.37, right after a
reboot of the host (2026-10-17), and the other seven crates at 1.37–6.28 as the other agents
resumed.

## Summary

- **M5's tail on `fx-hir` is mostly an artefact of the trace, not idle cores.** The release
  build's trace has no record of a split file's item threads, so its wall view counts a
  running split file as one busy core while its item threads run. On the same patched traces
  the idle core time in the tail is, with the item threads visible and without them: `fx-hir`
  2.2 against 10.6 core-ms (Q2), 4.0 against 13.0 (Q1); `fx-ide-db` 5.3 against 6.7 (Q2), 14.6
  against 19.2 (Q1) (**measured**). The release build reproduces M5's figures in both quiet
  windows (`fx-hir` tail 20.8% (Q2) and 22.3% (Q1) of the run against M5's 22.4%; `fx-ide-db`
  14.0% and 12.9% against 14.0%).
- **Rebuilt from on-CPU costs, today's schedule is within 0.8–3.2% of the work-conserving
  bound C on both crates** (median of per-run A/C over four collections of 30 runs; Q2:
  `fx-hir` 1.011, `fx-ide-db` 1.008) (**reconstructed**). On the quiet host the idle core
  time is 0.71 core-ms per run on `fx-hir` (0.79% of four cores) and 1.04 on `fx-ide-db`
  (1.10%); the loaded collections give 0.98–1.57 and 2.45–9.19. `fx-hir`'s tail is its
  split file's item threads finishing the units on all four cores; `fx-ide-db`'s, when it is
  long, is its split file's chain (the sequential part, then the 288 KB `const` unit) running
  past a quarter of the crate's work (7 of 30 Q2 runs, 14 of 30 Q1 runs).
- **Where the split file's sequential time goes (Q2 medians, on-CPU):** `hir/src/lib.rs`
  14.0 ms on its worker: the parse 13.0 ms (93%), the rest of the worker's time (reading,
  0.67 ms wall; the main visitor's set-up and unit list; the file's end; the comparison) 0.68
  ms, spawning and joining the item threads 0.23 ms, the stitch 0.11 ms; then 238 units, 18.7
  ms in all, the largest 2.95 ms. `lints.rs` 11.7 ms: the parse 10.2 ms (88%), the rest 0.93
  ms (reading 0.95 ms wall), spawn and join 0.21 ms, the stitch 0.23 ms; then 11 units, 15.2
  ms, the two `const` arrays 6.45 and 5.52 ms. The same shares hold in the loaded
  collections (parse 88–93%).
- **The bound each lever can reach (replays of the base's measured costs; Q2, then the range
  over all four collections):** (a) the worker formats units itself: no change, by
  construction; (b) item threads capped at the idle workers: with the cap fixed when the
  units phase starts, the split file runs alone (span +49.0% `fx-hir`, +23.6% `fx-ide-db`;
  +23.6…+49.1% over all); with a cap that follows the workers as they go idle, `fx-hir`'s
  tail −83% (−83…−93%, idle −12…−56%) with its span unchanged at the median (up to +71% in
  single runs), `fx-ide-db`'s tail +874% and span +9.8% (+7.8…+13.8%); (c) the two `const` arrays cut into 8 pieces at no cost:
  `fx-ide-db` tail −40% (−40…−87%), span −0.3% (−0.3…−5.8%), `fx-hir` nothing (no unit of
  100 KB); (d) the split file's parse on four threads at no cost: `fx-ide-db` tail −60%
  (−60…−91%), span −0.3% (−0.3…−6.9%), `fx-hir` tail −40% (−37…−60%) with idle +140% and span
  +1.1% (+1.0…+1.4%, up in 21–26 of 30 runs).
- **Lever (a) built and measured** (`0a6080c`): output identical to the base CLI (193 of 193
  comparisons). Its own traces, interleaved with the base's, reconstruct on the quiet host
  (Q2) `fx-hir`'s tail 1.28 → 1.54 ms and idle 0.71 → 1.71 core-ms, `fx-ide-db`'s tail 0.41 →
  0.21 ms and idle 1.04 → 0.48 (L2, loaded: 2.30 → 2.06 and 2.39 → 2.69); the wall-view tail
  rises on both crates in both builds (release Q2: 5.20 → 5.96 ms and 3.36 → 4.73 ms). What it
  measurably removes is one thread start per split file (the worker's spawn-and-join CPU 0.23
  → 0.16 ms on `fx-hir`, 0.21 → 0.16 ms on `fx-ide-db`). Not halved on `fx-hir` in any view:
  **reverted** (`1c1405e`) under the keep rule.
- **Lever (b) built and measured** (`67853b6`, the cap taken when a large file starts: its own
  worker, the threads no worker was started for, and the workers that have found the queue
  empty): the split file starts first, so it gets no helper and its units run on its worker.
  Interleaved with the base (L3, load 7.6–8.7): reconstructed span `fx-hir` 22.30 → 31.20 ms
  (+39.9%; the replay of the same collection's base costs predicted +49.4%), `fx-ide-db`
  20.79 → 24.48 ms (+17.8%; predicted +28.1%); tails 1.66 → 13.36 and 0.19 → 6.13 ms; release
  wall +6.2% and −0.1%. **Reverted** (`efd781c`). The measured build confirms the replay's
  direction and most of its size.
- **(c) and (d) not built:** (c) cannot change `fx-hir` (no unit of 100 KB) and (d) raises its
  span in the replay, so neither can pass the keep rule; on the quiet host both bound
  `fx-ide-db`'s span at −0.3%. Their place is a single large file run alone, where the replay
  gives `lints.rs` −13.3% (c) and −43.0% (d), and `hir/src/lib.rs` −51.5% (d) (Follow-up 2).
- **Decision: the hypothesis is refuted as stated.** On the quiet host the reconstructed
  tails are 0.71 and 1.04 idle core-ms per run: there is no tail of M5's size to halve, and no
  lever's replay halves both crates' tails without raising a span ((d) comes closest, −40% and
  −60%, and raises `fx-hir`'s span; (c) cannot touch `fx-hir`; (b) raises `fx-ide-db`'s span);
  the two built levers were measured and reverted. Kept:
  `benchmarks/schedule/split_sim.py` (the lever replays, `--blind`, `--compare`),
  `collect_ab.sh`, `identity_start.sh`, `trace_patch.py`'s `owner` variant and a README note on
  reading a tail from an unpatched trace. The branch head's Rust code is the base's (its CLI and `bench` are
  byte-identical to `03c61be`'s).

## Identity

| field | value |
|---|---|
| branch | `20261015-split-tail` from `03c61be` (integration branch `claude/chloro-perf-research-swarm-nrscj8`; code head `e96b530`, the best-known-good), worktree `/home/user/wt/split-tail` |
| commits | `0a6080c` (lever (a)), `1c1405e` (its revert, with the reason), `fc29026` (`split_sim.py`, `collect_ab.sh`, `trace_patch.py owner`, README), `a158b1d` (`split_sim.py`'s parts table for the lever (a) build), `53d50f4` (this record's first revision and the ledger row), `67853b6` (lever (b)), `efd781c` (its revert, with the reason), `4dcec09` (the record's lever (b) revision), `84824ec` (`identity_start.sh`), `2b1f6cb` (the gates) and this revision |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0`, valgrind 3.22.0, hyperfine 2.0.0; `cargo build --release -p chloro --features cli` and `cargo build --release -p chloro-core --examples` (fat LTO, one codegen unit) |
| host | 4 vCPU "Intel(R) Xeon(R) Processor @ 2.80GHz", 15 GiB; Linux 6.18.44-fc-v114 on 2026-10-15 (Q1, L1, L2), 6.18.44-fc-v139 after the reboot of 2026-10-17 (Q2). The on-CPU costs on the second are about 30% lower (the parse of `lib.rs` 13.0 against 18.4 ms): Q2 is compared with no other collection, only within itself |
| binaries (scratchpad `e29/bin/`) | `chloro-base`: `03c61be`'s CLI, sha256 `f48f3593…0bde00`, 2,196,120 B; `chloro-diag-base`: `03c61be` + `trace_patch.py base`, `e0813080…74fa0f4ea`, 2,207,992 B (rebuilt from the committed patch: the same sha256); `chloro-a`: `0a6080c`'s CLI, `5928cbdd…f2c6b29`, 2,193,072 B; `chloro-diag-a2`: `0a6080c` + the committed `trace_patch.py owner`, `e63db4b6…41c82b18`, used in Q2; `chloro-diag-a`: the same from a scratch copy of that patch with one more comment line (which moves source line numbers), `ee089c90…9c3537b`, 2,205,208 B, used in L2; `chloro-b`: `67853b6`'s CLI, `e3b2edfd…2883ef92`, 2,195,504 B; `chloro-diag-b`: `67853b6` + `trace_patch.py base`, `3e5c0c07…83700f0f`; `bench-base`: `03c61be`'s `bench`, `6816ff8a…a32fa7`, 1,882,072 B. The head's CLI and `bench` (rebuilt at `efd781c` after the reboot): `f48f3593…` and `6816ff8a…`, byte-identical to the base's |
| workloads | `/tmp/chloro-bench/formatted/<crate>/tree` (M4's and M5's copies). `fx-hir`: 15 files, `src/lib.rs` (236,066 B) split, `semantics.rs` (105,416 B) and `source_analyzer.rs` (74,746 B) below the 128 KiB threshold; `fx-ide-db`: 33 files, `src/generated/lints.rs` (639,771 B) split |

### Collections

All `--check --no-cache --threads 4`, unpinned, through `diag.py` (`posix_spawn`, `wait4`), 30
runs per crate and binary.

| name | when (UTC) | load (1 min) | what |
|---|---|---|---|
| Q2 | 2026-10-17 06:32:33–06:33:39 | `fx-hir` 0.88–1.05, `fx-ide-db` 1.05–1.37; the other seven 1.37–6.28 | `collect_ab.sh`, 5 blocks of 6 runs of `chloro-base`, `chloro-a`, `chloro-diag-base`, `chloro-diag-a2` in turn, nine crates (the first run of the session, `chloro-base` on `fx-hir`, took 541 ms: the binary's first start after the reboot) |
| Q1 | 2026-10-15 17:57:33–17:57:39 | 1.46–1.67 (5 min: 4.66) | `chloro-base` then `chloro-diag-base`, `fx-hir` and `fx-ide-db` |
| L1 | 2026-10-15 17:54:34–17:55:06 | 7.57–8.53 | `chloro-diag-base`, nine crates |
| L2 | 2026-10-15 18:03:53–18:05:07 | 4.63–7.56 | `collect_ab.sh`, 5 blocks of 6 runs of `chloro-diag-base` and `chloro-diag-a`, nine crates |
| L3 | 2026-10-17 06:47:49–06:49:33 | 7.6–8.7 (the other agents' callgrind runs) | `collect_ab.sh`, 5 blocks of 6 runs of `chloro-base`, `chloro-b`, `chloro-diag-base`, `chloro-diag-b` in turn, nine crates |

## Hypothesis and falsifiers

The tail of a crate whose last running file is a split file can be at least halved, with
byte-identical output, by changing how the split file's own work is scheduled: (a) its worker
formats units itself instead of waiting in `join`; (b) its item threads capped at the number
of idle workers; (c) its largest unit split below the item level where the unit is one item
whose body is a list of independent elements (`lints.rs`'s `const` arrays); (d) its parse run
in parallel; each measured separately, smallest first.

Falsifiers: no lever halves both tails (`fx-hir`, `fx-ide-db`) in the reconstruction; any
output difference at any thread count; CPU +5% or more at 4 threads on any crate; a
reconstructed gain not visible in the wall view's 30-run medians on a run at load ≤ 1.5.
Keep rule per lever: the reconstructed tail of both crates falls by at least half, no crate's
reconstructed four-core span rises, and the wall-view tail falls in the same direction.

## Method

- **The wall view** (`diag.py`, EXP-023's): work intervals are the files on their workers,
  less a split file's units phase, plus the item workers' intervals; the tail runs from the
  first worker's exit to the join, and its idle core time is 4 minus the open intervals over
  it. A release build's trace has no `item` lines (they come from `trace_patch.py`), so its
  wall view keeps a split file's whole span as one interval and sees none of its item
  workers. `split_sim.py --blind` recomputes the view of a patched trace without them.
- **The reconstruction** (`split_sim.py`): the measured on-CPU cost of every small file and,
  for each split file, of its worker's sequential part (the file's CPU less the stitch, and in
  lever (a)'s build less the worker's own units), each unit, the item workers' set-up and the
  stitch, replayed on four cores under processor sharing (every runnable thread runs at
  min(1, 4/runnable)); A is today's policy, the levers' policies are in the script's
  docstring; C = max(work/4, the longest chain). Tail: from the first worker finding nothing to
  take to the end; idle: core time with fewer than four runnable threads. Medians are of
  per-run values; a lever's span change is the median of its per-run ratio to A on the same
  costs; tail and idle changes are of the medians.
- **A lever's replay on the base's costs** isolates its schedule; the lever's own build,
  traced and replayed under its own policy beside the base's interleaved traces, adds what the
  change does to the costs.

### Commands

```sh
git -C /home/user/chloro worktree add /home/user/wt/split-tail -b 20261015-split-tail 03c61be
cd /home/user/wt/split-tail
cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples
cp target/release/chloro $E29/bin/chloro-base; cp target/release/examples/bench $E29/bin/bench-base
# the CPU-clock builds, in a scratch worktree, never committed
git -C /home/user/chloro worktree add --detach $E29/diag-wt 20261015-split-tail
python3 benchmarks/schedule/trace_patch.py $E29/diag-wt base      # at 03c61be
(cd $E29/diag-wt && cargo build --release -p chloro --features cli) && cp $E29/diag-wt/target/release/chloro $E29/bin/chloro-diag-base
(cd $E29/diag-wt && git checkout -- . && git checkout 0a6080c)
python3 benchmarks/schedule/trace_patch.py $E29/diag-wt owner     # lever (a)
(cd $E29/diag-wt && cargo build --release -p chloro --features cli) && cp $E29/diag-wt/target/release/chloro $E29/bin/chloro-diag-a2
(cd $E29/diag-wt && git checkout -- . && git checkout 03c61be)
# Q2 (L2 the same with two binaries); Q1 and L1 one binary at a time
benchmarks/schedule/collect_ab.sh $E29/q2 5 6 rbase=$E29/bin/chloro-base ra=$E29/bin/chloro-a \
    base=$E29/bin/chloro-diag-base a=$E29/bin/chloro-diag-a2
E23_MODE=check-nocache python3 -I benchmarks/schedule/diag.py $BIN /tmp/chloro-bench/formatted/$CRATE/tree 30 4 $OUT/$CRATE
# reading them
python3 benchmarks/schedule/split_sim.py [--runs] $E29/q2/base/$CRATE.b*.trace*.tsv   # parts, every lever's replay
python3 benchmarks/schedule/split_sim.py --blind $E29/q2/base/$CRATE.b*.trace*.tsv    # the wall view without item threads
python3 benchmarks/schedule/split_sim.py --compare $E29/q2 base:A a:a                 # each build under its own policy
```

`$E29` is the session scratchpad's `e29/`; `/home/user/chloro` is only read (its
`benchmarks/results/m5-t-trace/` holds M5's traces).

## Step 1: the tail, measured and rebuilt

### The wall view, as M5 read it and with the item threads (measured, provisional)

Medians of 30 runs; ms; the idle time in the tail in core-ms. "Owner split": runs whose last
file running at the first worker's exit is the split file.

| crate | build | window, load | wall | CPU | T | tail | tail / T | idle in tail, as the trace sees it | the same traces without item threads (`--blind`) | owner split |
|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | M5 `head` (release) | M5, 1.04 | 37.4 | 121.8 | 33.1 | 7.14 | 0.224 | 19.0 | (no item records) | — |
| fx-hir | `chloro-base` (release) | Q2, 0.88–1.05 | 32.1 | 102.5 | 28.3 | 5.20 | 0.208 | 14.0 | (no item records) | 20/30 |
| fx-hir | `chloro-diag-base` | Q2, 0.88–1.05 | 28.8 | 94.6 | 25.5 | 3.74 | 0.150 | **2.2** | **10.6** | 29/30 |
| fx-hir | `chloro-base` (release) | Q1, 1.46 | 45.9 | 142.6 | 41.9 | 9.05 | 0.223 | 22.4 | (no item records) | 20/30 |
| fx-hir | `chloro-diag-base` | Q1, 1.67 | 42.4 | 133.1 | 37.2 | 5.47 | 0.158 | **4.0** | **13.0** | 18/30 |
| fx-hir | `chloro-diag-base` | L1, 7.7 | 93.8 | 127.5 | 87.8 | 9.73 | 0.126 | 9.9 | 27.1 | — |
| fx-ide-db | M5 `head` (release) | M5, 1.20 | 39.6 | 125.5 | 36.1 | 5.07 | 0.140 | 14.7 | (no item records) | — |
| fx-ide-db | `chloro-base` (release) | Q2, 1.05–1.37 | 28.8 | 88.8 | 25.3 | 3.36 | 0.140 | 10.0 | (no item records) | 27/30 |
| fx-ide-db | `chloro-diag-base` | Q2, 1.37 | 30.7 | 95.1 | 27.6 | 2.28 | 0.070 | **5.3** | **6.7** | 22/30 |
| fx-ide-db | `chloro-base` (release) | Q1, 1.67 | 40.3 | 121.8 | 35.3 | 4.51 | 0.129 | 13.2 | (no item records) | 23/30 |
| fx-ide-db | `chloro-diag-base` | Q1, 1.67 | 40.1 | 127.5 | 36.1 | 6.37 | 0.179 | **14.6** | **19.2** | 24/30 |
| fx-ide-db | `chloro-diag-base` | L1, 8.0 | 83.2 | 116.2 | 77.3 | 4.10 | 0.047 | 11.9 | 13.6 | — |

- The release build reproduces M5's shape in both quiet windows (the tail's share of the run
  0.208 and 0.223 against 0.224 on `fx-hir`; 0.140 and 0.129 against 0.140 on `fx-ide-db`).
- On the same patched traces, 79% (Q2: 8.4 of 10.6 core-ms) and 69% (Q1: 9.0 of 13.0) of
  `fx-hir`'s idle core time in the tail is its split file's item threads, running: the wall
  view could not see them. On `fx-ide-db` it is 21% (Q2: 1.4 of 6.7) and 24% (Q1: 4.6 of
  19.2): the rest is real, the 288 KB unit running alone.
- The tail itself (first worker's exit to the join) is the split file's units phase on both
  crates; whether cores are idle in it is what the item records decide.

### The split file's parts (measured on-CPU, patched base)

Medians over 30 runs (min–max); ms. "Rest of the worker": the file worker's CPU less the parse,
the spawn and join, and the stitch: the read, the main visitor's set-up and `unit_ranges`, the
file's end after the stitch, the comparison with the input and the frees; its read is shown
by its wall time. "Sequential (sum)": the file worker's CPU, everything but the units.

| crate | collection | load | runs | read (wall) | parse | rest of the worker | spawn and join | stitch | sequential (sum) | units: n, total | largest unit | second | item set-up |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | Q2 | 0.9–1.1 | 30 | 0.67 (0.40–8.30) | 13.00 (11.53–38.73) | 0.68 (0.55–0.92) | 0.23 (0.17–0.36) | 0.11 (0.08–0.16) | 14.03 (12.44–39.65) | 238, 18.70 (17.45–22.97) | 2.95 (2.71–4.73) | 1.78 (1.65–2.51) | 0.26 (0.18–0.39) |
| fx-hir | Q1 | 1.7 | 30 | 0.73 (0.43–4.23) | 18.39 (16.66–31.45) | 0.95 (0.72–1.65) | 0.29 (0.18–3.17) | 0.14 (0.10–0.23) | 19.72 (17.74–33.96) | 238, 23.69 (20.38–33.20) | 3.55 (3.06–4.67) | 2.37 (1.81–3.54) | 0.41 (0.29–0.74) |
| fx-hir | L1 | 7.6–7.7 | 30 | 0.31 (0.22–9.55) | 18.44 (16.48–27.47) | 0.87 (0.71–1.18) | 0.32 (0.20–0.46) | 0.17 (0.12–0.29) | 19.82 (17.78–29.01) | 238, 24.53 (21.66–31.09) | 3.77 (3.26–6.12) | 2.32 (1.87–3.94) | 0.46 (0.34–0.65) |
| fx-hir | L2 | 4.6–5.1 | 30 | 0.32 (0.23–20.37) | 20.58 (16.10–32.59) | 1.02 (0.74–1.92) | 0.40 (0.20–0.69) | 0.21 (0.13–0.48) | 22.05 (17.58–35.38) | 238, 28.10 (22.46–40.88) | 4.57 (3.44–7.91) | 2.48 (2.01–4.43) | 0.48 (0.31–0.76) |
| fx-ide-db | Q2 | 1.4 | 30 | 0.95 (0.69–3.12) | 10.20 (8.91–38.77) | 0.93 (0.71–1.42) | 0.21 (0.14–0.56) | 0.23 (0.17–0.72) | 11.65 (10.10–40.74) | 11, 15.20 (13.34–30.85) | 6.45 (5.28–20.93) | 5.52 (5.01–8.06) | 0.05 (0.04–0.17) |
| fx-ide-db | Q1 | 1.7 | 30 | 1.42 (0.95–5.20) | 15.26 (12.72–26.28) | 1.40 (0.94–2.33) | 0.27 (0.17–0.48) | 0.39 (0.26–0.75) | 17.69 (14.73–28.72) | 11, 20.92 (16.26–32.88) | 9.35 (6.16–14.38) | 7.23 (6.16–13.12) | 0.09 (0.07–0.16) |
| fx-ide-db | L1 | 8.0 | 30 | 0.75 (0.60–15.08) | 14.60 (13.19–26.31) | 1.08 (0.89–1.65) | 0.33 (0.22–0.63) | 0.38 (0.26–0.73) | 16.52 (14.88–29.11) | 11, 18.97 (17.41–30.00) | 8.11 (7.32–16.11) | 7.52 (6.73–11.83) | 0.10 (0.07–0.15) |
| fx-ide-db | L2 | 5.1–5.2 | 30 | 0.83 (0.56–28.33) | 20.23 (12.04–26.83) | 1.32 (0.86–2.00) | 0.31 (0.22–0.60) | 0.54 (0.29–0.94) | 22.00 (14.36–29.79) | 11, 26.03 (17.81–38.16) | 12.56 (6.97–17.97) | 8.82 (6.62–13.69) | 0.12 (0.09–0.18) |

- The parse is 88–93% of the split file's sequential time on both crates in every
  collection; reading, the set-up around the item walk, spawning and joining, and the stitch
  together are 1.0–2.4 ms.
- The units phase is already parallel: four item threads take the units longest first while
  the other three workers format the crate's other files. The parse is already in parallel
  with the other files' formatting: the split file starts first and its worker parses while
  the other three format.
- The costs move with the host and the load (the parse of `lints.rs` 10.2 ms in Q2, 15.3 ms
  in Q1, 20.2 ms in L2; single runs to 38.8 ms): shared caches and memory bandwidth, not
  waiting for a core, which the thread clock excludes. Each run is replayed with its own costs.

### Today's schedule against the bound (reconstructed)

Medians of per-run values; ms; idle in core-ms and as a share of 4 × span.

| crate | collection | work/4 | chain | C | A (today) | A / C, median (min–max) | A − C | tail | idle (share) | runs with chain > work/4 |
|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | Q2 | 22.21 | 17.34 | 22.21 | 22.62 | 1.011 (1.006–1.063) | 0.25 | 1.28 | 0.71 (0.79%) | 2 |
| fx-hir | Q1 | 31.33 | 28.03 | 32.53 | 33.20 | 1.015 (1.005–1.095) | 0.52 | 3.50 | 1.57 (1.17%) | 4 |
| fx-hir | L1 | 30.19 | 24.32 | 30.41 | 31.54 | 1.012 (1.008–1.132) | 0.38 | 3.19 | 0.98 (0.79%) | 3 |
| fx-hir | L2 | 33.73 | 28.75 | 33.73 | 35.22 | 1.011 (1.007–1.130) | 0.40 | 2.30 | 1.02 (0.68%) | 1 |
| fx-ide-db | Q2 | 22.35 | 19.53 | 22.96 | 23.95 | 1.008 (1.000–1.095) | 0.17 | 0.41 | 1.04 (1.10%) | 7 |
| fx-ide-db | Q1 | 30.03 | 27.96 | 31.08 | 31.87 | 1.012 (1.001–1.103) | 0.38 | 3.46 | 9.19 (7.46%) | 14 |
| fx-ide-db | L1 | 27.42 | 25.15 | 27.53 | 29.20 | 1.024 (1.002–1.102) | 0.63 | 0.91 | 2.45 (2.24%) | 3 |
| fx-ide-db | L2 | 33.71 | 32.58 | 35.44 | 36.87 | 1.032 (1.001–1.109) | 1.18 | 2.39 | 6.50 (3.88%) | 10 |

- `fx-hir`: A is 1.1–1.5% above C and 0.7–1.2% of the four cores is idle. Its tail (1.3–3.5 ms)
  is the end of the units phase, in which the item threads keep the cores busy; what is idle
  is the last unit or two and the stitch (0.7–1.6 core-ms). M5's 19.0 core-ms was mostly the
  item threads' time, unseen.
- `fx-ide-db`: A is 0.8–3.2% above C, and C is its split file's chain in the runs where that
  chain is longer than work/4 (7 of 30 in Q2, 14 of 30 in Q1). Idle is 1.1% of capacity on the
  quiet host and 2.2–7.5% in the other collections, nearly all of it while the 288 KB unit runs
  alone at the end. Only a shorter chain recovers it.
- The other seven crates (Q2, L1 and L2, median of per-run A/C): 1.0006–1.0112 (`fx-parser`
  the highest); idle 0.05–1.50 core-ms; the replay ends on a small file in every run but one
  (`reg-regex-syntax`, Q2), including `fx-hir-ty` (one split file), `reg-regex-syntax` (four)
  and `reg-clap-builder` (two).

### The bound of each lever (reconstructed: replays of the base's costs)

Changes against A: span (median of per-run ratios) / tail / idle (of the medians).

| crate | collection | A span / tail / idle (ms, ms, core-ms) | (a) | (b) static cap | (b) dynamic cap | (a) + idle workers join | (c): `const` arrays in 8 | (d): parse on 4 threads | (c) + (d) |
|---|---|---|---|---|---|---|---|---|---|
| fx-hir | Q2 | 22.62 / 1.28 / 0.71 | +0.0% / +0% / +0% | +49.0% / +1070% / +6129% | −0.0% / −83% / −12% | +0.1% / −77% / −3% | +0.0% / +0% / +0% | +1.1% / −40% / +140% | +1.1% / −40% / +140% |
| fx-hir | Q1 | 33.20 / 3.50 / 1.57 | +0.0% / +0% / +0% | +30.2% / +389% / +2846% | −0.0% / −93% / −56% | +0.1% / −48% / +15% | +0.0% / +0% / +0% | +1.4% / −51% / +131% | +1.4% / −51% / +131% |
| fx-hir | L1 | 31.54 / 3.19 / 0.98 | +0.0% / +0% / +0% | +49.1% / +537% / +5984% | −0.0% / −92% / −27% | +0.2% / −84% / +27% | +0.0% / +0% / +0% | +1.3% / −60% / +178% | +1.3% / −60% / +178% |
| fx-hir | L2 | 35.22 / 2.30 / 1.02 | +0.0% / +0% / +0% | +43.0% / +809% / +5942% | −0.0% / −88% / −21% | +0.0% / −74% / +4% | +0.0% / +0% / +0% | +1.0% / −37% / +187% | +1.0% / −37% / +187% |
| fx-ide-db | Q2 | 23.95 / 0.41 / 1.04 | +0.0% / +0% / +0% | +23.6% / +1746% / +1921% | +9.8% / +874% / +836% | +0.0% / +0% / +0% | −0.3% / −40% / −49% | −0.3% / −60% / −69% | −0.2% / −52% / −60% |
| fx-ide-db | Q1 | 31.87 / 3.46 / 9.19 | +0.0% / +0% / +0% | +26.4% / +274% / +279% | +7.8% / +113% / +100% | +0.0% / +0% / +4% | −5.8% / −85% / −86% | −6.9% / −91% / −91% | −6.6% / −89% / −91% |
| fx-ide-db | L1 | 29.20 / 0.91 / 2.45 | +0.0% / +0% / +0% | +29.5% / +1201% / +1331% | +13.8% / +632% / +692% | +0.0% / +0% / +0% | −2.1% / −73% / −79% | −1.9% / −76% / −82% | −1.8% / −77% / −83% |
| fx-ide-db | L2 | 36.87 / 2.39 / 6.50 | +0.0% / +0% / +0% | +29.2% / +578% / +623% | +10.6% / +251% / +254% | +0.0% / +0% / +0% | −3.6% / −87% / −89% | −3.7% / −89% / −92% | −3.6% / −88% / −92% |

Per-run spread of the span change (min…max; runs of 30 in which the span rises):

| crate | collection | (b) dynamic | (a) + idle workers join | (c) | (d) |
|---|---|---|---|---|---|
| fx-hir | Q2 | −2.5…+46.4%, 3 | −0.1…+2.2%, 14 | 0, 0 | −36.5…+9.0%, 21 |
| fx-hir | Q1 | −8.7…+55.3%, 6 | +0.0…+3.7%, 15 | 0, 0 | −18.1…+20.1%, 25 |
| fx-hir | L1 | −11.6…+65.8%, 5 | +0.0…+1.4%, 18 | 0, 0 | −6.5…+12.1%, 26 |
| fx-hir | L2 | −7.0…+71.3%, 3 | −0.0…+2.0%, 14 | 0, 0 | −10.1…+13.9%, 23 |
| fx-ide-db | Q2 | −4.7…+32.8%, 26 | 0, 0 | −29.0…+0.6%, 6 | −37.5…+0.7%, 5 |
| fx-ide-db | Q1 | −3.4…+41.0%, 26 | +0.0…+0.7%, 5 | −18.0…+0.6%, 4 | −22.6…+0.7%, 6 |
| fx-ide-db | L1 | +0.8…+19.3%, 30 | 0, 0 | −12.1…+0.3%, 4 | −12.3…+0.4%, 3 |
| fx-ide-db | L2 | −7.1…+34.0%, 28 | +0.0…+0.5%, 1 | −16.2…+0.4%, 2 | −20.0…+0.4%, 5 |

The other seven crates in Q2: no lever's median span change exceeds 0.2% (the dynamic cap
−0.19% on `reg-regex-syntax`, −0.17% on `reg-clap-builder`; (c) and (d) within ±0.02%).

- **(a)** changes nothing in the replay, by construction: during a split file's units phase
  the runnable threads are the three other workers and four item threads either way (the
  sleeping worker is not runnable), so every thread runs at the same rate. What it saves is
  one thread start per split file, below the replay's resolution; the measurement is below.
- **(b) with the cap fixed when the units phase starts** gives the split file no helper: it is
  the largest file and starts first, and no worker is idle when its parse ends in any run of
  either crate, so its 238 or 11 units run on its own worker. **A cap that follows the
  workers** (a worker that finds the file queue empty takes units; no item threads) never
  oversubscribes and ends `fx-hir`'s tail metric, but it starts the split file's units late:
  when the parse runs long the units run alone (`fx-hir` up to +71% in single runs), and on
  `fx-ide-db` the second `const` array waits for a free worker (+7.8…+13.8% at the median).
- **(a) with idle workers joining** (no cap) halves `fx-hir`'s tail metric in three of four
  collections without changing its idle time or span: the workers that would have exited take
  units instead of the item threads, and the end is the same.
- **(c)** is the lever on `fx-ide-db`'s chain: with both arrays cut, its span reaches work/4
  in every collection (C_c8 = work/4). On the quiet host that is −0.3% (the chain is longer
  than work/4 in 7 runs of 30); under load −2.1…−5.8%. It cannot touch `fx-hir`, whose largest
  unit is a 2.95 ms `impl` (16% of its units).
- **(d)** shortens the same chain from the other end (the parse, 88–93% of the sequential
  part) and reaches the same span on `fx-ide-db`; on `fx-hir` three parse threads beside three
  busy workers delay the other files and the units, and the span rises in 21–26 of 30 runs.
  (c) + (d) is no better than either alone on `fx-ide-db` (the span is at work/4).

Bound per lever, as the hypothesis asks: on the quiet host no lever halves `fx-ide-db`'s tail
except (d) (−60%), which raises `fx-hir`'s span; (c) reaches −40%; for `fx-hir` the tail metric
halves only under schedules that leave its idle time and span where they are ((b) dynamic,
(a) + join) or raise its span ((d)), because its tail is busy.

## Lever (a): the split file's worker formats units (`0a6080c`, reverted in `1c1405e`)

`format_units` (`chloro-core/src/formatter/parallel.rs`) spawned `threads` item threads and
waited in `join`; it now spawns `threads − 1` and runs the same worker closure on the calling
thread before joining (14 lines). Units, order and stitch are unchanged.

Output: `benchmarks/schedule/identity.sh chloro-base chloro-a` (the CLI against `03c61be`'s
at 1, 2 and 4 threads on the nine crates, the nine raw crates and the fixtures): **0 of 193
comparisons differ** (**measured**).

### Q2 (quiet host for these two crates), medians of 30 runs

| crate | view | base | lever (a) | change |
|---|---|---|---|---|
| fx-hir | reconstructed span (A for the base, a for the lever) | 22.62 | 23.29 | +3.0% |
| fx-hir | reconstructed tail / idle | 1.28 / 0.71 | 1.54 / 1.71 | +20% / +141% |
| fx-hir | spawn and join on the split file's worker (on-CPU) | 0.23 | 0.16 | −0.07 ms |
| fx-hir | patched build: wall / CPU / wall tail / idle in tail | 28.8 / 94.6 / 3.74 / 2.22 | 29.6 / 95.7 / 5.33 / 2.42 | +2.8% / +1.2% / +43% / +9% |
| fx-hir | release build: wall / CPU / wall tail | 32.1 / 102.5 / 5.20 | 29.8 / 93.5 / 5.96 | −7.0% / −8.8% / +15% |
| fx-ide-db | reconstructed span | 23.95 | 21.36 | −10.8% |
| fx-ide-db | reconstructed tail / idle | 0.41 / 1.04 | 0.21 / 0.48 | −48% / −54% |
| fx-ide-db | spawn and join on the split file's worker | 0.21 | 0.16 | −0.05 ms |
| fx-ide-db | patched build: wall / CPU / wall tail / idle in tail | 30.7 / 95.1 / 2.28 / 5.26 | 29.4 / 88.3 / 3.42 / 8.46 | −4.2% / −7.2% / +50% / +61% |
| fx-ide-db | release build: wall / CPU / wall tail | 28.8 / 88.8 / 3.36 | 30.0 / 92.7 / 4.73 | +3.9% / +4.4% / +41% |

### L2 (loaded), medians of 30 runs per crate, patched builds

| crate | base A span | lever a span | change | base tail | lever tail | base idle | lever idle | base CPU | lever CPU | base wall | lever wall | base wall tail | lever wall tail | load |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 35.23 | 34.89 | −1.0% | 2.30 | 2.06 | 1.02 | 2.40 | 141.8 | 137.6 (−2.9%) | 125.1 | 115.3 | 12.98 | 19.98 | 4.6–5.1 |
| fx-ide-db | 36.87 | 34.53 | −6.4% | 2.39 | 2.69 | 6.50 | 7.61 | 142.4 | 131.3 (−7.8%) | 116.0 | 116.9 | 4.96 | 9.45 | 5.1–5.2 |
| fx-hir-ty | 100.89 | 99.16 | −1.7% | 0.08 | 0.06 | 0.17 | 0.12 | 413.0 | 405.7 (−1.8%) | 317.8 | 320.7 | 5.30 | 4.09 | 5.2–7.0 |
| fx-syntax | 17.86 | 17.91 | +0.3% | 0.12 | 0.11 | 0.22 | 0.21 | 76.4 | 75.9 (−0.6%) | 72.9 | 77.3 | 6.39 | 7.53 | 7.0–7.4 |
| fx-parser | 9.73 | 10.17 | +4.5% | 0.22 | 0.21 | 0.45 | 0.45 | 42.7 | 45.1 (+5.5%) | 41.9 | 41.5 | 7.29 | 7.36 | 7.4 |
| reg-tokio | 6.25 | 6.67 | +6.7% | 0.03 | 0.03 | 0.07 | 0.06 | 29.7 | 32.0 (+7.7%) | 24.0 | 24.5 | 3.55 | 3.76 | 7.4 |
| reg-regex-syntax | 77.45 | 75.91 | −2.0% | 0.54 | 0.27 | 1.50 | 0.64 | 319.9 | 311.8 (−2.5%) | 188.8 | 184.5 | 4.74 | 4.55 | 7.4–7.5 |
| reg-clap-builder | 28.64 | 32.63 | +13.9% | 0.04 | 0.05 | 0.10 | 0.11 | 123.1 | 138.3 (+12.3%) | 81.3 | 90.6 | 3.45 | 4.06 | 7.5–7.6 |
| reg-winnow | 29.67 | 28.78 | −3.0% | 0.11 | 0.12 | 0.26 | 0.27 | 123.5 | 120.0 (−2.8%) | 77.3 | 86.4 | 2.98 | 4.10 | 7.4–7.6 |

- **What (a) measured.** The one effect resolved in every collection is the thread start it
  removes: the split file's worker spends 0.05–0.07 ms less in spawning and joining (Q2). The
  tails do not follow: on `fx-hir` the reconstructed tail rises in Q2 (+20%) and falls 10% in
  L2; on `fx-ide-db` it falls 48% in Q2 and rises 13% in L2; the wall-view tail rises on both
  crates in both builds on the quiet host (+15…+50%). The span and CPU differences between
  the two builds (Q2: −10.8…+3.0% span, −8.8…+4.4% CPU, with opposite signs in the patched and
  release builds of the same crate; L2: −6.4…+13.9%, as large on the four crates with no split
  file, where the lever changes no instruction that runs) are the costs varying between
  blocks a minute apart, not the lever: the replay of equal costs says its schedule changes
  nothing.
- **Keep rule:** the tail falls by half on neither crate in L2 and on one of two in Q2, and the
  wall-view tail rises: **reverted** (`1c1405e`).
- Per-run rows: appendix B (Q2) and C (L2).

## Lever (b): item threads capped at the free cores (`67853b6`, reverted in `efd781c`)

`format_all` (`chloro/src/cli/orchestrate.rs`) gave a file of 128 KiB or more `--threads`
item threads whatever the other workers were doing. With the lever it gets at most its own
worker, the threads no worker was started for (a run with fewer files than threads, so a
single large file keeps its four), and the workers that have found the file queue empty, the
count taken when the file starts (9 lines). A file given one thread takes the sequential path,
as a file below the threshold does. The cap is taken at the file's start, not when its units
are ready: the replay's static cap (`bs`, taken when the units phase starts) found no idle
worker at that moment in any run of either crate either, so the two give the same schedule
here.

Output: `identity.sh chloro-base chloro-b`: **0 of 193 comparisons differ** (the same 1, 2 and 4 threads, nine crates, nine raw crates, the fixtures; **measured**).

### L3 (loaded, provisional), medians of 30 runs per crate and build

The lever build's split file is formatted on one thread, so its traces have no split file and
are replayed under A; the prediction is the replay of the static cap on the base's costs from
the same collection.

| crate | replay of the cap on the base's costs: span (change), tail, idle | base build: span, tail, idle | lever build: span (change), tail, idle | patched builds: wall, CPU | release builds: wall, CPU, wall tail | load |
|---|---|---|---|---|---|---|
| fx-hir | 33.51 (+49.4%), 15.21, 44.46 | 22.30, 1.66, 0.53 | 31.20 (+39.9%), 13.36, 39.13 | 51.5 → 57.5 (+11.7%), 93.6 → 90.2 (−3.6%) | 57.3 → 60.9 (+6.2%), 101.1 → 98.5 (−2.5%), 11.12 → 16.11 | 8.0–8.7 |
| fx-ide-db | 26.33 (+28.1%), 8.21, 24.42 | 20.79, 0.19, 0.38 | 24.48 (+17.8%), 6.13, 18.22 | 49.0 → 49.7 (+1.5%), 87.3 → 85.0 (−2.7%) | 48.9 → 48.9 (−0.1%), 86.9 → 85.7 (−1.4%), 2.67 → 5.57 | 8.0–8.4 |
| fx-hir-ty | 69.86 (−0.0%), 0.04, 0.09 | 69.87, 0.03, 0.08 | 69.84 (−0.0%), 0.04, 0.10 | 157.4 → 159.6, 286.5 → 284.8 (−0.6%) | 156.4 → 162.2, 287.5 → 295.4 (+2.8%), 1.11 → 1.23 | 7.8–8.4 |
| reg-regex-syntax | 48.12 (−0.3%), 0.11, 0.22 | 48.25, 0.15, 0.40 | 47.98 (−0.6%), 0.12, 0.29 | 206.1 → 220.3, 200.4 → 197.4 (−1.5%) | 199.7 → 215.8, 195.6 → 187.7 (−4.0%), 7.44 → 5.72 | 7.9–8.5 |
| reg-clap-builder | 20.02 (−0.1%), 0.03, 0.06 | 20.06, 0.03, 0.07 | 19.63 (−2.1%), 0.05, 0.10 | 66.9 → 84.3, 86.2 → 83.1 (−3.6%) | 67.2 → 74.4, 84.8 → 81.1 (−4.3%), 3.65 → 5.51 | 8.5–8.6 |

The four crates without a split file (the lever changes nothing that runs there), release
builds: wall −5.9…+4.9%, CPU −4.2…+2.4% (`fx-syntax`, `fx-parser`, `reg-tokio`, `reg-winnow`):
the noise of this window.

- The split file without helpers runs its whole units phase on one core while the other
  workers finish the crate: `fx-hir`'s tail grows 8x and its idle core time 74x, `fx-ide-db`'s
  32x and 48x. The measured spans rise by 40% and 18% where the replay said 49% and 28%: the
  sequential path costs less than the split one (no item set-up, no stitch, no extra red
  trees), which the replay of the base's costs does not know. The sign and most of the size
  are the replay's.
- CPU falls 1.4–3.6% on the two crates with fewer threads, the one gain the cap was for, and
  it does not pay for the lost parallelism.
- **Keep rule:** the tails grow on both crates and two spans rise: **reverted** (`efd781c`).
- Per-run rows: appendix D.

## Levers (c) and (d): not built

Both falsifiers that bind them hold on the replays: no lever halves both tails, and (c) and
(d) cannot pass the keep rule on these crates. (c) leaves `fx-hir` unchanged by construction
(its largest unit is a 2.95 ms `impl`, no unit reaches 100 KB), so its tail cannot fall by
half; (d) raises `fx-hir`'s reconstructed span in 21–26 of 30 runs in every collection. On the
quiet host both bound `fx-ide-db`'s span at −0.3%. What each would need, for a later experiment
whose target is a large file's chain (Follow-up 2):

- **(c)** cuts a `const` whose initialiser is an array of independent elements
  (`DEFAULT_LINTS`, `CLIPPY_LINTS` in `lints.rs`) into element ranges formatted on several
  threads and assembled by the array's list formatting. Exactness has to hold through
  rustfmt's overflow and tactic logic (`overflow.rs`, `lists.rs`: the tactic depends on every
  element's rewrite, and the last element may be tried at another shape), so the unit-record
  checks of `parallel.rs` do not carry over as they are: the element rewrites would be computed
  in workers and looked up by (element, shape) inside the sequential list formatting, with the
  sequential rewrite as the fallback for any shape not computed and for any element whose
  rewrite touched the run's shared state (skipped ranges, the macro and comment flags).
  Bound on the crates: `fx-ide-db` span −0.3% on the quiet host (−2.1…−5.8% under load), tail
  −40% (−73…−87%), nothing on the other eight.
- **(d)** is P9 of [../../2026-10-15-extensions-theory/parser-and-substrate.md](../../2026-10-15-extensions-theory/parser-and-substrate.md)
  §2 (300–500 lines, the element-equality harness). Bound on the crates: `fx-ide-db` span
  −0.3% on the quiet host (−1.9…−6.9% under load), tail −60% (−76…−91%); `fx-hir` span +1.1%
  (+1.0…+1.4%): three parse threads beside three busy workers delay everything else; it would
  need a rule that parses in parallel only when workers would otherwise idle.
- **Where they would pay: one large file alone.** Replayed with only the split file (Q2's
  costs, four threads, no other file, process start-up excluded): `lints.rs` 19.54 ms today,
  15.97 with (c) (−13.3%), 11.15 with (d) (−43.0%), 8.37 with both (−57.0%); `hir/src/lib.rs`
  18.81 ms today, 9.12 with (d) (−51.5%), unchanged by (c). This is the stdin and editor case
  (M5's measurement E: the split takes `stdin-lints` only to 0.857x of one thread), not the
  crate's tail.

## Gates

All on the head `efd781c` (its `chloro-core` and `chloro/src` are `03c61be`'s byte for byte:
`git diff 03c61be efd781c -- chloro chloro-core` is empty), run on 2026-10-17 after the
reboot; the release CLI and `bench` rebuilt there have the base's sha256 (`f48f3593…0bde00`,
`6816ff8a…a32fa7`). Each built lever was checked for output before it was measured.

| gate | command | result |
|---|---|---|
| format | `cargo fmt --all --check` | clean |
| lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| tests | `cargo test --workspace --all-features` | 308 passed, 0 failed: the CLI's 24 unit tests, `cli_cache` 11, `cli_files` 5, `cli_trace` 3, `cli_unit_cache` 3; `chloro-core`'s 208, `basic` 4, conformance 19, `dead_candidates` 5, `nodes` 10, `parallel_equivalence` 1, `rustfmt_version` 10, `unit_cache_equivalence` 1, `width_bound` 3 (also 308 at `fc29026`) |
| goldens, fixtures | `target/release/examples/bench -n 1 --check $GOLDEN/fixtures-e8d87c2.tsv --root /home/user/chloro/chloro-core/tests/conformance/fixtures`, and with `--threads 4` | 0 of 1238 outputs changed; at 4 threads also 0 of 1238 differ from `format_source` |
| goldens, registry | `target/release/examples/bench -n 1 --check $GOLDEN/registry-e8d87c2.tsv --root /root/.cargo/registry/src` | 0 of 25,077 outputs changed (20,850 recorded, 4,227 present and not recorded, reported and not counted) |
| conformance | `target/release/examples/conform -i`; `conform -i -V 1.8.0` | 1220/1220 identical, 0 not idempotent, at 1.9.0 and at 1.8.0 |
| CLI identity, the base | `benchmarks/schedule/identity.sh $E29/bin/chloro-base target/release/chloro $OUT` | 0 of 193 comparisons differ (and 0 of 193 for `chloro-a` and for `chloro-b`) |
| CLI identity, the start | `identity.sh` cannot run against `/home/user/wt/base/target/release/chloro` (`884f9c9`, sha256 `26d25ca9…610c1b7c`): that CLI has no `--threads` and no `--no-cache` and exits 1 with "Could not parse CLI arguments" on every comparison. Run instead: `benchmarks/schedule/identity_start.sh /home/user/wt/base/target/release/chloro target/release/chloro $OUT` (the start's `--write` against the head's `--write --no-cache --threads N`, N = 1, 2, 4, on the nine formatted and raw crates and the fixtures, written trees compared) | 57 tree comparisons, 3 differing lines, 1 file: `rust-analyzer/parser/test_data/parser/err/0024_many_type_parens.rs` at each thread count, the R2 fixture fix; the 12 registry files of R2 are not in these trees |
| instructions, fixture sample | `benchmarks/profile.sh --core 3 --runs 2 --per-file --bench $E29/bin/bench-head` | 220,957,335 and 220,957,563 (min 220,957,335); per file 236,243,340 over 49 processes, empty process 316,072, net 220,755,812; cross-file term −114,549. `bench-head` is byte-identical to `bench-base`, so the base's count is the same binary's; the record of the BKG (`e96b530`): 220,958,649–220,958,706, per file 220,755,045 net |
| instructions, registry sample | `benchmarks/profile.sh --core 3 --runs 2 --per-file --bench $E29/bin/bench-head --sample-list benchmarks/samples/registry49.txt` | 299,933,904 and 299,933,794 (min 299,933,794); per file 314,321,965, net 298,834,437; cross-file term +783,285; the BKG's: 299,933,504, per file 298,833,058 net |
| CPU at 4 threads | `hyperfine -N -i -w 2 -r 10 --metrics time_user,time_system`, base and head in both orders, nine crates (load 9.1–9.5, provisional) | head/base CPU 0.886–1.181 for two byte-identical binaries (below): the noise floor of this window; the CPU is the base's by construction |

The fixture count is 246 above M5's for the same code (220,957,089, `03c61be`'s commit
message) and 1.3 K below the BKG's record taken at the E25 merge; the registry count is 290
above that record; the per-file figures are 767 and 1,379 above it. All within or near the
run-to-run spread, for a `bench` built from the formatter code of the base: the sequential
path is untouched.

CPU at 4 threads, head against base (hyperfine, n = 20 per build per crate):

| crate | n per build | base wall min / mean ± σ | head wall min / mean ± σ | base CPU mean | head CPU mean | head/base CPU |
|---|---|---|---|---|---|---|
| fx-hir | 20 | 48.3 / 71.1 ± 13.4 | 44.0 / 61.8 ± 9.9 | 103.7 | 97.3 | 0.938 |
| fx-hir-ty | 20 | 134.8 / 184.3 ± 41.5 | 132.2 / 158.8 ± 22.1 | 269.5 | 278.6 | 1.034 |
| fx-ide-db | 20 | 53.1 / 70.5 ± 13.1 | 50.5 / 68.2 ± 10.3 | 84.7 | 86.4 | 1.019 |
| fx-syntax | 20 | 30.4 / 44.4 ± 11.4 | 28.5 / 41.6 ± 8.5 | 51.0 | 49.6 | 0.972 |
| fx-parser | 20 | 21.2 / 29.5 ± 6.5 | 17.5 / 31.1 ± 8.7 | 33.8 | 34.2 | 1.013 |
| reg-tokio | 20 | 9.6 / 21.7 ± 6.5 | 8.2 / 21.5 ± 11.0 | 22.7 | 20.1 | 0.886 |
| reg-regex-syntax | 20 | 89.0 / 141.3 ± 28.4 | 104.1 / 143.8 ± 22.0 | 191.8 | 196.3 | 1.024 |
| reg-clap-builder | 20 | 45.2 / 64.8 ± 14.1 | 37.5 / 58.1 ± 11.5 | 83.3 | 81.3 | 0.976 |
| reg-winnow | 20 | 47.9 / 71.8 ± 15.7 | 46.2 / 79.4 ± 18.4 | 81.8 | 96.6 | 1.181 |

CPU of the two levers (hyperfine, two rounds in reversed order, n = 30 per build, load 9.1–9.3,
provisional):

| crate | build | n | wall min / mean ± σ ms | CPU (user+sys) mean ms | CPU vs base |
|---|---|---|---|---|---|
| fx-hir | base | 30 | 44.2 / 63.6 ± 10.8 | 104.5 | +0.0% |
| fx-hir | a | 30 | 49.1 / 63.7 ± 11.3 | 101.0 | -3.4% |
| fx-hir | b | 30 | 48.0 / 67.3 ± 11.8 | 95.9 | -8.3% |
| fx-ide-db | base | 30 | 44.9 / 58.7 ± 8.8 | 94.0 | +0.0% |
| fx-ide-db | a | 30 | 39.5 / 58.5 ± 10.2 | 94.4 | +0.4% |
| fx-ide-db | b | 30 | 39.1 / 59.2 ± 10.0 | 89.4 | -4.9% |

## Decision

**Refuted as stated; nothing kept in the formatter or the CLI.**

- `fx-hir` has no idle tail of the size M5 reported: rebuilt from on-CPU costs its run loses
  0.7–1.6 core-ms (0.7–1.2% of four cores) and sits 1.1–1.5% above the work-conserving bound;
  M5's 19.0 core-ms (Q2's 14.0, Q1's 22.4) counted its split file's item threads as idle,
  because the release build's trace has no record of them. No change to the split file's
  scheduling halves a tail that is busy; the schedules that halve the tail metric leave its
  idle time and span where they are, or raise them.
- `fx-ide-db`'s tail is real and is its split file's chain (a 10–20 ms parse, then a 6–13 ms
  `const` unit), but small on the quiet host (1.04 core-ms, 1.1% of capacity; 2.2–7.5% under
  load). Scheduling levers (a) and (b) cannot shorten it; (c) and (d) can, each to the work/4
  line in the replay (−0.3% span on the quiet host, −1.9…−6.9% under load); neither was built.
- Lever (a), built, is exact (0 of 193) and schedule-neutral (replay 0.0%); what it measurably
  removes is one thread start per split file (0.05–0.07 ms); its tails moved both ways and its
  wall-view tails rose; reverted.
- Lever (b), built, is exact (0 of 193) and takes the split file's helpers away when its units
  are ready: spans +39.9% and +17.8%, tails ×8 and ×32 (measured, loaded; the replay predicted
  +49.4% and +28.1%); reverted. The cap's premise, that the split file's item threads cost the
  other files their cores, is not what the schedule shows: under processor sharing they only
  share them, and the work is the same.
- (c) and (d) not built: neither can pass the keep rule on these crates ((c) leaves `fx-hir`
  unchanged by construction, (d) raises its span), and on the quiet host both bound
  `fx-ide-db`'s span at −0.3%.
- Kept for the integrator: `split_sim.py`, `collect_ab.sh`, `identity_start.sh`,
  `trace_patch.py owner` and the README notes; `chloro-core` and `chloro/src` at the head are
  `03c61be`'s.

## Follow-up

1. **Read every tail from a patched trace.** M5's measurement T (and any wall view of a crate
   with a split file from a release build) overstates the idle tail by the item threads'
   time; `split_sim.py --blind` shows how much on any patched trace. If the release trace
   should be readable on its own, it could record the item threads' spans (wall stamps only,
   as the kept timeline does), which `diag.py` already reads.
2. **A large file's chain, as its own hypothesis, for a single file run alone** (stdin, an
   editor's format-on-save, `chloro <file>`): replayed with Q2's costs, (d) takes `lints.rs`
   from 19.54 to 11.15 ms (−43.0%) and `hir/src/lib.rs` from 18.81 to 9.12 ms (−51.5%), (c)
   takes `lints.rs` to 15.97 ms (−13.3%), both together to 8.37 ms (−57.0%); on crates their
   bound is −0.3% of `fx-ide-db`'s span on the quiet host (−2…−7% under load) and (d) needs a
   rule that keeps it off crates like `fx-hir` (parse in parallel only when no other file is
   waiting). Of the two, (d) is the general one (any large file's parse, P9), (c) the one with
   a local exactness argument (element rewrites at known shapes).
3. **The cost of a thread start per split file**, which (a) removes (0.05–0.07 ms of the
   worker's CPU), is below what the crate-level collections resolve (CPU moved ±9% between
   interleaved blocks, both ways); a pinned 1c4t comparison (EXP-023 D5's configuration) on
   the single files would resolve it, for a library caller that splits many files.

## Appendix A: per-run reconstruction, Q2 (patched base)

fx-hir, Q2, patched base build (5 blocks of 6 runs)

| run | load | wall | CPU | wall tail | wall idle in tail | parse | largest unit | work/4 | C | A: span, tail, idle | bd: span, tail, idle | A+c8: span, tail, idle | A+d: span, tail, idle |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0.88 | 30.4 | 99.5 | 2.41 | 1.80 | 12.30 | 2.90 | 23.39 | 23.39 | 24.19, 1.35, 2.94 | 23.64, 0.25, 0.72 | 24.19, 1.35, 2.94 | 25.07, 2.36, 6.45 |
| 1 | 0.88 | 30.2 | 105.4 | 3.11 | 2.88 | 16.45 | 4.73 | 24.95 | 24.95 | 25.17, 4.10, 0.58 | 25.17, 0.22, 0.58 | 25.17, 4.10, 0.58 | 25.53, 1.01, 2.04 |
| 2 | 0.88 | 32.6 | 110.2 | 1.00 | 2.32 | 12.77 | 2.75 | 25.73 | 25.73 | 26.25, 0.87, 1.78 | 25.91, 0.15, 0.44 | 26.25, 0.87, 1.78 | 26.12, 0.58, 1.26 |
| 3 | 0.88 | 35.9 | 108.4 | 1.69 | 1.82 | 13.46 | 3.18 | 25.40 | 25.40 | 26.27, 1.23, 3.22 | 25.61, 0.21, 0.55 | 26.27, 1.23, 3.22 | 26.27, 1.26, 3.21 |
| 4 | 0.88 | 33.8 | 109.3 | 3.97 | 1.98 | 17.24 | 4.65 | 25.56 | 25.56 | 25.73, 4.61, 0.35 | 25.73, 0.12, 0.34 | 25.73, 4.61, 0.35 | 26.07, 0.73, 1.72 |
| 5 | 0.88 | 28.7 | 97.4 | 4.16 | 1.85 | 15.89 | 2.87 | 22.75 | 22.75 | 22.99, 5.04, 0.72 | 22.99, 0.26, 0.72 | 22.99, 5.04, 0.72 | 23.34, 1.02, 2.12 |
| 6 | 1.05 | 49.8 | 116.3 | 27.19 | 67.53 | 38.73 | 2.71 | 27.54 | 42.36 | 44.08, 26.82, 65.98 | 57.15, 39.90, 118.41 | 44.08, 26.82, 65.98 | 28.00, 0.75, 1.63 |
| 7 | 1.05 | 26.9 | 90.0 | 3.62 | 2.21 | 11.53 | 3.19 | 21.06 | 21.06 | 21.61, 1.04, 1.97 | 21.21, 0.12, 0.34 | 21.61, 1.04, 1.97 | 21.47, 0.69, 1.39 |
| 8 | 1.05 | 33.3 | 98.9 | 12.04 | 22.64 | 23.09 | 2.89 | 23.38 | 27.05 | 28.74, 12.06, 21.27 | 42.08, 25.39, 74.74 | 28.74, 12.06, 21.27 | 23.75, 0.77, 1.29 |
| 9 | 1.05 | 25.6 | 89.2 | 3.79 | 2.06 | 13.01 | 2.80 | 21.09 | 21.09 | 21.28, 1.73, 0.48 | 21.28, 0.17, 0.48 | 21.28, 1.73, 0.48 | 21.60, 0.86, 1.77 |
| 10 | 1.05 | 26.4 | 90.7 | 4.44 | 2.21 | 13.28 | 2.87 | 21.39 | 21.39 | 21.54, 1.89, 0.39 | 21.54, 0.14, 0.39 | 21.54, 1.89, 0.39 | 21.64, 0.47, 0.78 |
| 11 | 1.05 | 35.1 | 91.1 | 8.21 | 2.36 | 12.61 | 3.18 | 21.44 | 21.44 | 21.69, 1.22, 0.72 | 21.69, 0.27, 0.72 | 21.69, 1.22, 0.72 | 21.65, 0.21, 0.56 |
| 12 | 1.05 | 28.8 | 90.3 | 3.17 | 2.86 | 12.29 | 3.11 | 21.16 | 21.16 | 21.38, 1.34, 0.64 | 21.38, 0.24, 0.64 | 21.38, 1.34, 0.64 | 21.46, 0.41, 0.96 |
| 13 | 1.05 | 30.5 | 95.7 | 2.23 | 2.07 | 13.71 | 2.88 | 22.45 | 22.45 | 22.71, 1.63, 0.75 | 22.72, 0.29, 0.79 | 22.71, 1.63, 0.75 | 22.96, 0.76, 1.77 |
| 14 | 1.05 | 29.7 | 94.6 | 3.34 | 1.84 | 12.15 | 3.06 | 22.13 | 22.13 | 22.62, 0.71, 1.71 | 22.27, 0.12, 0.35 | 22.62, 0.71, 1.71 | 22.31, 0.38, 0.49 |
| 15 | 1.05 | 29.0 | 96.4 | 4.57 | 2.23 | 12.60 | 3.16 | 22.66 | 22.66 | 23.31, 0.82, 2.27 | 22.91, 0.24, 0.66 | 23.31, 0.82, 2.27 | 24.79, 3.14, 8.20 |
| 16 | 1.05 | 28.7 | 96.5 | 4.77 | 2.46 | 13.66 | 2.84 | 22.73 | 22.73 | 22.91, 0.84, 0.39 | 22.93, 0.16, 0.48 | 22.91, 0.84, 0.39 | 24.97, 3.08, 8.64 |
| 17 | 1.05 | 27.2 | 92.3 | 4.72 | 2.79 | 12.44 | 2.95 | 21.68 | 21.68 | 21.88, 0.42, 0.53 | 21.91, 0.22, 0.65 | 21.88, 0.42, 0.53 | 22.21, 0.73, 1.83 |
| 18 | 1.05 | 26.6 | 90.5 | 3.89 | 2.10 | 12.30 | 2.95 | 21.28 | 21.28 | 21.44, 1.17, 0.40 | 21.44, 0.14, 0.40 | 21.44, 1.17, 0.40 | 21.60, 0.56, 1.05 |
| 19 | 1.05 | 27.3 | 87.3 | 7.18 | 4.06 | 12.99 | 2.80 | 20.58 | 20.58 | 20.87, 2.48, 0.97 | 20.87, 0.33, 0.97 | 20.87, 2.48, 0.97 | 20.96, 0.68, 1.35 |
| 20 | 1.05 | 28.0 | 96.6 | 2.84 | 2.56 | 13.06 | 2.98 | 22.79 | 22.79 | 23.06, 0.56, 0.68 | 23.06, 0.23, 0.68 | 23.06, 0.56, 0.68 | 23.40, 0.99, 2.04 |
| 21 | 1.05 | 27.0 | 88.9 | 6.25 | 3.38 | 12.78 | 3.46 | 20.80 | 20.80 | 20.96, 2.47, 0.46 | 20.96, 0.16, 0.46 | 20.96, 2.47, 0.46 | 21.26, 0.80, 1.66 |
| 22 | 1.05 | 25.5 | 87.7 | 3.31 | 2.15 | 12.31 | 2.82 | 20.62 | 20.62 | 20.86, 0.90, 0.67 | 20.88, 0.26, 0.74 | 20.86, 0.90, 0.67 | 21.14, 0.87, 1.78 |
| 23 | 1.05 | 27.9 | 89.2 | 3.70 | 1.72 | 12.96 | 2.79 | 20.94 | 20.94 | 21.15, 2.30, 0.59 | 21.15, 0.20, 0.59 | 21.15, 2.30, 0.59 | 21.23, 0.48, 0.89 |
| 24 | 1.05 | 29.0 | 94.9 | 7.16 | 2.48 | 13.14 | 4.28 | 22.29 | 22.29 | 23.00, 3.77, 2.62 | 22.43, 0.12, 0.32 | 23.00, 3.77, 2.62 | 24.35, 2.78, 8.01 |
| 25 | 1.05 | 26.8 | 92.2 | 1.64 | 1.95 | 12.30 | 2.78 | 21.72 | 21.72 | 22.16, 0.65, 1.47 | 21.93, 0.20, 0.55 | 22.16, 0.65, 1.47 | 22.78, 1.53, 3.96 |
| 26 | 1.05 | 35.0 | 94.7 | 2.01 | 1.86 | 13.02 | 3.36 | 22.31 | 22.31 | 22.62, 0.73, 0.89 | 22.62, 0.31, 0.89 | 22.62, 0.73, 0.89 | 22.52, 0.43, 0.51 |
| 27 | 1.05 | 33.6 | 107.2 | 0.96 | 3.06 | 16.86 | 2.96 | 25.41 | 25.41 | 25.65, 1.53, 0.68 | 25.65, 0.23, 0.68 | 25.65, 1.53, 0.68 | 26.01, 0.94, 2.15 |
| 28 | 1.05 | 28.0 | 92.6 | 1.27 | 3.94 | 12.86 | 3.18 | 21.73 | 21.73 | 21.94, 1.06, 0.60 | 21.94, 0.22, 0.60 | 21.94, 1.06, 0.60 | 22.18, 0.77, 1.55 |
| 29 | 1.05 | 28.0 | 93.1 | 4.99 | 1.84 | 13.04 | 2.94 | 21.95 | 21.95 | 22.20, 1.18, 0.70 | 22.20, 0.23, 0.70 | 22.20, 1.18, 0.70 | 22.13, 0.19, 0.43 |

fx-ide-db, Q2, patched base build (5 blocks of 6 runs)

| run | load | wall | CPU | wall tail | wall idle in tail | parse | largest unit | work/4 | C | A: span, tail, idle | bd: span, tail, idle | A+c8: span, tail, idle | A+d: span, tail, idle |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 1.37 | 28.0 | 87.6 | 1.29 | 3.80 | 10.37 | 5.87 | 20.50 | 20.50 | 20.62, 0.20, 0.43 | 22.84, 3.57, 9.30 | 20.70, 0.31, 0.77 | 20.55, 0.11, 0.17 |
| 1 | 1.37 | 29.9 | 95.1 | 0.72 | 2.06 | 9.77 | 5.54 | 22.31 | 22.31 | 22.60, 0.43, 1.14 | 22.75, 0.77, 1.72 | 22.62, 0.47, 1.20 | 22.62, 0.45, 1.20 |
| 2 | 1.37 | 34.2 | 97.7 | 4.21 | 9.30 | 10.01 | 10.25 | 23.09 | 23.09 | 24.00, 1.29, 3.60 | 27.25, 5.98, 16.61 | 23.20, 0.18, 0.40 | 23.18, 0.18, 0.32 |
| 3 | 1.37 | 27.9 | 94.0 | 0.70 | 1.81 | 9.37 | 7.50 | 22.15 | 22.15 | 22.68, 0.70, 2.02 | 23.88, 2.60, 6.84 | 22.60, 0.63, 1.72 | 22.60, 0.58, 1.70 |
| 4 | 1.37 | 25.1 | 83.7 | 0.43 | 1.45 | 9.33 | 5.57 | 19.55 | 19.55 | 19.70, 0.18, 0.52 | 21.51, 2.94, 7.79 | 19.64, 0.14, 0.27 | 19.63, 0.18, 0.25 |
| 5 | 1.37 | 28.9 | 100.0 | 0.33 | 1.12 | 8.93 | 5.28 | 23.72 | 23.72 | 23.91, 0.33, 0.66 | 24.53, 1.08, 3.18 | 23.92, 0.31, 0.71 | 23.83, 0.17, 0.33 |
| 6 | 1.37 | 26.4 | 85.0 | 0.97 | 2.71 | 9.48 | 6.01 | 20.00 | 20.00 | 20.08, 0.17, 0.28 | 22.17, 3.31, 8.64 | 20.05, 0.09, 0.13 | 20.05, 0.10, 0.15 |
| 7 | 1.37 | 26.2 | 85.9 | 3.05 | 5.66 | 9.19 | 6.28 | 20.08 | 20.08 | 20.17, 0.15, 0.33 | 22.27, 3.44, 8.72 | 20.15, 0.20, 0.24 | 20.17, 0.15, 0.33 |
| 8 | 1.37 | 25.2 | 82.2 | 2.65 | 6.99 | 8.91 | 5.53 | 19.35 | 19.35 | 19.45, 0.16, 0.33 | 21.49, 3.34, 8.50 | 19.38, 0.06, 0.07 | 19.43, 0.13, 0.25 |
| 9 | 1.37 | 30.6 | 96.5 | 8.59 | 17.15 | 18.41 | 6.60 | 22.98 | 27.08 | 27.09, 8.45, 16.38 | 35.97, 17.34, 51.97 | 24.84, 6.21, 7.41 | 23.10, 0.17, 0.41 |
| 10 | 1.37 | 31.0 | 96.4 | 4.38 | 7.80 | 15.38 | 5.80 | 22.87 | 22.87 | 24.32, 3.55, 5.73 | 25.17, 3.47, 9.14 | 23.22, 2.22, 1.34 | 24.29, 1.96, 5.61 |
| 11 | 1.37 | 26.7 | 88.2 | 3.10 | 4.86 | 9.55 | 5.77 | 20.86 | 20.86 | 20.92, 0.08, 0.19 | 22.26, 2.36, 5.58 | 20.94, 0.13, 0.28 | 20.94, 0.12, 0.27 |
| 12 | 1.37 | 33.2 | 112.2 | 0.45 | 1.41 | 9.23 | 8.89 | 26.63 | 26.63 | 26.76, 0.21, 0.49 | 27.86, 1.70, 4.89 | 26.75, 0.24, 0.42 | 26.72, 0.16, 0.33 |
| 13 | 1.37 | 27.9 | 87.1 | 6.24 | 13.54 | 13.63 | 5.80 | 20.58 | 20.77 | 22.14, 2.92, 6.20 | 23.40, 4.34, 11.23 | 20.83, 0.65, 0.98 | 20.64, 0.10, 0.22 |
| 14 | 1.37 | 26.6 | 85.8 | 0.73 | 2.11 | 9.94 | 5.75 | 20.14 | 20.14 | 20.23, 0.16, 0.35 | 22.67, 3.81, 10.11 | 20.20, 0.13, 0.23 | 20.20, 0.12, 0.21 |
| 15 | 1.37 | 32.0 | 94.0 | 3.15 | 7.71 | 12.31 | 9.56 | 22.20 | 23.05 | 25.24, 4.26, 12.12 | 25.10, 4.64, 11.55 | 22.34, 0.21, 0.50 | 22.27, 0.10, 0.22 |
| 16 | 1.37 | 30.8 | 104.1 | 0.64 | 1.72 | 10.81 | 5.56 | 24.79 | 24.79 | 25.03, 0.39, 0.93 | 26.13, 1.85, 5.33 | 25.08, 0.52, 1.14 | 25.19, 0.60, 1.59 |
| 17 | 1.37 | 46.0 | 109.3 | 22.52 | 58.40 | 13.30 | 20.93 | 25.89 | 35.48 | 37.34, 16.14, 45.67 | 40.57, 20.18, 58.57 | 26.51, 3.35, 2.37 | 30.53, 6.16, 18.42 |
| 18 | 1.37 | 30.4 | 101.2 | 1.53 | 3.77 | 14.66 | 9.19 | 24.06 | 25.22 | 27.22, 4.47, 12.57 | 26.63, 4.28, 10.20 | 24.24, 0.30, 0.64 | 24.26, 0.31, 0.72 |
| 19 | 1.37 | 32.6 | 94.5 | 0.76 | 2.32 | 9.20 | 9.79 | 22.40 | 22.40 | 22.99, 0.86, 2.32 | 27.54, 6.76, 20.51 | 22.49, 0.12, 0.32 | 22.44, 0.05, 0.09 |
| 20 | 1.37 | 31.2 | 88.7 | 8.15 | 17.09 | 10.03 | 8.68 | 21.04 | 21.04 | 22.43, 2.34, 5.50 | 26.69, 7.64, 22.54 | 21.18, 0.21, 0.50 | 21.10, 0.12, 0.21 |
| 21 | 1.37 | 28.1 | 91.4 | 1.05 | 2.94 | 10.85 | 6.39 | 21.56 | 21.56 | 21.63, 0.11, 0.22 | 24.88, 4.83, 13.20 | 21.72, 0.25, 0.55 | 21.72, 0.26, 0.58 |
| 22 | 1.37 | 33.5 | 110.5 | 0.43 | 1.40 | 9.63 | 5.80 | 26.09 | 26.09 | 26.25, 0.24, 0.60 | 26.27, 0.33, 0.71 | 26.21, 0.18, 0.41 | 26.30, 0.33, 0.81 |
| 23 | 1.37 | 36.7 | 123.2 | 2.18 | 6.51 | 15.51 | 10.04 | 29.32 | 29.32 | 29.80, 0.73, 1.83 | 34.09, 7.20, 18.96 | 29.40, 0.11, 0.22 | 29.39, 0.10, 0.18 |
| 24 | 1.37 | 39.7 | 103.8 | 3.23 | 9.71 | 14.85 | 6.52 | 24.55 | 24.55 | 25.29, 1.19, 2.87 | 28.83, 5.51, 17.04 | 24.60, 0.08, 0.12 | 24.65, 0.16, 0.32 |
| 25 | 1.37 | 37.1 | 95.0 | 3.97 | 11.62 | 14.12 | 7.57 | 21.95 | 23.34 | 24.80, 5.51, 11.31 | 23.64, 4.58, 6.66 | 22.36, 2.77, 1.51 | 22.03, 0.10, 0.21 |
| 26 | 1.37 | 29.4 | 88.4 | 4.05 | 10.45 | 10.80 | 5.75 | 20.77 | 20.77 | 20.89, 0.25, 0.39 | 23.59, 4.22, 11.22 | 20.92, 0.31, 0.54 | 21.01, 0.37, 0.88 |
| 27 | 1.37 | 58.3 | 120.8 | 34.43 | 95.21 | 38.77 | 11.94 | 28.80 | 52.68 | 52.72, 34.60, 95.51 | 60.73, 42.60, 127.65 | 45.80, 27.68, 67.82 | 32.95, 5.52, 16.41 |
| 28 | 1.37 | 35.2 | 106.9 | 2.38 | 7.13 | 12.45 | 8.67 | 25.32 | 25.32 | 25.42, 0.20, 0.31 | 29.66, 6.65, 17.27 | 25.56, 0.38, 0.88 | 25.58, 0.41, 0.96 |
| 29 | 1.37 | 31.6 | 107.8 | 0.52 | 1.58 | 9.62 | 6.99 | 25.33 | 25.33 | 25.45, 0.18, 0.39 | 26.37, 1.63, 4.09 | 25.48, 0.19, 0.52 | 25.41, 0.12, 0.24 |

## Appendix B: per-run, lever (a) against the base, Q2

fx-hir, Q2: per crate, 5 blocks; in each block 6 runs of each binary in the order release base, release (a), patched base, patched (a)

| run | load | release base: wall, CPU, tail | release (a): wall, CPU, tail | patched base: wall, CPU; A span, tail, idle | patched (a): wall, CPU; a span, tail, idle |
|---|---|---|---|---|---|
| 0 | 0.88 | 541.2, 130.5, 194.75 | 30.0, 101.6, 1.09 | 30.4, 99.5; 24.19, 1.35, 2.94 | 33.7, 104.4; 25.25, 6.56, 1.71 |
| 1 | 0.88 | 47.4, 110.9, 1.90 | 26.5, 92.5, 4.18 | 30.2, 105.4; 25.17, 4.10, 0.58 | 34.5, 91.6; 21.96, 1.00, 1.70 |
| 2 | 0.88 | 33.6, 98.2, 4.85 | 29.0, 89.2, 7.77 | 32.6, 110.2; 26.25, 0.87, 1.78 | 28.6, 90.8; 21.68, 0.86, 1.26 |
| 3 | 0.88 | 27.7, 91.7, 5.77 | 27.9, 89.4, 7.63 | 35.9, 108.4; 26.27, 1.23, 3.22 | 35.9, 90.9; 21.64, 1.96, 0.43 |
| 4 | 0.88 | 36.9, 115.9, 1.01 | 26.6, 90.1, 6.45 | 33.8, 109.3; 25.73, 4.61, 0.34 | 27.7, 91.3; 21.68, 2.33, 0.40 |
| 5 | 0.88 | 32.7, 115.9, 1.09 | 28.1, 93.6, 5.66 | 28.7, 97.4; 22.99, 5.04, 0.72 | 33.3, 104.7; 25.80, 1.09, 3.20 |
| 6 | 1.05 | 41.1, 103.8, 12.29 | 50.2, 116.8, 27.49 | 49.8, 116.3; 44.08, 26.82, 65.98 | 30.2, 95.7; 23.28, 6.09, 3.16 |
| 7 | 1.05 | 30.4, 103.7, 1.37 | 32.7, 104.0, 6.91 | 26.9, 90.0; 21.61, 1.04, 1.97 | 32.7, 111.6; 27.42, 1.58, 3.94 |
| 8 | 1.05 | 32.4, 108.3, 4.07 | 32.0, 104.2, 6.01 | 33.3, 98.9; 28.74, 12.06, 21.27 | 35.0, 101.6; 23.74, 0.80, 0.55 |
| 9 | 1.05 | 36.2, 104.8, 8.87 | 32.7, 100.2, 8.31 | 25.6, 89.2; 21.28, 1.73, 0.48 | 27.6, 97.2; 23.24, 0.40, 0.74 |
| 10 | 1.05 | 32.8, 102.3, 8.40 | 30.8, 99.8, 9.20 | 26.4, 90.7; 21.54, 1.89, 0.39 | 29.2, 97.8; 23.48, 0.90, 2.03 |
| 11 | 1.05 | 41.4, 116.2, 18.11 | 31.0, 95.1, 0.74 | 35.1, 91.1; 21.69, 1.22, 0.72 | 29.2, 98.9; 25.55, 3.05, 7.88 |
| 12 | 1.05 | 38.4, 99.6, 2.75 | 29.7, 90.4, 7.25 | 28.8, 90.3; 21.38, 1.34, 0.64 | 28.8, 95.4; 22.68, 1.35, 0.62 |
| 13 | 1.05 | 33.3, 97.8, 8.05 | 27.4, 90.1, 2.06 | 30.5, 95.7; 22.71, 1.62, 0.75 | 26.3, 91.5; 22.08, 0.99, 1.72 |
| 14 | 1.05 | 30.2, 102.6, 5.31 | 27.6, 89.1, 6.24 | 29.7, 94.6; 22.62, 0.71, 1.71 | 28.1, 89.9; 21.38, 1.33, 0.62 |
| 15 | 1.05 | 31.8, 103.3, 4.10 | 30.8, 99.5, 7.30 | 29.0, 96.4; 23.31, 0.82, 2.27 | 32.3, 104.9; 26.62, 2.62, 7.73 |
| 16 | 1.05 | 30.5, 101.2, 8.31 | 34.3, 105.1, 1.72 | 28.7, 96.5; 22.91, 0.84, 0.39 | 29.8, 96.9; 23.30, 0.84, 1.90 |
| 17 | 1.05 | 27.1, 89.5, 5.08 | 27.4, 97.1, 1.67 | 27.2, 92.3; 21.88, 0.42, 0.53 | 32.8, 100.3; 23.66, 5.53, 0.46 |
| 18 | 1.05 | 31.3, 95.3, 7.13 | 26.4, 88.5, 4.33 | 26.6, 90.5; 21.44, 1.17, 0.40 | 30.3, 95.6; 22.80, 0.27, 0.53 |
| 19 | 1.05 | 30.7, 105.0, 3.88 | 25.2, 85.7, 1.66 | 27.3, 87.3; 20.87, 2.48, 0.97 | 28.8, 94.5; 23.43, 1.84, 4.44 |
| 20 | 1.05 | 30.1, 92.9, 4.07 | 24.8, 85.5, 0.68 | 28.0, 96.6; 23.06, 0.56, 0.68 | 29.0, 95.7; 24.18, 2.06, 5.86 |
| 21 | 1.05 | 27.3, 91.2, 4.67 | 26.8, 87.2, 5.91 | 27.0, 88.9; 20.96, 2.46, 0.46 | 27.4, 89.2; 21.15, 1.87, 0.42 |
| 22 | 1.05 | 24.9, 86.9, 4.59 | 25.1, 86.4, 2.49 | 25.5, 87.7; 20.86, 0.90, 0.67 | 26.8, 89.9; 21.28, 2.45, 0.50 |
| 23 | 1.05 | 28.0, 92.7, 6.63 | 26.5, 89.0, 5.24 | 27.9, 89.2; 21.15, 2.30, 0.59 | 36.3, 97.8; 22.35, 0.48, 0.92 |
| 24 | 1.05 | 27.4, 88.2, 5.59 | 31.2, 97.4, 7.44 | 29.0, 94.9; 23.00, 3.77, 2.62 | 29.9, 89.9; 21.61, 0.74, 1.51 |
| 25 | 1.05 | 25.8, 85.2, 5.44 | 30.6, 96.8, 3.50 | 26.8, 92.2; 22.16, 0.65, 1.48 | 32.3, 99.4; 28.07, 6.13, 18.23 |
| 26 | 1.05 | 28.0, 88.7, 6.20 | 31.0, 100.6, 4.37 | 35.0, 94.7; 22.62, 0.73, 0.89 | 35.6, 97.3; 27.23, 5.89, 16.72 |
| 27 | 1.05 | 35.0, 113.4, 8.86 | 31.0, 96.3, 7.21 | 33.6, 107.2; 25.65, 1.53, 0.68 | 26.8, 93.6; 23.29, 1.67, 4.24 |
| 28 | 1.05 | 37.2, 125.1, 2.12 | 30.1, 90.9, 9.11 | 28.0, 92.6; 21.94, 1.06, 0.60 | 29.3, 97.1; 23.61, 1.03, 2.42 |
| 29 | 1.05 | 38.5, 123.5, 4.67 | 30.0, 93.5, 8.89 | 28.0, 93.1; 22.20, 1.18, 0.70 | 25.7, 86.7; 20.49, 1.50, 0.26 |

fx-ide-db, Q2: per crate, 5 blocks; in each block 6 runs of each binary in the order release base, release (a), patched base, patched (a)

| run | load | release base: wall, CPU, tail | release (a): wall, CPU, tail | patched base: wall, CPU; A span, tail, idle | patched (a): wall, CPU; a span, tail, idle |
|---|---|---|---|---|---|
| 0 | 1.37 | 1257.1, 99.1, 0.98 | 34.3, 96.6, 5.13 | 28.0, 87.6; 20.62, 0.20, 0.43 | 28.3, 98.5; 23.57, 0.18, 0.41 |
| 1 | 1.37 | 33.4, 108.0, 0.62 | 26.3, 82.3, 4.29 | 29.9, 95.1; 22.61, 0.43, 1.14 | 26.3, 90.0; 21.55, 0.18, 0.47 |
| 2 | 1.37 | 38.6, 99.5, 9.67 | 28.3, 85.7, 6.16 | 34.2, 97.7; 24.00, 1.29, 3.60 | 23.8, 80.0; 19.04, 0.12, 0.31 |
| 3 | 1.37 | 36.9, 117.4, 4.80 | 27.7, 86.7, 3.91 | 27.9, 94.0; 22.68, 0.70, 2.02 | 24.2, 81.6; 19.46, 0.20, 0.41 |
| 4 | 1.37 | 43.3, 124.0, 6.07 | 28.5, 87.8, 4.03 | 25.1, 83.7; 19.70, 0.18, 0.52 | 23.1, 79.5; 18.99, 0.12, 0.29 |
| 5 | 1.37 | 34.5, 100.4, 3.11 | 26.4, 84.1, 4.45 | 28.9, 100.0; 23.91, 0.33, 0.66 | 25.6, 83.3; 19.87, 0.23, 0.57 |
| 6 | 1.37 | 26.4, 84.6, 2.57 | 27.6, 83.5, 4.88 | 26.4, 85.0; 20.08, 0.17, 0.28 | 34.5, 102.1; 26.59, 3.43, 9.89 |
| 7 | 1.37 | 26.8, 91.1, 0.38 | 27.1, 84.1, 2.66 | 26.2, 85.9; 20.17, 0.15, 0.33 | 33.6, 104.7; 24.29, 0.26, 0.42 |
| 8 | 1.37 | 28.5, 89.2, 2.89 | 27.4, 82.7, 5.19 | 25.2, 82.2; 19.45, 0.16, 0.33 | 33.4, 98.4; 24.05, 1.19, 3.35 |
| 9 | 1.37 | 26.7, 87.8, 3.44 | 26.4, 85.3, 3.09 | 30.6, 96.5; 27.09, 8.45, 16.38 | 27.2, 83.9; 19.82, 0.18, 0.52 |
| 10 | 1.37 | 30.4, 95.7, 4.09 | 30.0, 93.5, 5.47 | 31.0, 96.4; 24.32, 3.55, 5.73 | 32.8, 88.5; 23.61, 4.13, 10.91 |
| 11 | 1.37 | 27.3, 85.8, 3.82 | 28.5, 93.9, 3.59 | 26.7, 88.2; 20.92, 0.08, 0.19 | 27.4, 88.1; 20.87, 0.11, 0.17 |
| 12 | 1.37 | 30.2, 85.2, 5.65 | 28.5, 96.3, 0.43 | 33.2, 112.2; 26.76, 0.21, 0.49 | 30.8, 86.5; 20.50, 0.22, 0.48 |
| 13 | 1.37 | 26.3, 85.2, 3.53 | 38.7, 97.2, 15.81 | 27.9, 87.1; 22.14, 2.92, 6.20 | 30.1, 91.0; 21.64, 0.26, 0.58 |
| 14 | 1.37 | 26.8, 85.7, 2.44 | 28.9, 94.9, 2.47 | 26.6, 85.8; 20.23, 0.16, 0.35 | 28.7, 85.0; 20.11, 0.09, 0.25 |
| 15 | 1.37 | 27.6, 90.0, 3.33 | 32.1, 94.1, 8.90 | 32.0, 94.0; 25.25, 4.26, 12.12 | 35.0, 106.1; 25.04, 0.12, 0.19 |
| 16 | 1.37 | 27.6, 87.6, 1.83 | 35.7, 110.4, 6.60 | 30.8, 104.1; 25.03, 0.39, 0.93 | 32.6, 91.8; 21.88, 0.29, 0.55 |
| 17 | 1.37 | 27.8, 87.0, 3.98 | 38.5, 106.3, 8.64 | 46.0, 109.3; 37.34, 16.14, 45.67 | 32.6, 90.3; 21.17, 0.42, 1.20 |
| 18 | 1.37 | 28.7, 87.4, 2.83 | 38.0, 100.4, 1.06 | 30.4, 101.2; 27.22, 4.47, 12.57 | 28.4, 92.4; 21.86, 0.14, 0.25 |
| 19 | 1.37 | 31.1, 88.4, 4.11 | 37.7, 102.8, 6.29 | 32.6, 94.5; 22.99, 0.86, 2.32 | 34.2, 86.7; 20.30, 0.13, 0.29 |
| 20 | 1.37 | 30.1, 95.5, 3.63 | 26.2, 84.7, 3.19 | 31.2, 88.7; 22.43, 2.34, 5.50 | 29.0, 84.3; 19.87, 0.19, 0.28 |
| 21 | 1.37 | 29.0, 92.9, 1.86 | 38.2, 100.0, 6.46 | 28.1, 91.4; 21.63, 0.11, 0.22 | 25.4, 84.1; 19.95, 0.18, 0.31 |
| 22 | 1.37 | 33.4, 101.4, 5.41 | 51.5, 97.2, 4.57 | 33.5, 110.5; 26.25, 0.24, 0.60 | 26.0, 85.5; 20.29, 0.12, 0.17 |
| 23 | 1.37 | 28.4, 87.8, 2.71 | 35.6, 103.2, 8.74 | 36.7, 123.2; 29.80, 0.73, 1.82 | 27.8, 87.6; 22.64, 2.90, 7.66 |
| 24 | 1.37 | 29.2, 95.9, 1.63 | 29.9, 91.8, 3.29 | 39.7, 103.8; 25.29, 1.19, 2.87 | 35.5, 96.9; 26.28, 5.52, 13.90 |
| 25 | 1.37 | 29.1, 90.3, 3.37 | 30.5, 94.7, 7.15 | 37.1, 95.0; 24.80, 5.51, 11.31 | 49.7, 96.5; 25.44, 3.61, 10.65 |
| 26 | 1.37 | 25.6, 83.2, 3.31 | 29.5, 88.5, 3.91 | 29.4, 88.4; 20.89, 0.24, 0.39 | 30.8, 97.4; 25.98, 5.97, 12.52 |
| 27 | 1.37 | 27.0, 83.8, 3.35 | 33.3, 88.3, 3.44 | 58.3, 120.8; 52.72, 34.60, 95.51 | 30.9, 94.4; 23.27, 1.60, 4.70 |
| 28 | 1.37 | 29.6, 87.7, 4.38 | 46.1, 91.9, 9.73 | 35.2, 106.9; 25.42, 0.20, 0.31 | 29.7, 87.0; 20.88, 0.38, 1.06 |
| 29 | 1.37 | 28.1, 88.0, 5.26 | 52.3, 91.6, 7.74 | 31.6, 107.8; 25.45, 0.18, 0.39 | 26.0, 81.7; 19.36, 0.21, 0.43 |

## Appendix C: per-run, lever (a) against the base, L2

fx-hir, L2 (interleaved blocks of 6 runs; block k of the base then block k of the lever)

| run | base: load, wall, CPU, wall tail | base A: span, tail, idle | lever (a): load, wall, CPU, wall tail | lever a: span, tail, idle |
|---|---|---|---|---|
| 0 | 4.63, 177.7, 185.8, 39.96 | 44.51, 6.35, 0.93 | 4.63, 143.4, 186.2, 36.41 | 45.70, 0.98, 2.65 |
| 1 | 4.63, 158.1, 170.7, 2.61 | 40.95, 5.54, 0.93 | 4.63, 130.7, 144.7, 19.13 | 34.62, 2.11, 0.46 |
| 2 | 4.63, 137.2, 171.2, 6.31 | 41.55, 1.15, 2.54 | 4.63, 102.7, 126.2, 39.24 | 30.34, 0.51, 1.06 |
| 3 | 4.63, 160.3, 148.9, 43.60 | 35.84, 3.05, 0.81 | 4.63, 146.8, 156.0, 14.02 | 38.88, 2.86, 6.47 |
| 4 | 4.63, 121.1, 129.5, 11.52 | 31.30, 1.05, 2.09 | 4.63, 108.7, 138.8, 33.21 | 35.35, 12.66, 8.91 |
| 5 | 4.63, 106.9, 138.5, 6.95 | 33.27, 2.48, 0.80 | 4.63, 108.1, 149.8, 9.40 | 35.87, 9.28, 0.79 |
| 6 | 4.74, 99.4, 132.2, 36.63 | 35.28, 14.13, 15.47 | 4.74, 116.0, 114.4, 31.04 | 27.39, 3.90, 0.76 |
| 7 | 4.74, 120.2, 156.9, 19.00 | 40.76, 4.63, 12.93 | 4.74, 176.7, 127.9, 26.06 | 31.43, 1.59, 3.94 |
| 8 | 4.74, 144.1, 161.5, 34.54 | 39.12, 0.89, 2.08 | 4.74, 115.8, 115.6, 17.62 | 27.66, 4.28, 0.69 |
| 9 | 4.74, 122.2, 148.8, 10.14 | 35.56, 0.59, 1.04 | 4.74, 158.7, 134.1, 17.09 | 36.21, 6.24, 17.28 |
| 10 | 4.74, 159.0, 130.6, 26.31 | 31.30, 1.97, 0.79 | 4.74, 162.5, 128.7, 41.33 | 31.89, 7.39, 4.87 |
| 11 | 4.74, 123.4, 126.8, 14.75 | 30.34, 1.13, 2.42 | 4.74, 132.7, 126.4, 27.05 | 30.03, 1.42, 0.38 |
| 12 | 4.74, 171.9, 168.6, 5.21 | 40.68, 2.42, 0.66 | 4.74, 148.0, 168.7, 23.94 | 40.47, 4.14, 0.49 |
| 13 | 4.74, 133.1, 125.3, 44.22 | 30.53, 1.22, 2.50 | 4.74, 148.2, 152.0, 11.44 | 37.09, 0.90, 2.15 |
| 14 | 4.74, 119.4, 128.0, 10.74 | 31.04, 1.37, 2.44 | 4.74, 120.4, 138.1, 24.17 | 36.97, 6.14, 16.61 |
| 15 | 4.74, 126.8, 134.1, 24.58 | 32.00, 5.93, 0.73 | 4.74, 118.0, 147.2, 47.35 | 35.15, 0.49, 0.62 |
| 16 | 4.74, 113.9, 121.7, 10.32 | 28.84, 3.33, 0.50 | 4.74, 114.8, 160.9, 7.75 | 40.59, 4.11, 9.39 |
| 17 | 4.74, 144.9, 145.0, 1.04 | 35.17, 1.26, 2.14 | 4.74, 112.2, 137.1, 10.91 | 33.39, 1.79, 3.92 |
| 18 | 4.74, 118.1, 172.8, 23.19 | 44.97, 15.07, 15.76 | 4.74, 102.4, 148.8, 6.77 | 35.96, 0.64, 1.71 |
| 19 | 4.74, 128.5, 172.7, 55.97 | 41.85, 10.06, 1.14 | 4.74, 112.2, 147.9, 16.07 | 37.86, 11.34, 9.99 |
| 20 | 4.74, 105.2, 137.1, 2.78 | 32.72, 0.50, 0.99 | 4.74, 107.5, 131.8, 10.75 | 31.57, 6.34, 0.54 |
| 21 | 4.74, 142.5, 137.6, 30.79 | 33.01, 5.53, 0.59 | 4.74, 105.3, 143.4, 31.18 | 34.10, 1.38, 0.41 |
| 22 | 4.74, 98.2, 131.3, 6.78 | 31.82, 0.87, 1.79 | 5.08, 100.2, 128.6, 45.87 | 36.20, 7.59, 22.06 |
| 23 | 4.74, 98.5, 121.2, 0.99 | 29.02, 3.78, 0.80 | 5.08, 114.2, 135.4, 25.88 | 32.46, 1.32, 0.45 |
| 24 | 5.08, 89.2, 157.2, 11.42 | 37.61, 2.17, 2.53 | 5.08, 98.6, 127.4, 20.84 | 30.65, 1.25, 1.86 |
| 25 | 5.08, 143.6, 172.5, 14.44 | 41.42, 2.02, 0.83 | 5.08, 149.9, 163.2, 4.35 | 40.06, 2.00, 4.28 |
| 26 | 5.08, 163.6, 178.6, 21.86 | 42.56, 8.96, 1.03 | 5.08, 94.8, 133.2, 10.24 | 32.59, 4.28, 3.94 |
| 27 | 5.08, 143.5, 191.2, 6.01 | 45.96, 6.31, 1.00 | 5.08, 82.8, 126.0, 2.87 | 30.58, 0.75, 1.55 |
| 28 | 5.08, 111.7, 123.7, 7.53 | 29.23, 1.81, 0.51 | 5.08, 84.9, 129.1, 1.56 | 31.57, 1.66, 2.90 |
| 29 | 5.08, 120.0, 132.8, 19.62 | 31.46, 1.50, 0.85 | 5.08, 155.0, 156.9, 28.89 | 38.56, 1.32, 3.21 |

fx-ide-db, L2 (interleaved blocks of 6 runs; block k of the base then block k of the lever)

| run | base: load, wall, CPU, wall tail | base A: span, tail, idle | lever (a): load, wall, CPU, wall tail | lever a: span, tail, idle |
|---|---|---|---|---|
| 0 | 5.08, 103.0, 129.2, 14.96 | 37.24, 11.82, 26.48 | 5.08, 113.2, 123.6, 56.65 | 33.00, 8.06, 15.25 |
| 1 | 5.08, 116.8, 142.3, 4.32 | 38.68, 9.49, 19.84 | 5.08, 99.4, 109.5, 8.48 | 26.06, 0.17, 0.24 |
| 2 | 5.08, 97.8, 164.6, 2.75 | 41.16, 2.64, 7.48 | 5.08, 79.8, 136.5, 8.42 | 36.42, 7.13, 15.89 |
| 3 | 5.08, 118.4, 160.8, 17.19 | 38.34, 0.27, 0.38 | 5.08, 127.9, 148.7, 3.97 | 38.94, 4.79, 13.89 |
| 4 | 5.08, 124.1, 142.5, 15.81 | 39.31, 7.97, 21.52 | 5.08, 88.6, 126.0, 0.52 | 30.17, 0.22, 0.50 |
| 5 | 5.08, 133.1, 159.3, 4.64 | 39.28, 1.78, 5.06 | 5.08, 86.8, 113.3, 3.78 | 26.95, 0.13, 0.21 |
| 6 | 5.08, 111.2, 128.9, 31.59 | 30.79, 0.49, 1.32 | 5.08, 77.2, 103.5, 6.41 | 24.66, 0.23, 0.49 |
| 7 | 5.08, 214.0, 155.8, 28.03 | 42.24, 9.07, 18.95 | 5.08, 115.6, 108.2, 23.71 | 25.51, 0.21, 0.43 |
| 8 | 5.08, 136.7, 134.9, 4.66 | 32.62, 1.03, 2.76 | 5.08, 173.0, 123.4, 56.31 | 35.30, 8.65, 23.99 |
| 9 | 5.08, 102.3, 130.7, 0.69 | 30.99, 0.07, 0.13 | 5.08, 100.7, 115.1, 4.34 | 27.35, 0.97, 2.61 |
| 10 | 5.08, 110.1, 131.0, 14.29 | 31.38, 0.14, 0.29 | 5.08, 111.3, 129.1, 8.81 | 30.56, 0.20, 0.49 |
| 11 | 5.08, 101.4, 108.6, 4.84 | 25.76, 0.27, 0.56 | 5.08, 115.7, 120.2, 16.23 | 33.48, 7.37, 21.88 |
| 12 | 5.16, 95.6, 128.4, 15.46 | 33.45, 5.38, 11.68 | 5.16, 124.7, 161.0, 16.52 | 42.29, 5.43, 13.97 |
| 13 | 5.16, 115.2, 144.2, 1.39 | 34.38, 0.22, 0.38 | 5.16, 107.0, 115.5, 7.50 | 27.36, 0.25, 0.65 |
| 14 | 5.16, 127.3, 187.1, 0.89 | 44.92, 0.40, 0.80 | 5.16, 131.3, 136.6, 0.63 | 34.30, 2.67, 7.43 |
| 15 | 5.16, 118.9, 184.1, 0.80 | 45.83, 2.89, 8.44 | 5.16, 119.7, 129.1, 13.30 | 35.44, 12.13, 18.59 |
| 16 | 5.16, 117.1, 180.2, 0.54 | 44.53, 2.13, 5.51 | 5.16, 126.7, 127.5, 72.14 | 35.59, 8.62, 20.79 |
| 17 | 5.16, 109.0, 175.5, 2.60 | 42.33, 0.23, 0.43 | 5.16, 146.6, 152.6, 49.04 | 36.89, 0.61, 1.57 |
| 18 | 5.16, 94.5, 152.8, 0.55 | 40.07, 6.76, 15.07 | 5.16, 134.1, 141.6, 46.77 | 38.11, 6.62, 16.94 |
| 19 | 5.16, 86.3, 124.8, 5.50 | 32.28, 4.50, 10.57 | 5.16, 92.2, 117.4, 10.09 | 32.19, 10.59, 18.20 |
| 20 | 5.16, 66.8, 107.5, 2.81 | 25.68, 0.34, 0.85 | 5.16, 117.3, 133.0, 15.77 | 33.71, 2.71, 8.03 |
| 21 | 5.16, 105.1, 132.7, 5.07 | 33.85, 3.25, 9.15 | 5.16, 180.7, 169.3, 35.70 | 42.56, 2.75, 7.80 |
| 22 | 5.16, 108.7, 144.5, 14.17 | 42.50, 16.60, 34.87 | 5.16, 147.1, 154.3, 10.85 | 37.47, 0.87, 2.42 |
| 23 | 5.16, 123.9, 145.0, 4.74 | 35.21, 0.71, 1.85 | 5.16, 105.8, 129.6, 4.14 | 36.04, 12.05, 21.64 |
| 24 | 5.16, 97.4, 120.8, 19.81 | 30.73, 3.86, 8.16 | 5.16, 102.5, 133.1, 14.05 | 35.09, 4.73, 13.79 |
| 25 | 5.16, 121.6, 135.0, 3.97 | 33.29, 1.48, 3.96 | 5.16, 128.2, 140.4, 4.29 | 33.60, 0.16, 0.36 |
| 26 | 5.16, 166.6, 177.5, 14.40 | 47.81, 7.98, 21.85 | 5.16, 123.4, 146.2, 0.64 | 35.03, 0.42, 0.82 |
| 27 | 5.16, 163.1, 184.9, 30.88 | 48.02, 5.64, 15.23 | 5.16, 116.5, 138.2, 20.60 | 33.17, 0.14, 0.22 |
| 28 | 5.16, 143.3, 129.1, 14.78 | 36.50, 9.03, 23.77 | 5.16, 133.0, 141.3, 1.27 | 34.75, 1.84, 4.23 |
| 29 | 5.16, 135.4, 123.4, 37.23 | 29.59, 1.13, 3.17 | 5.16, 124.0, 146.2, 2.51 | 37.07, 3.43, 9.33 |

## Appendix D: per-run, lever (b) against the base, L3

fx-hir, L3: 5 blocks; in each block 6 runs of each binary in the order release base, release (b), patched base, patched (b)

| run | load | release base: wall, CPU, tail | release (b): wall, CPU, tail | patched base: wall, CPU; A span, tail, idle | patched (b): wall, CPU; A span, tail, idle |
|---|---|---|---|---|---|
| 0 | 8.69 | 57.5, 89.4, 25.39 | 60.1, 95.8, 19.55 | 49.8, 96.0; 22.96, 1.43, 0.69 | 65.5, 94.4; 30.86, 11.41, 33.26 |
| 1 | 8.69 | 58.0, 87.3, 16.32 | 52.5, 87.0, 14.69 | 47.5, 88.8; 21.06, 1.15, 0.37 | 53.4, 91.1; 31.54, 13.12, 39.09 |
| 2 | 8.69 | 45.0, 87.5, 10.93 | 56.8, 87.1, 17.80 | 54.2, 89.3; 21.27, 0.95, 0.48 | 69.6, 87.1; 30.71, 13.51, 39.91 |
| 3 | 8.69 | 65.1, 112.7, 13.46 | 51.4, 85.6, 14.72 | 47.2, 88.2; 21.00, 1.10, 0.44 | 53.7, 88.1; 32.60, 15.75, 46.53 |
| 4 | 8.69 | 57.3, 107.3, 2.28 | 57.6, 103.6, 9.62 | 51.2, 92.7; 22.09, 0.36, 0.45 | 55.8, 101.8; 32.35, 10.92, 31.86 |
| 5 | 8.69 | 54.6, 98.0, 4.93 | 58.0, 97.9, 0.93 | 47.0, 91.9; 21.74, 0.28, 0.31 | 63.1, 90.6; 31.63, 13.73, 40.28 |
| 6 | 8.69 | 54.1, 90.5, 10.66 | 67.4, 114.4, 1.94 | 52.3, 93.7; 22.35, 1.48, 1.18 | 58.0, 89.9; 30.83, 12.90, 37.87 |
| 7 | 8.48 | 52.2, 93.9, 11.30 | 64.5, 104.8, 22.23 | 49.5, 92.7; 22.03, 1.82, 0.42 | 58.8, 88.3; 30.78, 13.60, 39.34 |
| 8 | 8.48 | 58.1, 101.0, 1.49 | 56.9, 98.8, 2.01 | 50.9, 92.7; 22.18, 2.21, 0.61 | 69.7, 112.8; 31.74, 7.91, 20.51 |
| 9 | 8.48 | 58.9, 101.3, 12.20 | 60.8, 88.2, 26.46 | 58.7, 97.3; 23.14, 2.13, 0.57 | 68.9, 97.6; 31.85, 11.99, 34.03 |
| 10 | 8.48 | 55.4, 101.4, 2.90 | 63.9, 104.8, 20.14 | 64.0, 113.5; 27.51, 0.95, 1.90 | 61.6, 107.8; 38.44, 19.54, 51.79 |
| 11 | 8.48 | 60.4, 112.4, 0.96 | 57.1, 97.1, 12.83 | 62.4, 114.5; 27.27, 3.67, 0.86 | 63.3, 113.1; 31.27, 7.12, 18.14 |
| 12 | 8.48 | 66.3, 112.0, 19.02 | 80.4, 114.7, 30.27 | 58.2, 108.1; 25.56, 0.47, 0.58 | 90.6, 88.2; 31.17, 13.86, 40.73 |
| 13 | 8.48 | 64.5, 123.7, 2.06 | 61.0, 101.7, 12.12 | 46.9, 90.5; 21.48, 1.92, 0.43 | 53.6, 88.4; 30.95, 13.67, 39.82 |
| 14 | 8.48 | 71.1, 127.6, 14.15 | 64.7, 97.9, 21.65 | 48.4, 94.3; 22.37, 1.40, 0.44 | 55.1, 88.1; 32.00, 15.00, 43.85 |
| 15 | 8.48 | 63.8, 114.2, 6.03 | 64.3, 98.2, 22.53 | 60.1, 110.3; 26.63, 0.70, 1.46 | 51.0, 86.0; 30.27, 13.14, 39.08 |
| 16 | 8.48 | 62.4, 118.7, 6.57 | 63.6, 104.2, 16.12 | 48.1, 91.8; 21.89, 1.41, 0.59 | 69.1, 117.8; 43.48, 21.33, 61.16 |
| 17 | 8.48 | 69.8, 124.6, 3.79 | 58.7, 97.9, 13.15 | 47.6, 93.5; 22.24, 1.69, 0.52 | 52.6, 94.9; 30.96, 11.35, 33.49 |
| 18 | 8.48 | 55.5, 93.7, 20.76 | 51.1, 92.3, 4.56 | 56.3, 107.5; 25.24, 4.82, 0.53 | 64.2, 89.0; 32.81, 16.06, 47.02 |
| 19 | 8.48 | 67.3, 118.8, 12.37 | 68.1, 106.0, 17.30 | 62.1, 104.8; 25.13, 0.94, 1.86 | 55.6, 88.8; 31.40, 14.17, 41.78 |
| 20 | 8.48 | 57.2, 111.4, 3.02 | 53.6, 95.9, 4.07 | 67.3, 115.3; 27.75, 0.97, 2.17 | 52.0, 89.4; 31.04, 13.21, 38.69 |
| 21 | 8.48 | 55.7, 91.3, 18.63 | 56.3, 90.2, 17.91 | 59.7, 108.0; 25.80, 6.30, 1.94 | 62.3, 93.9; 32.67, 14.66, 43.21 |
| 22 | 8.48 | 51.2, 91.7, 14.80 | 90.0, 150.8, 22.31 | 56.4, 105.6; 24.94, 5.01, 0.63 | 56.5, 92.0; 30.57, 11.91, 34.95 |
| 23 | 8.48 | 52.1, 92.4, 11.63 | 77.2, 128.2, 11.27 | 50.3, 90.1; 21.21, 2.68, 0.45 | 55.7, 89.9; 30.96, 13.07, 38.07 |
| 24 | 8.48 | 97.4, 147.4, 13.27 | 57.9, 94.4, 16.10 | 47.4, 86.6; 20.61, 2.09, 0.29 | 58.5, 95.1; 30.41, 12.10, 31.35 |
| 25 | 8.04 | 55.2, 101.2, 4.30 | 62.7, 101.9, 19.93 | 46.8, 91.1; 21.70, 1.64, 0.43 | 57.0, 94.8; 34.06, 15.51, 45.78 |
| 26 | 8.04 | 55.9, 100.1, 14.71 | 80.2, 117.1, 29.54 | 49.0, 90.3; 21.20, 2.12, 0.37 | 54.9, 85.2; 29.64, 13.19, 38.94 |
| 27 | 8.04 | 54.8, 98.5, 13.08 | 68.8, 109.6, 17.09 | 53.9, 92.5; 22.10, 2.87, 0.46 | 58.2, 90.5; 31.23, 13.00, 38.66 |
| 28 | 8.04 | 51.1, 96.2, 6.28 | 72.0, 127.8, 10.27 | 56.8, 96.1; 22.98, 2.36, 0.92 | 52.8, 86.0; 30.32, 13.61, 39.17 |
| 29 | 8.04 | 53.1, 95.5, 5.72 | 47.9, 85.4, 2.08 | 51.8, 95.8; 22.66, 3.33, 0.44 | 51.0, 88.2; 31.05, 13.85, 39.88 |

fx-ide-db, L3: 5 blocks; in each block 6 runs of each binary in the order release base, release (b), patched base, patched (b)

| run | load | release base: wall, CPU, tail | release (b): wall, CPU, tail | patched base: wall, CPU; A span, tail, idle | patched (b): wall, CPU; A span, tail, idle |
|---|---|---|---|---|---|
| 0 | 8.04 | 48.0, 99.6, 1.46 | 51.6, 92.5, 4.73 | 57.4, 101.5; 27.31, 5.33, 12.82 | 56.0, 84.1; 24.33, 5.80, 17.32 |
| 1 | 8.04 | 53.1, 89.4, 4.26 | 44.0, 85.7, 1.52 | 53.3, 102.4; 24.20, 0.15, 0.21 | 48.7, 81.0; 24.13, 6.64, 19.68 |
| 2 | 8.04 | 45.9, 86.4, 1.13 | 47.9, 85.2, 7.28 | 48.6, 92.3; 21.94, 0.23, 0.56 | 44.3, 82.4; 24.45, 6.60, 19.54 |
| 3 | 8.04 | 43.2, 82.9, 3.14 | 48.5, 85.3, 7.42 | 42.5, 83.4; 19.71, 0.14, 0.30 | 45.3, 81.7; 24.01, 6.13, 18.27 |
| 4 | 8.04 | 63.4, 107.3, 6.91 | 53.3, 89.0, 10.19 | 46.8, 84.9; 20.42, 0.64, 1.76 | 51.0, 80.9; 23.57, 5.88, 17.50 |
| 5 | 8.04 | 60.1, 106.9, 6.65 | 46.0, 86.7, 0.95 | 51.7, 85.4; 20.18, 0.16, 0.23 | 57.6, 86.9; 25.83, 7.08, 21.02 |
| 6 | 8.04 | 44.6, 85.1, 1.15 | 44.6, 83.2, 0.43 | 46.0, 84.3; 20.22, 0.56, 1.43 | 53.9, 95.4; 28.55, 7.75, 23.11 |
| 7 | 8.04 | 52.4, 87.0, 3.71 | 48.7, 86.7, 6.97 | 57.2, 90.4; 21.40, 0.22, 0.38 | 49.6, 92.8; 24.29, 3.08, 8.89 |
| 8 | 8.04 | 46.3, 84.9, 1.92 | 49.6, 87.4, 1.23 | 53.6, 100.3; 23.71, 0.32, 0.59 | 60.4, 92.4; 27.09, 6.86, 20.34 |
| 9 | 8.04 | 48.3, 86.9, 1.10 | 52.4, 85.9, 10.79 | 48.3, 95.5; 22.64, 0.15, 0.39 | 48.5, 81.6; 23.86, 6.12, 18.17 |
| 10 | 8.04 | 47.6, 83.9, 4.99 | 58.9, 88.7, 1.74 | 45.6, 85.2; 20.23, 0.20, 0.51 | 47.9, 79.2; 22.92, 5.54, 16.57 |
| 11 | 8.04 | 49.4, 85.1, 2.20 | 48.5, 83.1, 12.86 | 47.0, 80.6; 19.11, 0.13, 0.26 | 43.5, 81.7; 23.86, 6.04, 18.06 |
| 12 | 8.04 | 49.1, 84.4, 8.35 | 54.9, 89.0, 5.86 | 45.9, 85.5; 20.26, 0.15, 0.42 | 47.8, 81.4; 23.36, 5.48, 16.27 |
| 13 | 8.04 | 66.7, 110.5, 13.45 | 53.5, 82.1, 12.91 | 52.8, 91.2; 21.68, 0.12, 0.24 | 48.1, 84.1; 24.88, 6.51, 19.41 |
| 14 | 8.04 | 61.9, 107.2, 4.82 | 47.4, 82.4, 6.80 | 41.5, 84.4; 19.99, 0.12, 0.26 | 49.1, 86.3; 26.97, 8.72, 25.89 |
| 15 | 8.04 | 50.7, 92.1, 0.86 | 47.5, 84.2, 7.19 | 58.0, 100.6; 25.55, 3.13, 6.81 | 45.5, 83.5; 24.51, 6.22, 18.53 |
| 16 | 8.04 | 50.9, 95.8, 1.29 | 45.4, 81.3, 6.28 | 73.2, 99.6; 23.64, 0.22, 0.44 | 42.2, 80.8; 23.98, 6.49, 19.18 |
| 17 | 8.04 | 48.8, 96.2, 6.31 | 45.6, 81.8, 0.88 | 56.5, 107.6; 27.44, 3.17, 7.57 | 46.8, 81.6; 24.21, 6.50, 19.35 |
| 18 | 8.35 | 47.8, 83.6, 6.05 | 55.0, 98.3, 5.63 | 61.1, 86.5; 20.41, 0.20, 0.37 | 62.6, 89.8; 27.20, 8.06, 23.93 |
| 19 | 8.35 | 56.2, 86.4, 5.87 | 47.2, 82.6, 5.43 | 46.0, 88.2; 20.80, 0.10, 0.22 | 55.4, 99.6; 26.31, 3.75, 11.03 |
| 20 | 8.35 | 41.8, 85.3, 0.74 | 46.8, 80.4, 6.08 | 39.4, 84.4; 19.97, 0.11, 0.15 | 55.2, 92.2; 26.23, 5.60, 16.56 |
| 21 | 8.35 | 44.1, 83.6, 1.07 | 50.6, 87.5, 3.79 | 48.8, 92.6; 22.02, 0.10, 0.18 | 50.2, 89.4; 25.46, 5.75, 16.91 |
| 22 | 8.35 | 48.7, 83.0, 1.02 | 51.4, 81.1, 13.94 | 49.7, 86.2; 20.37, 0.09, 0.14 | 51.7, 84.9; 24.04, 5.45, 16.04 |
| 23 | 8.35 | 56.7, 86.1, 7.58 | 43.8, 81.7, 2.13 | 49.4, 100.5; 23.97, 0.22, 0.57 | 52.2, 87.6; 25.55, 6.32, 18.75 |
| 24 | 8.35 | 49.8, 87.6, 10.30 | 53.2, 91.9, 3.44 | 50.3, 85.8; 20.22, 0.18, 0.33 | 49.8, 82.9; 24.54, 6.59, 19.52 |
| 25 | 8.35 | 46.2, 88.6, 1.02 | 49.0, 85.7, 5.50 | 45.3, 85.3; 20.04, 0.16, 0.29 | 48.3, 85.1; 24.53, 5.83, 17.29 |
| 26 | 8.35 | 49.1, 89.8, 5.61 | 49.2, 87.4, 2.26 | 47.3, 84.6; 19.95, 0.16, 0.31 | 45.5, 85.2; 24.32, 5.54, 16.55 |
| 27 | 8.35 | 49.4, 87.8, 1.05 | 54.5, 102.7, 0.92 | 43.3, 86.1; 20.77, 0.79, 2.15 | 59.9, 85.0; 25.02, 6.54, 19.46 |
| 28 | 8.35 | 48.4, 91.7, 0.44 | 62.7, 95.0, 14.00 | 51.8, 94.0; 22.31, 0.22, 0.38 | 55.0, 95.3; 24.41, 2.30, 6.67 |
| 29 | 8.35 | 46.1, 86.9, 1.53 | 45.9, 85.3, 1.53 | 49.2, 93.8; 23.16, 1.46, 4.17 | 57.5, 102.5; 25.76, 1.97, 5.62 |
