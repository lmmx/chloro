# EXP-026: a lower bound on one-line width, from the tokens

Ledger row E26 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md). The brief is trick T7 of
[../../2026-10-15-unmined-tricks.md](../../2026-10-15-unmined-tricks.md) (§ 5), narrowed to its
exact half: a lower bound, not a width model. Its volume is rows 2–4 of
[../../2026-10-15-extensions-theory/decide-once-and-exact-reuse.md](../../2026-10-15-extensions-theory/decide-once-and-exact-reuse.md)
(15.6–18.1% of the instructions spent rendering candidates that are then discarded), its target
the nested-closure cliff of [EXP-019](EXP-019-cliffs.md) ("closures-expr": exponent 3.6, an
exact fix needs a lower bound on one-line width that holds for every rewrite). The rollback
rule of the external research brief (§ 1.2) does not bind here: a skipped attempt writes
nothing, and the effects a rendered attempt would have had on the run are handled by the run
guard below. Labels: **measured** (a tool's output on the builds below), **read** (from
source), **derived**.

## Identity

| field | value |
|---|---|
| branch | `20261015-width` from `e312e39` (integration branch; its code is the best-known-good `05ab7b4`), worktree `/home/user/wt/width` |
| kept commits | `ee62a16` (the bound, the fit predicate, the harness, and the closure-body pre-check), `c915da4` (the one-line `if`/`else` pre-check), `9006503` (the punctuation every rendering writes, counted); then this record |
| rejected | the one-line block pre-check (`rewrite_single_line_block`), reverted, diff in `scratchpad/e26/block-precheck.patch`; the first fit claim (one-line rewrites no wider than their shape), refuted by the harness on the registry and replaced by a one-column claim before the commits above were made (the superseded commits `a9d5858`, `f621dff`, `2f7a89d` are not on the branch); a pre-check order scanning the bound before the fit predicate (replaced) |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, valgrind 3.22.0, `rustfmt 1.9.0`; `cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples` (fat LTO, one codegen unit) |
| host | 4 vCPU Xeon @ 2.80GHz, shared with other experiments throughout (load average 4–25). `Ir` is unaffected by load; no wall-clock figure is claimed |
| `Ir` command | `benchmarks/profile.sh --core 3 --bench /home/user/wt/width/bench-XXXX --sample /tmp/chloro-bench/widthx49 --out <dir>`: a private sample path of the default's length, every binary copied to a path of one length (`bench-base`, `bench-A2xx`, `bench-B2xx`, `bench-D2xx`) |
| co-metrics | the parser's growth term: inclusive cost of the call arcs from `ra_ap_*` functions into `RawVec` growth (`finish_grow`, `grow_one`, `grow_amortized`, `do_reserve_and_handle`, `reserve`), from `callgrind_annotate --tree=caller` (script `scratchpad/e26/scripts/parsergrow.py`), and `Ir` net of it (EXP-020's heap-layout term); the per-file sum (each of the 49 sample files formatted by its own `bench -n 1` process under the same `valgrind` invocation, `scripts/perfile.sh`, as in EXP-019); `profile.sh --sim` |
| base | `bench-base`, built at `e312e39` in this worktree before any change: 232,047,989 `Ir`; parser growth 8,295,998; per-file sum 243,317,824 |
| golden hashes | `scratchpad/golden/fixtures-e8d87c2.tsv` (1238 fixtures) and `registry-e8d87c2.tsv` (25,024 registry files present, 20,850 recorded) |

## Hypothesis and falsifiers

rustfmt, and chloro after it, decides whether a candidate fits by rendering it and measuring.
Some attempts keep their result only if it is one line (or one line that fits). A lower bound
on the width of any one-line rendering of a node, computed from tokens that no formatting path
removes or shortens, rejects such an attempt before rendering whenever the bound exceeds the
width available, exactly; on nested shapes, where a node is rendered at many shapes the memo
cannot merge, this removes a measurable share of the speculation and bends the nested-closure
cliff.

Falsifiers (the brief's): the harness finds a node whose bound exceeds a one-line rendering
(the token class responsible is then removed, and the lever closes if none remains); `Ir` not
down by at least half the measured cost of the renders the bound rejects, per site; any output
change on any corpus or option value.

**Result.** The bound held on every one-line rewrite of both corpora and the cliff shapes. A
second claim the closure site needs, about how far a one-line rewrite can exceed its shape,
failed once on the registry and was corrected (below). The two kept sites move `Ir` by more
than half their measured rejected cost net of the heap-layout term and per file; the third
site (blocks) is below the resolution of every metric and was rejected. The cliff is bent:
closures-expr at depth 16 costs 5.81 M `Ir` against 105.45 M (−94.5%), memo misses per nest
33 against 1,286.

## 1. Measurement before code

### Method (measured)

A temporary instrumented build (not committed; `scratchpad/e26/instrumentation.patch`,
`e26.rs.txt`) wraps the render of every attempt at each site and records, per attempt, whether
it was rejected and whether the bound would have rejected it. A first run writes the outcome of
every attempt in order (`E26_RECORD`); a second run, under callgrind, reads it back
(`E26_REPLAY`) and routes each render through a function named by its outcome:
`<site>_rej` around the outermost rejected renders (a rejected render inside another rejected
render is not counted again), `<site>_bound` around the outermost renders the bound rejects.
Neither wrapper is ever active twice on the stack, so callgrind's inclusive cost of each is
exact and the sums within a category do not overlap; the wrappers carry distinct bodies so that
LTO does not merge them. The runs are deterministic (`bench -n 1`, one thread; two record runs
wrote identical files). "Rejected" means the attempt's result is not kept: the
render is `None`, or has a newline where one line is required, or is too wide.

The bound in this first measurement is the token-only bound of step 2 with macro token trees
counted, and the closure site's fit condition is "no wider than the shape" (both later
tightened, see § 3 and § 4); the last table repeats the measurement on the final code.

### Sites (read)

| site | function (chloro; rustfmt 1.9.0) | kept only if | needs the fit predicate |
|---|---|---|---|
| S1a | `rewrite_closure` → `rewrite_closure_expr`, body without a block, under `veto_multiline` (closures.rs; rustfmt closures.rs:197) | one line | yes: the veto does not measure |
| S1b | `try_rewrite_without_block` → `rewrite_closure_expr`, a block body unwrapped to its expression | one line | yes |
| S1c | `rewrite_last_closure`'s one-line form → `rewrite_closure_expr` (closures.rs:402-411) | one line | yes |
| S3 | `ControlFlow::rewrite_single_line` (expr.rs; rustfmt expr.rs:864-908) | both branches one line and `result.len() <= width` | no |
| S4 | `rewrite_single_line_block` (expr.rs; rustfmt expr.rs:577-600) | `result.len() <= shape.width` and one line | no |
| S5 | `try_overflow_last_item`'s re-render of an overflowed item with one newline at `nested_shape` (overflow.rs; rustfmt overflow.rs:502-513) | one line | yes |

Attempts read and not taken, because their result is kept multi-line too, so a one-line bound
cannot decide them: `choose_rhs`'s same-line and next-line candidates (`prefer_next_line` reads
the next-line candidate's lines), the match arm body (`combine_orig_body` for block bodies, the
slow path compares two candidates), the chain's last child at `one_line_shape` (kept when it has
five lines or more), `rewrite_pair`'s same-line right-hand side (kept multi-line when the left is
short or the right's first line ends with `{`), `rewrite_index`'s first index (the final
fallback), `try_overflow_last_item`'s overflow candidate, the `let`-`else` block (rendered, then
re-rendered), and literals and macro snippets (`wrap_str`: a clone and a measure, already
cheaper than any scan).

### Per site on the 49-file sample (measured; instrumented build, 232,925,214 `Ir`)

| site | attempts | rejected | bound rejects (with the fit predicate where needed) | bound rejects without the fit predicate | bound rejects that were kept | cost of the rejected renders (outermost) | cost of the renders the bound rejects (outermost) |
|---|---|---|---|---|---|---|---|
| S1a | 249 | 31 | 12 | 12 | 0 | 670,239 | 160,100 |
| S1b | 99 | 97 | 51 | 77 | 0 | 3,764,753 | 1,511,013 |
| S1c | 4 | 4 | 0 | 2 | 0 | 150,369 | 0 |
| S3 | 27 | 15 | 14 | 14 | 0 | 387,778 | 389,848 |
| S4 | 33 | 29 | 25 | 25 | 0 | 31,561 | 29,522 |
| S5 | 2 | 0 | 0 | 0 | 0 | 0 | 0 |
| all | 414 | 176 | 102 | 130 | 0 | 5,004,700 (2.15%) | 2,090,483 (0.90%) |

- The bound is computed at every attempt in this build: `one_line_min_width` 570,389 `Ir` and
  the fit predicate 138,124 (**measured**), about a third of what it saves; the kept code checks
  the source length first and the fit predicate before the bound (§ 4).
- S3's bound column exceeds its rejected column because some of its outermost bound rejections
  lie inside another site's rejected render (an `if` inside a closure body).
- The ceiling of the lever at these sites (every rejected render, by any width model) is 5.0 M,
  2.15% of the run: the sites keep most of their attempts (S1a 218 of 249), and the big
  speculation volume of the theory record (`choose_rhs`, match bodies, the chain's last child)
  is at decision points whose losing candidate can be multi-line, out of a lower bound's reach.

### closures-expr at depth 8 and 12 (measured; S1a only, no other site occurs)

| depth | attempts | rejected | bound rejects | run `Ir` (instrumented) | rejected renders | renders the bound rejects |
|---|---|---|---|---|---|---|
| 8 | 736 | 160 | 40 | 16,935,096 | 13,406,035 (79.2%) | 1,271,886 (7.5%) |
| 12 | 2,416 | 1,272 | 816 | 58,845,507 | 54,026,810 (91.8%) | 52,090,997 (88.5%) |

The token-only bound is about two thirds of a nested call's width (`run_1(|a_1| ` writes 12
columns, of which `run_1` and `a_1` are 8), so at depth 8 it rejects a quarter of the failing
attempts; at depth 12 the nests are wide enough for it to reject almost all of them.

### The same measurement on the final code (`9006503`; measured)

| run | attempts S1 / S3 | rejected S1 / S3 | bound rejects S1 / S3 | rejected renders S1 / S3 | renders the bound rejects S1 / S3 |
|---|---|---|---|---|---|
| sample | 352 / 27 | 132 / 15 | 85 / 15 | 4,564,074 / 388,749 | 2,278,480 / 401,440 |
| closures-expr, depth 8 | 736 | 160 | 120 | 12,298,939 (78.6%) | 9,817,899 (62.8%) |
| closures-expr, depth 12 | 2,416 | 1,272 | 1,176 | 50,994,146 (91.6%) | 50,992,274 (91.5%) |

S1 is S1a–S1c together (one function); the final bound counts punctuation (§ 2), excludes
macro token trees, and the closure site skips only when the bound exceeds the width by more than
one column (§ 3). No attempt the final pre-check rejects was kept.

## 2. The bound (`chloro-core/src/formatter/width.rs`; read)

`one_line_min_width(node)`: one pass over the node's elements in the flat tree (kind, offset and
length per element, no rendering), summing a lower bound on the display width of each counted
token and the punctuation the node kinds write; 0 if any comment is in the node. A counted
token's display width is at least its number of ASCII bytes other than `\n` and `\r`
(`unicode_str_width`, unicode-width 0.1.14, counts every other ASCII character as one column
and nothing as less than none; a byte length is at least the display width); a one-line
rendering contains each counted token whole and side by side, so its width is at least the sum.

| token class | counted | proof (the writer) |
|---|---|---|
| identifiers | yes, less 2 for `r#` | written from the source (path segments, patterns, fields, `rewrite_ident`); rustfmt 1.8.0 drops `r#` from labels (F1), so `r#` is never counted |
| lifetimes | yes, less 2 for `'r#` | copied as written; 1.8.0 writes `'r#a` as `'a` |
| keywords (`SELF_TYPE_KW`..=`YEET_KW`) | yes | every keyword a rewrite reads is written back (closure `for`/`const`/`static`/`async`/`gen`/`move`/`use` via `has_token`; `&mut`, `&raw const`; control flow; `as`; jump keywords; `true`/`false`; path keywords; type keywords, `force_explicit_abi` only adds). The one keyword the formatter drops, the `in` of `pub(in crate)` (`format_visibility`), only occurs in visibilities, which are in uncounted subtrees |
| literals | yes | copied from the source: `format_strings` off, `hex_literal_case` and `float_literal_trailing_zero` fixed at `Preserve` |
| argument-list parentheses | yes, 2 (`9006503`) | `overflow::rewrite_with_parens` writes `(` and `)` on every path, for calls and method calls |
| closure pipes | yes, 2 (`9006503`) | `rewrite_closure_fn_decl` writes `"|" + params + "|"` |
| `.` of a method call, field, `.await`; `?` of a try | yes, 1 (`9006503`) | `ChainItem::rewrite` |
| `&` of a reference, a prefix operator | yes, 1 (`9006503`) | `format_expr`, `op_token().text()` |
| a binary operator | yes, its length (`9006503`) | `bin_op_text`, written by `rewrite_pair`, `rewrite_all_pairs`, `rewrite_assignment` |
| index, tuple and array brackets | yes, 2 (`9006503`) | `rewrite_index`; `overflow` (`(`/`)`, `[`/`]`); a repeat array through `rewrite_pair` |
| other punctuation, `_` | no | trailing commas dropped on one line, `remove_nested_parens`, closure/match-arm/`use`-list braces removed, `...` written `..=`, `try!(e)` written `e?`, leading match-arm pipes follow `match_arm_leading_pipes` |
| whitespace | no | the formatter's own |
| comments | no bound | the brief's rule (a lost comment brings the source back, which would keep every token, but the rule is kept) |

Uncounted subtrees: attributes (`merge_derives`; rustfmt's `normalize_doc_attributes`), labels
(1.8.0 unwraps a labelled closure-body block and drops the label, F2), the name of a
struct-literal or struct-pattern field (`use_field_init_shorthand`: `x: x` → `x`, the value side
is counted), a macro call's path and token tree (`use_try_shorthand`; arguments formatted as
items or statements go through the item normalisations), `use` declarations and items.

## 3. The fit predicate, and its correction (read, measured)

The closure body is kept if it is one line, without being measured, so skipping it also needs
a bound on how much wider than its shape a one-line rewrite can be. Some rewrites return text
unmeasured: a one-line string literal in style edition 2024 (`rewrite_string_lit`),
`#[rustfmt::skip]` expressions, `asm!`, a raw string that does not fit, macro calls left as
written, `return`, `break`, `continue`, `yield`, `_`; a prefix in front of such a child passes
the excess on. `one_line_fits_checked(node)` holds when every node of the subtree is a path
(no generic arguments, no `<T>::`), a non-string literal, a call, tuple or array, a method call,
field, try or `.await`, a binary (not assignment), prefix, reference, parenthesised or index
expression, or a closure with untyped identifier parameters and a non-block body, and there is
no string literal, comment or assignment operator; the table in `width.rs` gives each kind's
width check.

The first form of the claim was "a one-line rewrite is no wider than its shape". The harness's
run over the registry (§ 5) failed on it in `arrayvec-0.7.6/src/arrayvec.rs`:
`unsafe { Ok(self.into_inner_unchecked()) }` in a one-line `if`/`else` attempt renders the call
in 31 columns for a shape of 30. The mechanism (**read**): the single argument is overflowed;
its rewrite at the overflow shape has one newline, so `try_overflow_last_item` replaces it by
its rewrite at `nested_shape` (rustfmt overflow.rs:502) and keeps the horizontal tactic without
measuring the replacement; `wrap_items` then keeps the list on the callee's line if
`first_line_width(items) + 1 <= shape.width - callee`, which leaves the closing parenthesis out.
So a list can exceed its shape by exactly one column, and by no more: the list re-measures its
items there, chains and `rewrite_pairs_one_line` end in `wrap_str`, and every other rule in the
table shifts its child's shape by what it writes before it, so no rule adds an excess to a
child's. The claim became "at most one column wider", and the closure pre-check skips only when
the bound exceeds the width by more than one. The fixtures and the cliff shapes never hit the
off-by-one; the commits were rebuilt with the corrected claim before any was kept, and every
figure in § 4 is of the rebuilt commits.

## 4. Commits (measured)

| commit | change | `Ir` before | `Ir` after | change | parser growth change | net of it | per-file sum change | measured cost the bound rejects | net / cost | per-file / cost | decision |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `ee62a16` | closure body pre-check (S1) | 232,047,989 | 231,121,947 | −926,042 (−0.40%) | +212,783 | −1,138,825 | −1,066,548 | 1,671,113 (S1a + S1b, step 1) | 68% (total 55%) | 64% | kept |
| `c915da4` | one-line `if`/`else` pre-check (S3) | 231,121,947 | 228,847,625 | −2,274,322 (−0.98%) | −1,923,868 | −350,454 | −342,827 | 389,848 | 90% | 88% | kept |
| `9006503` | punctuation counted | 228,847,625 | 230,281,855 | +1,434,230 (+0.63%) | +1,779,274 | −345,044 | −493,533 | 618,959 more (2,679,920 final S1 + S3 against 2,060,961) | 56% | 80% | kept, see below |
| — | one-line block pre-check (S4), on the pre-correction `f621dff` | 228,830,884 | 230,065,900 | +1,235,016 | +1,395,290 | −160,274 | −331,175 | 29,522 | — | — | rejected |
| **head against base** | | 232,047,989 | 230,281,855 | **−1,766,134 (−0.76%)** | +68,189 | **−1,834,323 (−0.79%)** | **−1,902,908 (−0.78%)** | | | | |

- The gate runs read 231,121,947, 228,847,625 and 230,281,855; the same code before the commit
  messages were rewritten read the same (the trees are identical).
- `c915da4` and `9006503` move the parser's vector growth by −1.92 M and +1.78 M: the heap layout
  the formatter leaves decides whether the parser's `Vec<Event>` grows in place on the next file
  (EXP-020). Their total `Ir` changes are that term, not the levers; net of it and per file, both
  levers move by more than half their measured cost. `9006503` reads +1.43 M in total `Ir` and is
  kept on the net and per-file figures and the cliff (§ 6); against the head's base the term has
  returned to +68 K, and the three commits together remove 1.83 M net (0.79%), 1.90 M per file.
- The bound costs 165,663 `Ir` in `min_width_up_to` over 113 scans on the sample at the head, plus
  the fit predicate inlined in `rewrite_closure_expr`: the source length answers most attempts
  without a scan, the fit predicate stops at the first excluded node, and the width scan stops as
  soon as the sum passes the width.
- S4, the block: its whole measured cost is 29,522 `Ir`; the net (−160 K) and per-file (−331 K)
  moves are five to eleven times that, so they are layout, and the lever is below the
  resolution of every metric; total `Ir` rose. Reverted.
- An earlier order of the closure pre-check (bound scanned in full before the fit predicate, no
  early exit) read −836,945, exactly half its site cost, at the pre-correction claim; the order
  kept (source length, fit predicate, early-exit scan) read −958,142 at the same claim.

`profile.sh --sim` (measured): base `Bcm` 2,937,498, `Bim` 708,716, `Bcm+Bim` 3,646,214, `I1mr`
4,712,463, `D1mr` 687,368; head 2,942,183, 704,034, 3,646,217, 4,659,498 (−1.12%), 684,638.
Branch mispredictions are flat; instruction-cache misses fall with the work removed.

## 5. The harness (measured)

- In debug builds `format_expr` checks, on every expression rewrite that returns one line, that
  the node's bound is at most the rendering's display width and, for a node the fit predicate
  accepts, that the rendering is at most one column wider than its shape; the two pre-check sites
  render every attempt they skip and assert that it fails. `tests/width_bound.rs` (in
  `cargo test`) formats the 1238 fixtures under the default options, under
  `use_field_init_shorthand` and `use_try_shorthand`, and under rustfmt 1.8.0, and the
  closures-expr shape at depths 2–16 (input and formatted output); `CHLORO_WIDTH_BOUND_ROOT`
  adds any directory.
- A release build with debug assertions (E17's method; `CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS
  =true`, LTO off for build time) over the fixtures (1238 files), the registry (25,024 files,
  404 MB) and the 217 files of the cliff generator at scales 1–8: 0 failures at the head. Before
  the correction of § 3 the registry run failed at `arrayvec` (the only failure kind it can
  report is the first; the run after the correction covers the whole corpus).
- The bound itself never exceeded a one-line rendering on any corpus: no token class was removed.
- Mutation checks: counting whitespace tokens fails at once (fixtures and cliff shapes);
  admitting string literals and `return` to the fit predicate fails on the fixtures (a literal
  7 columns wide at a shape of 2); counting commas does not fail (a one-line rendering's
  separators and delimiters always outweigh a dropped trailing comma, so the check cannot see
  it; commas stay uncounted on the argument that they can be dropped).
- Unit tests in `width.rs`: counted and uncounted classes, field shorthand, attributes, labels,
  raw prefixes, macro calls, non-ASCII text, comments; the fit predicate's exclusions; the
  early-exit scan and the one-column margin.

## 6. The nested-closure cliff (measured)

`benchmarks/cliffs/run.sh --shapes closures-expr --scales 1,2,3,4,5,6,7,8` (depths 2–16, 8
nests per file), callgrind as `profile.sh` runs it; every file identical to rustfmt's output at
`ee62a16` and `9006503` (`compare.tsv`).

| depth | 2 | 4 | 6 | 8 | 10 | 12 | 14 | 16 |
|---|---|---|---|---|---|---|---|---|
| base `Ir` | 1,000,328 | 1,414,371 | 3,784,530 | 14,923,797 | 29,772,404 | 50,856,618 | 72,289,247 | 105,449,815 |
| `ee62a16` | 999,510 | 1,415,977 | 3,787,427 | 14,388,302 | 21,284,725 | 9,629,967 | 5,987,173 | 6,234,843 |
| `c915da4` | 1,001,024 | 1,415,933 | 3,787,486 | 14,330,691 | 21,124,582 | 9,542,709 | 5,918,373 | 6,155,352 |
| `9006503` | 996,916 | 1,412,081 | 3,784,889 | 5,636,285 | 4,982,605 | 3,977,609 | 4,773,800 | 5,809,881 |
| base memo misses (8 nests) | 40 | 72 | 360 | 1,640 | 3,256 | 5,376 | 7,320 | 10,288 |
| `ee62a16`, `c915da4` | 40 | 72 | 360 | 1,504 | 2,088 | 784 | 352 | 296 |
| `9006503` | 40 | 72 | 360 | 496 | 368 | 200 | 232 | 264 |

Memo misses are the calls of `format_expr_uncached` from `memoize` in the callgrind profiles;
the base figures are EXP-019's (5, 9, 45, 205, 407, 672, 915, 1,286 per nest).

| fit (`gen.py fit`) | k_net | p (`a + b n^p`) | p against output bytes | local slopes (net) |
|---|---|---|---|---|
| base | 2.687 | 3.623 | 1.330 | 0.692 2.855 5.007 3.145 2.962 2.294 2.838 |
| `ee62a16` | 1.349 | 0.930 (a < 0) | 0.301 | 0.696 2.853 4.874 1.789 −4.455 −3.224 0.321 |
| `9006503` | 1.023 | 0.715 (a < 0) | 0.301 | 0.694 2.855 1.490 −0.589 −1.333 1.279 1.568 |

- The curve is no longer monotone, so a power law describes it badly (the constant comes out
  negative). Read by depth: with the token bound the cliff ends at depth 10 (the nest is then
  more than half again as wide as the space left, and the expression attempts are pruned from
  the top levels down); with the punctuation counted its steep part ends at depth 6, and from depth 8 to 16
  the cost is 3.98–5.81 M, growing with the output (each level's block form re-indents the rest
  of the nest) rather than with the cube of the depth.
- What is left: the step from depth 4 to 6 (local slope 2.86, 45 misses per nest, unchanged),
  where the nest is close to one line and the bound is below the width, so the attempts are
  rendered and fail on structure, not width. At depth 8 the final bound still lets 40 of 160
  failing attempts through. A lower bound cannot reject an attempt whose one-line form would fit
  if it existed; the mechanism needs the exact width of the unique one-line text and the
  nested-shape condition `R` of the theory record (§ 2 there) to know that the rendering will not
  be one line, which is a width model, not built here.
- rustfmt on the same files (EXP-019): 0.042 s at depth 8, 3.4 s at 16.

## Gates (measured)

| gate | scope | `ee62a16` | `c915da4` | `9006503` |
|---|---|---|---|---|
| `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | clean | clean | clean |
| `cargo test --workspace --all-features` | | 302 passed, 0 failed | 302 passed | 302 passed |
| `bench -n 1 --check fixtures-e8d87c2.tsv`, sequential and `--threads 4` | 1238 fixtures | 0 changed, 0 differ between 4 threads and `format_source` | same | same |
| `bench -n 1 --check registry-e8d87c2.tsv` | 25,024 registry files (20,850 recorded) | 0 changed | 0 changed | 0 changed |
| `conform -i`, `conform -i -V 1.8.0` | 1220 fixtures | 1220/1220, 0 not idempotent, both | same | same |
| `config_matrix -n 1220` | 31 option values against live rustfmt | every value identical on every accepted file (1022/1022; editions 844/844, 843/843, 838/838) | every value identical on every accepted file | every value identical on every accepted file |
| `profile.sh --core 3` | 49-file sample | 231,121,947 | 228,847,625 | 230,281,855 |
| `profile.sh --sim` | | | | § 4 |
| debug-assertion harness | fixtures, registry, cliff shapes | | | 0 failures (§ 5) |
| cliff generator | closures-expr, depths 2–16 | identical to rustfmt; § 6 | § 6 | identical to rustfmt; § 6 |

## Decision

Kept on the branch, for the integrator: `ee62a16`, `c915da4`, `9006503`. Together −1.77 M `Ir`
(−0.76%), −1.83 M net of the parser's growth term, −1.90 M per file on the sample, no output
change on either corpus or any option value; the nested-closure cliff bent from 105.4 M to 5.8 M
`Ir` at depth 16. Rejected: the block pre-check (below resolution), the first fit claim (refuted
by the harness, corrected). The lever's ceiling at the sites a lower bound can decide is 5.0 M
(2.15%) of rejected renders, of which the final bound reaches 2.7 M; the theory record's 15.6–
18.1% lies mostly at decision points whose losing candidate may be multi-line, which a lower
bound cannot decide.

Merge note: the pre-checks skip only in whole-file runs under rustfmt 1.9.0
(`may_skip_dead_candidates`, E18's guard), so `format_snippet` and 1.8.0 render as before. The
debug check in `format_expr` makes debug builds slower (the width test takes about two minutes
in a debug build).

## Follow-up

- **What a width model adds.** The exact width `W(n)` of the unique one-line text (decide-once
  record § 2) and the nested-shape condition `R` would decide "this attempt is one line and fits"
  as well as "it cannot be", taking over the S1 attempts whose bound is below the width but which
  fail on structure (47 of 132 on the sample, 40 of 160 at depth 8), the `if`/`else` attempts that
  fit, and `choose_rhs`'s same-line candidate (decide-once § 2, 2.1–3.4% after its own cost).
- **The fit predicate is narrow.** It excludes string literals, macro calls, struct literals,
  blocks, typed closure parameters and generic arguments; on the sample it cost S1b 26 of its 77
  bound rejections. Lists re-measure their items, so a string inside an argument list is safe and
  only a string in a line-propagating position (alone, after a prefix, as a closure body) is not;
  admitting kinds needs the per-kind argument of § 3 and the harness.
- **Caching the bound.** It is computed per attempt (113 scans on the sample, 1,176 at depth 12);
  a per-node cache keyed like the memo would make it once per node, if the cliff's remaining cost
  ever warrants it.
- **A negative memo.** "No one-line rendering of `n` at width ≤ w" is a fact of the node and the
  flags, not of the shape's indent; keyed that way it would merge the (indent, offset) shapes of
  the cliff that the memo keeps apart.
- The heap-layout term again decided the sign of two commits' total `Ir` (EXP-020): the parser's
  vector growth should be made layout-independent before levers of under 1 M are judged on the
  total.
