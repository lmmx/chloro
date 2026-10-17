# Benchmarking

chloro's performance work uses two kinds of measurement: instruction counts, which are repeatable on any machine, and wall-time comparisons with rustfmt, which need an idle machine and are compared only as ratios.

The scripts are in `benchmarks/`; `benchmarks/README.md` documents every option. They need a release build of both crates first:

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

`profile.sh` runs the CLI under valgrind's callgrind and reports the instructions executed (`Ir`). Two samples of 49 files are used, and a change is judged on both:

- the **fixture sample**, every 25th file of the conformance corpus (650 KB);
- the **registry sample**, 49 files from tokio, regex-syntax, clap\_builder and winnow (`benchmarks/samples/registry49.txt`, 732 KB), so that a change tuned on one sample is checked on code it was not tuned on.

Counts repeat to within about 2,000 instructions from run to run, so changes far below timing noise are visible. The script does two things to keep the counts stable:

- The script stages the binary and the sample under fixed-length paths and runs with address randomisation off (`setarch -R`) and an empty environment, because path lengths shift the heap's layout and with it the count.
- `--per-file` formats each file in its own process and sums the counts, which removes the effect one file's allocations have on the next file's. A change of less than about 2 million instructions is judged on the per-file figure.

`--phase parse` counts only the parse (`chloro_core::parse_only`), to see the parser's share. `--sim` adds callgrind's cache and branch simulation; `--shuffle N` repeats the count with randomised code layouts, to tell a real change from an effect of code layout.

Needs `valgrind` and `setarch`.

## Against rustfmt: `compare_rustfmt.sh`

```sh
benchmarks/compare_rustfmt.sh --list                              # the workloads
benchmarks/compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3      # the full comparison
benchmarks/compare_rustfmt.sh -r 5 --rounds 2 fx-hir fx-hir-ty    # some workloads
python3 benchmarks/report.py benchmarks/results/<stamp>           # tables from a run
```

The script times rustfmt and chloro with hyperfine on the same files: nine crates (five from rust-analyzer, four from crates.io), 287 small files, a 13-byte file for start-up, single large files through standard input, and in-process runs. For each workload it holds equal:

- **the bytes:** both tools read copies of the same files, by default formatted by rustfmt first, because rustfmt's `--check` builds a diff of every file that would change;
- **the file set:** taken from rustfmt's own list of the files it formats, then verified;
- **the configuration:** defaults for both, with an empty config file for rustfmt;
- **the order:** the command order is reversed in alternate rounds;
- **the output:** before timing, each command runs once and must succeed; whether the two outputs agree is recorded per workload.

It records wall time, CPU time and peak memory for each run under `benchmarks/results/<UTC stamp>/`, and `report.py` turns that into tables. Pinned rows (`--core`) put both tools on one core; unpinned rows let chloro use every core.

Needs `hyperfine` 2.0, `taskset`, `jq`, `python3`, and rustfmt 1.9.0 (the script resolves the real binary behind rustup's proxy).

> [!WARNING]
>
> **Wall time on a shared machine**
>
> Timings of the same binary on the same files varied by 20–40% between measurement sessions on the virtual machine chloro was measured on. Compare tools within one run, never across runs, and treat a wall-time change smaller than the run-to-run spread as no change.

## On any Cargo project: `just vs-rustfmt`

```sh
just vs-rustfmt path/to/project      # chloro, chloro --threads 1 and cargo fmt
just vs-ruff                         # the same on ruff, in ~/tmp/ruff (cloned if missing)
just vs-ruff ~/src/ruff 10           # another checkout, 10 runs
```

These recipes (`.just/bench.just`) give both tools the same files: the list `cargo fmt --all` formats, so each file is formatted once. A plain `rustfmt` over every `.rs` file of a project formats each module file several times, because rustfmt also formats the modules a file declares; on ruff that is 3.6 passes per file. Both tools run in check mode, so use an already-formatted tree. They need `hyperfine`.

They time the `chloro` on your `PATH`, first making sure it was built from this checkout. `just reinstall` installs the CLI with a build ID baked in, which `chloro --version` prints (`chloro 0.9.0-dev (2f9b060, build <id>)`); `just build-id` computes the ID of the checkout as it is on disk, from the content of every file that goes into the binary (committed or not), `Cargo.lock`, the toolchain and the install command. When the two match, the benchmark starts at once without running Cargo; when they differ, or the installed `chloro` has no build ID, the recipe runs `just reinstall` first, outside the timed runs.

## In process: `bench`

```sh
cargo run --release -p chloro-core --example bench -- -n 5                # MB/s on the corpus
cargo run --release -p chloro-core --example bench -- --root ~/.cargo/registry/src
cargo run --release -p chloro-core --example bench -- --phase parse       # the parse alone
```

`bench` times `format_source` over every file, excluding I/O and process start-up, and reports the best of `-n` rounds. It also records and checks output hashes ([Development](<https://lmmx.github.io/chloro/pr-preview/pr-108/maintainers/development/#checks-for-a-change>)).

## Other tools

| tool | what it is for |
| --- | --- |
| `benchmarks/cliffs/` | synthetic inputs of 20 shapes at growing sizes, fitted for how instruction counts scale, to find inputs where the work grows faster than the input |
| `benchmarks/schedule/` | turning a `CHLORO_TRACE` timeline into per-worker idle time, and rebuilding the schedule from CPU clocks |
| `examples/alloc_count.rs` | counting allocations |
| `alloc-timing`, `alloc-mimalloc`, `alloc-jemalloc`, `alloc-snmalloc` features | instrumenting or replacing the global allocator in measurement builds; all off by default |

## Measurement records

Six full measurements against rustfmt (M1 to M6) and the experiments behind each change are in the journal; the [project history](<https://lmmx.github.io/chloro/pr-preview/pr-108/history/index.md>) explains how to find them. The [Performance](<https://lmmx.github.io/chloro/pr-preview/pr-108/performance/index.md>) page summarises the latest.
