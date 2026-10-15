# 2026-10-15: Extensions on paper: where the next large gains are, and their ceilings

Follows [2026-10-14-autoresearch.md](2026-10-14-autoresearch.md). That programme cut the
formatter's instruction count on the 49-file sample from 587,988,576 to 312,733,055 with
byte-identical output on both corpora (ledger: [2026-10-14-autoresearch/experiments.md](2026-10-14-autoresearch/experiments.md)).
This entry investigates, on paper and from the final head's profile, the extensions the
earlier entries proposed and the levers beyond them, and bounds each. Labels as before:
**measured** (a tool's output on the builds named), **read** (from source), **derived**
(arithmetic on measured numbers), **hypothesis** (not established). No formatter code was
changed for this entry; the experiments it led to are ledger rows E12 onward.

The records, one per area, are under [2026-10-15-extensions-theory/](2026-10-15-extensions-theory/):

- [decide-once-and-exact-reuse.md](2026-10-15-extensions-theory/decide-once-and-exact-reuse.md):
  the formatting-logic ceiling: speculation inside a rewrite, width-first exactness, exact
  reuse across shapes, a shared sink instead of `Option<String>`, memo extensions.
- [parser-and-substrate.md](2026-10-15-extensions-theory/parser-and-substrate.md): the parse
  floor: parallel parsing on the flat array, per-item and streaming substrates, an own
  parser or lexer, fused passes, the tree's own read cost.
- [incremental-and-process.md](2026-10-15-extensions-theory/incremental-and-process.md): an
  item-level output cache, a resident process, `--check` early exit, the CLI's own I/O and
  hashing, the allocator, build and link levers, scheduling.
- [scanning-and-strings.md](2026-10-15-extensions-theory/scanning-and-strings.md): per-byte
  scanners, widths carried with strings, ASCII and SIMD fast paths, comment handling, list
  and string assembly, the `core`/`std` share.
- [already-formatted-fast-path.md](2026-10-15-extensions-theory/already-formatted-fast-path.md):
  whether canonical text can be recognised more cheaply than it is produced; a verify mode.
- [rustd-trial.md](2026-10-15-extensions-theory/rustd-trial.md): a bounded feasibility trial of
  the `rustd` build daemon on chloro-core's tests (measured, not theory): 100% of root functions
  compiled and 198 tests matching rustc, edit-to-binary 1.4 s against 2.5–2.9 s for incremental
  cargo, 1.3 GB daemon; no-go for the programme's loop, which is release builds.

## Lineage

The levers here are the ones the 2026-10-10 and 2026-10-11 entries named and did not take
up, with the flat syntax array now in place (its lineage is in the autoresearch entry,
"Lineage of the flat syntax array"): the decide-once formatter (change-of-direction option
7, ranked first there, bounded by X9 at 17–18.5% for repeated rewrites and found inexact
across indentation by X9c), the per-item substrate and streaming (options 2 and 4), an own
parser (option 5), the item cache X12b made possible ("the basis for formatting items in
parallel or caching them"; the parallel half was built as E1), and the open question at the
end of the first-attempts entry, recognising canonical text.

## The final head, measured

Profile of `target/release/examples/bench -n 1` on the 49-file sample under
`benchmarks/profile.sh` at `31ce20e` (the code of the best-known-good `bdfb9d1`), 312,733,055
instructions, exclusive instructions grouped by component (the grouping script and the
annotated profile are in this container's scratch directory; the groups follow
[2026-10-14-autoresearch/profile-flat.md](2026-10-14-autoresearch/profile-flat.md)):

| component | instructions | share |
|---|---|---|
| formatter (`chloro_core::formatter`) | 124,486,278 | 39.9% |
| parser and lexer (`ra_ap_parser`, `ra_ap_rustc_lexer`) | 87,406,038 | 28.0% |
| allocator (glibc `malloc` family, `RawVec`) | 47,003,376 | 15.1% |
| `core`/`alloc`/`std` (searchers, `trim`, `memchr`, `fmt`) | 24,589,974 | 7.9% |
| flat tree (`chloro_core::cst`: build and reads) | 16,693,112 | 5.4% |
| libc `memcpy`/`memcmp` | 10,720,235 | 3.4% |

The shares are of the current profile, not amounts that can be removed: each record bounds
what its levers can take from the row it addresses.

Cache and branch simulation of the same run (`valgrind --tool=callgrind --cache-sim=yes
--branch-sim=yes`, the simulator's 32 KB 8-way L1 caches and 34 MB LL; measured):

| event | count | rate |
|---|---|---|
| instructions | 312,733,310 | |
| L1 instruction-cache misses | 5,233,543 | 1.67% of instructions |
| L1 data read misses / write misses | 775,094 / 568,783 | 0.95% of reads / 1.02% of writes |
| last-level misses (instruction / data read / data write) | 11,377 / 11,985 / 40,454 | |
| conditional branches / mispredicted | 49,625,122 / 3,848,380 | 7.8% |
| indirect branches / mispredicted | 2,628,825 / 973,446 | 37.0% |

- The instruction-cache misses sit in the largest layout functions: `write_list` 266,326
  (5.1% of all), `overflow::Context::rewrite` and its recursive instance 203,379 and
  168,588, `_int_malloc` 151,957, `rewrite_chain_from` 126,751 and 97,962, `memoize` 94,882,
  `rewrite_fn_base` 94,711. This is the code-layout component that profile-guided
  optimisation addresses (M3: −21.7% instructions, 0.78–0.92 of the wall time).
- The data-cache read misses sit in the parser's event replay (`event::process` 176,191,
  22.7% of all), `memcpy` (78,673), the `parent()` search of `visit_assoc_item` (45,871) and
  `rustc_compat::subtree_rejected_by_rustc` (42,370).
- The indirect-branch mispredicts sit in the parser's output decode and the tree build:
  `intersperse_trivia` 206,312, the `TreeBuilder::steps` closure 193,687, `event::process`
  146,229, the lexer's `advance_token` 93,438 — dispatch on token and node kinds.
- Last-level misses are negligible: the working set fits the cache; the formatter is
  instruction-bound, with a code-layout component and a parser-dispatch component beyond
  the instruction count.

## Bounds, by record

Each paragraph states what its record measured, what it bounds, and what became an experiment
tonight (ledger rows E12–E17). Shares are of the final head's 312,733,055 instructions on the
sample unless marked; wall-time figures are M3's or ratios within one window.

**A correction to the premise** (already-formatted-fast-path.md, measured): the 49-file profile
sample is not formatted input: 31 of its 49 files change under chloro (361 of 27,423 lines), and
chloro's cost barely depends on it (481.0 Ir/byte as found, 472.4 on a formatted copy). The
profile therefore describes mostly-formatted code, and every bound below holds on it.

**Incremental formatting and the process**
([incremental-and-process.md](2026-10-15-extensions-theory/incremental-and-process.md)). Two
measured findings about the CLI's fixed costs opened the night: reading and hashing the 2 MB
executable for the cache key cost 2.3–3.3 ms on every run, more than the 1.7–2.4 ms start-up,
and a warm-cache run was mostly kernel time (33.7 ms system against 14.8 ms user) from one
`canonicalize` per file (15,488 `readlink` calls) and two path sorts. Both became E12 (merged
at `92fdef0`): warm `--check` of the 1238 fixtures 0.56–0.61 of before, the cache's per-run
overhead 2.8–2.9 ms → 0.6–1.0 ms, `readlink` calls 8,060 → 4, output identical. The record's
item-level cache (L1) became E15: exact on both corpora, 0.26–0.47 of the sequential formatter
in process after a one-item edit in a large file, 1.2–2.0x at process level pinned against
`--no-cache` and 1.11–2.04x against today's cache, at a cold-pass cost of +10–15% wall and an
8.6 MB unit file when on (so it is opt-in). The levers not taken yet: a resident process (L2,
removes 4.0–5.7 ms of the 4.3–6.0 ms a median-file save costs: the largest latency lever by
ratio, 93–95%), a unit queue across files (L8, 6.6–34.6% of 4-thread wall on crates), `--check`
early exit (L7, ≤ 0.78 of a differing file), BOLT (L12, 0–5%, hypothesis), THP, static linking,
`mmap` (each ≤ 0.3 ms or ≤ 5.7% CPU). PGO in the release workflow (L5) became E13 (merged at
`45e8470`). The allocator (L6, L9) is E14's measurement. Scenario bounds the record derives:
CI cold run of the fixture tree 434 ms → about 300 ms; CI warm run 28.9 → 13–16 ms (E12 took
most of it); editor save of a median file 4.3–6.0 ms → about 0.15 ms with the item cache in a
resident process; one huge file cold 27.6 → about 14 ms (parse plus the largest unit is the
critical path).

**Scanning, widths and string assembly**
([scanning-and-strings.md](2026-10-15-extensions-theory/scanning-and-strings.md)). The per-byte
work E3 left is small; the per-call work is not (measured, per-instruction counts):
`find_uncommented` costs 245 Ir per call and finds its needle on the first step in 94.5% of
41,643 calls; `CharSearcher::next_match` about 80 Ir per call on 22-byte haystacks;
`skip_comment` returns at its state test in 94.8% of calls; list-gap comment handling costs
839 Ir per list item, 12.77 M (4.08%) in all, for 3 post-comments and 32 pre-comments in the
sample; newline counting 7.88 M (2.52%) at 4.75 Ir/byte, mostly maintaining a `line_number`
for nested visitors that is read in two places. Sixteen levers: local ones bound 37.8 M
(12.1%, about 275 M), all of them 47.4 M (15.1%, about 265 M); width and line facts compose
exactly across a join when the right part starts with an ASCII byte (unicode-width measures
right to left), which makes a "rewritten text with its facts" type exact. The local levers
became E16 (running). The record also found each nested visitor allocating a buffer twice the
file size and dropping it unused (868 times on the sample).

**The parse floor and the substrate**
([parser-and-substrate.md](2026-10-15-extensions-theory/parser-and-substrate.md)). Everything
under parsing is 119.96 M Ir, 38.4% of the total, more than the 28.0% parser-and-lexer row:
the tree build, `rustc_compat`, 4.7% of allocator calls (the parser's vectors doubling from
empty, 11.04 M) and the macro-argument fragments, which are lexed three times and parsed
twice (17.9 M), sit under the same roots (measured). Peak heap on the largest files is the
parser's 24-byte event vector, not the 16-byte-element tree (12.58 of 16.65 MB on `nodes.rs`,
measured). Eleven levers: fusing the trivia interspersal with the tree build (P1, likely 2.5%,
ceiling 8.9%), an own token conversion and direct parser input (P2, ≤ 5.5%), sizing the
parser's vectors (P3, ≤ 3.1%, needs a pinned fork), a table-driven lexer (P4, ≤ 3.6%, medium
risk), macro arguments parsed once (P5, 1.9% exact), the event replay writing the tree
directly (P6, ≤ 4.8%, fork), `rustc_compat` during the build (P7, ≤ 1.9%), passing `in_trait`
down instead of a parent search (P8, 0.63% on the sample but 4.55% of `hir/src/lib.rs` and
6.62% of `config.rs`), parallel parsing of top-level items on the flat array (P9: six
conditions read from the parser under which the concatenated tree equals the whole-file tree;
E1's four-thread wall on the five largest files 1.17–1.61x; the corpus equality check not run),
per-item CST or streaming (P10: memory only, peak heap −22% to −55%), an own typed parser (P11:
1–1.6% beyond the rest, very high risk, 15 ledger cases re-opened). If every lever landed the
parse-related part would be 33–38 M (10.6–12.1%), giving 12.7–13.0 MB/s single-threaded on
M3's host if time follows instructions; the likely subset without an own lexer or parser gives
about 10.7 MB/s; a parse that cost nothing would leave the formatter's own 192.8 M, 15.2 MB/s.
79.4% of the simulated indirect mispredicts are the parser's dispatch. P8, P5 and P1 became E17
(running).

**The already-formatted fast path**
([already-formatted-fast-path.md](2026-10-15-extensions-theory/already-formatted-fast-path.md)).
On formatted text 75.5% of bytes are token text fixed by the token sequence; the layout lives
in the 24.3% whitespace bytes and 1.88 M gaps (measured). Any recogniser that needs the tree
has the parse as its floor (32.9% of the formatted sample's Ir, 27.6–45.8% on the five largest
files); a source-guided verifier for `--check` would run at 0.40–0.42x of today's instructions
if no decision needs a rendered alternative, up to 0.76x if every multi-line decision does
(the line-count and next-line rules in `chains.rs`, `expr.rs`, `closures.rs`, whose cost is not
measured), and a rewrite rustfmt gave up on is canonical whatever it contains, so such regions
always format. Copy-through of single-line nodes inside the formatter bounds at −5.9% (measured
rate, statements) to −13.2–17.2% (all single-line nodes, hypothesis) less the check's own 1–3%.
Both are a second implementation of rustfmt's decisions kept in lock-step with the first; the
record's first experiment is to measure the rendered-alternative cost before building either,
dropping the verifier if it exceeds 0.3 of `format_source`. The proof of concept's lead came
from not laying out about three quarters of its input (copying itself cost 73 M of its 144 M
formatting instructions), and the head now runs at 1.046x its instructions while conforming.
60.8% of the 24,582 registry files are already formatted, 27.8% of the bytes.

**rustd** ([rustd-trial.md](2026-10-15-extensions-theory/rustd-trial.md), measured, not theory):
100% of chloro-core's root functions compiled and 198 tests matching rustc, edit-to-binary
1.4 s against 2.5–2.9 s for incremental cargo, 1.3 GB daemon, debug output only, a pin that
disagrees with its own code at this revision: no-go for the programme's release-gated loop.

**Decide-once formatting, exact reuse and a shared sink**
([decide-once-and-exact-reuse.md](2026-10-15-extensions-theory/decide-once-and-exact-reuse.md)):
being written; its paragraph follows when it lands.

## Ranked bounds and the compound picture

(filled from the records as they land)

## Handoff (overnight run)

State at 01:50 UTC, 2026-10-15 (kept current as steps land). The orchestrator session is
`session_01MyJjiAfjdkyrd1k4ZcR6h3`; the quota window resets at 04:40 UTC. Two scheduled
tasks wake that session (verified with `list_triggers`; both carry
`persistent_session_id` = the orchestrator): `trig_01PPWgQpP6SucLHeqt3UJ7EE` at 03:15 UTC
(safety check-in) and `trig_01Pmv5niPZZ3mcLPpgMzR9cf` at 04:45 UTC (quota-reset resume: verify
the quota first, then follow this section). If the session cannot be resumed, a new session
on this branch starts from this section and the scratch paths named here; everything not in
the repository is in the container's scratch directory
`/tmp/claude-0/-home-user-chloro/9623d6dc-d933-5682-9d83-7762284f5c7f/scratchpad/` and is lost
with the container.

- Integration branch `claude/chloro-perf-research-swarm-nrscj8`, PR #88, head pushed; the
  best-known-good code is `bdfb9d1` (ledger). Nothing here changes formatter code yet.
- Theory records being written by sibling agents (read-only, one file each under
  2026-10-15-extensions-theory/): `decide-once-and-exact-reuse.md`, `parser-and-substrate.md`,
  `scanning-and-strings.md`, `already-formatted-fast-path.md`. `incremental-and-process.md`
  landed (commit `383fa28`). When a record lands: read its ranked table, spot-check two of its
  measured numbers against the profile in the scratch directory (`profile-final/`), commit
  it, fill its row of "Bounds, by record" above, push.
- E12 (`20261015-cli-waste`, worktree `/home/user/wt/cli-waste`, agent running): canonicalise
  once per root and per symlink instead of per file, one sort, a stamp-cached tool key;
  record `2026-10-14-autoresearch/experiments/EXP-012-cli-waste.md`. Landed and merged at
  `92fdef0` after the orchestrator reproduced the gates in the worktree (fmt, clippy, 37 CLI
  tests; `--check --no-cache` on the formatted fixture copy: same exit code and byte-identical
  stdout against the base binary) and the headline on this loaded host, base against new
  interleaved: warm `--check` of the 1238 fixtures on 4 threads 46.0 → 25.6 ms mean (0.56,
  n = 15); one file with the cache on, core 3, 7.3 → 4.3 ms against 3.3 ms with `--no-cache`
  (cache overhead 4.0 → 1.0 ms). The agent's own measurements are in the record. The README's
  warm-cache figure (28.9 ms, M3) stays until a quiet-host measurement (M4) is taken; the
  record notes one documented weakening of invalidation (a binary rewritten with identical
  path, size, mtime, inode, device and ctime).
- rustd trial (ledger row pending; record to write:
  `2026-10-15-extensions-theory/rustd-trial.md`): rustd is a rust-analyzer fork with a build
  daemon compiling functions from RA's MIR with Cranelift, debug output only, pinned to
  `nightly-2026-05-10` (`harness/toolchain`), revision `b9c283a` (2026-10-15 01:14 UTC),
  shallow clone at `/home/user/dioxuslabs/rustd` (a blob-less first clone sits unused at
  `/home/user/dioxuslabs/rustd-blobless`). Found so far: the workspace's `rust-version =
  "1.98"` refuses the pinned nightly (rustc 1.97.0-nightly); with `--ignore-rust-version`
  the pinned nightly fails on `hir-def`'s use of the then-unstable `substr_range` (scratch
  `rustd-trial/build4.log`): at revision `b9c283a` the harness pin and the code disagree. The
  pin governs the toolchain whose rlibs are linked, not the compiler that builds rustd, so
  attempt 5 builds rustd itself with stable 1.99.0 (installed) and keeps the nightly for the
  tested crate (log: `rustd-trial/build5.log`, started 01:47 UTC, time box 45 min; a
  background loop in the orchestrator waits for its `build exit` line). Next: if
  `target/release/rustd` exists, run
  `python3 harness/cratetest.py /home/user/wt/rustd-trial/chloro-core --test basic --json ...`
  from `/home/user/dioxuslabs/rustd` (the worktree `/home/user/wt/rustd-trial` is chloro at
  `383fa28` with the `basic` test binary pre-built under the nightly); then `--test unit`;
  record compiled/skipped by reason, match, timings, RSS; then the edit-loop bench if
  coverage is meaningful. Stop conditions: build failure, essential functions skipped, any
  change needed in rustd. Baseline build-loop timings are in scratch `rustd-trial/baseline.txt`
  (debug lib build 9.7 s, test build 9.3 s, release 34–35 s, lib tests run 21.8 s).
- E15 (`20261015-item-cache`, worktree `/home/user/wt/item-cache`, agent running since 01:50
  UTC, time box 4 h): the item-level unit cache of the study's section 1, exact by
  construction (units of `parallel.rs`, positions rebased, whole-file checks unchanged), with
  the identity harness over both corpora (cold and warm passes against the sequential
  formatter), the edit scenario on the five largest fixtures, and the cold-path overhead;
  record `2026-10-14-autoresearch/experiments/EXP-015-item-cache.md`. First result (02:45
  UTC, branch pushed, six commits to `b8c3882`): exact everywhere (1238 fixtures and 24,582
  registry files, cold, warm, moved and blanked, 1 and 4 threads; goldens and conform unchanged
  with the cache off; Ir +0.0005% off); in process 0.26–0.47 of the sequential formatter after
  a one-literal edit; at process level 1.2–2.0x pinned and 0.9–1.7x unpinned against
  `--no-cache` (fixed per-run costs, since removed by E12, and the 4-thread item split dilute
  it), 1.11–2.04x against today's fixed-point cache; cold pass with the cache on +10–15% wall,
  +4.6% instructions, an 8.6 MB unit file. The agent was sent back (02:50 UTC, one hour) to
  merge the E12/E13 head into the branch, make the unit cache opt-in (`--unit-cache`), and
  re-measure the edit scenario and the cold overhead. On landing: reproduce the fixture identity
  harness and the Ir gate, then merge with the cache off by default.
- E13 (`20261015-pgo-ci`, worktree `/home/user/wt/pgo-ci`, agent running since 02:00 UTC, time
  box 1.5 h): the profile-guided build in `.github/workflows/binaries.yml` for native targets,
  validated locally by running the workflow's commands (goldens, conform at both versions, Ir
  against 312.7 M, hyperfine on two crates); record
  `2026-10-14-autoresearch/experiments/EXP-013-pgo-release.md`. Landed and merged at `45e8470`
  after the orchestrator reproduced, on the branch's profile-guided `bench`: 0 of 1238 fixture
  outputs changed against the golden record and 245,255,322 instructions on the sample (0.784 of
  312,733,055; the agent measured 245,639,781, M3 244,712,537: the profile differs per training
  run); the workflow YAML parses and `pgo.sh` passes `bash -n`. The workflow step itself (Linux
  x86-64, macOS x86-64, Windows x86-64 natively; aarch64 cross builds keep the plain build) is
  confirmed only by the next release run: check its log for `no profile data available` lines
  and whether the `macos-13` runner still exists. The agent also noted that print mode with
  several threads writes files in a varying order (same bytes): a small follow-up.
- rustd trial: done, recorded in `rustd-trial.md` (verdict: no-go for adoption now, watch list).
  The rustd checkout and build at `/home/user/dioxuslabs/rustd` and the worktree
  `/home/user/wt/rustd-trial` can be removed; the record has the reproduction commands.
- E14 (`20261015-allocators`, worktree `/home/user/wt/allocators`, agent running since 02:47
  UTC, time box 2.5 h): the allocator's own CPU share measured directly with a wrapping global
  allocator behind an off-by-default feature; glibc against `mimalloc` (default and
  memory-bounded options), `tikv-jemallocator`, `snmalloc` on the M3 harness rows (wall, CPU,
  peak RSS); decision rule: RSS within 1.2x of glibc on every workload and wall below with
  disjoint ranges on most, else measurement only; record `EXP-014-allocators.md`.
- E16 (`20261015-scanner-paths`, worktree `/home/user/wt/scanner-paths`, agent running since
  02:55 UTC, time box 3.5 h): the local levers of `scanning-and-strings.md` section 7 (S3, S1,
  S2, S4, S7, S8, S5, S12, S15, S10, S13, S16; bound about 12% of instructions), one commit per
  lever with the E3 gates (Ir per commit, both goldens, conform at both versions); record
  `EXP-016-scanner-paths.md`. On landing: reproduce the goldens, conform and Ir on the branch
  head before merging.
- Theory records landed and committed: `incremental-and-process.md`, `scanning-and-strings.md`,
  `parser-and-substrate.md`, `already-formatted-fast-path.md`, `rustd-trial.md`; still being
  written: `decide-once-and-exact-reuse.md` (relaunched 02:16 UTC after the 01:31 interrupt
  cancelled the first set). When it lands: commit it, then fill "Bounds, by record" and "Ranked
  bounds and the compound picture" above from the five records' tables.
- E17 (`20261015-parse-floor`, worktree `/home/user/wt/parse-floor`, agent running since 03:05
  UTC, time box 3 h): the exact, fork-free parse-floor levers of `parser-and-substrate.md` (P8
  pass `in_trait` down instead of a parent search, P5 macro arguments lexed and parsed once, P1
  fused trivia interspersal and tree build gated by element equality over both corpora), one
  gated commit per lever; record `EXP-017-parse-floor.md`. On landing: reproduce goldens,
  conform and Ir on the branch head before merging.
  measurement (glibc vs `mimalloc` with memory options vs `tikv-jemallocator` vs `snmalloc`;
  wall, CPU, peak RSS on the M3 harness rows, and the allocator's own CPU share measured
  directly with a wrapping `#[global_allocator]` in an example binary; `perf` is absent in
  this container); E15 item-level cache prototype on a branch once the records are in.
- Morning report: commits and PR status, gates, M3 and PGO numbers, rustd verdict, RSS and
  allocator findings, failed experiments, risks and resume instructions, the single
  highest-value next step.

## Current State

- The five records under 2026-10-15-extensions-theory/ exist as listed above (those present
  at the time of this commit are linked; the others are written by sibling agents and are
  added when they land)
- The final head's component shares and cache/branch simulation are recorded above
  (measured at `31ce20e`)

## Missing

- The synthesis sections above are placeholders until every record has landed
