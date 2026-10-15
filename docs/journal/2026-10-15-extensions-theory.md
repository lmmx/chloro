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

(filled from the records as they land)

## Ranked bounds and the compound picture

(filled from the records as they land)

## Handoff (overnight run)

State at 01:45 UTC, 2026-10-15 (kept current as steps land; the orchestrator session is
`session_01MyJjiAfjdkyrd1k4ZcR6h3`; the quota window resets at 04:40 UTC and check-ins are
scheduled for 03:15 and 04:45 UTC).

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
  record `2026-10-14-autoresearch/experiments/EXP-012-cli-waste.md`. On landing: reproduce
  the gates (fmt, clippy, `cargo test -p chloro --all-features`, output identity of `--check`
  and `--write` against the base binary on fixture and registry copies) and the hyperfine
  rows before merging; ledger row E12; README cache figure if it moves.
- rustd trial (ledger row pending; record to write:
  `2026-10-15-extensions-theory/rustd-trial.md`): rustd is a rust-analyzer fork with a build
  daemon compiling functions from RA's MIR with Cranelift, debug output only, pinned to
  `nightly-2026-05-10` (`harness/toolchain`), revision `b9c283a` (2026-10-15 01:14 UTC),
  shallow clone at `/home/user/dioxuslabs/rustd` (a blob-less first clone sits unused at
  `/home/user/dioxuslabs/rustd-blobless`). Found so far: the workspace's `rust-version =
  "1.98"` refuses the pinned nightly (rustc 1.97.0-nightly), so the release build runs with
  `--ignore-rust-version` (log: scratch `rustd-trial/build4.log`; started 01:41 UTC; time box
  45 min). Next: if `target/release/rustd` exists, run
  `python3 harness/cratetest.py /home/user/wt/rustd-trial/chloro-core --test basic --json ...`
  from `/home/user/dioxuslabs/rustd` (the worktree `/home/user/wt/rustd-trial` is chloro at
  `383fa28` with the `basic` test binary pre-built under the nightly); then `--test unit`;
  record compiled/skipped by reason, match, timings, RSS; then the edit-loop bench if
  coverage is meaningful. Stop conditions: build failure, essential functions skipped, any
  change needed in rustd. Baseline build-loop timings are in scratch `rustd-trial/baseline.txt`
  (debug lib build 9.7 s, test build 9.3 s, release 34–35 s, lib tests run 21.8 s).
- Queued experiments, to launch when the four cores are free (one at a time): E13 PGO step in
  `.github/workflows/binaries.yml` with local validation of the same commands; E14 allocator
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
