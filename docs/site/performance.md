# Performance

<p class="lede">chloro does the work rustfmt does, in less time and memory. These are the
measured numbers, the conditions they were measured under, and what was not measured.</p>

## Summary

Against rustfmt 1.9.0, checking already-formatted crates:

- **One core:** 0.21–0.35 of rustfmt's wall time on each of nine crates, with every chloro
  run faster than every rustfmt run.
- **Four cores:** 0.08–0.17 of rustfmt's wall time, chloro using one thread per core and
  rustfmt running as it does (one thread).
- **Memory:** 4–10 MiB peak resident memory on one core and 5–23 MiB on four, against
  rustfmt's 43–86 MiB.
- **Unchanged trees:** a warm `--check` of 1238 formatted files in 16.2 ms on four cores.

Against chloro 0.8.0, the release before the performance work, the formatter executes 66%
fewer instructions on the same files and formats 2.7 times as many bytes per second on one
core.

!!! info "Conditions"

    All figures are from the sixth quiet-host measurement ([M6][m6], 2026-10-17), taken with
    `benchmarks/compare_rustfmt.sh` on an idle 4-core KVM guest (Intel Xeon, 2.80 GHz class,
    Linux), with rustfmt `1.9.0-stable (2d8144b788 2026-07-07)` and chloro built with the
    workspace's release profile. Each cell is 30 timed runs with hyperfine (15 runs in each of
    two rounds, the command order reversed in the second), after 2 warm-up runs. Both tools
    check copies of the same files, formatted by rustfmt first, with `--check`; chloro's cache
    is off. One-core rows pin both tools to the same core. Ratios are of mean wall times.

    Absolute times are this host's and move between measurement windows by up to 20–40%,
    so the comparisons that matter are the ratios, taken with both tools interleaved in the
    same window.

## Per crate

<div class="chl-chart" markdown="0">
<svg viewBox="0 0 680 398" role="img" aria-labelledby="perf-chart-title perf-chart-desc">
<title id="perf-chart-title">chloro's wall time as a fraction of rustfmt 1.9.0's, per crate</title>
<desc id="perf-chart-desc">For each of nine crates, two bars: chloro on one core and chloro on four cores, as a fraction of rustfmt's wall time (rustfmt = 1). One core ranges from 0.21 to 0.35; four cores from 0.08 to 0.17. The same values are in the table below.</desc>
<rect x="168" y="10" width="12" height="12" rx="2" class="s1"/><text x="186" y="20" class="sub">chloro, one core</text>
<rect x="298" y="10" width="12" height="12" rx="2" class="s2"/><text x="316" y="20" class="sub">chloro, four cores (one thread per core)</text>
<line x1="168.0" y1="38" x2="168.0" y2="372" class="grid"/>
<text x="168.0" y="388" text-anchor="middle" class="sub">0</text>
<line x1="286.0" y1="38" x2="286.0" y2="372" class="grid"/>
<text x="286.0" y="388" text-anchor="middle" class="sub">0.25</text>
<line x1="404.0" y1="38" x2="404.0" y2="372" class="grid"/>
<text x="404.0" y="388" text-anchor="middle" class="sub">0.5</text>
<line x1="522.0" y1="38" x2="522.0" y2="372" class="grid"/>
<text x="522.0" y="388" text-anchor="middle" class="sub">0.75</text>
<line x1="640.0" y1="38" x2="640.0" y2="372" class="grid"/>
<text x="640.0" y="388" text-anchor="end" class="sub">rustfmt = 1</text>
<text x="158" y="55" text-anchor="end" class="label">rust-analyzer hir</text>
<text x="158" y="68" text-anchor="end" class="sub">15 files, 646 KB</text>
<g><title>rust-analyzer hir, one core: 0.262 of rustfmt's wall time</title><path d="M168,44 h120.7 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-120.7 z" class="s1"/><rect x="168" y="43" width="472" height="14" fill="transparent"/></g>
<text x="297.7" y="54" class="value">0.26</text>
<g><title>rust-analyzer hir, four cores: 0.086 of rustfmt's wall time</title><path d="M168,58 h37.6 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-37.6 z" class="s2"/><rect x="168" y="57" width="472" height="14" fill="transparent"/></g>
<text x="214.6" y="68" class="value">0.09</text>
<text x="158" y="91" text-anchor="end" class="label">rust-analyzer hir-ty</text>
<text x="158" y="104" text-anchor="end" class="sub">122 files, 2.9 MB</text>
<g><title>rust-analyzer hir-ty, one core: 0.252 of rustfmt's wall time</title><path d="M168,80 h115.9 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-115.9 z" class="s1"/><rect x="168" y="79" width="472" height="14" fill="transparent"/></g>
<text x="292.9" y="90" class="value">0.25</text>
<g><title>rust-analyzer hir-ty, four cores: 0.077 of rustfmt's wall time</title><path d="M168,94 h33.3 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-33.3 z" class="s2"/><rect x="168" y="93" width="472" height="14" fill="transparent"/></g>
<text x="210.3" y="104" class="value">0.08</text>
<text x="158" y="127" text-anchor="end" class="label">rust-analyzer ide-db</text>
<text x="158" y="140" text-anchor="end" class="sub">33 files, 1.2 MB</text>
<g><title>rust-analyzer ide-db, one core: 0.214 of rustfmt's wall time</title><path d="M168,116 h98.0 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-98.0 z" class="s1"/><rect x="168" y="115" width="472" height="14" fill="transparent"/></g>
<text x="275.0" y="126" class="value">0.21</text>
<g><title>rust-analyzer ide-db, four cores: 0.084 of rustfmt's wall time</title><path d="M168,130 h36.6 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-36.6 z" class="s2"/><rect x="168" y="129" width="472" height="14" fill="transparent"/></g>
<text x="213.6" y="140" class="value">0.08</text>
<text x="158" y="163" text-anchor="end" class="label">rust-analyzer syntax</text>
<text x="158" y="176" text-anchor="end" class="sub">33 files, 405 KB</text>
<g><title>rust-analyzer syntax, one core: 0.324 of rustfmt's wall time</title><path d="M168,152 h149.9 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-149.9 z" class="s1"/><rect x="168" y="151" width="472" height="14" fill="transparent"/></g>
<text x="326.9" y="162" class="value">0.32</text>
<g><title>rust-analyzer syntax, four cores: 0.112 of rustfmt's wall time</title><path d="M168,166 h49.9 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-49.9 z" class="s2"/><rect x="168" y="165" width="472" height="14" fill="transparent"/></g>
<text x="226.9" y="176" class="value">0.11</text>
<text x="158" y="199" text-anchor="end" class="label">rust-analyzer parser</text>
<text x="158" y="212" text-anchor="end" class="sub">28 files, 235 KB</text>
<g><title>rust-analyzer parser, one core: 0.323 of rustfmt's wall time</title><path d="M168,188 h149.5 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-149.5 z" class="s1"/><rect x="168" y="187" width="472" height="14" fill="transparent"/></g>
<text x="326.5" y="198" class="value">0.32</text>
<g><title>rust-analyzer parser, four cores: 0.117 of rustfmt's wall time</title><path d="M168,202 h52.2 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-52.2 z" class="s2"/><rect x="168" y="201" width="472" height="14" fill="transparent"/></g>
<text x="229.2" y="212" class="value">0.12</text>
<text x="158" y="235" text-anchor="end" class="label">tokio</text>
<text x="158" y="248" text-anchor="end" class="sub">49 files, 215 KB</text>
<g><title>tokio, one core: 0.318 of rustfmt's wall time</title><path d="M168,224 h147.1 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-147.1 z" class="s1"/><rect x="168" y="223" width="472" height="14" fill="transparent"/></g>
<text x="324.1" y="234" class="value">0.32</text>
<g><title>tokio, four cores: 0.165 of rustfmt's wall time</title><path d="M168,238 h74.9 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-74.9 z" class="s2"/><rect x="168" y="237" width="472" height="14" fill="transparent"/></g>
<text x="251.9" y="248" class="value">0.17</text>
<text x="158" y="271" text-anchor="end" class="label">regex-syntax</text>
<text x="158" y="284" text-anchor="end" class="sub">33 files, 1.6 MB</text>
<g><title>regex-syntax, one core: 0.324 of rustfmt's wall time</title><path d="M168,260 h149.9 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-149.9 z" class="s1"/><rect x="168" y="259" width="472" height="14" fill="transparent"/></g>
<text x="326.9" y="270" class="value">0.32</text>
<g><title>regex-syntax, four cores: 0.111 of rustfmt's wall time</title><path d="M168,274 h49.4 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-49.4 z" class="s2"/><rect x="168" y="273" width="472" height="14" fill="transparent"/></g>
<text x="226.4" y="284" class="value">0.11</text>
<text x="158" y="307" text-anchor="end" class="label">clap_builder</text>
<text x="158" y="320" text-anchor="end" class="sub">57 files, 944 KB</text>
<g><title>clap_builder, one core: 0.318 of rustfmt's wall time</title><path d="M168,296 h147.1 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-147.1 z" class="s1"/><rect x="168" y="295" width="472" height="14" fill="transparent"/></g>
<text x="324.1" y="306" class="value">0.32</text>
<g><title>clap_builder, four cores: 0.126 of rustfmt's wall time</title><path d="M168,310 h56.5 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-56.5 z" class="s2"/><rect x="168" y="309" width="472" height="14" fill="transparent"/></g>
<text x="233.5" y="320" class="value">0.13</text>
<text x="158" y="343" text-anchor="end" class="label">winnow</text>
<text x="158" y="356" text-anchor="end" class="sub">61 files, 1.0 MB</text>
<g><title>winnow, one core: 0.345 of rustfmt's wall time</title><path d="M168,332 h159.8 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-159.8 z" class="s1"/><rect x="168" y="331" width="472" height="14" fill="transparent"/></g>
<text x="336.8" y="342" class="value">0.34</text>
<g><title>winnow, four cores: 0.127 of rustfmt's wall time</title><path d="M168,346 h56.9 a3,3 0 0 1 3,3 v6 a3,3 0 0 1 -3,3 h-56.9 z" class="s2"/><rect x="168" y="345" width="472" height="14" fill="transparent"/></g>
<text x="233.9" y="356" class="value">0.13</text>
</svg>
</div>

| crate | files | size | ratio, one core | ratio, four cores | rustfmt, one core | chloro, one core |
|---|---|---|---|---|---|---|
| `hir` | 15 | 646 KB | 0.262 | 0.086 | 327.9 | 85.9 |
| `hir-ty` | 122 | 2876 KB | 0.252 | 0.077 | 1021.9 | 257.9 |
| `ide-db` | 33 | 1169 KB | 0.214 | 0.084 | 369.8 | 79.1 |
| `syntax` | 33 | 405 KB | 0.324 | 0.112 | 145.7 | 47.2 |
| `parser` | 28 | 235 KB | 0.323 | 0.117 | 82.5 | 26.6 |
| tokio 1.53.3 | 49 | 215 KB | 0.318 | 0.165 | 54.4 | 17.3 |
| regex-syntax 0.8.11 | 33 | 1647 KB | 0.324 | 0.111 | 543.3 | 175.9 |
| clap_builder 4.6.7 | 57 | 944 KB | 0.318 | 0.126 | 210.2 | 66.9 |
| winnow 0.7.15 | 61 | 1005 KB | 0.345 | 0.127 | 211.5 | 73.0 |

The first five crates are rust-analyzer's. The ratios are chloro's mean wall time over
rustfmt's; the last two columns are mean wall times in milliseconds.

Five of the crates are from rust-analyzer, whose files are chloro's conformance corpus; the
other four come from crates.io, code chloro was not developed against.

## Other measurements

| quantity | chloro | rustfmt 1.9.0 |
|---|---|---|
| in-process throughput, one core, 1238 files (13.9 MB), best of 5 | 12.0–13.2 MB/s; 14.5–15.1 profile-guided; chloro 0.8.0 4.4–4.9 in the same window | not measured in process |
| 287 small files in one invocation (rust-analyzer's `parser/test_data`) | 0.23 of rustfmt's wall time on one core, 0.12 on four | |
| one 13-byte file (process start-up) | 1.8 ms, one core | 11.7 ms |
| `--check` of an unchanged 1238-file tree, warm cache | 16.2 ms on four cores, 32.1 ms on one | no cache |
| the same with `--no-cache` | 356 ms on four cores, 1290 ms on one | |
| one file of 128 KiB or more, four threads against one | 1.1–1.7 times as fast on the five largest corpus files, for 1.09–1.32 times the CPU | single-threaded |
| a 2,000-byte file of closures nested 16 deep | 5.5 M instructions; chloro 0.8.0: 242.5 M | |

## Instruction counts

Wall time on a shared virtual machine is noisy; the number of instructions executed is not.
chloro's performance work was steered by instruction counts from callgrind
(`benchmarks/profile.sh`) on two fixed samples of 49 files each, one from the conformance
corpus and one from four crates.io crates:

| sample | chloro 0.8.0 | now | profile-guided build |
|---|---|---|---|
| 49 rust-analyzer files (650 KB) | 587,988,576 | 202,822,485 (−65.5%) | 170,069,599 (−71.1%) |
| 49 crates.io files (732 KB) | 688,589,271 | 255,681,198 (−62.9%) | 219,987,557 (−68.1%) |

Of what remains, lexing and parsing are 34% of the instructions on the first sample and 42%
on the second.

## Profile-guided builds

`benchmarks/pgo.sh` builds chloro with profile-guided optimisation, training on the
conformance corpus. On these measurements the profile-guided build ran in 0.18–0.33 of
rustfmt's wall time on one core and 0.07–0.16 on four. The release workflow applies the same
training to the targets whose runners execute them natively; see
[Releasing](maintainers/releasing.md#profile-guided-builds).

## Not measured, and not claimed

- macOS and Windows: every figure here is from x86-64 Linux.
- rustfmt's throughput in process: rustfmt is only timed as a process, including its
  start-up (loading `librustc_driver` costs it about 30 ms and 34 MiB).
- Memory bandwidth, and runs on more than four cores.
- Unformatted input. The inputs are formatted by rustfmt first, because rustfmt's `--check`
  builds a diff of every file that would change, which would charge it for the diff rather
  than the formatting.

## Why it is faster

chloro makes the same decisions rustfmt does, so it cannot skip rustfmt's work; it does
that work more cheaply. In short: it reads syntax from one flat array instead of
pointer-linked nodes, lexes and builds the tree in one pass, memoises rewrites that deep
nesting would otherwise repeat, proves from a node's tokens when a one-line layout cannot
fit instead of rendering it to find out, keeps short-lived strings in a per-thread arena,
formats files and large files' items on all cores, and skips files it has already seen
formatted. [How it works](architecture/overview.md) explains each of these.

## Reproducing

Needs `hyperfine` 2.0, `taskset`, `jq` and `python3`; `valgrind` and `setarch` for
instruction counts; and rustfmt 1.9.0 on `PATH`.

```sh
cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples
benchmarks/compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3   # wall, CPU, RSS per workload; tables in report.md
benchmarks/profile.sh --core 3                                 # instruction count on the 49-file sample
cargo run --release -p chloro-core --example bench -- -n 5     # in-process MB/s on the corpus
```

[Benchmarking](maintainers/benchmarking.md) describes the workloads and the scripts. The
six measurement records, M1 to M6, and the experiments behind each change are in the
[project history](history.md).

[m6]: https://github.com/lmmx/chloro/blob/master/docs/journal/2026-10-14-autoresearch/measurements/m6-quiet-host.md
