# EXP-006: a fixed-point cache in the CLI

## Identity

| field | value |
|---|---|
| branch | `20261014-cli-cache` from `d797292` (integration branch `claude/chloro-perf-research-swarm-nrscj8`) |
| base BKG | `884f9c9` (`d797292` adds only documentation) |
| commits | `6e893f4` (code, tests, README), the following commit (this file, ledger row) |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo build --release --features cli`, default profile |
| host | 4 vCPU Xeon @ 2.80GHz, 15 GB, Linux 6.18; load at measurement time recorded per table |
| scope | `chloro/src/cli/` only; `chloro-core` has no diff |

## Hypothesis and falsification

A persistent cache in the CLI, keyed by the content digest of each file, the effective
`Config`, and the identity of the chloro binary, records "this content is a fixed point of
chloro under this configuration". On an unchanged tree, `chloro --check` and
`chloro --write` then hash every file and run the formatter on none of them, so the run
costs tens of milliseconds on the 1238 fixtures (13.9 MB) instead of the formatting time,
with no change to the formatted output of any file.

Falsified if the key admits a stale hit (a file reported as formatted that the formatter
would change), or if the warm run is not at least 10x faster than formatting on the fixture
tree.

## What is cached

One fact per entry: the BLAKE3 digest (256 bits) of file content that the formatter, in this
binary and under this configuration, returned unchanged. Nothing else: no formatted output,
no paths, no source-file timestamps. A hit therefore replays nothing; it asserts that the
content on disk already equals its own formatting, which is exactly what `--check` and
`--write` need to decide.

Which contents are recorded, by mode (`chloro/src/cli/worker.rs`):

- `--check`, `--write`, print mode: content the formatter returns unchanged (observed fixed
  point);
- `--write` on a file that changed: the written content, only if a second format pass
  returns it unchanged (observed fixed point; the second pass costs one more format of each
  rewritten file, and a file whose output is not idempotent is written but never recorded);
- a file that would change under `--check` or print mode: nothing.

A hit in print mode writes the original bytes to stdout, which equal the formatted output by
the recorded fact.

## Key design and soundness

The output of a format run is a function of three inputs. Each enters the key:

| input | where it enters | how |
|---|---|---|
| file content | entry digest | `blake3(bytes)` of the exact `String` handed to `format_source_with_config` (`cache::content_digest`); 256-bit, collision-resistant, so two contents share a digest only with a BLAKE3 collision |
| configuration | cache file name | `cache::config_key_bytes`: every `Config` field in declaration order, integers as little-endian `u64`, `Option` as a tag byte plus value, enums as one byte each; a destructuring pattern without `..` fails to compile when a field is added, and each enum `match` fails to compile when a variant is added; a unit test pins the 48-byte serialisation of `Config::default()` and checks that every settable option changes the key |
| formatter code | cache file name | `cache::tool_key`: `blake3` of the running executable's bytes (`std::env::current_exe`), with `CARGO_PKG_VERSION`, the cache format version, `std::env::consts::OS` and `ARCH` |

Inputs that do not enter the key, and why they need not:

- the path of the file: `format_source_with_config` takes content and configuration only
  (`chloro-core/src/formatter.rs`); the CLI reads no `rustfmt.toml` and passes no path;
- source-file size or mtime: not used at all, so a touched or copied file is judged by its
  content, and a file edited back to a recorded content is a hit;
- host-dependent configuration (`newline_style = Native`): the target OS is in the tool key,
  and the cache directory lives on the host that runs the binary;
- the number of threads and the chunking: they decide which thread formats a file, not the
  output.

Why the executable digest and not the version string alone: a build from git between
releases keeps the same `CARGO_PKG_VERSION` while the formatter changes. A cache keyed by
version alone would then report files formatted by the previous build as formatted by the
new one and hide output changes between builds. The executable digest changes with every
rebuild that changes any byte of the binary, including rebuilds that change only CLI code or
the compiler (over-invalidation, never under-invalidation). When `current_exe` or reading
it fails, the cache is disabled for the run.

The three inputs select the cache file (tool and configuration) and its entries (content),
so an entry recorded under one binary or configuration is never read under another, and
changing `--config` or rebuilding reads an initially empty file.

Attack surface: a hit requires a 32-byte record equal to the BLAKE3 digest of the file's
content. A corrupt or random record matches nothing. A party who can write the cache
directory can insert the digest of unformatted content and make `--check` pass on it; the
directory is in the user's cache directory, so that party could equally replace the
`chloro` binary.

## On-disk format and location

`<cache-dir>/<32 hex>.chloro-cache`, where the hex is the first 128 bits of
`blake3(tool_key || config_key_bytes)`. The file is a 16-byte header (`CHLOROFP`, format
version `u32`, run generation `u32`, little-endian) followed by 36-byte records: a 32-byte
content digest and the `u32` generation in which it was last seen. Records are sorted, so
the file is a function of its contents. A file whose magic, version, or length does not fit
is ignored and overwritten (tests: `parse_round_trip_and_rejects_corruption`,
`corrupt_cache_file_is_ignored_and_replaced`).

Reading happens once before the workers start; the set is shared read-only across threads.
Workers return the digest to record with their result; the main thread collects them and
writes the file once, to `.<pid>.<nanos>.tmp` in the same directory, then renames it over
the cache file. Two processes writing the same file both leave a complete file; the last
rename wins, and the loser's entries are re-derived on the next run. Entries are capped at
131,072 (4.7 MB per file); over the cap, entries with the lowest last-seen generation are
dropped. When a file is saved, other `*.chloro-cache` files in the directory not written for
30 days are removed, which bounds the files left by superseded binaries and configurations.

Location, first match (`cache::resolve_dir`):

1. `--cache-dir DIR`;
2. `$CHLORO_CACHE_DIR`;
3. `$XDG_CACHE_HOME/chloro`;
4. `%LOCALAPPDATA%\chloro\cache` on Windows;
5. `$HOME/Library/Caches/chloro` on macOS;
6. `$HOME/.cache/chloro`;
7. otherwise the cache is disabled for the run.

The user cache directory is chosen over a directory in the project because the key does not
depend on the path or the project: one cache serves every tree the user formats, nothing is
written into a repository (no `.gitignore` entry, no file next to `target/`), and the
directory is writable wherever `cargo` itself can write its registry.

Default on, `--no-cache` off (ruff's convention), because a hit changes no output and no exit
code, the directory is outside the project, and a run that cannot open or write the cache
continues without it. `-v` prints the cache path, the entry count, `Cache hits: N`, and
whether the file was written. Without `-v`, stdout, stderr and exit codes are unchanged
(test `check_output_and_exit_code_are_unchanged_without_verbose`).

## Alternatives rejected

| alternative | reason |
|---|---|
| key by version string plus cache format version only | a git build between releases keeps the version string and changes the formatter; a stale hit would hide an output change |
| a build-script hash of `chloro-core`'s sources | the path to those sources exists only in the workspace, not in a `cargo install` from crates.io; the executable digest covers both |
| size plus mtime of source files as a pre-check | a hit must be confirmed by content, so the content is read and hashed anyway; a `stat` per file saves nothing. Hashing the whole 13.9 MB corpus costs 3 ms (table below) |
| 64-bit hash (`DefaultHasher`) | birthday bound 2^32 contents on a 64-bit space; the algorithm of `DefaultHasher::new()` is unspecified across Rust versions |
| two 64-bit SipHash passes, dependency-free | 2.3 GB/s against 4.6 GB/s for BLAKE3 on this host; the two passes are not independent keys; no collision-resistance argument |
| `xxhash-rust` xxh3-128 | 14.7 GB/s, one dependency with no transitive deps, but no collision resistance: a crafted file could share a digest with a recorded one. The 2 ms it saves on the corpus is below the run-to-run noise |
| storing formatted output to replay it | the cache would then be a second copy of the tree; a stale entry would write wrong bytes rather than skip a check |
| recording the written content under `--write` without the second pass | the written content is a fixed point only if the formatter is idempotent on it; `conform -i` shows 0 non-idempotent files on the fixtures but the formatter does not guarantee it, so the entry is recorded only when observed |
| one file per entry | one `stat`/`open` per hit instead of one read of a 45 KB file |
| `.chloro_cache` in the current directory | the current directory is not the project root in general; writes a directory into repositories; needs a `.gitignore` entry |
| `blake3` with `features = ["pure"]` (no C toolchain at build) | 2.0 GB/s against 4.6 GB/s; the `cc` build-dependency stays in the tree either way, and cargo itself builds `blake3` with its defaults on every tier-1 target |

## Commands

```sh
# gates 1–3
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --release -p chloro-core --examples && taskset -c 3 target/release/examples/conform -i
cargo test -p chloro-core
cargo test -p chloro --all-features --test cli_cache     # gate 3

# gate 4 (the script is the one in the scratchpad; the commands it runs are these)
cargo build --release -p chloro --features cli
cp -r chloro-core/tests/conformance/fixtures $FF && target/release/chloro --write --no-cache $FF
export CHLORO_CACHE_DIR=$C
hyperfine -N -i -w 1 -r 15 \
  -n cold     --prepare "rm -rf $C" "target/release/chloro --check $T" \
  -n warm     --prepare ""          "target/release/chloro --check $T" \
  -n no-cache --prepare ""          "target/release/chloro --check --no-cache $T"
# peak RSS: python3 rss.py <command>  (getrusage(RUSAGE_CHILDREN).ru_maxrss)

# gate 5
ls -l target/release/chloro
cargo tree -e no-dev -i syn
```

## Results

All wall-clock figures: `hyperfine -N -i -w 1 -r 15`, release binary at the head commit, 4
threads unless `taskset -c 3` is named, host load average 0.5 at the start of the first table
and 2.7 (the previous table's own runs) at the start of the second. Peak RSS is hyperfine's
`Memory` column. "cold" clears the cache directory before every run; "warm" runs after one
populating run; "no-cache" passes `--no-cache`.

### `chloro --check`, 1238 fixture files, 13.9 MB

| workload | run | mean ± σ | min | max | peak RSS |
|---|---|---|---|---|---|
| raw fixtures (702 of 1238 would change) | cold | 823.2 ± 71.0 ms | 732.4 ms | 1013.4 ms | 50.9 MiB |
| raw fixtures | warm (536 hits) | 746.6 ± 64.0 ms | 655.9 ms | 870.0 ms | 51.3 MiB |
| raw fixtures | no-cache | 863.5 ± 53.7 ms | 789.1 ms | 983.9 ms | 51.3 MiB |
| formatted copy (every file a fixed point) | cold | 821.6 ± 53.8 ms | 719.2 ms | 891.1 ms | 51.1 MiB |
| formatted copy | warm (1238 hits) | **32.7 ± 2.2 ms** | 28.5 ms | 35.8 ms | 6.0 MiB |
| formatted copy | no-cache | 819.4 ± 65.3 ms | 661.8 ms | 925.8 ms | 51.3 MiB |
| formatted copy, `taskset -c 3` | warm | 28.9 ± 4.5 ms | 23.0 ms | 36.8 ms | 5.5 MiB |
| formatted copy, `taskset -c 3` | no-cache | 2904.6 ± 201.4 ms | 2663.2 ms | 3470.5 ms | 32.7 MiB |
| formatted copy | `find $FF -name '*.rs'` (directory walk alone) | 4.1 ± 1.1 ms | 3.2 ms | 6.8 ms | 3.1 MiB |

The formatted copy is the tree after `chloro --write --no-cache`; `chloro --check --no-cache`
on it exits 0. On that tree the warm run is 25.1x faster than the no-cache run with 4 threads
(819.4 / 32.7) and 100.7x faster on one core (2904.6 / 28.9). The warm run's time is process
start, the directory walk with one `canonicalize` per file, reading 14.2 MB, hashing it, and
the cache read and write. On the raw tree, 702 files are formatted whether or not the other
536 hit, and the three means lie within one σ of each other.

### Hashing and cache I/O, one core

| measurement | value |
|---|---|
| BLAKE3 over the 1238 fixture files, 13.9 MB, best of 7 | 3.02 ms, 4606 MB/s |
| BLAKE3 `pure` feature, same | 6.84 ms, 2033 MB/s |
| xxh3-128, same | 0.95 ms, 14,711 MB/s |
| two `DefaultHasher` (SipHash-1-3) passes, same | 5.96 ms, 2334 MB/s |
| BLAKE3 of the 2,704,336-byte `chloro` binary (tool key, once per run) | 1.06 ms |
| cache save, 1238 records (44,584 bytes): sort, serialise, write, rename, directory scan | 0.21 ms |
| cache open, 1238 records: read and parse into a `HashMap` | 0.03 ms |
| cache save at the 131,072-entry cap (4.7 MB) | 15.1 ms |
| cache open at the cap | 12.2 ms |

### Binary size (`target/release/chloro`, `cargo build --release --features cli`)

| build | bytes | change |
|---|---|---|
| `d797292` (no cache) | 2,516,048 | |
| this branch, `blake3` default features | 2,704,336 | +188,288 (+7.5%) |
| this branch, `blake3` `features = ["pure"]` | 2,673,624 | +157,576 (+6.3%) |
| this branch, `blake3` `features = ["no_avx512"]` | 2,703,784 | +187,736 (+7.5%) |

`cargo tree -e no-dev -i syn` prints nothing on this branch. `blake3 1.8.7` adds `arrayvec`,
`cfg-if`, `constant_time_eq`, `cpufeatures`, and the build-dependency `cc` (with
`find-msvc-tools`, `shlex`) to the `cli` feature's tree; `chloro-core`'s dependencies are
unchanged.

## Gate results

| gate | command | result |
|---|---|---|
| 1 | `cargo fmt --check` | clean |
| 1 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| 1 | `cargo test --workspace --all-features` | all passing (9 test binaries; 7 unit tests in `cli::cache::tests`, 10 integration tests in `chloro/tests/cli_cache.rs`) |
| 2 | `conform -i` | 1219/1220 identical, 0 not idempotent (`chloro-core` has no diff on this branch) |
| 2 | `cargo test -p chloro-core` | passing (part of the workspace run) |
| 3 | `cargo test -p chloro --all-features --test cli_cache` | 10/10: cold populates and warm hits with the same exit code and report; editing one file invalidates that file only; `--config` invalidates everything and a default-valued option reads the default cache; empty, foreign, truncated and wrong-magic cache files are ignored and rewritten; `--no-cache` reads and writes nothing; `--cache-dir` wins over `CHLORO_CACHE_DIR`; `--write` then re-run reports 0 changed with every file a hit; a file that would change is formatted on every run and leaves the cache file at its 16-byte header; print mode writes identical bytes on a hit and formats under another config; stdout, stderr and exit code without `-v` are byte-identical cold, warm and `--no-cache` |
| 4 | hyperfine tables above | warm 32.7 ms against 819.4 ms no-cache on the formatted tree (25.1x; 100.7x on one core) |
| 5 | `ls -l target/release/chloro`; `cargo tree -e no-dev -i syn` | +188,288 bytes; `syn` absent |

## Decision

Kept. The warm run on a tree of fixed points takes 32.7 ± 2.2 ms against 819.4 ± 65.3 ms
with 4 threads and 2904.6 ms on one core, above the 10x falsification bound by a factor of
2.5 (4 threads) and 10 (one core). No stale-hit construction was found: the three inputs of
`format_source_with_config`'s output each enter the key, the content digest is 256-bit
BLAKE3, and the gate 3 tests exercise the invalidation paths (content, configuration) and
the never-record path (content that would change). Output and exit codes are unchanged
with and without the cache on every tested run; the library is untouched.

Costs: +188 KB of binary (7.5%), five crates in the `cli` feature's dependency tree, 1 ms
per run to hash the executable, 3 ms per 14 MB to hash the files, and one extra format pass
per file rewritten by `--write`.

## Follow-up

- The warm run's remaining 29–33 ms is dominated by the directory walk (4 ms for `find`
  alone, more with a `canonicalize` per file in `discover_rust_files`), process start, and
  reading 14 MB; a profile of the warm run would rank these.
- `fs::read_to_string` followed by hashing the `String` means every file is read whether or
  not it hits; a hit needs only the read and the hash, so nothing is saved there, but the
  read could use a reused buffer per thread.
- With 4 threads the warm run is slower than on one core (32.7 ms against 28.9 ms): thread
  spawn and join cost more than they save when no file is formatted. A threshold (spawn
  threads only when a chunk's formatting work is non-trivial) is unknowable before the
  lookups; a cheaper variant is to hash and look up on the main thread and hand only misses
  to the workers.
- The executable digest costs 1 ms per run. If that matters, the tool key could be cached
  under the executable's path, size and mtime; the trade is one `stat` against one read of
  2.7 MB.
- `edition` enters `Config` and so the key; a future `rustfmt.toml` reader in the CLI would
  feed the same `Config` and need no change to the key, as long as every option it reads
  lands in a `Config` field.
- The 30-day removal of other cache files and the 131,072-entry cap are untested against a
  real multi-month usage pattern; both are measured only in unit tests.
