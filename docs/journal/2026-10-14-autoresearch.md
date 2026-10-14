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

**E7, flat syntax array (`20261014-flat-cst`, feasibility; prototype on the branch, record
merged).** A preorder array of (kind, subtree end, byte range) built from the same parser
events, with `Copy` node handles and a typed layer generated from rust-analyzer's grammar
(`rust.ungram`: 154 structs, 15 enums, 501 accessors, the same set as `ra_ap_syntax`'s
generated `nodes.rs`, 0 differences). On the 42 parsing files of the sample: build plus free
27.8 M instructions against rowan's 90.1 M (3.24x); a query workload that imitates
`format_expr_uncached`, `stmts_of` and `rewrite_path_segments` costs 15.5 M against 118.9 M
over red nodes (7.66x), 132 against 1,012 instructions per accessor result; element-by-element
equality with the rowan tree on all 1,042 parsing fixtures, and 10,142 macro-argument
fragments laid out at their file offsets without padding tokens. Estimated formatter-level
gain, with the arithmetic in the record: 146.5 M of 587.2 M (24.9%), assuming the formatter's
typed queries cost what the workload's do. The port touches 2,140 sites in 23 modules: 1,962
mechanical (imports and names), 164 needing an equivalent operation, 14 design sites in
`macro_args.rs`, `context.rs` and `formatting.rs`. The design problem is ownership: handles
borrow their tree, and fragment trees are created mid-run and held by the macro-argument
cache and the memo. E7b, the port, runs on `20261014-flat-port` with the arena design
(trees owned for the run's lifetime, memo keyed by tree id and index).

**E8, `rustfmt_version` (`20261014-rustfmt-version`, merged `79d69c9`).** A chloro-only
`Config` field with two values, `1.9.0` (default, unchanged path) and `1.8.0` (the build
shipped with Rust 1.88.0); `1.7.0` is refused by `Config::set` with the reason from the
matrix. Six formatting classes (F1–F6) and three acceptance classes (P1, P2, P8) are
branched next to the rustfmt code they reproduce, each with a comment naming the class and
the rustfmt change. Against `rustup run 1.88.0 rustfmt`: 1219/1220 fixtures at edition 2024
and at 2021 (the differing file is `0024_many_type_parens.rs`, as for 1.9.0); on a
20,019-file registry, 20,007 identical, where the 12 differences are the same files chloro
gets wrong under 1.9.0 as well, plus 3 files both releases reject and chloro formats. Two of
the three cases the matrix called partial turned out exact (F2's labelled closure bodies,
including a fourth site the matrix missed; F6's panic on Unicode whitespace); F5 stays
partial above 20 `_`-prefixed names, where the old sort's non-transitive comparator makes
the order depend on the sort algorithm. Snapshots: `snapshots/ra-1.8.0/` holds the one
fixture whose 1.8.0 output differs from the 1.9.0 snapshot, and a 32-case reproducer suite
is snapshotted for both releases at both editions, regenerated with
`CHLORO_RUSTFMT_VERSIONS=1.8.0=1.88.0`. The default path's instruction count moves by
−0.04%.

**E7b, the flat substrate under the formatter (`20261014-flat-port`, merged after the orchestrator reproduced every gate on `b4fdd1a`: 390,690,264 instructions on the sample, −24.7%).** The port replaces
rowan's red and green trees with the flat preorder array of E7: `chloro-core/src/cst/`
holds the `Tree`, `Copy` node and token handles borrowing it, a `TreeBuilder` fed by the
same parser events, a `typed-arena` of trees owned by the run (the file is tree 0, each
macro-argument fragment a later tree laid out at its file offset), the checked-in
`rust.ungram`, the generator `gen_ast.py` and its output `generated.rs` (the same 501
accessors as `ra_ap_syntax`'s generated layer), and a hand-written `ast.rs` for the
accessors rust-analyzer writes by hand. The memo is keyed by (tree id, node index, tag,
shape, flags) and stores no node; `rustc_compat` walks `Node` children; E4's block cache
stays as a handle-free cache keyed by (tree id, index). `rowan` and `ra_ap_syntax` become
dev-dependencies, kept for the equality tests that compare the flat tree with rowan's
element by element. No `unsafe`. Gates on the branch after merging E4 and then E8: 0 of
1238 fixture and 0 of 20,022 registry outputs changed, `conform -i` 1219/1220 at 1.9.0 and
at 1.8.0, `config_matrix` identical on all 31 option values, workspace tests, clippy, fmt,
`syn` absent. Measured on the sibling host against the E4-merged base built with the same
profile in the same window: instructions on the sample 518,953,599 → 391,198,779 (−24.6%;
390,689,701 after the E8 merge), `bench -n 5` 2.505–2.607 s → 1.843–1.982 s (5.3–5.6 →
7.0–7.5 MB/s, ranges apart), allocations on all fixtures 14,612,436 → 8,309,517 (−43%),
red nodes none by construction, the CLI binary 2,059,824 → 1,923,184 bytes, chloro-core's
runtime dependency tree 37 → 16 crates. The hardest bug was in the prototype's hand-written
`MacroDef::args`/`body`, which took the first and second token trees where rust-analyzer
takes the arguments only when there are two and the body as the last; the equality test
also showed that `first_token`/`last_token` must follow the first- and last-child chain,
not scan the preorder.

**Profile of the flat build** ([profile-flat.md](2026-10-14-autoresearch/profile-flat.md),
same sample and method as the first profile). 390,496,192 instructions: chloro's own code
44.1%, parser and lexer 22.3%, allocator 14.7% (glibc 12.8%; DHAT blocks 628,959 →
278,743, of which `String` creation and growth are now 63%), `core`/`std` 11.4%, the `cst`
tree build 2.8% and its reads and typed layer 4.1% in place of rowan's 87.6 M. The
tree-building callback costs 40.6 instructions per step against 229 before. Unchanged to
the instruction: `CharClasses::next` 28.3 M, `unicode_str_width` 20.7 M,
`find_uncommented` 10.5 M, `span_ends_with_comma` 8.8 M, the line table 4.9 M, the memo's
3.8% hit rate. Grown: the parser by 6.0 M (a codegen difference, cause not examined),
`rustc_compat::walk` by 0.9 M, and 4,345 more `Rc` blocks from a second `Rc<Cell<bool>>`
per `RewriteContext` that E7b introduced. Remaining levers by upper bound: a CLI allocator
≤12.8% (mimalloc rejected by E2 on memory), fewer intermediate `String`s per rewrite
≤10.7%, the parser's output decoded twice (`event::process` then `intersperse_trivia`)
≤7.4% but only with a parser change, `CharClasses` ≤7.2%, `unicode_str_width` ≤5.3%; E3
holds the last two and part of the second.

**E3, hot paths (`20261014-hot-paths`, merged `7cf3c9e`).** Nineteen levers, each a
separate commit kept only when the instruction count on the sample fell by more than 0.1%
with every output unchanged, replayed in order onto the flat build to attribute the gain:
390,690,354 → 316,301,692 instructions (−19.0%; the orchestrator's reproduction
316,076,716). The largest are an eight-byte check in `unicode_str_width` for ASCII strings
without a newline (−2.75%), first and last line widths without splitting the string
(−2.55%), an expression without attributes returning its rewrite directly instead of
passing through `combine_strs_with_missing_comments` (−2.39%), and
`span_ends_with_comma` reading only the last two code characters, since its answer depends
on nothing else (−2.22%); the comment scanner's `skip_plain` and a new `skip_comment` move
past stretches where the state machine cannot change, searching eight bytes at a time
(−4.4% over five commits); the line-start table is built only up to the position asked
(−1.10%); 35 hot `format!` calls with string arguments become one exact allocation (−0.98%).
Every skip has a test that compares it with the full state machine over every string of a
small alphabet up to length five. `malloc` calls on the sample fell 233,281 → 211,520 and
`realloc` 45,794 → 27,277; `bench -n 5` on the sibling host 7.67–7.82 → 9.30–9.50 MB/s,
ranges apart. Rejected with numbers: a vectorised newline count (+4.47 M), per-byte
lookahead in `skip_plain` (+784 K), the line table via `match_indices` (+519 K). The
`node_text` lever (−0.78% on the rowan build) was dropped as superseded by E7b. The levers
removed scans or copies rather than speeding the state machine up: `CharClasses::next`
itself keeps its structure and now costs 4.47 M.

**R2, the conformance ledger and fixes (`20261014-conformance-fixes`, merged `113367a`).** [conformance-ledger.md](2026-10-14-autoresearch/conformance-ledger.md)
records 24 cases: every mismatch found on the fixtures under all 31 option values, on the
20,850-file registry, and in the rustc-recovery and `#[rustfmt::skip::*]` gaps the earlier
entries left open, each with a reproducer, both outputs, the cause in rustfmt's or rustc's
code and the fix. Fourteen fixes, one commit each with a regression test: trailing
semicolons in nested blocks of a macro definition (`from_context` copied `is_macro_def`);
a repeat array `[x; n]` is not block-like; doc comments in macro arguments are tokens;
`rustc_compat` rejects `<-`, `..a = b`, `a..=`, an ambiguous `+` in types, unbraced const
parameter defaults, `builtin #`, pattern parameters in 2015 trait methods, misplaced doc
comments and C strings before 2021; a new `rustc_syntax` module rebuilds the tree where
rustc's parser accepts what rust-analyzer's rejects (the parenthesised first bound of a bare
trait object behind `0024_many_type_parens.rs`, `s.1e0` field names, const block items,
`impl impl Trait {}`); `#[rustfmt::skip::macros(..)]` and `::attributes(..)`; unknown
literal prefixes before 2021. Reproduced here on `b464243`: `conform -i` 1220/1220 at
1.9.0 and at 1.8.0 with 0 non-idempotent; all 31 option values fully identical (1022/1022,
editions 844/844, 843/843, 838/838); the fixture goldens change exactly `0024`, the
registry goldens exactly the 12 files the fixes target; 20540/20540 registry files
identical in the agent's live comparison. Six cases stay open and documented: doc comments
as attributes of expressions (no registry file affected), anonymous trait-method
parameters in edition 2015 (rust-analyzer's parser rejects them), rustfmt's own internal
error on one rustyline file (chloro's output equals what rustfmt still writes), rustfmt's
run time on one pest_meta file, non-idempotent outputs that are rustfmt's own (chloro on
rustfmt's first output equals rustfmt's second), and a trailing comma in `builtin #
offset_of`. The branch first cost 401,021,302 instructions on the sample against 390,690,264 for
its base (+2.6%), attributed to `rustc_compat::walk` (+5.5 M: new rules run at every node),
a path building blocks outside E4's cache (+0.6 M), the per-context skip state (+0.4 M) and
the allocations behind them (+1.6 M). Four commits removed it: the skip state became an
`Option<Rc<SkipContext>>` touched only by items whose attribute path starts with `rustfmt`;
const block items take their block from `RunState::block_of`; `rustc_compat` became one pass
over the preorder array, with ancestors found on demand after a rule's cheap conditions hold
(5.89 M against 9.38 M before the branch); `rustc_syntax::reparse` reuses the first lex and
parse and skips edits whose trigger text is absent. After the E3 merge the branch measures
312,736,691 against 316,299,929 for its base (−1.13%); reproduced here as 312,734,032
against 316,076,716.

## Milestone 1, measured on a quiet host (M1)

[measurements/m1-quiet-host.md](2026-10-14-autoresearch/measurements/m1-quiet-host.md)
measures `884f9c9` (base), `dc20cd7` (head: E6, E1, E2 merged) and a profile-guided build
of the same head against rustfmt 1.9.0 in an otherwise idle container (4 vCPU Xeon @
2.10GHz, a different host class from the 2.80GHz machine of the earlier entries, so only
ratios carry). `benchmarks/compare_rustfmt.sh`, 30 runs per cell, rustfmt-formatted input,
the cache disabled for every sweep but the cache sweep.

| crates (9), `--check`, formatted input | head/base | head-pgo/base | head/rustfmt | head-pgo/rustfmt |
|---|---|---|---|---|
| all four cores (chloro 4 threads, rustfmt 1) | 0.52–0.88 | 0.48–0.77 | 0.149–0.283 | 0.162–0.271 |
| one core (both pinned), wall | 0.76–0.98 | 0.67–0.96 | 0.566–0.890 | 0.498–0.834 |
| one core, CPU time | 0.76–0.99 | | 0.56–0.90 | |

- On all four cores every head and head-pgo run is below every rustfmt run on all nine
  crates. Head is below base with non-overlapping ranges on `hir`, `hir-ty`, `ide-db` and
  `tokio`; the other five overlap. Pinned, head is below rustfmt with non-overlapping ranges
  on `hir` and `hir-ty` only, and every pinned head-vs-base range overlaps: the per-core
  gain of this milestone is real in CPU time (0.76–0.99) but inside the host's spread in
  wall time.
- The cache: `--check` on the 1238 formatted fixtures takes 24.5 ± 2.9 ms warm against
  650.8 ± 44.4 ms with `--no-cache` (26.6x) and 773.4 ± 60.0 ms for base (31.6x); on one
  core 24.9 ms against 2342.3 ms (94x).
- In process, one core: base 5.44–5.99 MB/s, head 6.53–6.94 MB/s (ranges do not overlap),
  head-pgo 6.91–8.14 MB/s. Instructions on the sample: base 587,989,174; head 554,185,312
  (−5.75%); head-pgo 448,212,267 (−23.77%).
- Parallel items, one file at a time on four cores: head/base 0.57 (`hir/src/lib.rs`), 0.58
  (`nodes.rs`), 0.63 (`config.rs`), 0.65 (`hover/tests.rs`), 0.69 (`lints.rs`); CPU time
  1.00–1.23x, peak RSS up to 1.52x.
- Regressions, all inside the spread: start-up on a 12-byte file 2.2 ms against 2.0 ms on
  four cores (one core 1.4 against 1.5); `stdin-nodes` with four threads on one core 1.06;
  CPU time on `regex-syntax` 1.027; peak RSS of the split files up to 1.52x. No cell has
  any chloro build above rustfmt in mean, minimum, CPU or RSS.
- Harness defects found by the run and fixed afterwards on the integration branch:
  `benchmarks/pgo.sh` passed a relative `-Cprofile-use` path that rustc resolves from
  another directory (the root is now made absolute, and the script also builds a
  profile-guided `bench`); the harness had no way to disable the cache, so a formatted-input
  sweep of a cache-carrying build measured the cache after the warmup (M1 worked around it
  by unsetting `HOME`; the harness now passes `--no-cache` to any chloro that accepts it
  unless `--with-cache` is given).

Milestone 1 does not include E4 (−6.4% instructions, merged after M1 started) or anything
from E3, E7b, E8 and the conformance branch; milestone 2 measures the next integrated head
the same way.

## Milestone 2, measured on a quiet host (M2)

[measurements/m2-quiet-host.md](2026-10-14-autoresearch/measurements/m2-quiet-host.md):
`884f9c9` (base), `8d418ee` (head: E6, E1, E2, E4, E8, E7b) and a profile-guided build of
the head against rustfmt 1.9.0, in an idle container of the 2.80GHz host class of the
first profile (not M1's 2.10GHz class). Same method as M1, 30 runs per cell, formatted
input, the cache disabled through the harness.

| crates (9), `--check`, formatted input | head/base | head-pgo/base | head/rustfmt | head-pgo/rustfmt |
|---|---|---|---|---|
| all four cores (chloro 4 threads, rustfmt 1) | 0.475–0.620 | 0.364–0.536 | 0.130–0.201 | 0.104–0.188 |
| one core (both pinned), wall; CPU equal within 0.01 | 0.532–0.668 | 0.476–0.575 | 0.438–0.611 | 0.367–0.513 |

- Ranges: on four cores every head run is below every base run on 8 of 9 crates (`parser`
  overlaps) and every head and head-pgo run is below every rustfmt run on all 9; pinned,
  head is below rustfmt on all 9 (M1: 2 of 9) and below base on 7 of 9, head-pgo below both
  on all 9. Peak RSS: head below base on every crate; rustfmt 43–85 MiB against head
  5–31 MiB.
- In process, one core: base 4.94–5.03 MB/s, head 7.25–7.73, head-pgo 8.91–9.27, three
  disjoint ranges. `conform -i` 3785 ± 136 ms against 5839 ± 160 for base. Instructions on
  the sample: base 587,989,243; head 390,689,247 (−33.55%); head-pgo 306,889,893 (−47.81%).
- The cache: warm 30.4 ms against 523.4 ms with `--no-cache` (17.2x) and 917.2 ms for base
  (30.2x) on four cores; on one core 38.5 against 1952.4 and 3171.2 ms.
- Parallel items, one file at a time on four cores: head/base 0.386 (`hir/src/lib.rs`),
  0.403 (`nodes.rs`), 0.447 (`hover/tests.rs`), 0.496 (`config.rs`), 0.562 (`lints.rs`);
  CPU time now 0.68–0.92 of base (M1: 1.00–1.23; the substrate pays for the split).
- Regressions: start-up on a 12-byte file within 0.03–0.09 of base with overlapping ranges;
  peak RSS of split files up to 1.44x (`lints.rs` 15.1 against 10.5 MiB). No cell has any
  chloro build above rustfmt in mean, minimum, CPU or RSS, and no crate, stdin, cache or
  in-process cell has head or head-pgo above base.
- M2 reported the CLI's `--check` exiting 1 on the formatted copies of `hir/src/lib.rs` and
  `config.rs` while `fmt_stdin` agreed with rustfmt on them. The cause is in the harness, not
  the formatter: for stdin workloads the formatted copy was made by running rustfmt in file
  mode on the copy, which follows the crate root's `mod` declarations, finds no module files
  and formats nothing, so both copies stayed as found; `fmt_stdin` and `rustfmt < input.rs`
  then agreed with each other on the unformatted text, and the CLI's exit 1 was right. On a
  copy formatted through stdin the CLI's print mode, `--check`, `--threads 1` and
  `fmt_stdin` all return the text unchanged (checked here). The harness now formats stdin
  copies through stdin. M2's stdin rows for those two files timed a full format in every
  build, so the ratios stand.

## Current State

- The formatter reads syntax from a flat preorder array with a generated typed layer
  (chloro-core/src/cst/: mod.rs, generated.rs, ast.rs, gen_ast.py, rust.ungram); rowan and
  ra_ap_syntax are dev-dependencies for the equality tests only
  (chloro-core/Cargo.toml); chloro-core's runtime dependency tree is 16 crates
- Instructions on the 49-file sample under `benchmarks/profile.sh`: 312,734,032 at
  `113367a` against 587,988,576 at `884f9c9` (−46.8%); 306,889,893 for the profile-guided
  build of `8d418ee` (measurements/m2-quiet-host.md)
- In process on one core of the 2.80GHz host class: 7.25–7.73 MB/s on the 1238 fixtures
  against 4.94–5.03 for `884f9c9`; against rustfmt 1.9.0 on nine crates, formatted input,
  `--check`: 0.438–0.611 of its wall time on one core and 0.130–0.201 on four, with every
  chloro run below every rustfmt run (measurements/m2-quiet-host.md)
- Conformance is 1220/1220 identical with 0 non-idempotent outputs at `rustfmt_version`
  1.9.0 and 1.8.0, and all 31 option values are fully identical (`conform -i`,
  `config_matrix -n 1220` at `113367a`); 20540/20540 registry files identical in the live
  comparison; against the `884f9c9` output record the whole programme changed exactly the
  13 files the conformance fixes target (experiments.md, best-known-good)
- The CLI hands files to one worker per core from a largest-first queue and formats the
  top-level items of files of 128 KiB and more on all threads
  (chloro/src/cli/orchestrate.rs, chloro-core/src/formatter/parallel.rs); `--threads N`
  sets the count; `format_source_with_config_parallel` is the library entry
- The CLI records content digests of files found already formatted and skips formatting
  them on later runs; `--no-cache` disables it (chloro/src/cli/cache.rs); a check of 1238
  formatted files takes about 30 ms warm
- `Config::rustfmt_version` selects rustfmt 1.9.0 (default) or 1.8.0 output; 1.7.0 is
  refused with the reason (chloro-core/src/formatter/config.rs; `conform -V 1.8.0`;
  snapshots under chloro-core/tests/conformance/snapshots/ra-1.8.0/ and
  chloro-core/tests/conformance/rustfmt-versions/)
- The release profile is fat LTO with one codegen unit (Cargo.toml); `benchmarks/pgo.sh`
  builds a profile-guided CLI and `bench`
- `benchmarks/compare_rustfmt.sh` times rustfmt and chloro on the same file set per crate
  (read from rustfmt's `-v` output), on rustfmt-formatted copies by default, with the cache
  disabled unless `--with-cache`, with hyperfine JSON exports; `benchmarks/profile.sh`
  counts instructions on the 49-file sample; `benchmarks/report.py` writes the tables
- The research protocol, ledger, corpus manifest, profiles and measurement records are in
  docs/journal/2026-10-14-autoresearch/

## Missing

- Six conformance cases stay open and documented in conformance-ledger.md (cases 19–24):
  doc comments as attributes of expressions, edition-2015 anonymous trait-method
  parameters, rustfmt's internal error on one rustyline file, rustfmt's run time on one
  pest_meta file, non-idempotent outputs that are rustfmt's own, and a trailing comma in
  `builtin # offset_of`
- No quiet-host measurement of the final head (milestone 3)
- No profile-guided build in the release workflow; `.github/workflows/binaries.yml` is
  unchanged and the proposed step is in experiments/EXP-002-cli-build.md

## Divergence

- README badges state 33 dependencies and a 2.5 MB binary; the release CLI is 1,928,728
  bytes and chloro-core's runtime tree is 16 crates (the badges are regenerated by `just
  emblazon`, which was not run)
