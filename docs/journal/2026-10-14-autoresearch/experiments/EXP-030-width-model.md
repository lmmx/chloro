# EXP-030: an exact one-line width model, and what step 1 left of it

Ledger row E30 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md). The brief is the follow-up of [EXP-026](EXP-026-width-bound.md)
("What a width model adds", "The fit predicate is narrow", "Caching the bound", "A negative
memo") and row 4 of [../../2026-10-15-extensions-theory/decide-once-and-exact-reuse.md](../../2026-10-15-extensions-theory/decide-once-and-exact-reuse.md)
(the exact one-line width `W(n)`, the nested-shape condition `R`, 2.1–3.4% of instructions
after its own cost). Labels: **measured** (a tool's output on the builds below), **read** (from
source), **derived**.

## Identity

| field | value |
|---|---|
| branch | `20261015-width-model` from `4ad3b54` (was `03c61be`) (integration head; its code is the best-known-good `a26b460` (was `e96b530`)), worktree `/home/user/wt/width-model` |
| kept commits | `9b2a6d2` (was `2c6bc21`) (a closure body that renders a `match` arm is never one line: the S1 pre-check skips it), `288b1e8` (was `5274f96`) (a line reference in its documentation; the release `bench` is byte-identical); then this record |
| rejected | the chain-width rule (a chain wider than `chain_width` is never one line): committed as `94b0c6b` (was `3d471d9`), reverted in `b8ba4b5` (was `d6b4d73`) on the registry condition (also `scratchpad/e30/chain-width-rule.patch`) |
| not built | the exact width `W(n)` and its three uses, the negative memo, the bound cache: every one has a measured bound under the brief's 0.3% (§ 1) |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, valgrind 3.22.0, `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`; `cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples` (fat LTO, one codegen unit) |
| host | 4 vCPU, shared with E29 and E31 throughout; the session was interrupted from 2026-10-15 18:31 to 2026-10-17 06:31 UTC (account rate limit, host reboot); every binary's sha256 was checked after the reboot. `Ir` is load-independent; no wall-clock figure is claimed |
| `Ir` command | `benchmarks/profile.sh --core 3 --runs 2|3 --per-file --bench <bin> [--sample-list benchmarks/samples/registry49.txt]` |
| binaries (`bench` sha256) | base `85110ac371288350` (built at `4ad3b54` (was `03c61be`) in this worktree before any change), commit `9b2a6d2` (was `2c6bc21`) `287ea8261aa476d2`, chain-width rule `4a6dab74795d0d40` |
| base | fixtures 220,957,398 whole, 220,754,624 per file net of the empty process (316,079); registry 299,933,776 whole, 298,832,156 per file (minimum of 3 runs; 298,834,408 of 2) |
| golden hashes | `scratchpad/golden/fixtures-e8d87c2.tsv` (1238 fixtures) and `registry-e8d87c2.tsv` (25,077 registry files present) |

## Hypothesis and falsifiers

An exact one-line width `W(n)` for a set of node kinds, with the nested-shape condition, decides
"this attempt renders on one line of exactly `W(n)` columns" without rendering, and skipping or
short-circuiting the attempts it decides saves at least 1.0% of instructions per file on the
fixture sample, with no registry regression and byte-identical output.

Falsifiers (the brief's): the kept commits total below 1.0% per file on the fixture sample; any
registry regression; any verification mismatch; any output change.

**Result.** Falsified by step 1 before any width model was built. The attempts an exact width
decides and the lower bound does not are few and cheap: 0.05% of the fixture run at the closure
site, 0.02% at the `if`/`else` site, 0.09% at `choose_rhs`'s same-line candidate (registry 0.08%,
0, 0.19%). What the bound leaves at the closure site is attempts that fail on structure: 41 of
the 47 failing renders on the fixtures (1.02% of the run) have no one-line rendering at any
shape. A width cannot say so; a structural predicate can, and two were built: a body that
renders a `match` arm (kept, −801,783 `Ir` per file, −0.363%, registry −2,718) and a chain wider
than `chain_width` (−688,476 per file on the fixtures, rejected for +10,135 on the registry,
which the decomposition puts in libc's `memcmp` and `malloc`, not in the lever's code). The kept total,
−0.363%, is below the 1.0% the hypothesis needs; the step-1 ceiling of every decidable render
together (2,915,778 `Ir`, 1.30% of the fixture run; 1,181,541, 0.39% of the registry run) left no
room for it once the structural part proved to be five constructs of a few attempts each.

## 1. Measurement before code

### Method (measured)

A temporary instrumented build (not committed; `scratchpad/e30/instr/instrumentation.patch`,
`instr/e30.rs`, `instr/apply.py`, `instr/oracle.py`), E26's record-and-replay with three sites
instead of six:

- **S1**: `rewrite_closure_expr` under the one-line veto in a whole-file 1.9.0 run (E26's S1a–c
  together), after E26's bound pre-check;
- **S3**: `ControlFlow::rewrite_single_line` after E26's pre-check, with the outcome of the
  `single_line_if_else_max_width` test in `rewrite_cond` that follows it;
- **RHS**: the same-line candidate of `choose_rhs` (`rewrite_assign_rhs_expr`'s
  `ex.rewrite(orig_shape)`) for an expression right-hand side.

The record run writes, per attempt, the outcome, the shape (width, block indent, offset), the
lower bound, E26's fit predicate, the context flags, the node's text, and the width of the
node's rendering at an unbounded shape (`Shape::legacy(8096, Indent::empty())`, a probe made
with the memo bypassed and no attempt inside it counted, so the replay's sequence is the same).
The probe's width is the node's unique one-line text where one exists (decide-once § 2) and is
what an exact `W(n)` would compute; a probe that is multi-line or `None` means the node has no
one-line rendering even at the widest shape. Each attempt gets a category; the replay run
under callgrind routes each render through a function named by its category, outermost per
category, so the inclusive costs within a category do not overlap. The runs are `bench -n 1`
on the two samples (instrumented totals 224,309,047 and 304,089,555 `Ir`).

### S1, the closure body under the veto (measured)

| | fixtures | registry |
|---|---|---|
| attempts | 353 | 56 |
| skipped by E26's bound | 82 | 4 |
| rendered | 271 | 52 |
| kept (one line) | 224 (2,366,704 `Ir`) | 43 (334,988) |
| failed: no one-line rendering at the unbounded shape | **41 (2,278,414, 1.02%)** | 0 |
| failed: one-line text wider than the width + 1 (an exact `W` decides; outside E26's fit predicate) | 3 (2) (48,640) | 9 (9) (258,127, 0.08%) |
| failed: one-line text within the width (the nested shape, measured limits) | 3 (59,773) | 0 |

The 41 attempts without a one-line rendering, by cause (each sub-category replayed on its own;
the three costs sum to the 2,278,414 above):

| cause (read from the text) | attempts | cost of the renders | decidable by |
|---|---|---|---|
| a `match` with arms in the body (`arg.as_type().is_some_and(\|ty\| match ty.kind() { .. })`) | 14 | 860,506 (0.38%) | structure: § 2, rule M |
| a chain of two or more items whose bound exceeds `chain_width` (60) | 10 | 786,803 (0.35%) | structure and the bound: § 2, rule C |
| the rest: calls over `fn_call_width` or with a struct literal over `struct_lit_width`, chains whose bound is under `chain_width` but whose width is over it, `if` without `else`, `if`/`else` over `single_line_if_else_max_width` | 17 | 631,105 (0.28%) | per construct limit, each smaller (the two `if` without `else` about 62 K) |

### S3, the one-line `if`/`else` (measured)

| | fixtures | registry |
|---|---|---|
| attempts | 27 | 17 |
| skipped by E26's bound | 15 | 7 |
| rendered, kept | 2 (13,502) | 2 (5,938) |
| rendered, discarded by `single_line_if_else_max_width`, the lower bound already over `min(width, max)` | 8 (83,109) | 8 (102,553) |
| discarded, only an exact width over `min(width, max)` | 2 (36,837) | 0 |

### RHS, `choose_rhs`'s same-line candidate (measured)

| | fixtures | registry |
|---|---|---|
| candidates | 856 | 1,677 |
| used (fast path, or kept after the comparison) | 832 (24,742,658) | 1,638 (11,756,706) |
| dead, the next-line candidate one line that fits, the bound already over the width | 12 (213,558) | 7 (235,672) |
| dead, the same, only an exact width over the width | 7 (195,447) | 31 (585,189) |
| dead, the same, no one-line rendering at all | 0 | 0 |
| dead, a multi-line next-line candidate preferred, or neither | 5 (1,881,376) | 1 (115,475) |

When the next-line candidate is one line and fits, `choose_rhs` returns it whatever the same-line
candidate was, once the same-line candidate has missed the fast path (**read**: `filtered_str_fits`
passes and `prefer_next_line` is true on a one-line candidate); so rendering the next line first
when the same line provably misses would skip the dead candidate exactly, with the bound alone for
the first row. The last row is not decidable by widths: `prefer_next_line` compares line counts and
first-line endings of two multi-line candidates.

### The negative memo and the bound cache (measured)

| | fixtures | registry |
|---|---|---|
| failed S1 renders with an earlier failure of the same node, tag and flags at a width at least as large, same block indent | 4 (311,896) | 0 |
| the same, only at another block indent | 12 (528,413) | 2 (63,293) |
| kept S1 renders with such an earlier failure | 0 | 0 |
| bound scans (`min_width_up_to` calls, base profile) | 113 | 20 |
| their cost (replay) | 195,124 (0.09%) | 39,627 (0.01%) |

On the cliff shape the scans weigh more (base and `9b2a6d2` (was `2c6bc21`) alike, `run.sh` profiles):
closures-expr at depth 8, 64 scans for 8 nests, 206,224 `Ir` (3.70% of that run); at depth 16, 120
scans, 387,328 (6.80%). Each scan stops once the sum passes the width, so a scan costs about the
same at any depth (3.2 K); a per-node cache removes only the scans of a node made again, which
the samples' totals bound at 0.09% / 0.01%.

A memo keyed without the block indent is not exact (**read**): list items are rendered at
`nested_shape`, block indent plus `tab_spaces`, so an attempt that fails at a deep block indent
because an argument does not fit there can succeed at a shallower one with a narrower width
(decide-once § 3, the `R` counterexample). With the block indent in the key, monotonicity in the
width is not established either; its whole volume, 0.14%, is under the bar. The samples show no
counterexample (0 kept renders with an earlier failure), and the indent-free volume overlaps the
structural rules below (the 12 are mostly repeats of the `match` and chain bodies).

### Bounds per use (derived from the tables)

| use | fixtures | registry | built |
|---|---|---|---|
| (1) S1, exact `W(n)` over the width + 1, `R` | 108,413 (0.05%) | 258,127 (0.08%) | no: under 0.3% |
| (1) S1, never one line | 2,278,414 (1.02%) | 0 | as two rules: M (0.38%) kept, C (0.35%) rejected; the rest (0.28% in five constructs) not |
| (1) S3, the lower bound against `single_line_if_else_max_width` | 83,109 (0.04%) | 102,553 (0.03%) | no |
| (1) S3, exact `W` | 36,837 (0.02%) | 0 | no |
| (2) `choose_rhs` same-line, bound or exact `W` | 409,005 (0.18%) | 820,861 (0.27%) | no: under 0.3% on the fixtures |
| (3) negative memo, block indent in the key | 311,896 (0.14%) | 0 | no |
| (3) bound cache | ≤ 195,124 (0.09%) | ≤ 39,627 (0.01%) | no |
| every failed or dead render at the three sites that some rule could decide (S1, S3, `choose_rhs` decidable rows) | 2,915,778 (1.30%) | 1,181,541 (0.39%) | |

Every use a width (exact or the bound) decides is under the brief's 0.3% per use, and their
sum, 637,364 `Ir` (0.28%) of the fixture run and 1,181,541 (0.39%) of the registry run before the
model's own cost, is under the 1.0% the hypothesis needs; so `W(n)` was not built and its
harness has nothing to check. The volume E26 named, "the S1 attempts whose bound is below the width but which fail on structure (47 of 132)",
is 41 attempts without a one-line rendering and 6 others: structure, not width.

## 2. The structural predicates (read)

`width.rs` gains `never_one_line`: a node every rewrite of which that returns text has a line
break, so that an attempt keeping only a one-line result fails whatever it renders. The
exactness argument per construct, in the module documentation:

| rule | holds when | why no rewrite is one line | the source fallbacks excluded |
|---|---|---|---|
| M: a rendered `match` arm (`renders_match_arm`, kept) | a `MATCH_ARM` node in the subtree, outside a pattern's `const` block, a generic argument list or an item; no `COMMENT` token, no `ATTR`, no `ASM_EXPR` | `rewrite_match` writes `"{\n"` after the condition on every path that returns text when the arm list is not empty (chloro matches.rs:72, rustfmt matches.rs:73; the one source path, `match x { /* c */ }`, has no arm); every rewrite of an expression contains the rewrites of the subexpressions it writes (calls, chains, closures with or without the block, pairs, prefixes, parentheses, index, fields, struct literal values, `return`/`break` values): delimiters, labels, field names and attributes are dropped, subexpressions never; a block is written through its expression on one line or through a nested visitor, whose result has a line break before `}` once the block has a statement | `recover_comment_removed` (only with a comment in the source), `#[rustfmt::skip]` (an attribute), `asm!` (returned as written), the 2021 slice-pattern fallback, type and generic-argument rewrites that copy source (`const` block patterns, generic argument lists and items are not counted); a `match` in a macro call is in the token tree, not a node of the tree |
| C: a chain wider than `chain_width` (`chain_too_wide`, rejected) | a method call, field, `.await` or `?` chain with at least two items after its parent; the parent a path, an index, or a call of a path; no attribute of its own; the bound of the receiver of the last item above `tab_spaces`; the node's bound above `chain_width` (so no comment) | `child_count` > 1, so `format_last_child` measures against `min(shape.width, chain_width)`; `format_root` absorbs items only while the root is at most `tab_spaces − offset` bytes, so the last item stays a child; the parent rules out a one-line root for which `last_line_extendable` holds (one ending in `"#`, or of closing delimiters only), so the budget subtracts every rewrite but the last; `fits_single_line` needs the whole line within `chain_width`, otherwise `join_rewrites` puts items on lines of their own or returns `None` under `force_one_line_chain` | `#[rustfmt::skip]` on the node; comments (no chain item is a comment, `recover_comment_removed` keeps the rewrite) |

Both read no shape; rule M reads no option, rule C reads `chain_width` and `tab_spaces` from the
run's options at the site. The S1 site tests them after E26's bound and only for a body whose
source is longer than the shape is wide (the source length bounds the bound from above, and a
shorter body mostly fits, so the test is spared where it would rarely find anything); rule M
scans the elements only when the source has a `=>` (a `memchr` for `>` first).

Debug builds assert, on every expression rewrite (`format_expr`) that returns one line, that
neither rule holds for the node, and the S1 site renders every attempt it skips and asserts that
the result is `None` (E26's checks).

## 3. Commits (measured)

### `9b2a6d2` (was `2c6bc21`): rule M at the S1 site (kept)

| sample | base | `9b2a6d2` (was `2c6bc21`) | change |
|---|---|---|---|
| fixtures, whole | 220,957,398 | 220,264,955 | −692,443 (−0.313%) |
| fixtures, per file net (base 2 runs, head 3) | 220,754,624 | 219,952,841 | **−801,783 (−0.363%)** |
| registry, whole | 299,933,776 | 299,932,845 | −931 |
| registry, per file net (3 runs) | 298,832,156 | 298,829,438 | **−2,718** |

- Keep rule: the use's bound is 860,506; per file the fixtures fall by 93% of it (half is
  430,253) and the registry does not rise. Per file, two fixture files carry it:
  `hir-ty/src/infer/coerce.rs` −692,454 and `ide-assists/src/handlers/destructure_tuple_binding.rs`
  −109,478; no other file moves by more than 3,185.
- By function (whole-sample exclusive `Ir`): fixtures −632,691 in the formatter and parser,
  −59,144 in libc; registry +3,178 in the formatter (`memchr` +1,843, the `=>` search +804,
  `rewrite_closure_expr` +529) against −4,110 in libc's `memcmp`, a layout term (EXP-027's
  `memcmp` alignment) that the next binary moved the other way.
- The order of the tests was measured, per file against the base: the `=>` substring search
  alone fixtures −714,697 / registry +18,702 (runs 2, before the clippy fix), −751,767 (runs 2) /
  +9,607 (runs 3) after it, a `memchr` for `>` and then the scan −765,808 / +13,334, the source-length
  gate with the `memchr` −802,238 / +5,562, the gate, the `memchr` and the `=>` search (the
  commit) −801,783 / −2,718. Each step removes cost on the registry bodies that cannot have an
  arm; the fixture gain is the 14 renders either way.

### The chain-width rule on top of `9b2a6d2` (was `2c6bc21`) (rejected)

| sample | `9b2a6d2` (was `2c6bc21`) | with rule C | change |
|---|---|---|---|
| fixtures, whole | 220,264,955 | 219,616,465 | −648,490 |
| fixtures, per file net (3 runs) | 219,952,841 | 219,264,365 | −688,476 (87% of its 786,803) |
| registry, whole | 299,932,845 | 299,939,565 | +6,720 |
| registry, per file net (3 runs) | 298,829,438 | 298,839,573 | **+10,135** |

- A repeat of both registry measurements read 298,830,247 and 298,844,884 (+14,637): the rise is
  a stable property of this binary, above the run-to-run spread of the minimum-of-3 figure
  (1.8–2.3 K, two measurements of one binary; single runs of one binary differ by 10–14 K), so
  protocol § 3 does not keep it, whatever the fixture gain: committed as `94b0c6b` (was `3d471d9`) and reverted in
  `b8ba4b5` (was `d6b4d73`).
- Decomposition (whole-sample exclusive `Ir`): the formatter +805 (`rewrite_closure_expr` +726,
  `Node::first_child` +96), libc +5,887 (`memcmp` +3,591; `vfscanf`, `strtoul`, `getdelim` at
  startup, glibc reading `/proc/self/maps`, whose cost follows the binary's mappings; the empty
  process moved from 316,079 to 318,352). Per file the largest move is
  `windows-sys-0.48.0/src/Windows/Win32/System/Rpc/mod.rs`, +5,608, which formatted alone under
  callgrind reads +1,801, of which `memcmp` +1,518 and no formatter function. So the rise is a
  layout term, not the rule's work (+805). Split per file the same way (each of the 49
  processes' callgrind profile, net of the empty process, mean of the three runs): the formatter,
  parser and std code −678, libc +8,100, `bench`'s own timing output −2,679. The startup term
  (glibc reading `/proc/self/maps`, +2,273 per process) is the same in the empty and the real
  processes and nets out. The same split puts `9b2a6d2` (was `2c6bc21`)'s own code at +3,819 per file on the
  registry (whole-sample +3,178), offset by libc −3,779: the kept commit's registry figure is
  flattered by the same kind of term that sinks rule C's. Function-order shuffling (`--shuffle`)
  would not separate it (the term is in data layout, not code order) and was not run.
- The rule passed the harness over the fixtures (three option sets), the closures-expr depths,
  the registry and the cliff files (§ 4) in the commit that carried it.

## 4. The harness (measured)

- `tests/width_bound.rs` (in `cargo test`, debug build) formats the 1238 fixtures under the
  default options, `use_field_init_shorthand` + `use_try_shorthand`, and rustfmt 1.8.0, and the
  closures-expr shape at depths 2–16 (input and formatted output); the debug checks of § 2 run on
  every expression rewrite, with E26's.
- A release build with debug assertions (`CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true`, LTO off,
  `target-da`) runs the same tests and `CHLORO_WIDTH_BOUND_ROOT` over the registry (25,077
  `.rs` files) and over the 109 files of the cliff generator (`gen.py`, scales 1, 2, 4, 8): 0
  failures for `9b2a6d2` (was `2c6bc21`) (68.9 s for the fixtures and the registry) and 0 for `94b0c6b` (was `3d471d9`) (63.8 s;
  and 154.8 s for an earlier build of rule C with a full bound scan). Mismatches: 0 (every check
  is an assertion; the first failure would stop the run).
- Unit tests in `width.rs`: rule M with and without arms, comments, attributes, macro calls,
  generic arguments, items; rule C's item count, the absorbed receiver, the parents, `?`s,
  `.await`, fields, comments, a division (the rejected patch carries its tests).
- The exact width was not built, so the brief's equality check (`W(n)` equals the width of the
  one-line rendering wherever `exact(n)` holds) has no `W` to check; step 1's probe gave the
  width each `W` would have to equal, and its uses are in § 1.

## 5. The cliff shapes (measured)

`benchmarks/cliffs/run.sh --core 2|3 --shapes 'closures-expr closures-block chain-closures
generic-calls nested-blocks'` (scales 1, 2, 4, 8; closures-expr depths 2, 4, 8, 16): all 20
files identical to rustfmt's output for the base and `9b2a6d2` (was `2c6bc21`).

| shape | base `Ir` at scale 8 | `9b2a6d2` (was `2c6bc21`) | base k_net / p | `9b2a6d2` (was `2c6bc21`) k_net / p |
|---|---|---|---|---|
| closures-expr | 5,692,353 | 5,690,780 | 1.143 / 1.046 | 1.143 / 1.046 |
| closures-block | 10,734,214 | 10,734,089 | 1.040 / 1.309 | 1.040 / 1.308 |
| chain-closures | 5,082,934 | 5,084,493 | 0.948 / 0.985 | 0.949 / 0.986 |
| generic-calls | 10,078,849 | 10,086,331 | 0.936 / 1.160 | 0.937 / 1.161 |
| nested-blocks | 4,922,209 | 4,922,144 | 0.935 / 1.254 | 0.935 / 1.254 |

No exponent rises by more than 0.001; the brief's "no exponent above 1.1" already fails at the
base for closures-expr's k_net (1.143) and the fitted `p` of three shapes, which the commit does
not change. The shapes have no `match` in a closure body, so the rule only costs its tests there
(+7,482 at generic-calls scale 8, the largest).

## Gates (measured)

| gate | scope | `9b2a6d2` (was `2c6bc21`) (kept) | `94b0c6b` (was `3d471d9`) (rule C, reverted) |
|---|---|---|---|
| `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | clean | clean |
| `cargo test --workspace --all-features` | | 309 passed, 0 failed | 310 passed, 0 failed |
| `bench -n 1 --check fixtures-e8d87c2.tsv`, sequential and `--threads 4` | 1238 fixtures | 0 changed; 0 differ between 4 threads and `format_source` | same |
| `bench -n 1 --check registry-e8d87c2.tsv` | 25,077 registry files | 0 changed | 0 changed |
| `conform -i`, `conform -i -V 1.8.0` | 1220 fixtures | 1220/1220, 0 not idempotent, both | same |
| `config_matrix -n 1220` | 31 option values against live rustfmt | every value identical on every accepted file (28 at 1022/1022; editions 844/844, 843/843, 838/838) | every value identical on every accepted file (same counts) |
| `profile.sh --per-file` | both samples | § 3 | § 3 (registry rise) |
| debug-assertion harness | fixtures (3 option sets), closures-expr 2–16, registry, 109 cliff files | 0 failures | 0 failures |
| cliff generator | 5 shapes, 20 files | identical to rustfmt; § 5 | identical to rustfmt; exponents within 0.004 of the base |

## Decision

Kept on the branch, for the integrator: `9b2a6d2` (was `2c6bc21`), −801,783 `Ir` per file on the fixture sample
(−0.363%), −2,718 on the registry sample, no output change on either corpus or any option value.
Rejected: the chain-width rule (`94b0c6b` (was `3d471d9`), reverted in `b8ba4b5` (was `d6b4d73`); −688,476 per file on the
fixtures, +10,135 and +14,637 on the registry in two measurements, a layout term by its
decomposition but above the spread). Not built: the exact width `W(n)` and its uses at S1, S3
and `choose_rhs`, the negative memo and the bound cache, each under 0.3% by step 1. The
hypothesis is falsified: the kept total is under 1.0%, and step 1 put the ceiling of every
width-decidable attempt at 0.28% of the fixture run and 0.39% of the registry run before the
model's own cost; the volume beyond E26's bound is structural, not a matter of width.

Merged at `2e1bc8f` (was `96c9abc`) (rule M only) after the integrator reproduced the gates on the merged tree,
which carries E31's lexer front end above this branch's base: 0 of 1238 fixture hashes changed
(sequential and 4 threads), 0 of 25,077 registry hashes; `conform -i` 1220/1220 at 1.9.0 and
1.8.0; `Ir` 202,822,327–202,822,529 on the fixture sample over three runs (−0.162% against
the E31 merge's 203,151,226; per file 202,143,814 net, −0.390% against 202,934,484) and
255,681,216–255,681,242 on the registry sample (per file 255,654,657, +0.000% against
255,654,373, inside the run-to-run spread); fmt, clippy, 309 tests, the option matrix
identical on every value; `closures-expr` 5,485,338 at depth 8 and 5,513,281 at depth 16 with
identical output; the debug-assertion `bench` over both corpora and the cliff fixtures: 0
changed, no assertion fired. The ledger closes the width-model lever at its measured ceiling.

## Follow-up

- **Never one line, per construct.** The rest of the structural volume at S1, 0.28% on the
  fixtures, is in five constructs (calls over `fn_call_width`, struct literals over
  `struct_lit_width`, chains whose width but not bound exceeds `chain_width`, `if` without
  `else`, `if`/`else` over `single_line_if_else_max_width`); each needs its own argument like
  rule C's, and the chain case needs the exact width of the chain, which is where a width model
  would first pay.
- **Rule C** needs a registry figure free of the data-layout term before it can be kept: its own
  code moves the registry by −678 per file (+805 whole) by the split of § 3, and it removes 688 K
  per file on the fixtures. The split itself (code against libc and `bench`'s output, per file)
  would serve as that figure if the protocol adopted it; function-order shuffling does not.
- **`choose_rhs`** (0.18% / 0.27%): the bound alone decides 12 / 7 of the dead same-line
  candidates and the order change is exact (§ 1); the registry's 31 that need an exact width are
  the larger part there.
- **The lower bound against `single_line_if_else_max_width`** at S3 is exact and costs nothing
  new (0.04% / 0.03%).
- The registry sample has almost no structural S1 volume (0 of 9 failures); levers at this site
  are fitted to rust-analyzer's closure style and should be judged on the registry first.
