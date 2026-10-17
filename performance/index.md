# Performance

chloro's speed and memory use measured against rustfmt, the conditions of the measurements, and what was not measured.

## Summary

Against rustfmt 1.9.0, checking already-formatted crates:

- **One core:** 0.21–0.35 of rustfmt's wall time on each of nine crates, with every chloro run faster than every rustfmt run.
- **Four cores:** 0.08–0.17 of rustfmt's wall time, chloro using one thread per core and rustfmt running as it does (one thread).
- **Memory:** 4–10 MiB peak resident memory on one core and 5–23 MiB on four, against rustfmt's 43–86 MiB.
- **Unchanged trees:** a warm `--check` of 1238 formatted files in 16.2 ms on four cores.

Against chloro 0.8.0, the release before the performance work, the formatter executes 66% fewer instructions on the same files and formats 2.7 times as many bytes per second on one core.

> [!NOTE]
>
> **Conditions**
>
> All figures are from measurement [M6](<https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-14-autoresearch/measurements/m6-quiet-host.md>) (2026-10-17), taken with `benchmarks/compare_rustfmt.sh` on an idle 4-core KVM guest (Intel Xeon, 2.80 GHz class, Linux), with rustfmt `1.9.0-stable (2d8144b788 2026-07-07)` and chloro built with the workspace's release profile. Each cell is 30 timed runs with hyperfine (15 runs in each of two rounds, the command order reversed in the second), after 2 warm-up runs. Both tools check copies of the same files, formatted by rustfmt first, with `--check`; chloro's cache is off. One-core rows pin both tools to the same core. Ratios are of mean wall times.
>
> Absolute times depend on this host and varied by up to 20–40% between measurement sessions, so compare the ratios, which were taken with both tools interleaved in the same session.

## Per crate

| crate | files | size | ratio, one core | ratio, four cores | rustfmt, one core | chloro, one core |
| --- | --- | --- | --- | --- | --- | --- |
| `hir` | 15 | 646 KB | 0.262 | 0.086 | 327.9 | 85.9 |
| `hir-ty` | 122 | 2876 KB | 0.252 | 0.077 | 1021.9 | 257.9 |
| `ide-db` | 33 | 1169 KB | 0.214 | 0.084 | 369.8 | 79.1 |
| `syntax` | 33 | 405 KB | 0.324 | 0.112 | 145.7 | 47.2 |
| `parser` | 28 | 235 KB | 0.323 | 0.117 | 82.5 | 26.6 |
| tokio 1.53.3 | 49 | 215 KB | 0.318 | 0.165 | 54.4 | 17.3 |
| regex-syntax 0.8.11 | 33 | 1647 KB | 0.324 | 0.111 | 543.3 | 175.9 |
| clap\_builder 4.6.7 | 57 | 944 KB | 0.318 | 0.126 | 210.2 | 66.9 |
| winnow 0.7.15 | 61 | 1005 KB | 0.345 | 0.127 | 211.5 | 73.0 |

The ratios are chloro's mean wall time divided by rustfmt's; the last two columns are mean wall times in milliseconds. The first five crates are from rust-analyzer, whose files make up chloro's conformance corpus. The other four come from crates.io and were not used during chloro's development.

## A large workspace: ruff

A single measurement of the whole [ruff](<https://github.com/astral-sh/ruff>) workspace, taken with `just vs-ruff` ([Benchmarking](<https://lmmx.github.io/chloro/maintainers/benchmarking/#on-any-cargo-project-just-vs-rustfmt>)) on a 20-core workstation, against `cargo fmt` with the rustfmt ruff's toolchain pins, `1.10.0-stable (b940084d7e 2026-09-28)`. Both tools check the 2,008 files `cargo fmt --all` formats, in an already-formatted checkout, with hyperfine: 5 timed runs each after 1 warm-up, not pinned to cores, chloro with its cache off.

| command | wall time, mean ± σ | share of `cargo fmt`'s | peak memory |
| --- | --- | --- | --- |
| `chloro --check`, 20 threads (the default) | 212.3 ± 4.8 ms | 0.038 | 89.9 MiB |
| `chloro --check --threads 1` | 1598.3 ± 12.6 ms | 0.284 | 16.2 MiB |
| `cargo fmt --all -- --check` | 5633.0 ± 104.8 ms | 1 | 194.5 MiB |

- The one-thread row is the like-for-like comparison, since rustfmt formats on one thread: 0.28 of `cargo fmt`'s time, inside the one-core range of the nine crates above, though on a different machine and with fewer runs.
- Peak memory is that of the largest single process, which for `cargo fmt` is rustfmt.
- The recipe passes the file list to chloro through `xargs`, which can split a list this long into several chloro runs, one after another; their start-up and lost parallelism count against chloro ([\#99](<https://github.com/lmmx/chloro/issues/99>)).
- The outputs agree on all but one file. In a separate check of ruff at commit `8d122c1` (2,023 files) with the same rustfmt, `cargo fmt --check` found nothing to change and `chloro --check` reported one file, which has three `\r\n` line endings among `\n` ones: rustfmt counts it as formatted and leaves it as it is, and chloro 0.9.0 rewrote it. chloro now decides as rustfmt does ([\#97](<https://github.com/lmmx/chloro/issues/97>)).

## Other measurements

| quantity | chloro | rustfmt 1.9.0 |
| --- | --- | --- |
| in-process throughput, one core, 1238 files (13.9 MB), best of 5 | 12.0–13.2 MB/s; 14.5–15.1 profile-guided; chloro 0.8.0 4.4–4.9 in the same window | not measured in process |
| 287 small files in one invocation (rust-analyzer's `parser/test_data`) | 0.23 of rustfmt's wall time on one core, 0.12 on four |  |
| one 13-byte file (process start-up) | 1.8 ms, one core | 11.7 ms |
| `--check` of an unchanged 1238-file tree, warm cache | 16.2 ms on four cores, 32.1 ms on one | no cache |
| the same with `--no-cache` | 356 ms on four cores, 1290 ms on one |  |
| one file of 128 KiB or more, four threads against one | 1.1–1.7 times as fast on the five largest corpus files, for 1.09–1.32 times the CPU | single-threaded |
| a 2,000-byte file of closures nested 16 deep | 5.5 M instructions; chloro 0.8.0: 242.5 M |  |

## Instruction counts

Wall-time measurements on a shared virtual machine are noisy, so chloro's performance work was guided by the number of instructions executed, which barely varies between runs. The counts come from callgrind (`benchmarks/profile.sh`) on two fixed samples of 49 files each, one from the conformance corpus and one from four crates.io crates:

| sample | chloro 0.8.0 | now | profile-guided build |
| --- | --- | --- | --- |
| 49 rust-analyzer files (650 KB) | 587,988,576 | 202,822,485 (−65.5%) | 170,069,599 (−71.1%) |
| 49 crates.io files (732 KB) | 688,589,271 | 255,681,198 (−62.9%) | 219,987,557 (−68.1%) |

Of what remains, lexing and parsing are 34% of the instructions on the first sample and 42% on the second.

## Profile-guided builds

`benchmarks/pgo.sh` builds chloro with profile-guided optimisation, training on the conformance corpus. On these measurements the profile-guided build ran in 0.18–0.33 of rustfmt's wall time on one core and 0.07–0.16 on four. The release workflow applies the same training to the targets whose runners execute them natively; see [Releasing](<https://lmmx.github.io/chloro/maintainers/releasing/#profile-guided-builds>).

## What was not measured

- macOS and Windows: the measurements other than ruff's are from x86-64 Linux.
- rustfmt's throughput in process: rustfmt is only timed as a process, including its start-up (loading `librustc_driver` costs it about 30 ms and 34 MiB).
- Memory bandwidth. Apart from the ruff measurement, nothing was measured on more than four cores.
- Unformatted input. The inputs are formatted by rustfmt first, because rustfmt's `--check` builds a diff of every file that would change, and that would add the cost of the diff to rustfmt's time.

## Optimisations

chloro makes the same formatting decisions as rustfmt, so it does the same work, with less overhead. It reads syntax from one flat array instead of pointer-linked nodes, lexes and builds the tree in one pass, memoises rewrites that deep nesting would otherwise repeat, proves from a node's tokens when a one-line layout cannot fit instead of rendering it to find out, keeps short-lived strings in a per-thread arena, formats files and large files' items on all cores, and skips files it has already seen formatted. [How it works](<https://lmmx.github.io/chloro/architecture/overview/index.md>) explains each of these.

## Reproducing

Needs `hyperfine` 2.0, `taskset`, `jq` and `python3`; `valgrind` and `setarch` for instruction counts; and rustfmt 1.9.0 on `PATH`.

```sh
cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples
benchmarks/compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3   # wall, CPU, RSS per workload; tables in report.md
benchmarks/profile.sh --core 3                                 # instruction count on the 49-file sample
cargo run --release -p chloro-core --example bench -- -n 5     # in-process MB/s on the corpus
```

[Benchmarking](<https://lmmx.github.io/chloro/maintainers/benchmarking/index.md>) describes the workloads and the scripts. The six measurement records, M1 to M6, and the experiments behind each change are in the [project history](<https://lmmx.github.io/chloro/history/index.md>).
