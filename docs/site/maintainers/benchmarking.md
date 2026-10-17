# Benchmarking

<p class="lede">chloro's performance work uses two kinds of measurement: instruction counts,
which are repeatable on any machine, and wall-time comparisons with rustfmt, which need an
idle machine and are compared only as ratios.</p>

The scripts are in `benchmarks/`; `benchmarks/README.md` documents every option. They need
a release build of both crates first:

```sh
cargo build --release -p chloro --features cli
cargo build --release -p chloro-core --examples
```

## Instruction counts: `profile.sh`

```sh
benchmarks/profile.sh --core 3
benchmarks/profile.sh --core 3 --sample-list benchmarks/samples/registry49.txt
benchmarks/profile.sh --core 3 --runs 3 --per-file
benchmarks/profile.sh --core 3 --runs 3 --per-file --phase parse
```

`profile.sh` runs the CLI under valgrind's callgrind and reports the instructions executed
(`Ir`). Two samples of 49 files are used, and a change is judged on both:

- the **fixture sample**, every 25th file of the conformance corpus (650 KB);
- the **registry sample**, 49 files from tokio, regex-syntax, clap_builder and winnow
  (`benchmarks/samples/registry49.txt`, 732 KB), so that a change tuned on one sample is
  checked on code it was not tuned on.

Counts repeat to within about 2,000 instructions from run to run, so changes far below
timing noise are visible. The script does two things to keep the counts stable:

- The script stages the binary and the sample under fixed-length paths and runs with
  address randomisation off (`setarch -R`) and an empty environment, because path lengths
  shift the heap's layout and with it the count.
- `--per-file` formats each file in its own process and sums the counts, which removes the
  effect one file's allocations have on the next file's. A change of less than about
  2 million instructions is judged on the per-file figure.

`--phase parse` counts only the parse (`chloro_core::parse_only`), to see the parser's
share. `--sim` adds callgrind's cache and branch simulation; `--shuffle N` repeats the count
with randomised code layouts, to tell a real change from an effect of code layout.

Needs `valgrind` and `setarch`.

## Against rustfmt: `compare_rustfmt.sh`

```sh
benchmarks/compare_rustfmt.sh --list                              # the workloads
benchmarks/compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3      # the full comparison
benchmarks/compare_rustfmt.sh -r 5 --rounds 2 fx-hir fx-hir-ty    # some workloads
python3 benchmarks/report.py benchmarks/results/<stamp>           # tables from a run
```

The script times rustfmt and chloro with hyperfine on the same files: nine crates (five from
rust-analyzer, four from crates.io), 287 small files, a 13-byte file for start-up, single
large files through standard input, and in-process runs. For each workload it holds equal:

- **the bytes:** both tools read copies of the same files, by default formatted by rustfmt
  first, because rustfmt's `--check` builds a diff of every file that would change;
- **the file set:** taken from rustfmt's own list of the files it formats, then verified;
- **the configuration:** defaults for both, with an empty config file for rustfmt;
- **the order:** the command order is reversed in alternate rounds;
- **the output:** before timing, each command runs once and must succeed; whether the two
  outputs agree is recorded per workload.

It records wall time, CPU time and peak memory for each run under
`benchmarks/results/<UTC stamp>/`, and `report.py` turns that into tables. Pinned rows
(`--core`) put both tools on one core; unpinned rows let chloro use every core.

Needs `hyperfine` 2.0, `taskset`, `jq`, `python3`, and rustfmt 1.9.0 (the script resolves
the real binary behind rustup's proxy).

!!! warning "Wall time on a shared machine"

    Timings of the same binary on the same files varied by 20–40% between measurement
    sessions on the virtual machine chloro was measured on. Compare tools within one run, never across runs,
    and treat a wall-time change smaller than the run-to-run spread as no change.

## In process: `bench`

```sh
cargo run --release -p chloro-core --example bench -- -n 5                # MB/s on the corpus
cargo run --release -p chloro-core --example bench -- --root ~/.cargo/registry/src
cargo run --release -p chloro-core --example bench -- --phase parse       # the parse alone
```

`bench` times `format_source` over every file, excluding I/O and process start-up, and
reports the best of `-n` rounds. It also records and checks output hashes
([Development](development.md#checks-for-a-change)).

## Other tools

| tool | what it is for |
|---|---|
| `benchmarks/cliffs/` | synthetic inputs of 20 shapes at growing sizes, fitted for how instruction counts scale, to find inputs where the work grows faster than the input |
| `benchmarks/schedule/` | turning a `CHLORO_TRACE` timeline into per-worker idle time, and rebuilding the schedule from CPU clocks |
| `examples/alloc_count.rs` | counting allocations |
| `alloc-timing`, `alloc-mimalloc`, `alloc-jemalloc`, `alloc-snmalloc` features | instrumenting or replacing the global allocator in measurement builds; all off by default |

## Measurement records

Six full measurements against rustfmt (M1 to M6) and the experiments behind each change are
in the journal; the [project history](../history.md) explains how to find them. The
[Performance](../performance.md) page summarises the latest.
