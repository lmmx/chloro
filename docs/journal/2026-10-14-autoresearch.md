# 2026-10-14: Autoresearch programme: speed and conformance

Follows [2026-10-11-phase-2-close.md](2026-10-11-phase-2-close.md). That entry closed phase
2 by decision with the port at 4.0–4.1 MB/s on the fixtures, 1219/1220 conformance, and a
list of deferred levers. This entry opens a new programme with a different method: several
agents investigate separate hypotheses on separate branches at the same time, a fixed set
of gates decides what is kept, and a ledger records every attempt including the failed
ones. Supporting documents live in [2026-10-14-autoresearch/](2026-10-14-autoresearch/):

- [protocol.md](2026-10-14-autoresearch/protocol.md): the objective function, the
  acceptance corpora, the metrics and their noise, the experiment procedure, the promotion
  rule for the best-known-good commit, and the plateau rule;
- [experiments.md](2026-10-14-autoresearch/experiments.md): the append-only ledger and the
  current best-known-good;
- [profile-baseline.md](2026-10-14-autoresearch/profile-baseline.md): where instructions
  and allocations go on the starting commit;
- [baseline.md](2026-10-14-autoresearch/baseline.md): process-level measurements against
  rustfmt and the harness that produces them (`benchmarks/`);
- [conformance-ledger.md](2026-10-14-autoresearch/conformance-ledger.md): every known
  mismatch with rustfmt 1.9.0, root-caused;
- [rustfmt-versions.md](2026-10-14-autoresearch/rustfmt-versions.md): where rustfmt 1.7.0,
  1.8.0 and 1.9.0 differ on real code, and the design for supporting more than one;
- `experiments/EXP-NNN-*.md`: experiments with more than a page of evidence.

Labels as in the earlier entries: **measured**, **read** (from source), **hypothesis**.

## What was asked

- Larger speedups against rustfmt than the current 2–3x, with correctness and conformance
  as constraints and the library's structure preserved.
- Full conformance rather than 1219/1220.
- Support for more than one rustfmt version.
- Parallel investigation by agents on isolated branches, named `YYYYMMDD-slug`, with the
  evidence recorded so that a sceptical reviewer can check every claim without this
  conversation.
- No edits to `docs/JOURNAL.md`.

## State found (measured, this host)

Host: 4 vCPU "Intel Xeon Processor @ 2.80GHz", 15 GB, Linux 6.18; `rustc 1.97.0`,
`rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`; valgrind, hyperfine; no `perf`.

- `target/release/examples/bench -n 5`: 1238 files, 13.9 MB, 3.041 s, **4.57 MB/s**. The
  five slowest files are the same as in the phase-2 entry (`lints.rs` 78.6 ms,
  `nodes.rs` 70.0 ms, `hir/src/lib.rs` 67.2 ms, `lower.rs` 50.3 ms, `eval.rs` 38.2 ms).
- `conform -i`: **1219/1220 identical, 0 not idempotent**. The one difference is
  `parser/test_data/parser/err/0024_many_type_parens.rs`: for
  `Box<(?Sized) + (for<'a> Trait<'a>) + (Copy)>` rustfmt drops the parentheses of the
  first bound and chloro keeps them. The first bound is `(?Sized)` or `(for<'a>
  Trait<'a>)`, a parenthesised bare trait object; where the first bound is `(Copy)`, a
  parenthesised path, both tools keep the parentheses. **Read**: rustc's
  `parse_ty_tuple_or_parens` sends a `TyKind::Path` through `parse_remaining_bounds_path`
  with `parenthesized = true` and a one-bound `TyKind::TraitObject` through
  `parse_remaining_bounds`, which carries no parenthesis flag; rustfmt prints what the AST
  holds. The fix belongs in chloro's type rewriting, not in the parser.
- Process level, first order (hyperfine, 5 runs each, `--check`, both tools on the same
  crate, rustfmt following `mod` declarations from the crate root, chloro given the
  directory): `hir` (15 files, 624 KB) rustfmt 641.6 ± 34.7 ms against chloro
  112.2 ± 24.7 ms; `hir-ty` (122 files, 2.8 MB) rustfmt 1611.8 ± 138.0 ms against chloro
  280.6 ± 29.4 ms. Peak RSS 216.9 MiB against 21.0 MiB and 115.9 MiB against 34.5 MiB.
  rustfmt is single-threaded; the chloro CLI runs one thread per core. Per core, from these
  figures, chloro does the work in roughly 0.7x of rustfmt's time on `hir-ty`
  (280.6 × 4 / 1611.8); the fixture files are not already formatted, so both tools lay out
  every file. These are first-order numbers taken before the harness existed, and they
  overstate chloro's lead: on input that is not yet formatted, rustfmt's `--check` builds
  and prints a line diff for every changed file, which on `regex-syntax` took 1706 ms and
  423 MiB against 592 ms and 66 MiB for `rustfmt --emit stdout` on the same bytes
  (R4, [baseline.md](2026-10-14-autoresearch/baseline.md) "Cost of `--check`"). chloro's
  `--check` compares two strings. The harness therefore formats each crate copy with rustfmt
  first, so that both tools lay out every file and neither reports anything; on that input,
  under a load average of 6–10 from the other agents, the crate-level wall ratios were
  0.48–1.14 on all cores (chloro 4 threads, rustfmt 1) and the one-core CPU-time ratios
  0.62–1.00. `winnow` ran slower under chloro than under rustfmt in that window (1.14 on
  all cores, 1.00 on one core). Every wall-clock figure in baseline.md is provisional until
  the quiet rerun; the ratios above are the ones to beat, not the 5.7x of the first-order
  runs.
- CLI scheduling (**read**, `chloro/src/cli/orchestrate.rs:29-61`): files are sorted by
  path and cut into `threads × 4` static chunks; a chunk is one thread's whole share, so the
  wall time of a crate is bounded by the chunk that holds its largest files, and a single
  large file is formatted by one thread.
- Installed rustfmt versions for the version question: 1.7.0 (`rustup run 1.80.0`), 1.8.0
  (`1.85.0` and `1.88.0`, different commits), 1.9.0 (stable). rustfmt sources for v1.7.0,
  v1.8.0 and v1.9.0 are checked out under `/home/user/ref/` (outside the repository).
- Registry corpus: 577 crates, 20,570 `.rs` files, 709 MB, fetched by the manifest recorded
  in [corpus/](2026-10-14-autoresearch/corpus/). It is larger than and different from the
  5659-file registry of the earlier entries, so earlier registry figures are not comparable
  with figures on this corpus.

## What the earlier entries bound, and what they leave open

The earlier entries measured most of the formatter-internal levers, and several are closed
with numbers:

- repeated rewrites (the same node under another shape) are 17–18.5% of `format_source`;
  reuse across indentation is not exact under any rule tested (X9c), so the reachable part
  of that share is small;
- answering comment queries from a token index cost 3% more instructions than rescanning
  text (X8); source-side text scanning is 4–5% in all;
- `mimalloc` gave 5–10% in the benchmark, inside wall-clock noise, and a library cannot
  pick its users' allocator;
- `lto = "fat"` with one codegen unit gave 2–8%, at the edge of noise, and was not pursued;
- the rowan tree's construction caps throughput at about 13–14 MB/s for any formatter on
  that tree; a flat array builds for 0.32 s where rowan takes 0.56 s on the fixtures, but the
  formatter reads the tree through several hundred typed accessors that only exist for red
  nodes, and the formatter-level effect of another substrate is not measured;
- red-node traversal costs 4–9x the green walk for the same answers; G1 and G2 moved
  attribute and keyword queries to the green tree for −2.5% and −1.2% instructions;
- the allocator family is about 22% of instructions, red nodes about half of all
  allocations, strings and lists the rest; 50.2 MB of candidate strings are built for
  14.2 MB of output;
- top-level items format the same standalone as in-file once gap text is set aside, on
  13,650 fixture units and 965,193 registry units (X12b) — the basis for formatting items
  in parallel or caching them, neither of which was built;
- parsing is single-threaded and the parse-drop thread is gone (D1), so instruction
  counts are deterministic.

What is open, and what this programme tests, falls into four groups.

**Work the formatter does per byte.** Allocation (22%), red-node creation (10% plus half
the allocations), comment scanning (`comment` module, 55 M of 595 M on the sample), width
measurement over built strings (`utils`, 19 M), and the parse-plus-build third of wall
time. Each has an upper bound that the baseline profile fixes before any experiment starts;
the experiments then go after the largest rows first, and the plateau rule in the protocol
stops variants of a closed lever.

**Work the formatter does not need to do on this machine.** rustfmt runs one thread. The
chloro CLI already runs one thread per core, with static chunks; the time of a crate is the
time of its heaviest chunk, and the time of a single large file is the time of one thread.
Two levers follow: dynamic scheduling of files across threads (largest first), and
formatting the top-level items of one file on several threads, which X12b makes possible.
Both change wall time without changing instruction counts, so they are judged by the
process-level harness alone.

**Work the binary does before and after formatting.** Build profile (LTO, codegen units,
PGO), the allocator for the CLI binary only, process startup, file reading and writing.
These change the CLI's numbers against rustfmt without touching the library's semantics.

**Work that can be skipped soundly.** A formatter run in CI or on save mostly sees files it
formatted before. A persistent cache keyed by the content hash, the configuration and the
chloro version turns that case into hashing. This is a CLI feature with no effect on
`format_source`; the risk is in the key, not in the formatter.

## Plan

Wave 1, running in parallel on this host (one core each, pinned):

| id | question | output |
|---|---|---|
| R1 | where do instructions and allocations go on `884f9c9`; upper bound per lever; LTO re-measured | `profile-baseline.md` |
| R2 | every mismatch with rustfmt 1.9.0 (default, 31 option values, registry corpus, rustc-recovery cases), root-caused; fixes with regression tests on `20261014-conformance-fixes` | `conformance-ledger.md` |
| R3 | which rustfmt releases differ from 1.9.0 on the fixtures and the registry, by construct; a design for a version knob | `rustfmt-versions.md` |
| R4 | a fair process-level harness (crate-level single-process rustfmt against chloro on the same file set; single-core and all-core; stdin; many small files) with hyperfine and JSON output; a baseline run | `benchmarks/`, `baseline.md` |

Wave 2, experiments on `20261014-*` branches, each with its own ledger row. Chosen before
the profile is in because the earlier entries already motivate them:

| id | branch | hypothesis |
|---|---|---|
| E1 | `20261014-parallel-items` | formatting a file's top-level items on several threads, stitched by the visitor's gap handling, cuts the wall time of a large file by close to the core count with identical output (X12b) |
| E2 | `20261014-cli-build` | LTO, one codegen unit, PGO, a CLI-only allocator and largest-first dynamic scheduling each move the process-level numbers; measured one at a time |
| E6 | `20261014-cli-cache` | a content-hash cache makes `--check` on an unchanged tree I/O-bound |

Chosen after R1 ranks the rows:

| id | lever |
|---|---|
| E3 | allocation count: buffer reuse, writing into a shared buffer instead of returning `Option<String>` where rustfmt's control flow allows, list and chain scratch vectors |
| E4 | green-tree reads on the top red-node creation paths (the "re-ranking on the D1 baseline" deferred in phase 2) |
| E5 | parse and tree construction: a `NodeCache` shared across files, `to_input` copies, `intersperse_trivia` |
| E7 | a flat syntax array behind the typed accessor names, prototyped on one hot path and measured at the formatter level before anything wider |
| E8 | the version knob from R3, one branch point per behaviour class |

Integration: each kept branch merges into `claude/chloro-perf-research-swarm-nrscj8` after
the gates in the protocol, the golden hashes are re-recorded, and the ledger's
best-known-good row moves. The README's figures change only to numbers from
[baseline.md](2026-10-14-autoresearch/baseline.md) taken on the merged tree.

## Results so far (measured unless labelled)

Each experiment's own record is under `2026-10-14-autoresearch/experiments/`; the ledger
row names the gate results and the commit. What follows is the orchestrator's reading.

**E6, CLI cache (`20261014-cli-cache`, merged `d9f5a86`).** The cache records the BLAKE3
digest of each file's content found to be a fixed point, in a file named by the digest of
the running binary and of the serialised `Config`. On a copy of the 1238 fixtures that is
entirely fixed points, `--check` takes 32.7 ± 2.2 ms warm against 819.4 ± 65.3 ms with
`--no-cache` on the sibling host (25.1x; 100.7x on one core); reproduced here under load
average 8.8 at 46.6 ± 9.3 ms against 1517.8 ± 158.3 ms (32.5x). The library is untouched.
Costs: +188 KB of binary (`blake3`), one extra format of each file `--write` rewrites (the
new content is recorded only after a second pass returns it unchanged, so no idempotence is
assumed), and a cache directory under `~/.cache/chloro` by default. On a tree where most
files would change, the cache changes nothing measurable: the changed files are formatted
regardless.

**E1, items in parallel (`20261014-parallel-items`, merged `7faa8c9`).** A file is parsed
once; worker threads build their own red trees over the shared green tree and format
top-level units (an item, or a `use`/`mod`/`extern crate` group that `reorder.rs` sorts
together) from a longest-first queue, each with its own visitor and `RunState`. The gap
before a unit is not written by the worker: the unit's first `format_missing*` call is
recorded and replayed by the stitching visitor on the real buffer, which keeps every
comment, blank line and skipped item on the sequential path. Equality: 0 of 1238 fixture
outputs and 0 of 20,847 registry outputs differ between 4 threads and `format_source`,
checked here. Gain on the sibling host (Xeon @ 2.10GHz, 4 cores, hyperfine 15 runs,
minima): 1.50–1.88x on the five largest fixtures, 1.20–1.82x on the fixture crates; the
largest-first file queue alone is 1.24–1.49x of the crate gain. The ceiling on this corpus
is about 2x, not the core count: parsing stays sequential (12–29% of a file) and the
longest unit bounds the rest (`lints.rs` is two `const` arrays of 288 KB and 223 KB).
Splitting medium files hurts crates (oversubscription); the CLI splits files of 128 KiB and
more. CPU time per split file rises 4–21%. `format_source` and `format_source_with_config`
are unchanged; the parallel path is `format_source_with_config_parallel`.

**E2, build profile, PGO, allocator, scheduler (`20261014-cli-build`).** Fat LTO with one
codegen unit: −8.09% instructions on the sample (R1), `bench` pairs −18.5%, −8.2%, −9.5%
on the sibling host, binary −26%, build time 21 s → 47 s; kept as `[profile.release]`.
PGO on top: `bench` pairs base against combined −29.0%, −17.0%, −19.6% with ranges that do
not overlap (2.48–2.53 s against 1.76–2.06 s); kept as `benchmarks/pgo.sh` with a proposed
`binaries.yml` step in the record, not applied to the workflow. `mimalloc` in the CLI:
rejected — no wall-time gain outside the host's spread and 4–10x peak RSS (`hir` 23.5 →
142 MiB) from eager arena commits; relaxing the commit policy removed the gain with the
memory. `panic = "abort"`: rejected, since the CLI joins worker threads with
`unwrap_or_default()` and goes on to report the other files after a panic, which abort
would end. `target-cpu=native`: headroom only, not shippable. E2's scheduler is the same
design as E1's and is not merged twice.

**R4, the harness and the first honest baseline.** The crate-level comparison runs rustfmt
on the crate root and chloro on exactly the files rustfmt's `-v` output names, on copies
that rustfmt has formatted first (see "State found" for why). Under load, chloro's CLI on
four cores takes 0.48–1.14 of rustfmt's wall time per crate and 0.62–1.00 of its CPU time
on one core; rustfmt's start-up is 9–13 ms against chloro's 2–3 ms; peak RSS is 43–85 MiB
against 7–37 MiB. These are the numbers the integrated build is measured against on a quiet
host; nothing in this entry claims a ratio from them yet.

**R3, rustfmt releases.** On the fixtures at edition 2024, rustfmt 1.8.0 (Rust 1.88.0)
formats 1 of 1001 accepted files differently from 1.9.0 and rejects 3 that 1.9.0 accepts;
on the 20,593-file registry, 2 differing and 5 rejections. The two 1.8.0 builds (Rust
1.85.0 and 1.88.0) format every file identically and differ only in parser acceptance. The
differences fall into 14 formatting classes (F1–F14: `r#` on labels, a closure label, `use
self::self`, `super let`, pre-2024 ordering of `_` imports, a panic on Unicode whitespace,
…) and 8 acceptance classes (P1–P8: let chains before 2024, `[const]` bounds, frontmatter,
…). 1.7.0 has no 2024 style rules: 540 of 981 fixtures differ, 528 of them only in import
order, and `version=Two` is unavailable to it. chloro already produces 1.9.0's output on
every class reproducer except F13 (a `macro_rules!` body that is one `const {}` block). The
design — a chloro-only `rustfmt_version` setting branched like `style_edition`, with
snapshots for the differing files only — is being implemented as E8 for 1.8.0; 1.7.0 is
recorded as unsupported with the numbers above.

## Current State

- Formatting runs rustfmt 1.9.0's algorithm single-threaded in process at 4.57 MB/s on
  the 1238 fixture files, measured with `target/release/examples/bench -n 5` at `884f9c9`
- Conformance is 1219/1220 identical with 0 non-idempotent outputs
  (`target/release/examples/conform -i` at `884f9c9`)
- The CLI hands files to one worker per core from a largest-first queue and formats the
  top-level items of files of 128 KiB and more on all threads
  (chloro/src/cli/orchestrate.rs, chloro-core/src/formatter/parallel.rs); `--threads N`
  sets the count
- The research protocol, ledger and corpus manifest are in
  docs/journal/2026-10-14-autoresearch/ (protocol.md, experiments.md, corpus/)
- `benchmarks/compare_rustfmt.sh` times rustfmt and chloro on the same file set per crate
  (read from rustfmt's `-v` output), on rustfmt-formatted copies by default, with hyperfine
  JSON exports; `benchmarks/profile.sh` counts instructions on the 49-file sample;
  `benchmarks/report.py` writes the tables (benchmarks/README.md)
- The CLI records content digests of files found already formatted and skips formatting
  them on later runs; `--no-cache` disables it (chloro/src/cli/cache.rs, merged from
  `20261014-cli-cache`, E6)

## Missing

- No quiet-host run of `benchmarks/compare_rustfmt.sh`; every wall-clock figure in
  baseline.md was taken at load average 6–10
- No support for a rustfmt version other than 1.9.0 (E8 running on the design in rustfmt-versions.md)
- `0024_many_type_parens.rs` differs from rustfmt (R2 running)
