# benchmarks

Measurement harness for chloro against `rustfmt 1.9.0 --edition 2024`, and for one chloro
build against another. Every speed figure the research programme states comes from these
scripts (see `docs/journal/2026-10-14-autoresearch/protocol.md` §3); the quiet-host runs
the README quotes are under `docs/journal/2026-10-14-autoresearch/measurements/`:
`m1-quiet-host.md`, `m2-quiet-host.md`, `m3-quiet-host.md` and `m4-quiet-host.md` (the README's figures).

| file | measures |
|---|---|
| `compare_rustfmt.sh` | process wall time, CPU time and peak RSS of rustfmt and chloro on named workloads, with hyperfine (`--export-json`) |
| `report.py` | turns a results directory into markdown tables with chloro/rustfmt ratios |
| `profile.sh` | callgrind total instructions (`Ir`) of `examples/bench` on the fixed 49-file sample |
| `pgo.sh` | not a measurement: the profile-guided build of the CLI and of `examples/bench` that the `head-pgo` rows measure, and the training phase the release workflow runs |

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
benchmarks/profile.sh --core 3 --runs 2 --sim                 # Ir plus branch and cache simulation
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
   build's (the script stages both at the same path lengths, EXP-027), and `examples/bench
   -n 5` interleaved with the baseline binary (`compare_rustfmt.sh --baseline <dir> --groups
   e --rounds 3`, or by hand) with non-overlapping min–max MB/s ranges over at least 3 pairs.
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

`setarch -R` (no ASLR) `valgrind --tool=callgrind` on `examples/bench -n 1 --root <sample>`,
under `env -i`, printing the total `Ir` of each run. The sample is the 1238 `.rs` files under
`chloro-core/tests/conformance/fixtures`, paths sorted bytewise, every 25th file (lines 25,
50, …, 1225): 49 files, 650,213 bytes. The binary and the sample are copied under a staging
directory of fixed length before each invocation (*Path independence*, below), so the count
does not depend on where the binary, the fixtures or `--out` are.

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
  compared only between runs of `profile.sh` itself.
- **Run-to-run spread.** Repeated runs of one binary agree to within about 400 instructions,
  except that about one run in five reads about 1.8K more (2 of 10 runs of `9d78713`:
  237,768,233–237,768,557 and 237,770,183–237,770,270). Both come from `bench`'s own report,
  not from the formatter: the order in which it sorts the files by their measured time (the
  smaller spread) and the decimal printing of the timings, which takes `core`'s exact
  big-number path for some values (`float_to_decimal_common_exact` +1,239 and
  `Big32x40::mul_pow2` +314 in a run that read +1,950; EXP-027). Take the minimum of
  `--runs 3` or more when a difference under about 2K matters.
- **Path independence (EXP-027).** Before EXP-027 the script ran the binary where it was and
  read the sample at `--sample` (default `/tmp/chloro-bench/sample49`), and the count moved
  with the lengths of those paths: the same binary read 239.51 M with the sample at a
  6-character name, 237.77 M at the default's 8 and 237.90 M at 12. The process holds those
  strings: `argv[0]` and every file path are heap allocations whose sizes (malloc's size
  classes) set the heap layout the parser's vector growth meets (EXP-020's term), and they
  sit on the stack; and glibc reads `/proc/self/maps`, which lists the binary's own path,
  when Rust's runtime asks for the main thread's stack at start-up (`pthread_getattr_np`,
  about 40 instructions per byte of the path). Only lengths reach the count, not the text.
  So the script now copies the binary and the sample under `mktemp -d
  /tmp/chloro-prof.XXXXXXX` (24 bytes, removed at exit) and runs `<stage>/bench_…` (47
  bytes) with `--root <stage>/s` (26 bytes). Those are the lengths of the strings the script
  used before at its default (`/home/user/chloro/target/release/examples/bench` and
  `/tmp/chloro-bench/sample49`), so the count at the old default is unchanged: 237,768,412–
  237,768,605 for `9d78713` (three runs; the ledger's 237,768,533). `--sample` is accepted
  and ignored. Verification (`9d78713`, three runs each, `--core 3`):

  | script | where the sample was (`--sample`, or `--out` for the new script) | binary | `Ir` of three runs |
  |---|---|---|---|
  | before | `/tmp/chloro-bench/e27a01` | `target/release/examples/bench` | 239,510,992, 239,512,663, 239,511,218 |
  | before | `/tmp/chloro-bench/e27a0001` | same | 237,768,585, 237,768,477, 237,768,564 |
  | before | `/tmp/chloro-bench/e27a00000001` | same | 237,901,113, 237,899,350, 237,899,366 |
  | after | `/tmp/chloro-bench/e27a01` | `/tmp/chloro-bench/e27b/bench` (28 bytes) | 237,768,550, 237,768,641, 237,768,583 |
  | after | same | `target/release/examples/bench` (51 bytes) | 237,768,641, 237,768,639, 237,768,647 |
  | after | same | a copy at a 115-byte path | 237,768,602, 237,768,687, 237,768,381 |
  | after | `/tmp/chloro-bench/e27a0001` | 28 bytes | 237,768,425, 237,770,094, 237,768,444 |
  | after | same | 51 bytes | 237,770,188, 237,768,315, 237,768,375 |
  | after | same | 115 bytes | 237,768,504, 237,768,538, 237,768,516 |
  | after | `/tmp/chloro-bench/e27a00000001` | 28 bytes | 237,768,385, 237,768,452, 237,768,417 |
  | after | same | 51 bytes | 237,770,058, 237,768,452, 237,770,211 |
  | after | same | 115 bytes | 237,768,475, 237,770,207, 237,768,523 |

  After the change the minima of the nine cells lie within 326 instructions of each other
  (237,768,315–237,768,641); the readings near 237,770,1xx are the report's outlier above.
  Before, a binary at the 115-byte path also read 2.6–4.5K more than the same binary in the
  worktree, at any sample path. A relative `--root` of the same 26 bytes (the sample as the
  working directory) reads about 1.2K more than the absolute one: relative paths cost more to
  compare and walk; the absolute form keeps the old count.
- `--annotate N` prints the top of `callgrind_annotate` for the first run; the profile
  files stay in `--out`.
- `--sim` runs callgrind with `--cache-sim=yes --branch-sim=yes` (same `env -i`, `setarch -R`
  and pinning) and prints, for each run, the simulator's totals: `Bc` and `Bcm` (conditional
  branches executed and mispredicted), `Bi` and `Bim` (indirect branches), `I1mr` (L1
  instruction-cache read misses), `D1mr` and `D1mw` (L1 data read and write misses) and the
  last-level misses, then the ten functions with the most `I1mr` and the ten with the most
  `Bcm` (exclusive counts, from `callgrind_annotate --sort`). This is the co-metric of
  EXP-022 for code-layout and dispatch changes, which the instruction count does not see.
  `Ir` is the same with and without `--sim`. The simulator models a 32 KB 8-way L1 and a
  2004-era predictor: conditional branches from a table of 16,384 two-bit counters indexed by
  address and recent global history, indirect branches from a 512-entry table of last
  targets indexed by address alone (no history; current x86 cores predict indirect branches
  from history too, so `Bim` overstates their cost on real hardware). A run takes about two
  and a half times as long as one without `--sim` (14 s against 5.6 s for the head of
  2026-10-15). The counts are as repeatable as `Ir`: three runs of one binary gave the same
  `I1mr`, `Bim`, `D1mr` and `D1mw`, `Bcm` within 4 and `Ir` within 220
  (`docs/journal/2026-10-14-autoresearch/experiments/EXP-022-layout.md`).

## pgo.sh

```sh
benchmarks/pgo.sh                     # target/pgo/release/{chloro,examples/bench}
benchmarks/pgo.sh /path/to/root cli   # another target root, the CLI's features
benchmarks/pgo.sh --phase train       # profile only: target/pgo-gen/profiles/merged.profdata
benchmarks/pgo.sh --phase use         # the -Cprofile-use builds from that profile
benchmarks/pgo.sh --target x86_64-unknown-linux-gnu   # every build with --target
```

The train phase builds the CLI and `examples/bench` with `-Cprofile-generate`, runs
`chloro --check` on `chloro-core/tests/conformance/fixtures` and `bench -n 1`, and merges
the raw profiles with `llvm-profdata` (rustup component `llvm-tools-preview`). The use
phase builds both with `-Cprofile-use=<merged.profdata>
-Cllvm-args=-pgo-warn-missing-function`; the second flag makes LLVM print a warning for
every function the profile has no record of, which shows a profile that does not match
the code (it does not fail a `-D warnings` build). `--target` defaults to
`$CARGO_BUILD_TARGET`; training runs the instrumented binaries, so the target has to be
the host. Anything already in `RUSTFLAGS` is kept in both phases.

The release workflow (`.github/workflows/binaries.yml`, step "Train PGO profile") runs
`pgo.sh --phase train --target <target>` on each matrix target whose triple equals the
runner's host (`x86_64-unknown-linux-gnu` on `ubuntu-22.04`, `x86_64-apple-darwin` on
`macos-13`, `x86_64-pc-windows-msvc` on `windows-2022`), then appends `-Cprofile-use` and
the warning flag to `RUSTFLAGS`, so `taiki-e/upload-rust-binary-action`'s own `cargo build
--release --target <target>` is the profile-guided build. The two `aarch64` targets are
cross-compiled on those runners and keep the plain build; if any command of the training
fails, or no profile is left, the step prints a warning and the binary is the plain build.
The instrumented build passes `--target` like the action does: the symbol names the
profile is keyed by differ between a build with `--target` and one without, and on
`x86_64-unknown-linux-gnu` a profile trained without `--target` left 1017 functions of
`chloro` and `chloro_core` without profile data in the action's build, against none with
it. With `--target`, `RUSTFLAGS` also stays off build scripts and proc macros, so their runs
during the build write no raw profiles (2 files and 1.0 MB merged, against 13 and 4.1 MB).
These commands were run locally on `x86_64-unknown-linux-gnu`
(`docs/journal/2026-10-14-autoresearch/experiments/EXP-013-pgo-release.md`); the macOS and
Windows paths of the step have not been run.
