# M4: base, head and head-pgo against rustfmt on a quiet host (the final head, E12–E18)

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md). Milestone 4 of
the autoresearch programme: the harness of `benchmarks/` run on a host with no other agent,
on the three chloro builds of the integration head after the overnight merges (E12 CLI
waste, E13 PGO in the release workflow, E14 allocator measurement, E15 unit cache, E16
scanner paths, E17 parse floor, E18 dead candidates) and on rustfmt 1.9.0, with the method of
[m3-quiet-host.md](m3-quiet-host.md), [m2-quiet-host.md](m2-quiet-host.md) and
[m1-quiet-host.md](m1-quiet-host.md). Every figure in this file is **measured** in the
windows named below; nothing is estimated. The host is the 2.80 GHz host class of
[../baseline.md](../baseline.md), M2 and M3 (protocol §3: figures from different hosts are
not compared; the "against M3" section sets within-window ratios side by side).

Labels used throughout:

| label | build |
|---|---|
| `base` | `884f9c9` (master at the programme start), default release profile — the binary users had |
| `head` | `6b78728` (the integration branch after the E12–E18 merges; three docs-only commits above the ledger's best-known-good `e640fbf`, same code; the formatter code is M3's `4867896` plus R5, E12, E15, E17, E18 and E16, with E13 touching the release workflow and E14 adding off-by-default features) |
| `head-pgo` | `head` built by `benchmarks/pgo.sh` (`-Cprofile-generate`, training on the fixtures through `--check` and `bench -n 1`, `-Cprofile-use`; the script also builds the profile-guided `bench`) |
| `rustfmt` | `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`, the toolchain binary, `--edition 2024 --config-path <empty file>` |

## Identity

- **Commits:** `head` `6b78728ab59fde69663c8b4ca9ec32ca55748c08` (worktree clean,
  `worktree_dirty=0`; `git diff --stat e640fbf HEAD` touches three Markdown files under
  `docs/journal/` and no source); `base` `884f9c93f735676a752c67fab91d08959f410430` (`git
  worktree add /home/user/wt/base 884f9c9`, its own `target/`).
- **Binaries (sha256):**

| binary | path | sha256 | bytes |
|---|---|---|---|
| `base` chloro | `/home/user/wt/base/target/release/chloro` | `26d25ca997cecebebb665181f87bbdda7268a06a5dd0f2950aaa0191610c1b7c` | 2,516,048 |
| `base` bench | `/home/user/wt/base/target/release/examples/bench` | `80f4319b41dab836aacc0efc095c5d167faf514bce87416b55fd0b5d8862657d` | 2,389,072 |
| `base` fmt_stdin | `/home/user/wt/base/target/release/examples/fmt_stdin` | `27afec59e6b40f35ca925ca35b77ff3b163212e98141b4cd0213b466c079e2da` | 2,264,112 |
| `base` conform | `/home/user/wt/base/target/release/examples/conform` | `ff82ac02966c1e5766d3c856b35925515e70510a3ea0d466209d1182c184c109` | 2,426,776 |
| `head` chloro | `target/release/chloro` | `37d1d72d07248151bf2f0a03d8cc8fcb7b8d0a087a6c03cd6a43d8b424763033` | 2,147,536 |
| `head` bench | `target/release/examples/bench` | `819d6f68ebd5c415721c4fd32466d0a61e253d3c51b804d48b44d4a2c3cf69ac` | 1,842,304 |
| `head` fmt_stdin | `target/release/examples/fmt_stdin` | `93ee7b03e8fd238f140e58f0d9ebb1f5c72a42098895b849394fb1889e9029a2` | 1,684,744 |
| `head` conform | `target/release/examples/conform` | `b287b617bae7fcf92a9158e81be16ae561cbe14a403c4da8689ea1f1328e5fd4` | 1,814,192 |
| `head-pgo` chloro | `target/pgo/release/chloro` | `218d001c5d831ec6c541d3df10766768cb949598bf65686ff05170ed3b67247b` | 2,924,784 |
| `head-pgo` bench | `target/pgo/release/examples/bench` | `1f78c6bca6534ad6ce0a8f9431f3200988fcfa03aa6c7fc9ce762d08d868b378` | 2,631,544 |
| rustfmt | `/root/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustfmt` | `fbdba7404a80bc6d36fa2bb4cdd7ca3fc7f060a109207ef03eaef63058cd1216` | 5,278,736 |

- The four `base` hashes and the rustfmt hash equal the ones recorded in M1, M2 and M3: the
  default-profile build of `884f9c9` reproduces byte for byte on this toolchain.
- The `head` chloro binary is 2,147,536 bytes against 2,010,696 for M3's `4867896` build and
  2,516,048 for `base` (0.854x base; 1.068x M3's head: the CLI gained the unit cache, the
  identity stamp and the E16 paths); `head-pgo` is 2,924,784 bytes (1.162x base, 1.078x M3's
  2,714,112).
- `cargo tree -e no-dev -p chloro-core --prefix none | sort -u | wc -l`: 17 lines for `head`,
  38 for `base` (the M2 and M3 counts; E14's allocator crates are behind off-by-default
  features and do not appear).
- **Toolchain:** `rustc 1.97.0 (2d8144b78 2026-07-07)`, host `x86_64-unknown-linux-gnu`, LLVM
  22.1.6; `cargo 1.97.0`; `llvm-tools-preview` (for `llvm-profdata`, used by `pgo.sh`);
  hyperfine 2.0.0; valgrind 3.22.0.
- **Host:** KVM guest, `nproc` 4, "Intel(R) Xeon(R) Processor @ 2.80GHz" (`model name` in
  `/proc/cpuinfo`), 16,479,424 kB `MemTotal`, Linux 6.18.44-fc-v114. The 2.80 GHz host class
  of M2, M3 and [../baseline.md](../baseline.md).
- **Load:** no other agent ran on the host (the overnight experiment agents had all finished
  and their worktrees were cleaned before the first command). The 1-minute load average read
  0.01 at the identity step. The chain ran as one script
  (`scratchpad/m4/chain.sh`, reproduced in Method) with no gap between sweeps; the load
  average at the start of every later sweep is the harness's own processes from the previous
  sweep (the `mc` rows run chloro with 4 threads). `cpu3` columns are `/proc/stat` ticks of
  core 3 (USER_HZ 100) over the sweep: steal / busy / idle. A first start of the chain at
  04:28 UTC was stopped after one minute because the five stdin workload copies on this disk
  predated the harness's stdin preparation (two of the five were not fixed points: exit 1
  for `base`, `head` and `head-pgo` alike); the five copies were rebuilt through rustfmt's
  stdin (`--groups c -r 1`), verified as fixed points for rustfmt, `base`, `head` and
  `head-pgo` (exit 0 on all five), and the chain restarted from the identity step. Nothing of
  the first start is used.

TODO-M4-LOADS

## Method

- All process-level figures: `benchmarks/compare_rustfmt.sh` with hyperfine 2.0 `-N -i`,
  `--metrics time_wall_clock,time_user,time_system,memory_peak_resident`, 2 warmup runs and
  **15 timed runs per command per round, 2 rounds with the command order reversed** (n = 30
  per cell), pinned rows on core 3, `mc` rows unpinned, `--mode check`. `report.py` merges
  the rounds; the tables below are built from the same JSON exports (min / mean ± σ wall ms
  / peak RSS MiB; CPU is mean user+sys ms over all threads).
- **The cache is disabled in every sweep except measurement C** by the harness itself:
  `compare_rustfmt.sh` passes `--no-cache` to any chloro binary whose `--help` lists the flag
  unless `--with-cache` is given (`env.txt` records `chloro_cache_flag=--no-cache`); `base`
  has no cache and gets no flag. Since E15 `--no-cache` also turns off the opt-in unit cache,
  which no sweep enables. Verified before the sweeps: `head -v --check --no-cache` prints
  `Cache: disabled (--no-cache)`.
- Workload copies: `/tmp/chloro-bench/formatted/<workload>/tree` (rewritten by rustfmt
  before timing, both tools exit 0) and `/tmp/chloro-bench/raw/...` (bytes as found). The
  crate and files copies are the ones on this disk from the earlier records (same
  preparation, same bytes: `tiny-test-data` 287 formatted and 289 raw files); the five
  stdin copies were rebuilt as described under Load and are fixed points for every build
  (`stdin-hir-lib` 236,066 bytes and `stdin-config` 169,196, M3's sizes). The registry
  crates `tokio-1.53.3`, `regex-syntax-0.8.11`, `winnow-0.7.15` and `clap_builder-4.6.7`
  were present in `~/.cargo/registry`.
- The results directories (gitignored, under `benchmarks/results/`):

| directory | command |
|---|---|
| `m4-a1-head-base-formatted` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b,c,d --baseline /home/user/wt/base/target/release` (`chloro` = head, `chloro-base` = base) |
| `m4-a2-pgo-base-formatted` | same, `--groups a,b,d --chloro target/pgo/release/chloro --baseline <base>` (the stdin rows need `fmt_stdin`, which has no PGO build) |
| `m4-a3-head-raw` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b,c,d --input raw` (head only) |
| `m4-d-inproc-harness` | `compare_rustfmt.sh -r 3 -w 1 --rounds 3 --core 3 --groups e --baseline <base>` (`bench -n 5` interleaved pairs and `conform -i`) |
| `m4-c-cache` | by-hand hyperfine, measurement C (commands in the section) |
| `m4-d-inproc` | by-hand `taskset -c 3 bench -n 5 --root <head fixtures>` triples, `benchmarks/profile.sh --runs 2 --core 3` per binary, `taskset -c 3 conform -i` per build (and `-V 1.8.0` for head) |
| `m4-e-parallel-items` | by-hand hyperfine, measurement E |
| `m4-noise-hir-ty-{1,2,3}` | `compare_rustfmt.sh -r 15 -w 2 --rounds 2 --core 3 --groups a,b --baseline <base> fx-hir-ty`, three times back to back |
| `m4-identity` | `env.txt`, `sha256.txt`, `sizes.txt`, `fixed-point.txt`, `loads.tsv` (the per-sweep load and `/proc/stat` lines) |

- Ratios are of means unless a column says `min`. `head/base` and `head/rustfmt` use sweep 1;
  `pgo/base` and `pgo/rustfmt` use the `chloro-base` and `rustfmt` runs of sweep 2 (same
  window as `head-pgo`); the `base` column shows sweep 1's base. The `ranges` column lists,
  in order, head vs base, head-pgo vs base, head vs rustfmt, head-pgo vs rustfmt: `below` =
  every run of the first command took less wall time than every run of the second; `overlap`
  = the min–max ranges intersect.
TODO-M4-METHOD-TAIL

TODO-M4-BODY
