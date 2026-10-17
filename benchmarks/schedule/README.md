# Schedule diagnostic: where the four-core wall time goes

The diagnostic of EXP-023
(`docs/journal/2026-10-14-autoresearch/experiments/EXP-023-unit-queue.md`): per file, the
start and end on its worker, its size and whether its items were split; per worker, its span;
per run, the phases (arguments, cache open, walk, sort, read, hash, format, output, cache
save); and from these the tail (first worker idle to the end of formatting), the idle core
time and its owner. The CLI writes the timeline itself when `CHLORO_TRACE=<file>` is set
(`chloro/src/cli/trace.rs`, wall-clock stamps only). A build patched by `trace_patch.py`
adds each thread's CPU clock and the in-file parallel path's units, from which `sim.py`
rebuilds the schedule as it would run on four idle cores: that view does not depend on
what else the host is running.

| file | does |
|---|---|
| `collect.sh` | runs `diag.py` on the nine crate workloads in six configurations (4 threads and 1 pinned thread; no cache, cold cache, `--write` on raw copies) |
| `diag.py` | runs a traced binary N times, keeps the traces, derives the wall view per run and prints the medians |
| `trace_patch.py` | the diagnostic build's patch (thread CPU clocks, parse/units/stitch/unit records in `parallel.rs`); applied to a scratch worktree, never committed; variant `owner` for EXP-029's lever (a) at `0a6080c` |
| `sim.py` | from patched traces: today's scheduler under processor sharing (A), a unit queue across files (B) and the work-conserving bound (C), all from on-CPU costs |
| `split_sim.py` | EXP-029: from patched traces, a split file's parts (parse, the rest of its worker's time, units, stitch) and the replay of the levers on its own schedule (the worker formats units, capped or dynamic item threads, units cut below the item, a parallel parse); `--blind` recomputes the wall view without the item workers, as an unpatched trace sees it; `--compare` reads `collect_ab.sh` |
| `collect_ab.sh` | several traced binaries on the nine crates, interleaved in blocks (same load window) |
| `report.py` | the record's tables from a `collect.sh` directory (`d1` measured, `d2` reconstructed, `d4` phases) |
| `cpu_split.py` | user and system CPU, faults and context switches at 1 thread on one core, 4 threads on one core and 4 threads on four cores, interleaved |
| `identity.sh` | a CLI's output against another's (`--check`, `--write`, print; 1, 2 and 4 threads; crates, raw crates and the fixtures tree) |

## Run

```sh
cargo build --release -p chloro --features cli
CHLORO_TRACE=/tmp/t.tsv target/release/chloro --check --no-cache /tmp/chloro-bench/formatted/fx-hir-ty/tree

# the CPU-clock build, in a scratch worktree
python3 benchmarks/schedule/trace_patch.py /path/to/scratch-worktree base
(cd /path/to/scratch-worktree && cargo build --release -p chloro --features cli && cp target/release/chloro /tmp/chloro-diag)

benchmarks/schedule/collect.sh /tmp/chloro-diag /tmp/e23            # 20 runs at 4 threads, 10 at 1
python3 benchmarks/schedule/report.py /tmp/e23 d1 d2 d4
python3 benchmarks/schedule/cpu_split.py target/release/chloro 10 fx-hir-ty reg-tokio
benchmarks/schedule/identity.sh /path/to/chloro-base target/release/chloro /tmp/e23-identity
```

## Trace format

Tab-separated, nanoseconds since `main` was entered:

- `mark NAME T`: the main thread's phases, in order `args`, `cache`, `walk`, `sort`, `spawn`,
  `joined`, `formatted`, `saved`, `finish`;
- `file WORKER BYTES SPLIT START READ HASHED FORMATTED END`: `HASHED` is 0 without the cache,
  `FORMATTED` is 0 on a cache hit, `SPLIT` is 1 when the file's items went to several threads;
- `worker ID START END`: a worker from its first instruction to its exit.

The patched build adds `cpu START NS` after each file (its worker's CPU time over the file),
a fifth field on `worker` lines (the worker's CPU time) and `item KIND START END NS N` lines.
An unpatched trace has no `item` lines, so its wall view counts a split file's whole span as
one busy core and its item workers as idle: its idle core time in the tail overstates the
loss whenever a split file is running then (EXP-029: on the same quiet-host traces, 10.6
core-ms blind against 2.2 seen on `fx-hir`, 6.7 against 5.3 on `fx-ide-db`; `split_sim.py
--blind`). Read the tail of a crate with a split file from a patched trace.
On a loaded host the wall view mixes the schedule with waiting for a core, and the waiting
shows up as gaps between a worker's files (in EXP-023, 0.9 ms over 122 files with one thread
alone on a core, 84–131 ms per worker with four threads sharing one core). Read the wall view
on a quiet host and the reconstruction anywhere.
