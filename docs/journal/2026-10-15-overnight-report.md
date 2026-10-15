# Overnight run, 2026-10-15: what landed, what was measured, what is left

The morning deliverable of the overnight mandate on the chloro performance programme
([2026-10-14-autoresearch.md](2026-10-14-autoresearch.md), protocol in
[2026-10-14-autoresearch/protocol.md](2026-10-14-autoresearch/protocol.md); the theory records
and the run's handoff in [2026-10-15-extensions-theory.md](2026-10-15-extensions-theory.md)).
Eight items, in the order asked. Every number names its record; nothing here is estimated
unless the sentence says so. Times are UTC.

## 1. What changed: commits, hashes, PR status

Integration branch `claude/chloro-perf-research-swarm-nrscj8`, PR
[lmmx/chloro#88](https://github.com/lmmx/chloro/pull/88) (open, ready for review, mergeable
against `master` `884f9c9`). The code head is the E16 merge `e640fbf`; the commits above it
are documentation (ledger, journal, README, this entry). Merges since the previous session's
end (`bdfb9d1`, R5), in order, each reproduced on the integration host before merging:

| merge | time | experiment | record | what it changes |
|---|---|---|---|---|
| `6802902` | 00:22 | M3 | [measurements/m3-quiet-host.md](2026-10-14-autoresearch/measurements/m3-quiet-host.md) | the quiet-host measurement of the E3 + R2 head |
| `92fdef0` | 02:10 | E12, CLI waste | [EXP-012-cli-waste.md](2026-10-14-autoresearch/experiments/EXP-012-cli-waste.md) | one canonicalisation per root and symlink, one sort, the executable's digest recorded under an identity stamp (`<hex>.chloro-exe`) |
| `45e8470` | 02:34 | E13, PGO in CI | [EXP-013-pgo-release.md](2026-10-14-autoresearch/experiments/EXP-013-pgo-release.md) | a "Train PGO profile" step in `.github/workflows/binaries.yml` for the three natively built targets; `benchmarks/pgo.sh --phase train\|use` |
| `4e48bdb` | 02:50 | E15, item cache | [EXP-015-item-cache.md](2026-10-14-autoresearch/experiments/EXP-015-item-cache.md) | `format_source_with_unit_cache`, the `UnitCache` trait, the opt-in `--unit-cache` CLI flag and `.chloro-units` file, `examples/unit_cache.rs` identity harness |
| `a3a60bd` | 03:16 | E17, parse floor | [EXP-017-parse-floor.md](2026-10-14-autoresearch/experiments/EXP-017-parse-floor.md) | `in_trait` passed down (P8), macro arguments lexed and parsed once (P5), trivia interspersal fused with the tree build (P1; old path kept as `steps_interspersed`, element-equality tests over both corpora) |
| `387d723` | 03:29 | E18, dead candidates | [EXP-018-dead-candidates.md](2026-10-14-autoresearch/experiments/EXP-018-dead-candidates.md) | the chain's dead vertical candidate skipped under a run-level guard (`RunState::whole_file`, `may_skip_dead_candidates()`) |
| `3fd39b4` | 03:42 | E14, allocators | [EXP-014-allocators.md](2026-10-14-autoresearch/experiments/EXP-014-allocators.md) | measurement only: `alloc-timing` and `alloc-{mimalloc,jemalloc,snmalloc}` features, all off by default; glibc stays |
| `e640fbf` | 04:12 | E16, scanner paths | [EXP-016-scanner-paths.md](2026-10-14-autoresearch/experiments/EXP-016-scanner-paths.md) | eleven local levers in the comment scanner, list gaps, `line_number`, width checks and string assembly; one SSE2 block |

Also committed: the lineage of the flat syntax array (`31ce20e`), six theory records under
`2026-10-15-extensions-theory/` (`383fa28`, `cf79ced`, `27b6221`, `af68024`, `5309fc3`,
`b27bcc8`), the synthesis entry with its ranked bounds, and the M4 record (see item 3).

## 2. Gates on the final head (verified, not inferred)

On `e640fbf` after the last merge, on the integration host (the same commands as every
earlier merge; protocol.md):

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | no warnings (the E14 features included) |
| `cargo test --workspace --all-features` | 290 tests pass, 0 fail |
| `bench -n 1 --check` against the golden `fixtures-e8d87c2.tsv`, sequential and `--threads 4` | 0 of 1238 outputs changed |
| `bench -n 1 --check` against `registry-e8d87c2.tsv`, sequential and `--threads 4` | 0 of 25,024 registry files changed |
| `conform -i` | 1220 of 1220 identical, 0 not idempotent (rustfmt 1.9.0 snapshots) |
| `conform -i -V 1.8.0` | 1220 of 1220 identical, 0 not idempotent (rustfmt 1.8.0 snapshots) |
| `examples/unit_cache --threads 1` and `--threads 4 --shift --blank` | 0 differences between the cached and the sequential formatter |
| `benchmarks/profile.sh --core 3`, two runs | 248,202,094 and 248,202,144 instructions |

Each experiment also passed the same gates on its own branch before the merge (its record);
E16 additionally ran the full `config_matrix -n 1220` (31 option values, identical on every
file). M4 (item 3) re-ran `conform -i` on the measured binaries: 1220 of 1220 identical at rustfmt 1.9.0 and at 1.8.0, 0 not idempotent, on the measured `head` binary; the instruction count 248,202,137 and 248,203,822 (two runs), 206,922,054 profile-guided; output agreement with rustfmt on every file of every harness workload, formatted and raw, recomputed with the final binaries.

## 3. Performance, before and after

Instruction count on the 49-file profile sample (`benchmarks/profile.sh`, callgrind, exact to
within a few thousand), the programme's gate metric:

| build | Ir | share of the start | record |
|---|---|---|---|
| `884f9c9`, the start | 587,988,576 | 1.000 | baseline.md |
| `113367a` / `4867896`, M3's head (E1–E8, R2) | 312,734,032 | 0.532 | M3 |
| `a3a60bd` (+ E12, E13, E15, E17) | 290,493,068 | 0.494 | ledger |
| `387d723` (+ E18) | 280,278,276 | 0.477 | ledger |
| `e640fbf` (+ E14, E16), the final head | 248,202,094 | 0.422 | ledger, M4 |
| the final head, profile-guided | 206,922,054 | 0.352 | M4 |

Tonight's merges took the count from 312,734,032 to 248,202,094 (−20.6%); the programme as a
whole from 587,988,576 (−57.8%).

Wall time against rustfmt 1.9.0 and against the start, on an idle 4-core KVM guest (2.80 GHz
class), rustfmt-formatted input, `--check`, cache disabled, n = 30 per cell
(`benchmarks/compare_rustfmt.sh`); M3 measured M3's head, M4 the final head, in separate
windows (protocol §3: ratios within a window, not absolute times across windows):

| quantity | M3 (`4867896`) | M4 (final head) | M4, profile-guided |
|---|---|---|---|
| nine crates, one core, chloro / rustfmt wall (range of means) | 0.32–0.54, every run below on all nine | 0.26–0.44, every run below on all nine | 0.24–0.38, every run below on all nine |
| nine crates, four cores (chloro one thread per core) | 0.105–0.203, every run below | 0.092–0.179, every run below on all nine | 0.072–0.175, every run below on all nine |
| nine crates, one core, chloro / `884f9c9` wall | 0.46–0.60 | 0.38–0.60 (0.38–0.50 without one noisy `reg-tokio` cell), every run below | 0.36–0.43, every run below |
| nine crates, four cores, chloro / `884f9c9` | 0.30–0.58 | 0.25–0.52, below on 8 of 9 | 0.22–0.45, below on 9 of 9 |
| in-process throughput, one core, 1238 fixtures (13.9 MB), best of 5, three interleaved triples | 9.39–9.54 MB/s (base 5.22–5.27) | 11.22–11.79 MB/s (base 4.96–5.09 in this window) | 13.03–13.24 MB/s |
| `--check` on the unchanged 1238-file tree, warm cache, four cores | 28.9 ms (434 ms `--no-cache`, 839 ms base) | 16.9 ms (372.7 ms `--no-cache`, 909.4 ms base) | 13.9 ms (316.9 ms `--no-cache`) |
| the same, one core | 37.1 ms (1564 ms `--no-cache`, 2917 ms base) | 24.4 ms (1305.9 ms `--no-cache`, 3069.7 ms base) | – |
| one 13-byte file (process start-up), one core | 1.7 ms (rustfmt 9.8) | 1.8 ms (rustfmt 11.9) | 2.2 ms (rustfmt 13.4), one slow run in 30 |
| a 640 KB file, four threads against one | 0.71 (1.41x) | 0.87 (1.15x; 0.60–0.87 over the five largest files, 1.15x–1.66x) | – |
| `conform -i`, one core, wall | 2.96 s (base 5.42) | 2.64 s (base 5.51); harness means 2.59 against 5.70 s, every head run below | – |

PGO: the release workflow now trains and applies a profile on the natively built targets
(E13). On this host the profile-guided final head is 0.8337x the plain head's instruction count on the sample (M3's profile was worth 0.7825x on M3's head: the E16–E18 levers shortened the paths the profile used to lay out), 0.24–0.38 of rustfmt's wall time on one core against 0.26–0.44 plain, 13.03–13.24 MB/s in process against 11.22–11.79.

## 4. rustd: verdict and evidence

**No-go for adoption now; on the watch list.** Record:
[2026-10-15-extensions-theory/rustd-trial.md](2026-10-15-extensions-theory/rustd-trial.md).
Bounded trial (time-boxed, stop conditions set in advance, release gates untouched), on
`DioxusLabs/rustd` revision `b9c283a`:

- The pinned toolchain (`nightly-2026-05-10`, rustc 1.97) does not build rustd itself at that
  revision (`rust-version = "1.98"`, then `substr_range` when the check is bypassed); rustd was
  built with stable 1.99.0 and the pin kept for the tested crate, which the harness's design
  allows.
- On `chloro-core`'s `basic` test binary: 3004 of 3004 functions compiled by rustd's
  Cranelift path, 0 skipped; 198 unit tests, every result matching cargo's debug build (the
  first MISMATCH was a harness artefact: no `rustfmt` component in the nightly; installed,
  rerun, MATCH).
- Edit-to-binary 1.4 s against 2.5–2.9 s for cargo's incremental debug build; the daemon
  holds about 1.3 GB resident.
- Why no-go: rustd produces debug builds only, and every gate of this programme runs on
  release builds (`lto = "fat"`, `codegen-units = 1`); the 1.1–1.5 s saved per debug
  iteration does not reach the release-gated loop; the pin/code mismatch means a fresh clone
  does not build as documented. What would change it is in the record (a consistent pin;
  rustd on its own test suite at rungs 2–3; a release or test-runner story).

## 5. Memory and the allocator, measured directly

E14 ([EXP-014-allocators.md](2026-10-14-autoresearch/experiments/EXP-014-allocators.md))
replaced the earlier instruction-share inference with direct measurement, as asked:

- glibc malloc's own CPU share, measured with a wrapping `#[global_allocator]` and `rdtsc`
  around every call on one thread: 13.7–16.3% of the formatter's time on the 49-file sample
  and 12.7–13.9% on the full fixture tree, against its 15.1% instruction share.
- Alternatives at the same sites: jemalloc 10.8% and snmalloc 9.9% of the time inside their
  calls, mimalloc 22.3% at defaults (`realloc` 911 ticks against glibc's 305) and 10.9% with
  huge pages off.
- Peak RSS (hyperfine `memory_peak_resident`, M3 harness rows): mimalloc at defaults 9.7–10.2x
  glibc on the warm `--check` (`madvise(MADV_HUGEPAGE)` with this host's THP at `madvise`),
  3.5x with THP and eager commit off, 1.05x with purge delay 0 but then slower; jemalloc
  1.3–1.7x (dirty pages held for 10 s, longer than the process lives); snmalloc 1.7–3.1x.
- Wall-clock ranges overlapped glibc's on all seven workloads for every allocator and setting.
  The rule set in advance (RSS within 1.2x on every workload, wall below with disjoint ranges
  on most) is met by none. **glibc stays.** The features (`alloc-timing`, `alloc-mimalloc`,
  `alloc-jemalloc`, `alloc-snmalloc`) are in the tree, off by default, for anyone who wants to
  re-measure.
- Peak RSS of the shipped configuration against rustfmt (hyperfine, max of 30 runs, M3 and
  M4): head 4–14 MiB on one core and 5–31 MiB on four cores across the nine crates, against 6–29 and 6–37 MiB for the start and 43–85 MiB for rustfmt (M4; M3 read the same ranges for head). The one cell where head's peak RSS exceeds the start's by more than 0.4 MiB is the parallel-items split on the 640 KB `lints.rs` (14.7 vs 10.4 MiB, four threads).

## 6. Experiments that failed or were rejected, with numbers

| experiment | what was tried | result | decision |
|---|---|---|---|
| E18 row 7 | literal widths computed once per literal | +1.75% Ir on top of row 5 (1,448 misses against 58 hits) | rejected, reverted; diff kept in the record |
| E16 S10 | `from_context` without the file-sized buffer | +481,878 Ir: `memcpy` from `_int_realloc` +1.44 M (blocks that grew in place now move) | reverted |
| E16 S16 | `contains('\r') && contains("\r\n")` in `normalize_src`; then `find_any_byte` | +333,280 and +50,562 Ir | reverted |
| E16 S12, the study's form | `rewrite_path` buffer sized from the source path | +205,695 Ir (the `reserve` calls belong to `single_ident_path`) | reverted; replaced by the kept `5950f4c` |
| E16 S7, second round | 18 more `format!` sites | +1,184,202 Ir (`memcpy` from `_int_realloc` +1.54 M) | reverted |
| E16 S5 without intrinsics | a flag loop for the compiler to vectorise | +7,562,583 Ir (no longer inlined) | replaced by the SSE2 form before commit |
| E15 as stated | the item cache as a default path with a 1.5x process-level falsifier on every file over 128 KiB | exact everywhere; in process 0.26–0.47; at process level the falsifier is met (pinned 1.19–2.03x against `--no-cache`, unpinned 0.90–1.71x; cold pass +10–15% wall, +4.6% Ir, 8.6 MB unit file) | refuted as stated; kept opt-in (`--unit-cache`) |
| E14 | mimalloc (four settings), jemalloc, snmalloc | no setting meets the RSS and wall rule (item 5) | glibc stays; measurement kept |
| rustd | the build daemon for the test loop | debug-only; pin mismatch; 1.4 s against 2.5–2.9 s | no-go now (item 4) |
| EXP-009 (previous session) | glibc malloc tunables | inside the spread | lever closed |

Nothing that was rejected is in the final head; every rejected lever's numbers are in its
record with the commit or diff that produced them.

## 7. Risks, unfinished work, how to resume

Risks in the shipped tree:

- One `unsafe` block entered `chloro-core` with E16 S5: an SSE2 unaligned 16-byte load from a
  `&[u8; 16]` in `comment.rs` (`sixteen_byte_flags`), gated on the target feature, with a
  randomised test against the scalar search. The programme had been `unsafe`-free until then.
- The unit cache (E15) is opt-in; its `.chloro-units` file is 8.6 MB for the fixture tree
  and the cold pass with it on costs +10–15% wall. Its correctness is exact by construction
  and harness-verified on both corpora; the risk is disk and the cold cost, not output.
- E12 records the executable's digest under an identity stamp (path, size, mtime, inode,
  device, ctime). A binary rewritten in place with all six unchanged would reuse a stale
  digest; the record documents this residual.
- E13's PGO step in `binaries.yml` is validated by running its commands locally on
  `x86_64-unknown-linux-gnu`, not by a release run; the first release should be checked for
  `no profile data available` lines and whether the `macos-13` runner still exists. A target
  whose training fails keeps the plain build.
- Print mode with several threads writes files in a varying order (same bytes); a small
  follow-up noted by E13's agent.
- README badges for dependency count and binary size are stale until `just emblazon` runs.
- Six conformance cases stay open (conformance-ledger.md cases 19–24).

Unfinished: the release run that validates the PGO workflow step; `just emblazon` for the README badges; the resident process and shared sink have no experiment.

To resume: the handoff section of
[2026-10-15-extensions-theory.md](2026-10-15-extensions-theory.md) has every experiment's
state and merge hash; the ledger
([experiments.md](2026-10-14-autoresearch/experiments.md)) has the best-known-good row and
the gates; `benchmarks/README.md` names the harness commands and the measurement records.
The base build for comparisons is `git worktree add /home/user/wt/base 884f9c9` and
`cargo build --release -p chloro --features cli && cargo build --release -p chloro-core
--examples` in it (its binaries' sha256 are in every measurement record and reproduce byte for
byte on this toolchain). Nothing needed to continue is outside the repository except the
golden hash files, which `bench --record` regenerates at the best-known-good commit.

## 8. The single highest-value next step

A resident chloro process that keeps the item cache (ranked first in the synthesis table of
the theory entry, with the item cache measured in E15 and the fixed per-run costs measured in
E12): an editor save on a large file would cost its parse plus one unit instead of a process
start, cache open and whole-file format. The measured components bound it at about 0.15 ms
per request against 4.3–6.0 ms today for a median file. It changes no formatter decision and
no output (the unit cache is exact by construction and harness-verified), it is the only
lever left whose gain is an order of magnitude rather than a percentage, and it is the one
the wall-clock benchmark against rustfmt does not show. The throughput lever behind it is the
shared sink (decide-once row 1; ≤ 8.8–10.5% Ir, exact by construction, `write_list` first).
