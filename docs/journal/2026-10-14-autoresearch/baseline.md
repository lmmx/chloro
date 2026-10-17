# Baseline: chloro against rustfmt, process level, and the harness that measures it

Part of [../2026-10-14-autoresearch.md](../2026-10-14-autoresearch.md) (task R4). The
harness lives in `benchmarks/` (`compare_rustfmt.sh`, `report.py`, `profile.sh`,
`README.md`); `benchmarks/README.md` holds the method, the confounders and the decision rule.
Every figure below is **measured** unless labelled otherwise. All multi-core rows, and the
pinned rows too, were taken while other agents loaded the host (load average 6–10 on 4
vCPUs); they are provisional until rerun on a quiet host.

## Identity

- **Code:** `884f9c9`. The runs happened at HEAD `76e31d6`, `b8c8531` and `ee99c87`; the
  commits after `884f9c9` touch only `docs/` and `benchmarks/` (`git diff --stat 884f9c9
  HEAD -- chloro chloro-core Cargo.toml Cargo.lock` is empty).
- **Binaries:** `target/release/chloro` (sha256 `26d25ca997cecebe…`),
  `examples/fmt_stdin` (`27afec59e6b40f35…`), `examples/bench` (`75c5feaaa8ca9dfc…`),
  `examples/conform`, all with mtime 14:44 UTC, built before this task started.
- **Build flags:** the workspace has no `[profile.release]` and no `.cargo/config.toml`;
  `~/.cargo/config.toml` is absent and `RUSTFLAGS` is unset — default release profile
  (opt-level 3, 16 codegen units, thin-local LTO, no debug info, unwind).
- **Toolchain:** `rustc 1.97.0 (2d8144b78 2026-07-07)`; `rustfmt 1.9.0-stable (2d8144b788
  2026-07-07)`, run as `/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt`
  (not the rustup proxy); hyperfine 2.0.0; valgrind 3.22.0.
- **Host:** KVM guest, 4 vCPUs "Intel Xeon Processor @ 2.80GHz" (1 thread per core), 15 GB,
  Linux 6.18.44.
- **Load during the runs** (`env.txt` of each results directory): load average 6.3 → 8.3
  (raw-input run, 15:10–15:17 UTC), 9.1 at the start of the formatted-input run
  (15:28 UTC), 9.8 → 9.4 (groups d and e, 15:34 UTC). Over the last run core 3 recorded
  16,585 busy, 0 idle and 256 steal ticks (USER_HZ 100). `ps` showed other agents' unpinned
  processes (a `chloro --check` from `/home/user/wt/cli-cache`, `rustfmt`, `rustc`) on
  core 3, so `taskset -c 3` did not give these runs a core of their own.

## Commands

Results directories (gitignored, in the working tree): `benchmarks/results/<name>/`, each with
`env.txt`, `commands.txt` (every hyperfine line as run), the JSON exports and `report.md`.

| name | command |
|---|---|
| `baseline-main` | `benchmarks/compare_rustfmt.sh -r 5 -w 2 --rounds 2 --core 3 --input raw` (the default at the time) |
| `baseline-formatted` | `benchmarks/compare_rustfmt.sh -r 5 -w 2 --rounds 2 --core 3` (groups a, b, c completed; the run stopped at `tiny-test-data`, see Harness changes) |
| `baseline-formatted-de` | same, `--groups d,e` |
| `spread-hir-ty-{1,2,3}` | `… -r 5 -w 2 --rounds 2 --core 3 --groups a,b --input raw fx-hir-ty`, three times in a row |
| `mode-check`, `mode-stdout`, `input-formatted` | `… --groups b --input raw` / `--mode stdout --input raw` / `--input formatted`, on `fx-hir fx-hir-ty reg-regex-syntax` |
| `profile` | `benchmarks/profile.sh --core 3 --runs 2` |

Every row below has n = 10 timed runs per command (2 rounds of 5, command order reversed in
round 2, 2 warmup runs per round). Ratios are chloro / rustfmt. "ranges" is `below` when
every chloro run took less wall time than every rustfmt run, `overlap` otherwise. "CPU
ratio" divides mean user+sys time (all threads).

## Instructions on the sample (`profile.sh`)

| binary | runs | Ir |
|---|---|---|
| `examples/bench` at `884f9c9` | 2 | 587,988,576 · 587,990,380 |

- The sample is 49 files, 650,213 bytes (every 25th line of `find . -name '*.rs' |
  LC_ALL=C sort` under `chloro-core/tests/conformance/fixtures`).
- The same binary counted 587,211,777 in [profile-baseline.md](profile-baseline.md), where
  the 49 files are copied flat under other names, and 588,096,020 under `profile.sh`'s
  sample path without `env -i` — the absolute count depends on path strings and environment
  size, so counts are compared only between `profile.sh` runs.

## In process

| measurement | window | result |
|---|---|---|
| `taskset -c 3 examples/bench -n 5`, 1238 files, 13.9 MB | 15:16 UTC (`baseline-main`) | 3.189 s, 4.36 MB/s · 2.960 s, 4.70 MB/s |
| same | 15:40 UTC (`baseline-formatted-de`, core 3 0 idle ticks) | 6.339 s, 2.19 MB/s · 6.332 s, 2.20 MB/s |
| `taskset -c 3 examples/conform -i`, hyperfine, 6 runs | 15:16 UTC | 7639 ± 628 ms (6748–8284), CPU 6616 ms, 51 MiB |
| same | 15:41 UTC | 10,390 ± 3,639 ms (6688–14,060), CPU 6762 ms, 50 MiB |

`conform -i` reported 1219/1220 identical and 0 not idempotent in every run (exit 0).

## Crate level: `--check` on rustfmt-formatted input (the harness default)

Each crate copy was rewritten by rustfmt first; both tools then exit 0 (nothing to report)
after parsing and laying out every file. `baseline-formatted`, 15:28–15:34 UTC.

All CPUs (`mc`; chloro runs 4 threads, rustfmt 1):

| workload | files | KB | rustfmt ms: mean (min–max) | chloro ms: mean (min–max) | wall ratio mean | wall ratio min | ranges | CPU ratio | peak RSS MiB r / c |
|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 15 | 646 | 357 (306–417) | 262 (191–410) | 0.73 | 0.62 | overlap | 0.78 | 56 / 25 |
| fx-hir-ty | 122 | 2876 | 1233 (1039–1557) | 619 (385–843) | 0.50 | 0.37 | below | 0.66 | 85 / 35 |
| fx-ide-db | 33 | 1169 | 424 (350–496) | 205 (166–249) | 0.48 | 0.48 | below | 0.71 | 58 / 21 |
| fx-syntax | 33 | 405 | 181 (147–246) | 153 (119–233) | 0.85 | 0.81 | overlap | 0.85 | 51 / 13 |
| fx-parser | 28 | 235 | 109 (80–147) | 83 (61–107) | 0.76 | 0.76 | overlap | 0.78 | 47 / 10 |
| reg-tokio | 49 | 215 | 46 (42–60) | 45 (34–63) | 0.96 | 0.79 | overlap | 1.00 | 43 / 7 |
| reg-regex-syntax | 33 | 1647 | 586 (448–762) | 320 (273–393) | 0.55 | 0.61 | below | 1.01 | 66 / 37 |
| reg-clap-builder | 57 | 944 | 228 (197–280) | 220 (162–374) | 0.96 | 0.82 | overlap | 0.95 | 55 / 18 |
| reg-winnow | 61 | 1005 | 212 (186–290) | 243 (193–321) | 1.14 | 1.04 | overlap | 1.19 | 59 / 18 |

Both pinned to core 3 (`1c`):

| workload | files | KB | rustfmt ms: mean (min–max) | chloro ms: mean (min–max) | wall ratio mean | wall ratio min | ranges | CPU ratio | peak RSS MiB r / c |
|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 15 | 646 | 391 (301–492) | 283 (227–360) | 0.72 | 0.75 | overlap | 0.71 | 56 / 16 |
| fx-hir-ty | 122 | 2876 | 1807 (1016–2830) | 1316 (713–1623) | 0.73 | 0.70 | overlap | 0.66 | 85 / 18 |
| fx-ide-db | 33 | 1169 | 1090 (720–1400) | 653 (479–786) | 0.60 | 0.67 | overlap | 0.62 | 58 / 15 |
| fx-syntax | 33 | 405 | 338 (280–424) | 318 (237–452) | 0.94 | 0.85 | overlap | 0.92 | 51 / 9 |
| fx-parser | 28 | 235 | 219 (163–283) | 161 (136–218) | 0.74 | 0.84 | overlap | 0.74 | 47 / 7 |
| reg-tokio | 49 | 215 | 110 (89–137) | 72 (70–73) | 0.65 | 0.78 | below | 0.67 | 43 / 6 |
| reg-regex-syntax | 33 | 1647 | 1205 (1032–1380) | 1128 (940–1358) | 0.94 | 0.91 | overlap | 0.92 | 66 / 29 |
| reg-clap-builder | 57 | 944 | 497 (427–625) | 418 (343–492) | 0.84 | 0.80 | overlap | 0.84 | 55 / 12 |
| reg-winnow | 61 | 1005 | 450 (409–560) | 445 (406–484) | 0.99 | 0.99 | overlap | 1.00 | 59 / 11 |

- In the `1c` table wall time is about twice CPU time for both tools (for example
  `fx-hir-ty` rustfmt 1807 ms wall against 1169 ms CPU) — core 3 was shared with other
  processes during this run; CPU time excludes the time a command waited for the core, and
  the CPU ratio column reads 0.62–1.00.
- Output agreement: every file of every crate copy formats to the same bytes under both
  tools (`identical_outputs` 15/15, 122/122, 33/33, 33/33, 28/28, 49/49, 33/33, 57/57,
  61/61).
- `reg-tokio` holds 49 of the 378 `.rs` files under tokio's `src/` (215 KB of 3.5 MB):
  rustfmt does not follow `mod` declarations written inside tokio's `cfg_*!` macro calls,
  and the harness gives chloro the same 49 files.

## Crate level: `--check` on input as found

`baseline-main`, 15:10–15:17 UTC. The fixtures and most registry crates are not rustfmt
1.9.0 `--edition 2024` output, so both tools exit 1 and rustfmt prints a diff for each
changed file (`reg-clap-builder` is already formatted: exit 0).

| workload | files | KB | `mc` rustfmt ms | `mc` chloro ms | `mc` wall ratio | `mc` ranges | `1c` rustfmt ms | `1c` chloro ms | `1c` wall ratio | `1c` ranges | `1c` CPU ratio | peak RSS MiB r / c (`1c`) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| fx-hir | 15 | 625 | 778 (620–1311) | 219 (163–292) | 0.28 | below | 635 (563–694) | 221 (193–252) | 0.35 | below | 0.34 | 217 / 16 |
| fx-hir-ty | 122 | 2816 | 1507 (1373–1775) | 556 (370–828) | 0.37 | below | 1495 (1328–1785) | 706 (649–747) | 0.47 | below | 0.47 | 116 / 17 |
| fx-ide-db | 33 | 1160 | 412 (376–454) | 240 (195–298) | 0.58 | below | 447 (381–576) | 257 (220–340) | 0.57 | below | 0.57 | 66 / 15 |
| fx-syntax | 33 | 394 | 225 (191–275) | 164 (126–206) | 0.73 | overlap | 227 (170–288) | 129 (113–206) | 0.57 | overlap | 0.58 | 57 / 9 |
| fx-parser | 28 | 232 | 152 (96–189) | 88 (74–104) | 0.58 | overlap | 109 (91–131) | 88 (73–112) | 0.80 | overlap | 0.79 | 48 / 7 |
| reg-tokio | 49 | 215 | 65 (53–105) | 47 (30–86) | 0.72 | overlap | 45 (43–48) | 51 (38–62) | 1.12 | overlap | 1.13 | 43 / 6 |
| reg-regex-syntax | 33 | 1657 | 1565 (1459–1759) | 462 (368–537) | 0.30 | below | 1811 (1564–2078) | 596 (466–783) | 0.33 | below | 0.33 | 423 / 29 |
| reg-clap-builder | 57 | 944 | 268 (217–412) | 174 (133–305) | 0.65 | overlap | 260 (203–334) | 217 (187–294) | 0.83 | overlap | 0.82 | 55 / 12 |
| reg-winnow | 61 | 1005 | 218 (193–259) | 216 (152–306) | 0.99 | overlap | 273 (226–322) | 224 (199–307) | 0.82 | overlap | 0.84 | 59 / 11 |

Times are mean (min–max) ms. These ratios include rustfmt's diff construction (next
section) and are not like-for-like.

## Cost of `--check` (pinned to core 3, 15:20–15:23 UTC)

Three consecutive runs on the same three crates; rustfmt and chloro both pinned.

| workload | rustfmt `--check`, raw input | rustfmt `--emit stdout`, raw input | rustfmt `--check`, formatted input | chloro `--check`, raw | chloro print mode, raw | chloro `--check`, formatted |
|---|---|---|---|---|---|---|
| fx-hir | 739 ms, 217 MiB | 404 ms, 56 MiB | 341 ms, 56 MiB | 267 ms | 221 ms | 223 ms |
| fx-hir-ty | 1509 ms, 116 MiB | 1181 ms, 85 MiB | 1180 ms, 85 MiB | 836 ms | 790 ms | 771 ms |
| reg-regex-syntax | 1706 ms, 423 MiB | 592 ms, 66 MiB | 627 ms, 66 MiB | 501 ms | 514 ms | 568 ms |

- rustfmt `--check` on raw input takes 1.3x (`hir-ty`) to 2.9x (`regex-syntax`) the time of
  rustfmt `--emit stdout` on the same bytes, and up to 6.4x the peak RSS — the difference is
  the line diff rustfmt builds and prints for each changed file.
- chloro's three figures per crate lie within 21% of each other (`fx-hir` 221–267 ms,
  `fx-hir-ty` 771–836 ms, `regex-syntax` 501–568 ms); chloro's `--check` compares two
  strings per file.
- The chloro/rustfmt ratio on `regex-syntax` moves from 0.29 (raw `--check`) to 0.87 (raw,
  stdout) and 0.91 (formatted `--check`) with the formatter work unchanged — this is the
  reason the harness default became `--input formatted`.

## One file on stdin (pinned to core 3)

rustfmt reads stdin and writes stdout; chloro is `examples/fmt_stdin` (no threads, no file
I/O). Both write to `/dev/null`. Output identical on all five files in both inputs.

| file | KB | raw: rustfmt ms | raw: chloro ms | raw ratio | raw ranges | formatted: rustfmt ms | formatted: chloro ms | formatted ratio | formatted ranges | formatted CPU ratio |
|---|---|---|---|---|---|---|---|---|---|---|
| `lints.rs` | 640 | 187 (144–225) | 110 (86–141) | 0.59 | below | 327 (277–369) | 216 (172–317) | 0.66 | overlap | 0.66 |
| `nodes.rs` | 322 / 358 | 127 (91–164) | 86 (79–96) | 0.68 | overlap | 223 (196–292) | 190 (172–217) | 0.85 | overlap | 0.85 |
| `hir/src/lib.rs` | 228 | 97 (90–118) | 92 (68–116) | 0.95 | overlap | 207 (181–253) | 155 (135–169) | 0.75 | below | 0.73 |
| `hover/tests.rs` | 210 / 211 | 32 (30–36) | 25 (18–41) | 0.78 | overlap | 85 (63–111) | 42 (34–51) | 0.49 | below | 0.47 |
| `config.rs` | 167 | 56 (43–69) | 30 (26–48) | 0.53 | overlap | 107 (92–145) | 70 (57–92) | 0.66 | overlap | 0.66 |

- The raw column used `baseline-main` (15:16 UTC), the formatted column `baseline-formatted`
  (15:33 UTC); the formatted window had wall about twice CPU for both tools.
- The phase-2 entry measured `lints.rs` at 0.105 s (chloro) against 0.242 s (rustfmt), 0.43,
  through `~/.cargo/bin/rustfmt`, best of 5; the figures above use the toolchain binary and
  report mean and range.

## Many files and start-up

| workload | input | variant | rustfmt ms | chloro ms | wall ratio | ranges | CPU ratio | peak RSS MiB r / c |
|---|---|---|---|---|---|---|---|---|
| `tiny-one` (13 bytes) | raw | 1c | 11.0 (9.0–13.2) | 2.4 (1.8–3.9) | 0.22 | below | 0.17 | 34 / 4 |
| `tiny-one` | raw | mc | 13.1 (10.3–19.6) | 3.0 (2.4–4.8) | 0.23 | below | 0.20 | 34 / 4 |
| `tiny-test-data` (289 files, 65 KB) | raw | 1c | 69 (62–79) | 34 (25–47) | 0.49 | below | 0.48 | 44 / 5 |
| `tiny-test-data` | raw | mc | 78 (63–102) | 22 (17–26) | 0.28 | below | 0.40 | 44 / 5 |
| `tiny-test-data` (287 files, 67 KB) | formatted | 1c | 135 (108–180) | 43 (22–64) | 0.32 | below | 0.40 | 44 / 5 |
| `tiny-test-data` | formatted | mc | 63 (49–83) | 25 (22–30) | 0.39 | below | 0.41 | 44 / 5 |

- rustfmt on a one-line file takes 9.0–13.2 ms pinned in the raw-input window (9–29 ms
  across all windows) and 34 MiB; the performance checkpoint's estimate used about 61 ms of
  start-up per process.
- Through the rustup proxy (`~/.cargo/bin/rustfmt`) the same one-line file took 17.1 ms at
  minimum against 10.4 ms for the toolchain binary (hyperfine, 20 runs each, core 3, load
  average 9.1); the proxy adds a few milliseconds to every rustfmt process.
- `tiny-test-data` is `parser/test_data` restricted to files rustfmt parses: 289 of 491
  raw; 287 formatted, because rustfmt's own output for
  `parser/inline/ok/closure_range_method_call.rs` (`|| .. .field` → `|| ...field`) and
  `use_tree_abs_star.rs` does not parse again.

## Noise

`fx-hir-ty`, raw input, the same command three times back to back (15:17–15:20 UTC, load
average 4.8–6.7), plus the `baseline-main` and `mode-check` runs of the same workload:

| run | `1c` rustfmt ms | `1c` chloro ms | `1c` ratio | `mc` rustfmt ms | `mc` chloro ms | `mc` ratio |
|---|---|---|---|---|---|---|
| `baseline-main` | 1495 (1328–1785) | 706 (649–747) | 0.47 | 1507 (1373–1775) | 556 (370–828) | 0.37 |
| `spread-1` | 1466 (1262–1664) | 727 (656–865) | 0.50 | 1538 (1376–1673) | 349 (287–498) | 0.23 |
| `spread-2` | 1388 (1227–1625) | 870 (705–1051) | 0.63 | 1428 (1211–1556) | 441 (383–476) | 0.31 |
| `spread-3` | 1552 (1382–1816) | 824 (699–926) | 0.53 | 1554 (1344–1930) | 481 (417–612) | 0.31 |
| `mode-check` | 1509 (1301–1901) | 836 (677–982) | 0.55 | – | – | – |

- Across these windows the mean of one command moves by up to 23% (`1c` chloro 706–870 ms)
  and the `mc` chloro mean by 59% (349–556 ms); within one run of 10, max/min reaches 1.3x
  (`1c` rustfmt) and 2.2x (`mc` chloro, `baseline-main`).
- The `1c` ratio ranged 0.47–0.63 and the `mc` ratio 0.23–0.37 over five runs of identical
  binaries and bytes.
- Instruction counts on the same binary: 587,988,576 and 587,990,380 (1,804 apart, 0.0003%).

## Harness changes made during the baseline

- The default input became `--input formatted` after the `--check` cost measurement above
  (`benchmarks/compare_rustfmt.sh`); `baseline-main` used `--input raw`.
- `compare_rustfmt.sh` now resolves rustfmt through `rustup which rustfmt`; the raw-input
  smoke runs before that change went through the rustup proxy and are not reported.
- For the `files` kind with formatted input, files that rustfmt cannot parse after its own
  formatting are dropped; the first formatted run stopped at that check (`tiny-test-data`:
  287 formatted, 289 expected), which is why groups d and e ran separately.
- `env.txt` records the `/proc/stat` busy, idle and steal ticks of `--core` from the
  formatted runs on.

## Current State

- `benchmarks/compare_rustfmt.sh` runs 16 table-driven workloads (9 crates, 5 stdin files,
  2 many-file sets) plus `examples/bench -n 5` and `examples/conform -i`, with hyperfine
  `-N -i --export-json`, warmup, run count, interleaved rounds, `taskset` pinning, and
  wall, user, system and peak-RSS metrics.
- The crate workloads give chloro exactly the files rustfmt formats from the crate root,
  taken from rustfmt's `--verbose` "Formatting" lines and re-verified on the copy before
  timing (`compare_rustfmt.sh` `prepare`).
- `benchmarks/report.py` prints chloro/rustfmt ratios of means and minima, CPU ratio,
  range overlap, peak RSS, output agreement and exit codes, and a chloro/chloro-base ratio
  when `--baseline` names a second build.
- `benchmarks/profile.sh` prints callgrind `Ir` on the 49-file sample under `setarch -R`
  and `env -i`: 587,988,576–587,990,380 for `884f9c9`.
- Formatted input, `--check`, under load: chloro takes 0.48–1.14x rustfmt's wall time with
  all 4 CPUs and 0.62–1.00x rustfmt's CPU time pinned to one core, across the nine crates.

## Missing

- No quiet-host run: every process-level figure here was taken with load average 6–10 on 4
  vCPUs and with other agents' unpinned processes on core 3; the orchestrator's rerun
  replaces these numbers.
- The stdin workloads use `examples/fmt_stdin`, not the chloro CLI, which has no stdin mode
  (`chloro/src/cli.rs`) — the stdin rows exclude chloro's thread start and file I/O while
  rustfmt's include its own start-up.
- Registry crates written for editions before 2024 that use `gen` as an identifier (`syn`
  1.0.109, 2.0.119, 3.0.9; `regex-automata` 0.4.18) fail to parse under `rustfmt
  --edition 2024` and are absent from the workload table.
- `mc` rows compare one rustfmt thread with four chloro threads; no workload runs several
  rustfmt processes in parallel (for example one per crate root), which is how `cargo fmt`
  on a workspace spreads work.
- The 49-file sample definition is reconstructed (bytewise sort, `NR % 25 == 0`); the
  original command is not recorded in the earlier entries.
