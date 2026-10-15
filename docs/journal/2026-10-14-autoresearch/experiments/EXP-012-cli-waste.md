# EXP-012: the CLI's work outside formatting (directory walk, path sorts, executable digest)

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md). The levers are
L4 and L3 of [../../2026-10-15-extensions-theory/incremental-and-process.md](../../2026-10-15-extensions-theory/incremental-and-process.md)
(§ 2, § 3, "Ranked bounds"). Labels: **measured**, **read**, **derived**.

## Identity

| field | value |
|---|---|
| branch | `20261015-cli-waste` from `383fa28` (integration branch `claude/chloro-perf-research-swarm-nrscj8`), worktree `/home/user/wt/cli-waste` |
| base BKG | `bdfb9d1` (`383fa28` adds only documentation after it) |
| commits | `9f3a262` (walk), `de23ab4` (executable digest record), the following commit (this file, ledger row) |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo build --release -p chloro --features cli` (fat LTO, one codegen unit) |
| binaries | `chloro-base` (`383fa28`, 2,017,472 B, sha256 `97523539…`), `chloro-new` (`de23ab4`, 2,043,368 B, sha256 `e593833c…`), both under the scratchpad's `e12/` |
| host | 4 vCPU Xeon @ 2.80GHz, Linux 6.18; another session's `rustc` and `memcheck` ran throughout: load 2.4–5.3, recorded per table. Wall times are **measured, provisional** (protocol § 6); every before/after pair ran in the same `hyperfine` invocation, in both orders |
| scope | `chloro/src/cli*` and `chloro/tests/cli_*.rs` only; `chloro-core` has no diff, so the formatter's output and `Ir` cannot move |

## Hypothesis and falsification

The CLI spends a measurable, avoidable part of every run outside formatting: one
`canonicalize` per discovered file, two sorts of the path list, and a read and BLAKE3 of the
2 MB executable for the cache's tool key. Removing them cuts the warm-cache `--check` of the
1238 fixtures toward the hashing floor and the per-run cache overhead (one-file run with the
cache against `--no-cache`) to well under 1 ms, with identical output, exit codes and cache
invalidation.

Falsified for the walk if the warm mean drops by less than 5 ms or `readlink` is not near 0
(theory L4); for the digest record if the one-file cache-on minus cache-off delta stays above
1 ms (theory L3); for either if any output, exit code, `-v` line (other than the cache file
name) or invalidation case differs.

## Changes

| commit | change | test |
|---|---|---|
| `9f3a262` | `discover_rust_files` canonicalises the root once and walks it as a canonical directory: an entry that is not a symlink is `dir/name` (canonical, since it adds no `.`, `..` or symlink component), its type comes from the listing (`DirEntry::file_type`, `d_type`), and a `.rs` file's size from `DirEntry::metadata` (one `statx` relative to the open directory) into a `SourceFile { path, len }` that `format_all` orders by, instead of a second `metadata` by path. Symlinks, entries of unknown type, a directory whose listing fails, and every entry under a root that cannot be canonicalised keep the old rule (`is_dir` following links, then `canonicalize`). Each directory's entries are visited in name order, so a tree without symlinked directories comes out in `Path` order; `main` sorts and de-duplicates once (`discover_rust_files` no longer sorts). A symlink to a directory already on the walk's stack is not followed | `tests/cli_files.rs`: the existing duplicate test, plus a symlinked directory and a relative symlink out of the root (canonical paths, each file once), a symlinked `.rs` file whose target ends in `.txt` (processed as the target; a symlink named `.txt` is not), a cycle (`self -> .` and `sub/up -> ..`, two files, exit 0), and the processing order of equal-size files (`B.rs`, `a/b.rs`, `a/c/d.rs`, `a/c.rs`, `a-b.rs`, `a.rs`, `z/e.rs`, with `0link -> z`) |
| `de23ab4` | `Cache::open` takes the tool key from `tool_key_in(dir)`: the executable is opened (`/proc/self/exe` on Linux) and its identity taken with `fstat` on the open file: the path `current_exe` names, size, modification time and, on Unix, device, inode and `ctime`. The record `<dir>/<hex>.chloro-exe` (name from BLAKE3 of the identity) holds magic, format version, the identity, the BLAKE3 of the executable's bytes, and a BLAKE3 check of the rest; a matching record gives the digest, anything else reads and hashes the open file and rewrites the record (temporary file and rename, as cache files), unless the identity changed during the read. `tool_key` (read every time) is kept and equals `tool_key_in`. The size and modification-time check before a save (review finding 2) is unchanged. The stale sweep also removes records and their temporary files after 30 days | `cache::tests`: `exe_record_round_trips`, `exe_record_rejects_corruption_and_other_identities` (every single-bit flip), `a_changed_size_or_modification_time_recomputes`, `a_rewrite_with_its_modification_time_restored_recomputes` (Unix: same size, `mtime` put back, `ctime` differs), `a_stale_record_with_a_wrong_digest_is_replaced`, `no_record_is_written_when_the_executable_changes_while_read`, `tool_key_in_is_tool_key`, sweep test extended; `tests/cli_cache.rs`: `a_changed_binary_reads_another_cache_file` (a copy of the binary, run twice, then one byte appended with `mtime` put back, then that byte changed with `mtime` put back: each reads a new cache file, 3 records) |

The cache tests that counted every file in the cache directory now count cache files
(`*.chloro-cache`) and additionally assert one executable record and nothing else; the
`--no-cache` and option-over-environment tests still assert that the directory is empty.

### Why the identity is taken from the open file (measured)

The first version took the identity with `stat("/proc/self/exe")`. Under valgrind that
`stat` returns valgrind's own tool binary (2,482,912 B) while `open("/proc/self/exe")`
gives the client: the record paired the tool's identity with chloro's digest, so a chloro
rebuilt at the same path and run under valgrind again would have hit a stale record. `fstat`
on the opened file makes the identity and the bytes a miss reads the same file; with it the
valgrind runs and the native runs share one record (checked: one `.chloro-exe`, the same
cache file name).

### What the record does not see (derived)

The key keeps its meaning (the digest of the running executable's bytes); the record only
saves recomputing it. It answers wrongly only when the executable's bytes changed while the
identity stayed equal. On Unix every write to a file sets its `ctime` to the current time,
`ctime` cannot be set from user space, and a replacement by rename or copy gives a new inode
or a new `ctime`. What remains is a write that lands within the file system's timestamp
granularity of the previous change *after* a chloro run recorded it (milliseconds with
coarse timestamps; a running executable cannot be opened for writing on Linux), a clock set
back to the previous `ctime`, or a write below the file system (block device, another
system mounting it). On Windows the identity is path, size and modification time: a rewrite
in place with the same size and the modification time restored is not seen. Before, the
same cases were caught by reading the bytes. Compared with today the cache is otherwise as
strict: a record is per identity, so returning to an older binary looks up that binary's own
record; a record for another identity, a corrupted one, or a copy under another name is
rejected by the embedded identity and check; and a writer of the cache directory could
already add entries to cache files. A side effect: a cache-enabled run now creates the cache
directory at open when it writes a record, also on runs that end before the save (for
example a source path that does not exist).

### A cheaper exact alternative (read, derived)

A digest of the final binary embedded at build time by `build.rs` is impossible: `build.rs`
runs before the crate is compiled and linked, and embedding a digest in the binary changes
the bytes it digests (a hash fixed point). A post-link step could zero a placeholder, hash,
and patch the digest in, but `cargo install` runs no post-link step, so the fallback would
still be needed. The ELF build-id (`BuildID[sha1]` is present in this build) is a linker
hash of the output, readable from the mapped image without reading the file, but it depends
on the linker driver passing `--build-id` (this distribution's does; rustc does not ask for
it), survives `strip` and other post-link rewrites that change the bytes, has other forms on
macOS (`LC_UUID`) and Windows (PDB GUID), and reading it needs ELF note parsing or FFI. Not
taken: the identity record is exact up to the case above and portable.

## Measurements

Workloads (scratchpad `e12/`): `fx`, a copy of the 1238 fixtures (14.2 MB) rewritten once by
`chloro-base --write --no-cache`, so `--check` exits 0; `one/one.rs`, 13 bytes
(`fn main() {}`); `tiny`, the 491 files of `rust-analyzer/parser/test_data` from `fx`.
`CHLORO_CACHE_DIR` is `e12/cache`, shared by both binaries (different tool keys, so different
cache files; each holds the `fx` and `one.rs` digests, 1238–1239 entries, so the one-file
run reads and rewrites a 44 KB cache file, as a real cache would).

```text
hyperfine -N -i -w 2 -r 20 --export-json <cell>.json "<bin> <args>" ...      # 10 runs for --no-cache fx
taskset -c 3 hyperfine ...                                                   # 1c rows (one thread: available_parallelism honours the mask)
cells: --check fx | --check --no-cache fx | --check one/one.rs | --check --no-cache one/one.rs | --check tiny
round 1: chloro-base then chloro-new in each invocation; round 2: chloro-new then chloro-base
strace -c -f chloro --check fx ; strace -c -f chloro --check one/one.rs
valgrind --tool=callgrind chloro --check fx | --check one/one.rs | --check --no-cache one/one.rs
callgrind_annotate --threshold=100 --inclusive=yes
```

### Wall time (measured, provisional; ms; min / mean ± sd / max, peak RSS)

| cell | round | `chloro-base` | `chloro-new` | new ÷ base, means | ranges |
|---|---|---|---|---|---|
| warm `--check fx`, mc | 1 (load 4.7) | 25.58 / 28.48 ± 1.64 / 31.80, 4.9 MiB | 14.79 / 17.40 ± 2.35 / 23.22, 4.7 MiB | 0.61 | disjoint |
| | 2 (load 4.2) | 26.10 / 28.13 ± 2.05 / 33.70, 4.8 MiB | 14.01 / 16.69 ± 1.87 / 20.07, 4.6 MiB | 0.59 | disjoint |
| warm `--check fx`, 1c | 1 (load 4.7→4.5) | 31.15 / 34.39 ± 4.46 / 49.44, 4.5 MiB | 21.01 / 26.82 ± 4.05 / 33.54, 4.2 MiB | 0.78 | overlap (33.54 > 31.15) |
| | 2 (load 4.2→4.1) | 33.17 / 36.15 ± 4.11 / 47.81, 4.4 MiB | 21.07 / 21.91 ± 0.84 / 24.57, 4.2 MiB | 0.61 | disjoint |
| `--check one.rs`, cache on, 1c | 1 | 4.08 / 4.68 ± 0.95 / 7.98, 4.5 MiB | 2.22 / 2.42 ± 0.17 / 2.95, 3.4 MiB | 0.52 | disjoint |
| | 2 | 4.29 / 4.61 ± 0.24 / 5.23, 4.4 MiB | 2.52 / 2.65 ± 0.11 / 2.87, 3.4 MiB | 0.57 | disjoint |
| `--check --no-cache one.rs`, 1c | 1 | 1.62 / 1.74 ± 0.08 / 1.92 | 1.62 / 1.80 ± 0.17 / 2.33 | 1.03 | overlap (unchanged path) |
| | 2 | 1.71 / 1.85 ± 0.14 / 2.28 | 1.72 / 1.95 ± 0.24 / 2.59 | 1.05 | overlap |
| `--check one.rs`, cache on, mc | 1 | 3.97 / 4.56 ± 0.94 / 8.01 | 2.32 / 2.48 ± 0.09 / 2.66 | 0.54 | disjoint |
| | 2 | 4.38 / 5.28 ± 1.16 / 8.37 | 2.61 / 2.86 ± 0.15 / 3.09 | 0.54 | disjoint |
| warm `--check tiny` (491 files), mc | 1 | 10.01 / 10.62 ± 0.43 / 11.45 | 4.73 / 5.19 ± 0.35 / 6.26 | 0.49 | disjoint |
| | 2 | 10.42 / 11.58 ± 1.58 / 17.54 | 5.38 / 6.82 ± 1.33 / 10.68 | 0.59 | disjoint |
| warm `--check tiny`, 1c | 1 | 10.38 / 11.36 ± 0.74 / 13.40 | 5.09 / 5.60 ± 0.29 / 6.15 | 0.49 | disjoint |
| | 2 | 10.45 / 11.40 ± 1.33 / 16.07 | 5.94 / 7.37 ± 1.67 / 11.67 | 0.65 | disjoint |
| `--check --no-cache fx`, 1c | 1 (load 5.3→4.2) | 1510.8 / 1572.8 ± 64.2 / 1704.2, 23.3 MiB | 1537.6 / 1609.7 ± 53.8 / 1693.7, 23.5 MiB | 1.02 | overlap |
| | 2 (load 5.0→4.2) | 1513.1 / 1573.2 ± 58.5 / 1670.5 | 1518.3 / 1558.3 ± 30.8 / 1616.3 | 0.99 | overlap |
| `--check --no-cache fx`, mc | 1 (load 4.5→5.3) | 549.0 / 656.7 ± 142.2 / 927.7, 41.9 MiB | 804.0 / 839.5 ± 33.8 / 906.7, 43.4 MiB | (noise) | overlap |
| | 2 (load 4.1→5.0) | 808.3 / 848.8 ± 36.8 / 940.2 | 798.0 / 831.6 ± 18.3 / 860.2 | 0.98 | overlap |

Per-run cache overhead (cache-on minus `--no-cache`, one file, 1c, **derived** from the rows
above): base 2.94 and 2.76 ms in means (2.46 and 2.58 in minima), new 0.62 and 0.70 ms in
means (0.60 and 0.80 in minima). Exit codes were 0 in every run of every cell.

The step-1 baseline alone, before any change (load 2.5–4.2): warm `fx` mc 31.73 / 36.48 ±
3.81 / 46.38, 1c 31.67 / 35.24 ± 4.47 / 47.41; `--no-cache fx` mc 789.9 / 850.4 ± 67.3, 1c
1495.2 / 1575.7 ± 75.2; one file 1c cache on 4.07 / 6.15 ± 2.12, off 1.68 / 1.79 ± 0.09.

### System calls (measured, `strace -c -f`, counts)

| run | syscall | base | new |
|---|---|---|---|
| warm `--check fx` | `readlink` | 8,060 (8,058 `EINVAL`: one per path component of each file) | 4 |
| | `getcwd` | 1,238 | 1 |
| | `statx` | 3,858 | 2,494 |
| | `openat` / `close` / `read` / `getdents64` | 1,378 / 1,378 / 2,493 / 260 | 1,379 / 1,379 / 2,493 / 260 |
| | all | 18,951 | 8,304 |
| `--check one.rs` | all | 154 | 159 (the record's `openat`/`read` and an `fstat` replace one 2 MB `read`) |

The remaining `statx` per file are one `DirEntry::metadata` for the queue order and the
`fstat` of `read_to_string`.

### Instructions (measured, callgrind, one run each; inclusive)

| run, function | base | new |
|---|---|---|
| warm `--check fx`, whole run | 86,950,788 | 63,096,059 (−27.4%) |
| — main thread (`chloro::cli::main`) | 32,719,420 | 8,793,319 |
| — `driftsort_main` of the file list | 18,869,279 (`PathBuf`, two sorts) | 1,619,296 (`SourceFile`, one sort of sorted input) |
| — `std::path::compare_components` | 18,144,489 | 1,576,519 |
| — `std::sys::fs::canonicalize` | 2,478,894 (1,238 calls) | 1,098 (the root) |
| — size `stat`s | 919,456 (`fs::metadata`) | 281,144 (`DirEntry::metadata`) |
| — `blake3::hash` | 48,251,110 | 43,205,761 (the executable's 5.05 M gone; the rest is the 14.2 MB of content) |
| `--check one.rs`, whole run | 6,646,539 | 1,604,092 |
| `--check --no-cache one.rs`, whole run | 491,087 | 490,929 |
| cache overhead (derived: difference of the two) | 6,155,452 | 1,113,163, of which 577,203 is the sort of 1,239 records at save |

## Gates

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test -p chloro --all-features` | 20 unit (13 before), `cli_cache` 11 (10), `cli_files` 5 (1), 1 doc test; all pass |
| output identity, fixtures (fresh copies of the 1238 raw fixtures, one per binary) | `--check --no-cache`, `--check` cold, `--check` warm: exit 1 and `Error: 703 file(s) need formatting` from both; `-v --check` sorted, and `-v --check --threads 1 --no-cache` in order (1,254 lines), identical after the root and the `Cache` lines are normalised; print mode `--threads 1 --no-cache`: 14,190,145 bytes of stdout identical; `--write`, `--check` after it, `--write` again: exit 0, 0, 0 for both; `diff -r` of the written trees: identical (703 files rewritten) |
| output identity, registry (a full copy of `~/.cargo/registry/src`, 720 crates, plus a `links/` directory holding two symlinked crate directories, a symlinked `lib.rs` and a directory with `self -> .`; 24,583 files found by both) | `--check` no-cache, cold, warm: exit 1 and 9,629 files from both; `-v` sorted (39,553 lines) and single-thread in order (24,599 lines) identical; print of the symlinked file identical; `--write` exit 0 for both, `--check` after it exit 1 with 6 files (registry files whose output is not idempotent, the same 6 for both), `--write` again exit 0; `diff -r --no-dereference` of the written trees: identical (9,629 files rewritten) |
| `bench --check` goldens, `conform -i`, `config_matrix`, `benchmarks/profile.sh` | not run: `chloro-core` has no diff, so no formatted output and no formatter instruction can change; the identity runs above compare the CLI's whole output |

## Decision

Kept, for the integrator. The hypothesis holds on every cell it names: the warm `--check` of
the fixtures takes 0.59–0.61 of the base mean on four threads with disjoint ranges in both
rounds (about 28 → 17 ms), 0.61–0.78 on one core (disjoint in round 2, overlapping by 2.4 ms
in round 1); `readlink` falls from 8,060 calls to 4; the one-file cache overhead falls from
2.8–2.9 ms to 0.6–0.7 ms (6.16 M → 1.11 M instructions); the main thread's instructions
from 32.7 M to 8.8 M. The theory's bound for the walk (≥ 0.45 of a 37 ms warm run, 15–17 ms)
is approached but not reached at 11–12 ms saved in mean: the theory's path had 12.5
components per file and this copy 6.5, so `canonicalize` cost less to begin with. Runs
without the cache are unchanged within noise. No output, exit code or `-v` line other than
the cache file name differs on either corpus; invalidation differs only in the documented
residual above.

## What is left

- The rest of the one-file cache overhead (≈ 0.6 ms, 1.1 M instructions) is the cache file
  itself: every run parses all entries into a `HashMap`, sorts them and rewrites the whole
  file (44 KB here) with a new generation. Appending only new digests, or skipping the save
  when nothing new was seen and accepting coarser eviction ages, would remove most of it; it
  changes the file's update rule, so it is a separate experiment.
- The warm run's floor is now the per-file work: `open`, `fstat`, `read`, UTF-8 validation
  (6.5 M instructions), BLAKE3 of 14.2 MB (43 M) and one `statx` per file for the queue
  order. The size `statx` could go if the queue order were taken from `read_to_string`'s
  `fstat` in the workers, but the largest-first order needs the sizes before the first file
  starts.
- `Args::thread_count` calls `available_parallelism` (cgroup reads) each time it is called,
  three times per verbose run.
- A symlink cycle used to be followed until the kernel refused the path, 40 links deep
  (exponential with two links: a directory holding `self -> .` and `up -> ..` did not
  finish within two minutes on the base binary, **measured**); it now ends at the first
  repeat. The set of files is the same whenever the old walk ended (one `self -> .`:
  identical, **measured**).
- Wall times here are provisional (load 4–5 from another session); a quiet-host rerun of
  the warm rows would firm up the ratios.
