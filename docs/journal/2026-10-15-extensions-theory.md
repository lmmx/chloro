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
([decide-once-and-exact-reuse.md](2026-10-15-extensions-theory/decide-once-and-exact-reuse.md)).
Speculation inside a rewrite, which X9 left unmeasured, is about one sixth of the run: 15.6–18.1%
by three estimates from per-call-site and per-branch counts of the instruction-level profile
(measured counts, derived shares), and it is the same work as X9's repeats. Half of it is one
point, `choose_rhs`, which renders the next-line right-hand side for 177 of 703 right-hand sides
(30.61 M, 9.79% at that site) and discards it 160 times. `write_list` never runs for a tactic that
is then not taken, and fn signatures render once: the discarded work is whole candidates. A
node's one-line text is the same at every shape except through source fallbacks (a failed macro
rewrite, a lost comment), but whether it is chosen depends on per-construct width limits, on
items fitting at the nested indent and, for blocks, on a rendered line count; so only the one-line
fragment maps onto Oppen/Wadler/Prettier-style measure-then-render printing, and comparing line
counts of rendered multi-line alternatives, failure as control flow, `force_one_line_chain` and
the side effects of attempts (flags that reach macro-body output, `format_failure` under 1.8.0)
break the equivalence. Seven levers: a shared sink with rollback by truncation (≤ 8.83–10.52%,
exact by construction, 191 `-> Option<String>` signatures and 38 `Rewrite` impls in 19 files);
multi-line reuse by tracked slack (≤ 10.2%, exact in principle, about 250 width tests must record
their slack: high risk); a one-line memo without the shape in its key (≤ 1.47%, up to 7.9%,
exact under five stated conditions); a bottom-up one-line width pass (2.1–3.4% net, medium-high
risk, 1,500–3,000 lines); the chain's dead vertical candidate skipped when the one-line candidate
fits (≤ 3.03%, estimate 0.75–2.79%, exact by argument with a run-level guard, about 20 lines);
the nested visitor's doubled buffer (0.44%, 5 lines); literal widths computed once (0.28%).
Compound: all speculation removed exactly plus the sink gives 229–238 M (−24% to −27%); the
levers exact without a width model (sink, chain skip, buffer, literal widths) give 269–281 M.
The chain skip and the literal widths became E18 (running); the buffer is E16's S10; the sink is
the next large refactor, testable on `write_list` alone first.

## Ranked bounds and the compound picture

The records' levers, merged and ranked by the strongest supported gain per unit of risk and
cost. "Ir" rows are shares of 312,733,055 on the sample (the formatter's single-thread cost);
"wall" rows are process-level effects the instruction count does not show. Overlaps are stated.

| rank | lever (record) | gain | evidence | risk | cost | state |
|---|---|---|---|---|---|---|
| 1 | resident process with the item cache (incremental L2 + L1) | editor save 4.3–6.0 ms → about 0.15 ms per request; a one-item edit in a large file 0.26–0.47 in process | measured per component (E15 in process; start-up and cache costs measured) | low for output; lifecycle | a new mode of about the CLI's size | not started |
| 2 | shared sink (decide-once row 1) | ≤ 8.8–10.5% Ir, exact by construction; absorbs scanning S6/S9 | derived from DHAT and the profile | low on output, high on implementation | every rewrite signature | not started; `write_list` first |
| 3 | scanner and list-gap fast paths (scanning S1–S16 local) | 37.8 M, 12.1% Ir bound; the record's per-call counts | measured per call site | low | ~60 lines per lever | E16 running |
| 4 | parse floor without a fork (parser P1, P2, P5, P7, P8) | about 37 M, 12% Ir likely (P1 2.5–8.9%, P2 ≤ 5.5%, P5 1.9%, P7 ≤ 1.9%, P8 0.6%, 4.6–6.6% on large files) | measured shares, hypotheses on the fused cost | low–medium | 150–300 lines each | E17 running (P8, P5, P1) |
| 5 | PGO in the release binaries (incremental L5) | −21.7% Ir, 0.78–0.92 wall per crate | M3; E13 reproduced 0.784 | none on output | CI step, 2x release build time | merged (E13) |
| 6 | CLI per-run waste (incremental L3, L4) | warm run 0.56–0.61, cache overhead 2.8 → 0.6–1.0 ms | measured | low | done | merged (E12) |
| 7 | speculation removed with a width model (decide-once rows 2–4) | up to 15.6–18.1% Ir; with the sink 229–238 M (−24 to −27%) | measured volume; the width model's exactness is the open proof | medium–high | 1,500–3,000 lines plus every decision site | not started |
| 8 | dead chain candidate and literal widths (decide-once rows 5, 7) | 0.75–3.03% and 0.28% Ir | measured site costs, estimate on the firing rate | low (run-level guard) | ~60 lines | E18 running |
| 9 | parallel parse of top-level items (parser P9) | four-thread wall on files ≥ 128 KiB 1.17–1.61x | derived from M3; six equality conditions read, corpus check not run | low with a sequential fallback | 300–500 lines | not started |
| 10 | unit queue across files (incremental L8) | 6.6–34.6% of four-thread wall on crates | derived from M3 CPU/4 | low | ~150 lines | not started |
| 11 | allocator (incremental L6, L9) | ≤ 8.0–8.4% Ir; wall unproven; RSS is the risk | E14 measuring directly | RSS | 3 lines or the sink | E14 running |
| 12 | copy-through of single-line nodes (fast path F1) | −5.9% to −17% Ir | measured rate for statements; hypothesis beyond | medium–high: a second one-line formatter in lock-step | 1–2k lines | not started |
| 13 | source-guided verifier for `--check` (fast path V1) | 0.40–0.76x Ir on formatted input | derived; the rendered-alternative cost is the open measurement | high: a second implementation of every decision | the formatter's size | not started; first experiment: measure that cost |
| 14 | own lexer or parser (parser P4, P11) | ≤ 3.6% and about 1–1.6% beyond the rest | hypotheses | medium / very high (15 ledger cases re-opened) | 600–1,000 / 5,195 lines | not planned |
| 15 | per-item CST, streaming (parser P10) | memory only: peak heap −22% to −55% on the largest files | measured peaks | low | high | not planned (peak RSS is already 4–31 MiB) |
| — | rustd | edit-to-binary 1.4 s against 2.5–2.9 s incremental cargo on chloro-core | measured | debug-only, 1.3 GB, pin mismatch | — | no-go now |

Compound picture for the formatter's single-thread cost (derived; the levers' overlaps are
removed as each record states, and every figure is a bound until its experiment runs): the
exact levers without a width model (sink 8.8–10.5%, scanning 12.1%, parse floor about 12%, chain
skip and literal widths 1–3%, PGO on top) bound the sample at about 200–215 M instructions
before PGO against 312.7 M today, 0.64–0.69x; with the width-model levers, about 165–180 M,
0.53–0.58x; PGO (0.784x measured) on top of either: 0.50–0.54x and 0.42–0.45x. On M3's host,
where 312.7 M corresponds to 9.4 MB/s in process, that is 13.6–14.7 MB/s exact-only and
16.2–17.7 MB/s with the width model, before PGO (hypothesis: wall time follows instructions,
which the parser's mispredict share argues against for its part). The parse floor's irreducible
33–38 M caps the formatter-free ceiling at 77–89 MB/s and the realistic one at 12.7–13.0 MB/s if
only the parser levers landed.

For latency, the ranking inverts: a resident process removes the 4–6 ms fixed cost that
dominates a median-file save (E12 took 2 ms of it; 3.8 ms remains as process start and cache
open), and the item cache makes a large-file edit cost its parse plus one unit; together they
bound the save at about 0.15 ms against 4.3–6.0 ms today. Neither touches the single-thread
instruction count, which is why both the throughput levers above and the latency levers are
worth pursuing, and why the throughput benchmark alone understates what the levers above do
for a developer's loop.

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
  +4.6% instructions, an 8.6 MB unit file. The agent was sent back (02:38 UTC, one hour) to
  merge the E12/E13 head into the branch, make the unit cache opt-in (`--unit-cache`), and
  re-measure the edit scenario and the cold overhead. Landed (`8ae8a92`: merged the E12/E13 head,
  `--unit-cache` opt-in, `--no-cache` turns both caches off; re-measured pinned minima all five
  files 1.56–2.09x against `--no-cache`, unpinned 1.13–1.52x) and merged at `4e48bdb` after the
  orchestrator reproduced in the worktree: identity harness `examples/unit_cache` on the fixtures
  at 1 and 4 threads with `--shift --blank`, 0 differences (13,595 units, 13,203 found shifted;
  13,672 checked blanked); goldens with the cache off 0 of 1238 sequential and at 4 threads;
  `conform -i` 1220/1220 at both versions; `Ir` 312,735,284 (+0.0004%); fmt, clippy and every
  workspace test (189 core unit tests now). The ledger row E15 is on the merge; the library
  entry point is `format_source_with_unit_cache`, the CLI flag `--unit-cache`.
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
  disjoint ranges on most, else measurement only; record `EXP-014-allocators.md`. Landed
  (`8e6e36c`) and merged at `3fd39b4` (features off by default; Ir and goldens unchanged, reproduced).
  Result: no allocator meets the rule. glibc's own time share measured directly with a wrapping
  allocator and `rdtsc`: 13–16% of the formatter's time on one thread (13.7–16.3% on the sample,
  12.7–13.9% on the full tree), against its 15.1% instruction share; jemalloc 10.8% and snmalloc
  9.9% inside their calls, mimalloc 22.3% at defaults (realloc 911 against 305 ticks) and 10.9%
  with huge pages off. Peak RSS: mimalloc defaults 9.7–10.2x glibc on the warm `--check`
  (`madvise(MADV_HUGEPAGE)`, this host's THP is `madvise`), 3.5x with THP and eager commit off,
  1.05x with purge delay 0 but then slower; jemalloc 1.3–1.7x; snmalloc 1.7–3.1x. Wall ranges
  overlapped glibc's on all seven workloads for every setting. glibc stays.
- E16 (`20261015-scanner-paths`, worktree `/home/user/wt/scanner-paths`, agent running since
  02:39 UTC, time box 3.5 h): the local levers of `scanning-and-strings.md` section 7 (S3, S1,
  S2, S4, S7, S8, S5, S12, S15, S10, S13, S16; bound about 12% of instructions), one commit per
  lever with the E3 gates (Ir per commit, both goldens, conform at both versions); record
  `EXP-016-scanner-paths.md`. Landed (`4e8080c`: eleven levers kept, −11.14% Ir on its base,
  S10, S16 and the record's form of S12 rejected on numbers, the full `config_matrix -n 1220`
  identical on all 31 values, `bench` 1.11–1.12x in three disjoint pairs) and merged at `e640fbf`
  after the orchestrator reproduced on the branch (goldens, conform at both versions, Ir
  278,125,396, fmt, clippy, 231 core tests) and resolved two code conflicts with E15/E18
  (both imports in `context.rs`; `parallel.rs` keeps E15's cache path and `stitch()` with E16's
  `line_number()` accessor). Gates on the merged head: goldens 0 of 1238 and 0 of 25,024,
  sequential and 4 threads; `conform -i` 1220/1220 at both versions; the unit-cache identity
  harness 0 differences at 1 and 4 threads; Ir 248,202,094–248,202,144 (−57.8% from the start,
  −20.6% tonight); fmt, clippy with every feature, 290 workspace tests. One `unsafe` block
  entered chloro-core with S5 (an SSE2 unaligned 16-byte load from a `&[u8; 16]`, gated on the
  target feature, with a randomised test against the scalar search). The best-known-good is
  `e640fbf`.
- All six records are committed and the synthesis sections above are written (02:58 UTC).
- E18 (`20261015-dead-candidates`, worktree `/home/user/wt/dead-candidates`, agent running since
  02:57 UTC, time box 2 h): the chain's dead vertical candidate skipped with a run-level guard
  and literal widths computed once (decide-once rows 5 and 7); record `EXP-018-dead-candidates.md`.
  Landed (`a02cec5`: the chain skip kept at −3.40% Ir with a run-level guard, firing 2,024 times
  on the sample; the literal-width cache rejected at +1.75%, 1,448 misses against 58 hits) and
  merged at `387d723` after the orchestrator reproduced on the branch (goldens, conform at both
  versions, Ir 302,372,347 on its base, fmt, clippy, tests) and then ran the gates on the merged
  head itself: goldens 0 of 1238 and 0 of 25,024 registry files, sequential and 4 threads;
  `conform -i` 1220/1220 at both versions; Ir 280,278,276–280,279,614 (−52.3% from the start,
  −10.4% tonight); fmt, clippy and every workspace test. The best-known-good is `387d723`.
- M4 (05:03 UTC): the quiet-host chain ran on the idle host from 04:30:43 to 05:02:54 UTC as
  one script (`scratchpad/m4/chain.sh`; identity, a1/a2/a3 harness sweeps, in-process harness,
  cache, bench triples with `profile.sh` and `conform -i`, parallel items, three noise runs;
  every sweep exit 0). Record: `2026-10-14-autoresearch/measurements/m4-quiet-host.md`. The
  first start was stopped after a minute because the stdin copies on disk predated the
  harness's stdin preparation; rebuilt and verified as fixed points, then restarted. Found on
  the way: the harness's `identical outputs` figure is computed when a copy is made and
  stored in its `meta`, so a reused copy reports a stale count (raw `tiny-test-data` showed
  M2's 288/289 for a head that produces 289/289); every workload's agreement was recomputed
  with the final binaries (all identical, both inputs); the harness now recomputes the
  figure per binary (`agreement()` in `compare_rustfmt.sh`, the commit after the record). README,
  ledger, the autoresearch entry and this entry carry M4; the overnight report is
  `2026-10-15-overnight-report.md`.
- Wave 3 (launched 09:25 UTC from `8ebd3bb`; the brief and ranking are in
  `2026-10-15-unmined-tricks.md` §7): four agents on worktrees, each with a 4–5 h time box,
  a record under `2026-10-14-autoresearch/experiments/`, and the branch to push:
  E19 `20261015-cliffs` (/home/user/wt/cliffs; EXP-019-cliffs.md), E20 `20261015-sink`
  (/home/user/wt/sink; EXP-020-sink.md), E21 `20261015-resident` (/home/user/wt/resident;
  EXP-021-resident.md), E22 `20261015-layout` (/home/user/wt/layout; EXP-022-layout.md). On
  each report: reproduce gates 1–5 in the worktree (goldens sequential and 4 threads, conform
  at both versions, Ir, fmt, clippy, tests; for E22 also the `--sim` co-metric), merge one
  experiment per merge, re-run gates on the merged head, ledger row, best-known-good only on
  reproduced gates; then wave 4 (E23–E27 of the same table) and a quiet-host M5. If this
  session is not resumed, a new session starts from this bullet and the four branches.
- Wave 3 progress (11:15 UTC): E20 reported and **merged at `60de639`** after the integrator
  reproduced its gates in the worktree (goldens 0 of 1238 sequential and 4 threads, 0 of
  25,024 registry; conform 1220/1220 at both versions; `Ir` 238,032,982 against the agent's
  238,033,138; fmt, clippy, 291 tests) and re-ran gates 1–3 on the merged head (identical
  goldens and conform; `Ir` 238,033,298 at the default sample path). Best-known-good is
  `60de639`, −4.10% from `e640fbf`. E22 reported (co-metric `profile.sh --sim` kept;
  `steps_fused` kept, −0.105% `Ir`, −3% `Bcm`; hot/cold splits, flag tables and BOLT rejected;
  the layout lever closed under the plateau rule), its gates were reproduced in the worktree
  (goldens, conform, `Ir` 247,935,835, co-metric, fmt, clippy, 290 tests) and it is **merged at
  `9d78713`**; post-merge gates identical, `Ir` 237,768,533 at the default sample path, which is
  the best-known-good. E19 reported (six exact levers, −4.59% on its base, output identical; a
  real cliff in nested closures with exponent 3.6 that no small exact change fixes, rustfmt
  itself exponential there; a per-file co-metric isolating the heap-layout term) and was asked
  to merge the integration head into `20261015-cliffs` first, because its `f18dac3` is the same
  lever as E20's `c40a441` in lists.rs; it re-gated on the merged branch (232,050,509 at the default path, −2.41%; per-file sum
  −3.08%), the integrator reproduced every gate in the worktree (goldens, conform, `Ir`
  232,050,590, co-metric, fmt, clippy, 296 tests) and **merged it at `05ab7b4`** (post-merge
  gates identical; `Ir` 232,047,700). Best-known-good is `05ab7b4`, −60.5% from the start. The
  unfixed cliff (nested closures, exponent 3.6; rustfmt itself exponential there) goes to E26's
  brief: an exact fix needs a lower bound on one-line width. E26 (`20261015-width`,
  /home/user/wt/width, from `e312e39`) launched 11:58 UTC with that brief: a token-sum lower
  bound on one-line width with a per-token-class proof, a verification harness over both
  corpora, pre-checks at one-line-only attempt sites, the cliff exponent before and after;
  record EXP-026-width-bound.md. E21 is running; E24 and E27 run from `2f00988`. Wave 4's E24 (memo side array) and E27 (harness: registry
  sample, parse-only phase, path-independent count, per-file mode) launch from this head. The external research brief and
  report are journalled in `2026-10-15-external-research-brief.md` with seven adopted
  additions (a registry validation sample, rollback as state, generation-stamped side arrays,
  the additive term in cliff fits, imbalance diagnostics first, a parse-only phase, the daemon
  client's own latency) that change the wave-4 briefs and the protocol.
- Order of the remaining morning work once E14, E16, E17 and E18 report: reproduce and merge each
  (Ir, goldens sequential and 4 threads, conform both versions; for E14 the RSS rule); then the
  quiet-host measurement M4 of the final head against `884f9c9` and rustfmt with
  `benchmarks/compare_rustfmt.sh` and `profile.sh` when no other job runs; README and
  best-known-good from M4; PR #88 body; the morning report (the user's eight items).
- E17 (`20261015-parse-floor`, worktree `/home/user/wt/parse-floor`, agent running since 02:41
  UTC, time box 3 h): the exact, fork-free parse-floor levers of `parser-and-substrate.md` (P8
  pass `in_trait` down instead of a parent search, P5 macro arguments lexed and parsed once, P1
  fused trivia interspersal and tree build gated by element equality over both corpora), one
  gated commit per lever; record `EXP-017-parse-floor.md`. Landed (`7f6e98e`: all three kept,
  −7.11% Ir, tree equality 0 mismatches on 1,330 + 34,008 file trees and 14,579 + 625,316
  fragment trees) and merged at `a3a60bd` after the orchestrator reproduced: goldens 0 of 1238
  sequential and 4 threads, 0 of 25,024 registry files; `conform -i` 1220/1220 at both versions;
  Ir 290,493,068; fmt, clippy, chloro-core tests including the two fused-build equality tests.
  The best-known-good moves to this merge (ledger).
  measurement (glibc vs `mimalloc` with memory options vs `tikv-jemallocator` vs `snmalloc`;
  wall, CPU, peak RSS on the M3 harness rows, and the allocator's own CPU share measured
  directly with a wrapping `#[global_allocator]` in an example binary; `perf` is absent in
  this container); E15 item-level cache prototype on a branch once the records are in.
- Morning report: commits and PR status, gates, M3 and PGO numbers, rustd verdict, RSS and
  allocator findings, failed experiments, risks and resume instructions, the single
  highest-value next step.

## Current State

- Six records under 2026-10-15-extensions-theory/ (five theory records and the rustd trial)
  are committed, each with its ranked table and its scratch evidence paths; the synthesis
  above merges their tables (docs/journal/2026-10-15-extensions-theory/)
- The final head's component shares and cache/branch simulation are recorded above
  (measured at `31ce20e`, the code of the best-known-good)
- Experiments the records led to, all merged with their gates reproduced on the integration
  host: E12 (`92fdef0`), E13 (`45e8470`), E15 (`4e48bdb`, opt-in), E17 (`a3a60bd`), E18
  (`387d723`), E14 (`3fd39b4`, measurement only), E16 (`e640fbf`, the best-known-good); ledger
  rows in 2026-10-14-autoresearch/experiments.md
- M4, the quiet-host measurement of the final head
  ([measurements/m4-quiet-host.md](2026-10-14-autoresearch/measurements/m4-quiet-host.md)):
  0.26–0.44 of rustfmt's wall time per crate on one core and 0.092–0.179 on four, every run
  below; 248,202,137 `Ir` (0.4221 of the start), 206,922,054 profile-guided; warm cache 16.9
  ms; the README carries it
- The morning report of the overnight run:
  [2026-10-15-overnight-report.md](2026-10-15-overnight-report.md)

## Missing

- The resident process and the shared sink are the two ranked levers with no experiment; the
  M4 record lists what it did not measure (no PGO build of `fmt_stdin` or `conform`, no pinned
  cold-cache row, the harness's stored agreement figure)
- The resident process (rank 1) and the shared sink (rank 2) have no experiment yet
- Every bound in the tables above is a bound until its experiment runs; the width-model levers
  rest on an exactness proof that does not exist
