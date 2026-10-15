# benchmarks

Measurement harness for chloro against `rustfmt 1.9.0 --edition 2024`, and for one chloro
build against another. Every speed figure the research programme states comes from these
scripts (see `docs/journal/2026-10-14-autoresearch/protocol.md` §3); the quiet-host runs
the README quotes are under `docs/journal/2026-10-14-autoresearch/measurements/`:
`m1-quiet-host.md`, `m2-quiet-host.md` and `m3-quiet-host.md` (the README's figures).

| file | measures |
|---|---|
| `compare_rustfmt.sh` | process wall time, CPU time and peak RSS of rustfmt and chloro on named workloads, with hyperfine (`--export-json`) |
| `report.py` | turns a results directory into markdown tables with chloro/rustfmt ratios |
| `profile.sh` | callgrind total instructions (`Ir`) of `examples/bench` on the fixed 49-file sample |

Requirements: `hyperfine` 2.0 (the JSON schema below is 2.0's), `taskset`, `jq`, `python3`
(standard library only) for the report, `valgrind` and `setarch` for `profile.sh`, the
release binaries (`cargo build --release -p chloro --features cli` and `cargo build
--release -p chloro-core --examples`). The scripts exit with status 2 and a message when a
tool, binary or workload source is missing.

## Running

```sh
benchmarks/compare_rustfmt.sh --list                          # the workload table
benchmarks/compare_rustfmt.sh                                 # everything, 10 runs, 1 round
benchmarks/compare_rustfmt.sh -r 5 --rounds 2 fx-hir fx-hir-ty  # 10 runs per command, interleaved
benchmarks/compare_rustfmt.sh --groups b,c --core 3           # single-core workloads only
benchmarks/compare_rustfmt.sh --mode stdout fx-hir            # formatted text to stdout, no --check
benchmarks/compare_rustfmt.sh --input raw fx-hir              # inputs as found (rustfmt --check prints diffs)
benchmarks/compare_rustfmt.sh --baseline /path/to/other/target/release --groups b,e
python3 benchmarks/report.py benchmarks/results/<stamp>       # tables again (also written to report.md)
benchmarks/profile.sh --core 3                                # Ir on the sample
```

Results go to `benchmarks/results/<UTC stamp>/` unless `--out` is given:
`env.txt` (commit, host, toolchain, binary hashes, options, load average at start and end, and the `/proc/stat` busy, idle and steal ticks of `--core` over the run),
`commands.txt` (every hyperfine command line as run), `<workload>.<variant>.r<k>.json`
(one hyperfine export per round), `<workload>.<variant>.meta` (file count, bytes, output
agreement), `*.validate.log` (exit codes of the validation run), `*.bench.txt`
(`examples/bench` lines) and `report.md`.

chloro builds with the fixed-point cache (E6) get `--no-cache` on every command unless
`--with-cache` is given: on formatted input the warmup runs would otherwise fill the cache
and the chloro column would measure cache hits, not the formatter.

Workload copies are cached in `--work` (default `${TMPDIR:-/tmp}/chloro-bench/<input>/`);
`--fresh` rebuilds them.

## Workloads

Defined in the `WORKLOADS` table at the top of `compare_rustfmt.sh`; one line per
workload (name, kind, source, crate root). Groups select what runs:

| group | kind, variant | rustfmt command | chloro command | pinning |
|---|---|---|---|---|
| a | crate, `mc` | `rustfmt --check <copy>/<crate root>` | `chloro --check <copy>` | `--cpus` (default unpinned) |
| b | crate, `1c` | same | same | both on `--core` |
| c | stdin, `1c` | `rustfmt` < file > null | `examples/fmt_stdin` < file > null | both on `--core` |
| d | files, `mc` and `1c` | `rustfmt --check f1 f2 ...` | `chloro --check <copy>` | as a and b |
| e | inproc | – | `examples/bench -n 5` (text), `examples/conform -i` (hyperfine) | `--core` |

Every rustfmt command also carries `--edition 2024 --config-path <empty file>`.

- **crate** workloads: fixture crates of rust-analyzer (`hir` 624 KB, `hir-ty` 2.8 MB,
  `ide-db`, `syntax`, `parser`) and registry crates (`tokio`, `regex-syntax`,
  `clap_builder`, `winnow`).
- **files**: `tiny-test-data` is `parser/test_data` filtered to the files rustfmt parses on
  their own (each is its own root with no out-of-line module); `tiny-one` is one 13-byte
  file and measures process start-up alone.
- **stdin**: the five largest fixture files (`lints.rs` 640 KB, `nodes.rs` 322 KB,
  `hir/src/lib.rs` 228 KB, `hover/tests.rs` 210 KB, `config.rs` 167 KB) — the method of the
  phase-2 journal entry, now with hyperfine.

## What is held equal

- **Bytes.** Each workload is copied into `--work`; both tools read the copy. With
  `--input formatted` (the default) the copy is first rewritten by rustfmt itself, so
  neither tool finds a change and both exit 0; both still parse and lay out every file.
  `--input raw` keeps the bytes as found (the rust-analyzer fixtures and most registry
  crates are not rustfmt 1.9.0 `--edition 2024` output, so most files change).
- **File set.** rustfmt given a crate root formats that file and every out-of-line module it
  declares, recursively (`skip_children` is nightly-only). The harness lists the files
  rustfmt formats from its `--verbose` output (`Formatting <path>`, one line per file,
  including files that need no change; `-l` lists changed files only), copies exactly
  those files, re-runs rustfmt on the copy and stops if the formatted set differs from the
  files present. chloro then gets the copy's directory, so it reads the same files.
  `mod` declarations inside macro calls (tokio's `cfg_*!` blocks) are not followed by
  rustfmt, so those files are absent from the copy and neither tool formats them.
- **Configuration.** rustfmt reads an empty file through `--config-path`, so no
  `rustfmt.toml` in a registry crate or a parent directory applies; chloro uses its
  defaults. `--config-path` also skips rustfmt's search for a config file.
- **Output destination.** hyperfine sends stdout and stderr of both to `/dev/null`.
  `--mode check` (default): both run `--check`. `--mode stdout`: rustfmt
  `--emit stdout`, chloro's print mode; both write every formatted file to stdout.
- **Order and time.** `--rounds K` runs each hyperfine invocation K times with the command
  order reversed on even rounds; the report merges the rounds. Warmup runs precede each
  round, so the copies are in the page cache for every timed run.
- **Validation.** hyperfine runs with `-i` because both tools exit 1 when a file needs
  formatting. Before timing, each command runs once; the script stops on an exit status
  above 1 or on `error`, `panicked` or `Errors:` on stderr. The report lists the exit codes
  seen in the timed runs.
- **Output agreement.** At preparation, both tools write formatted copies and the meta file
  records how many files are byte-identical (`identical_outputs`), so a ratio is not read
  as like-for-like when the outputs differ.

## Known confounders

- **`--check` costs differ.** rustfmt `--check` builds a line diff of every changed file and
  prints it; chloro `--check` compares two strings and prints a one-line count. On raw
  input, where most files change, the diff dominates rustfmt's time on some crates:
  `regex-syntax` took rustfmt 1706 ms with `--check` against 592 ms with `--emit stdout`
  and 627 ms with `--check` on formatted input, and 423 MiB peak RSS against 66 MiB
  (measured, `docs/journal/2026-10-14-autoresearch/baseline.md`). This is why
  `--input formatted` is the default; `--input raw` figures are not like-for-like.
- **The rustup proxy.** `~/.cargo/bin/rustfmt` is rustup's proxy, which resolves the
  toolchain and execs the real binary. The harness runs `rustup which rustfmt` (recorded in
  `env.txt`) so rustfmt's figures do not include the proxy.
- **Process start-up.** rustfmt loads `librustc_driver` (about 30 ms and 34 MiB on this
  host for a one-line file); chloro is a static Rust binary of 2.5 MB. `tiny-one` measures
  this; it dominates small inputs.
- **Parallelism.** rustfmt formats on one thread. The chloro CLI starts
  `available_parallelism()` threads, which honours the `taskset` mask, and splits the sorted
  file list into `threads × 4` static chunks. `mc` rows show the CLI as shipped; `1c` rows
  show per-core cost. The CPU column (user + sys) is the total work in both cases.
- **The stdin rows use `examples/fmt_stdin`,** not the CLI: no threads, no file I/O, no
  directory walk.
- **Other load.** Other processes on the host change wall time. `env.txt` records the load
  average at start and end. Multi-core rows taken while other jobs run are provisional;
  pinned rows are affected less but still share caches, memory bandwidth and the
  hypervisor.
- **Hypervisor steal.** The host is a KVM guest. A pinned vCPU can still lose time to the
  hypervisor (steal), which no `taskset` mask prevents; `env.txt` records the steal ticks of
  `--core` over the run.
- **CPU frequency.** The host is a VM; the frequency governor and turbo state are not
  visible or controllable. Comparisons hold only between binaries measured in the same
  window, interleaved.
- **Peak RSS** is the maximum resident set of the timed process (`getrusage`), the whole
  process for chloro's threads.
- **Parse failures.** rustfmt stops on a file it cannot parse; chloro formats it anyway.
  Crates that do not parse under `--edition 2024` are not in the table (`syn` 1.x/2.x/3.x
  and `regex-automata` use `gen` as an identifier, a reserved keyword in 2024), and
  `tiny-test-data` drops the parser's error-recovery inputs.
- **`conform -i`** formats every fixture twice (once for the comparison, once for
  idempotence) and compares with the tracked snapshots, so its time is about twice
  `bench`'s for one round.

## Reading the JSON (hyperfine 2.0)

```text
results[]                        one entry per command
  .name                          the label: rustfmt, chloro, chloro-base
  .command                       the command as run
  .measurements[]                one per timed run
    .time_wall_clock.value       seconds
    .time_user.value, .time_system.value   seconds (all threads)
    .memory_peak_resident.value  bytes
    .exit_code
  .summary.<metric>.{mean,stddev,median,min,max,count}
```

```sh
jq -r '.results[] | [.name, .summary.time_wall_clock.mean, .summary.time_wall_clock.min,
  .summary.memory_peak_resident.max] | @tsv' results/<stamp>/fx-hir.1c.r1.json
```

`report.py` merges the rounds of one workload and variant and prints, per row: mean ± sd
(min–max) wall time for each tool, chloro/rustfmt of the means and of the minima, whether
the two min–max ranges overlap (`below` means every chloro run took less time than every
rustfmt run), peak RSS, mean CPU time, `identical_outputs` and exit codes.

## Decision rule for a speedup claim

The rule of `protocol.md` §3, restated for these tools:

1. **Single-threaded change** (no new threads): `profile.sh` `Ir` lower than the baseline
   build's on the same sample path, and `examples/bench -n 5` interleaved with the baseline
   binary (`compare_rustfmt.sh --baseline <dir> --groups e --rounds 3`, or by hand) with
   non-overlapping min–max MB/s ranges over at least 3 pairs.
2. **Change that adds parallelism or touches the CLI:** `compare_rustfmt.sh --baseline
   <dir>` with at least 10 runs per command over at least 2 interleaved rounds, the `mc`
   and `1c` rows both reported, and the chloro and chloro-base ranges not overlapping
   (`ranges vs base`).
3. **Against rustfmt:** ratios come from `--input formatted` (the default) or `--mode
   stdout`, never from `--input raw --mode check`. A ratio is quoted with its workload, variant, mode, input, `n`, the
   min–max ranges of both tools and the load recorded in `env.txt`; a ratio from
   overlapping ranges is reported as overlapping, not as a speedup.
4. Figures from different windows or hosts are not compared.

## profile.sh

`setarch -R` (no ASLR) `valgrind --tool=callgrind` on `examples/bench -n 1 --root
/tmp/chloro-bench/sample49`, under `env -i`, printing the total `Ir` of each run. The
sample is the 1238 `.rs` files under `chloro-core/tests/conformance/fixtures`, paths sorted
bytewise, every 25th file (lines 25, 50, …, 1225): 49 files, 650,213 bytes.

- The journals name the sample "every 25th file of the sorted fixture list" without the
  command. Of five constructions tried on `884f9c9` (component-wise or bytewise order;
  the 1220 rust-analyzer files or all 1238; first or 25th file), this one lands near the
  586.8M recorded for `f6c5be9` in the parse-drop-thread entry; the others give 421M, 661M,
  796M and 857M. `profile-baseline.md` (same day) selects the same 49 files with
  `find . -name '*.rs' | LC_ALL=C sort | awk 'NR%25==0'`.
- The absolute count depends on the sample's layout and the process environment (path
  strings, stack and heap layout), not only on the code. For the same `target/release/
  examples/bench` (`884f9c9`): 587,988,576–587,990,380 from `profile.sh` (directory tree at
  `/tmp/chloro-bench/sample49`, `env -i`); 588,096,020 with the caller's environment;
  587,211,777 in `profile-baseline.md` (files copied flat as `NN_<basename>.rs`). Counts are
  compared only between runs of `profile.sh` itself; under it, repeated runs agree to within
  about 2K instructions, pinned or not, from any working directory.
- `--annotate N` prints the top of `callgrind_annotate` for the first run; the profile
  files stay in `--out`.
