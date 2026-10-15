# 2026-10-15: Decide-once formatting, exact reuse and a shared sink (theory)

Part of [../2026-10-15-extensions-theory.md](../2026-10-15-extensions-theory.md). Labels: measured, read, derived, hypothesis. No code was changed.

## Question

How much of the final head's 312,733,055 instructions on the 49-file sample the formatting logic
itself can still give up, by which mechanism and with what exactness: (1) candidate layouts
computed inside one rewrite and then discarded, the part X9 did not count; (2) deciding one-line
layouts from widths before any string is built; (3) reusing a result computed under one shape at
another; (4) writing every rewrite into one buffer instead of returning an `Option<String>`;
(5) extending the memo. Each answer carries a bound with its arithmetic, an exactness argument
and the experiment that would refute it.

## Method

Code read at `bdfb9d1` (`git diff --stat bdfb9d1 HEAD -- chloro-core/src` is empty) and rustfmt
1.9.0 at `/home/user/ref/rustfmt-v1.9.0/src`. `$S` = `.../scratchpad/theory-decide-once/`,
`$P` = `.../scratchpad/profile-final/`; binaries in `target/release/examples/`, `bench` sha256
`fdfce77d2f43c19a` (the profile's).

```text
callgrind_annotate --inclusive=yes --threshold=99.9 $P/callgrind.1.out        # inclusive Ir ($S/incl.txt)
python3 -I scripts/cgparse.py $P/callgrind.1.out final.pkl                     # call arcs (profile2's scripts, copied)
python3 -I scripts/{incl,arcs}.py final.pkl REGEX...                           # inclusive costs, caller -> callee arcs
valgrind --tool=callgrind --dump-instr=yes bench -n 1 --root /tmp/chloro-bench/sample49   # cg_instr.out, 312,734,715 Ir
python3 -I scripts/callsites.py cg_instr.out CALLER [CALLEE]                   # calls and inclusive Ir per call site
python3 -I scripts/instr.py cg_instr.out FN ADDR...                            # execution count of one instruction
objdump -d --no-show-raw-insn --start-address=A --stop-address=B bench         # call sites and branches -> source
valgrind --tool=dhat bench -n 1 --root /tmp/chloro-bench/sample49              # dhat.final.json: 237,991 blocks, 123,468,129 B
alloc_count /tmp/chloro-bench/sample49 ; dump_tree <each sample file>         # 237,431 allocations; node kinds
fmt_stdin max_width=N < case.rs                                                # 4 inputs x 24 widths ($S/cx/)
```

- The instruction-level callgrind run, the DHAT run and `alloc_count` were taken in this container
  at 01:19–01:24 UTC by the cancelled first attempt of this record (`cg_instr.log`, `dhat.log`,
  `alloc_sample.txt` in `$S`); every figure here was recomputed from those raw files.
- The release build has no line tables, so each call site was mapped to source by its callee, its
  order among the calls and the branches around it in `objdump` output; the mapping is stated where
  it carries a number.
- "Outer" means callgrind's outermost activation of a function (deeper recursion levels are named
  `'2`): outer inclusive costs of one function never overlap; costs of different functions can (a
  `let` inside a closure inside a chain counts under both).
- "Uniform-cost estimate" splits a call site's inclusive cost over its outcomes in proportion to
  their counts (hypothesis: kept and discarded calls of one site cost the same on average).
- Compute: under 3 minutes in this session.

## 1. Speculation inside a rewrite

### Try-in-order points (read)

| point (chloro; rustfmt 1.9.0) | computed before the decision | discarded | decision reads |
|---|---|---|---|
| overflow, last argument: `try_overflow_last_item` (overflow.rs:438-575; rustfmt overflow.rs:483) | all items but the last at `nested_shape` (`leave_last`, lists.rs:843-848, overflow.rs:577-591); the last item at the overflow shape (overflow.rs:446-478); `definitive_tactic` | the overflow candidate when the tactic is not `Horizontal`; the last item is then rewritten at `nested_shape` (overflow.rs:520-524) | widths and predicates (lists.rs:278-314) plus the overflow candidate's rendered first line as placeholder (overflow.rs:420-424) and its newline count (overflow.rs:502) |
| chain, last child: `format_last_child` (chains.rs:611-695; rustfmt chains.rs:712) | the child at `one_line_shape` (`rw`), then at `child_shape` (`new_rw`) unless `rw` fits and has ≥ 5 lines | one of `rw`, `new_rw` | width of `rw`'s first line against `one_line_budget` (chains.rs:659) and the line counts of both (chains.rs:660, 674) |
| assignment right-hand side: `choose_rhs` (expr.rs:2059-2108; rustfmt expr.rs:2293) | the RHS on the same line; if multi-line, too wide or absent, the RHS on the next line | one of the two | fast path: width and `contains('\n')` (expr.rs:2070-2073); slow path: `filtered_str_fits` on every line of the next-line candidate, then `prefer_next_line`: newline counts and first lines ending in `(`, `{`, `[` (expr.rs:2088-2104, 2135-2146) |
| match arm body: `rewrite_match_body` (matches.rs:407-571; rustfmt matches.rs:397) | the body on the arm's line; unless block-like or one line that fits, the body on the next line | one of the two | fast path: width and `contains('\n')` (matches.rs:533-541); slow path: `prefer_next_line`, `first_line_width`, `contains('\n')` (matches.rs:548-570) |
| closure body: `rewrite_closure`, `rewrite_closure_expr` (closures.rs:48-85, 179-207; rustfmt closures.rs:29, 197) | the body without a block, rendered in full | the rendering when it is multi-line and the body kind may not be (`veto_multiline`, closures.rs:201-204); the block form follows | predicate on text (`contains('\n')`) |
| last-argument closure: `rewrite_last_closure` (closures.rs:366-431; rustfmt closures.rs:361) | for forced blocks, the block form, then the expression form if the block has ≤ 7 lines | the block form when the expression form is one line | rendered line count (closures.rs:405) |
| block: `rewrite_block_inner` (expr.rs:674-699; rustfmt expr.rs:668) | the block through a nested visitor, then the one-line form if ≤ 3 lines | the visitor result when the one-line form fits | rendered line count (expr.rs:693), width (expr.rs:590) |
| `if`/`else` on one line: `ControlFlow::rewrite_single_line` (expr.rs:864-908) | both branches on one line | all of it unless the whole fits | widths and `contains('\n')` |
| binary pair: `rewrite_pair` (pairs.rs:159-243, one-line attempt 185-208; rustfmt pairs.rs:169) | lhs; rhs on the same line | the same-line rhs when the line does not fit | widths, `contains('\n')`, first line ending in `{` |
| fn signature: `rewrite_fn_base` (items.rs:715-997) | parameters once at the multi-line budget, one tactic, one `write_list` | nothing | width and `contains('\n')` (items.rs:800) |

- `write_list` runs once per list, after the tactic is fixed: outer `overflow::Context::rewrite`
  takes 5,081 tactic decisions and makes 5,081 `write_list` calls (**measured**, call sites
  0x8a145, 0x8a615), so no list is written for a tactic that is then not taken; a written list is
  discarded only together with the enclosing candidate.

### Outcomes and candidate costs (measured)

Counts are executions of call sites and branch targets in `cg_instr.out`, outer level with the
`'2` level in parentheses; costs are outer inclusive Ir of the candidate call sites.

| point | decisions with both candidates computed | first kept | second kept | first-candidate site | second-candidate site |
|---|---|---|---|---|---|
| overflow, last argument | 35 (93) | 0 | 35 (93) | 698 attempts (1,586), 38.37 M = 12.27% | 3,346 nested-shape rewrites (2,044), 37.41 M = 11.96% |
| chain, last child | 1,276 (1,183) | 1,175 (1,043) | 101 (140) | 1,367 calls, 24.66 M = 7.88% | 1,276 calls, 9.48 M = 3.03% |
| `choose_rhs` | 177 (52) | 160 (45) | 17 (7) | 703 calls, 35.84 M = 11.46% | 177 calls, 30.61 M = 9.79% |
| match arm body | 61 (20) | 35 (8) | 26 (12) | 560 calls, 32.32 M = 10.33% | 61 calls, 5.47 M = 1.75% |
| closure, block body | 108 tried (88) | 80 (19) | 28 (69) | 29.21 M = 9.34% | 2.05 M |
| closure, expression body | 206 tried (155) | 188 (140) | 18 (15) | 6.27 M = 2.00% | 0.76 M |
| block, one-line form | 17 tried (20) | 13 (20) | 4 (0) | visitor: 126 calls (99), 34.63 M = 11.07% | 0.02 M |
| `if`/`else` one line | 43 branch renders | 1 result kept | — | 0.71 M | — |

- Overflow (outer): of 5,081 decisions 1,072 have no argument and 4,009 have at least one (the
  first item's `ListItems` closure runs 4,009 times, 0x89168); overflow was attempted 698 times
  (235 closures, 371 other expressions, 92 non-expression items), produced a placeholder 674
  times (0x8a159) and kept it 663 times (0x8a15d: 643 single-argument, 20 multi-argument); 11
  placeholders lost to a vertical tactic and 24 attempts produced nothing; 3,346 = 4,009 − 663
  last items were rewritten at `nested_shape` (0x8a20e); the single-newline re-render (0x8a4af)
  ran 0 times outer and 2 times at `'2`.
- Chain (outer): the call sites after the children loop run 108, 1,367 and 1,276 times; `new_rw` is
  computed only after a successful `rw`, which fixes 0xb4783 as `rw` and 0xb4e97 as `new_rw`. Of
  1,276 decisions, 1,175 keep `rw` because `new_rw` has at least as many lines (0xb4f96), 77 keep
  `new_rw` because `rw` missed `one_line_budget` (0xb4f59, the `clone` branch), 24 because `new_rw`
  has fewer lines (0xb4f24); `new_rw` was never `None` (0xb4f40 not executed).
- `choose_rhs` (outer): the four `concat` sites decode by their arguments: 526 at 0xa0090 (`" "` +
  same-line, fast path), 160 at 0xa0050 (same-line after `prefer_next_line` false), 14 at 0x9ffba
  (indent + next-line after `prefer_next_line` true), 3 at 0xa0003 (no same-line result);
  `filtered_str_fits` on the next-line candidate ran 174 times and never returned false.
- Match body (outer): 563 bodies, 560 same-line renders (0xcf611), 502 accepted at once
  (`combine_orig_body`, 0xcf673), 61 next-line renders (0xcf38f); `combine_next_line_body` 26
  (3 + 23), `combine_orig_body` after the comparison 35 (0xcf540).
- Closures (outer): fallbacks are the block-form calls after the attempt without a block: 28 at
  0xc1e14 (`rewrite_closure_block`'s body rewrite), 18 at 0xc1a05 (`rewrite_closure_with_block`);
  `rewrite_closure_expr` renders a body 324 times and returns 278.
- Block (outer): the one-line result test after 0x9ecdd is `Some` 4 times (0x9ecef).

### Bounds (derived)

| discarded work (outer) | arithmetic | uniform-cost estimate | hard bound |
|---|---|---|---|
| `choose_rhs` next-line candidate | 160/177 × 30.61 M | 27.67 M (8.85%) | 30.61 M |
| chain `new_rw` | 1,175/1,276 × 9.48 M | 8.73 M (2.79%) | 9.48 M |
| closure without block, block body | 28/108 × 29.21 M | 7.57 M (2.42%) | 29.21 M |
| match next-line candidate | 35/61 × 5.47 M | 3.14 M (1.00%) | 5.47 M |
| overflow candidate | 35/698 × 38.37 M | 1.92 M (0.61%) | 38.37 M |
| chain `rw` | 101/1,367 × 24.66 M | 1.82 M (0.58%) | 24.66 M |
| match same-line candidate | 26/560 × 32.32 M | 1.50 M (0.48%) | 32.32 M |
| block visitor result | 4/126 × 34.63 M | 1.10 M (0.35%) | 34.63 M |
| `choose_rhs` same-line candidate | 14/703 × 35.84 M | 0.71 M (0.23%) | 35.84 M |
| `if`/`else` one-line attempts | all | 0.71 M (0.23%) | 0.71 M |
| closure without block, expression body | 18/206 × 6.27 M | 0.55 M (0.18%) | 6.27 M |
| sum (points overlap where nested) | | 55.4 M (17.7%) | |

- Two other estimates of the same volume (**derived**): X9's repeat share scaled to this head,
  16.6–18.1% (51.8–56.7 M: 16.9% of 595.3 M and 18.5% of 595.3 M at `9202b42`, times 0.5145 =
  146,781,516 / 285,298,480, the ratio of `memoize::<format_expr>` inclusive Ir here to
  `884f9c9`'s in profile-baseline.md; hypothesis: repeats shrank with the rest of expression
  rewriting); and the second-candidate sites, each a re-render of a node already rendered at
  another shape: 30.61 + 9.48 + 5.47 + 2.05 + 0.76 + 35/3,346 × 37.41 = 48.8 M (15.6%).
- The three figures, 15.6%, 16.6–18.1% and 17.7%, put the volume of speculation at about one sixth
  of the run (**derived**). Speculation inside a rewrite and X9's repeats are the same work seen
  from the two candidates of a decision: when the first candidate is discarded, the kept second is
  a repeat of about its cost; when the second is discarded, it is the repeat (**derived** from the
  outcome table; the per-event costs are a hypothesis). X9's 17–18.5% is therefore not a floor
  under further inside-call speculation; the inside-call figures locate it.
- One point carries half of the estimated discarded work: `choose_rhs` renders the right-hand side
  again on the next line for every expression right-hand side (`let`, `if let`, assignment,
  `const`, `static`, enum discriminant) whose same-line result is multi-line or too wide (177 of
  703), and discards that render 160 times (90%).
- The other points mostly keep their first candidate: overflow 663 of 698, chain `rw` 1,175 of
  1,276, match same-line 502 of 560 at once, `choose_rhs` 526 of 703 at once.

### Effects of discarded attempts that reach the output (read)

A skipped candidate also skips its effects on the run; three of them change text:
- `macro_rewrite_failure` is set by every failed macro rewrite (macros.rs:153, 192-196), including
  inside discarded candidates, and `format_snippet` then leaves a macro definition body as written
  (formatting.rs:270-279); rustfmt does the same (rustfmt macros.rs:165-185, lib.rs:302-326).
- `lost_comment` (comment.rs:1199-1213) and skipped ranges (`add_skipped_span`, macros.rs:146-176)
  are read only in snippet runs (formatting.rs:276-279; re-indentation at macros.rs:839).
- `format_failure`, under `rustfmt_version = "1.8.0"` only, is set during missed-span processing of
  any visitor (missed_spans.rs:190-200), also a nested one inside a discarded candidate, and makes
  `format_text` return the file unchanged (formatting.rs:105).
- At the top level of a file none of the three changes the text (formatting.rs:95-121,
  formatter.rs:74-100). A guard for skipping candidates therefore tests the run, not the context:
  "not a snippet run" (a `RunState` bit set under `format_snippet`) and `rustfmt_version = 1.9.0`;
  `context.is_macro_def` does not serve, because nested visitors do not inherit it
  (visitor.rs:139-147).

## 2. Width-first exactness

### One-line text is unique per node (read)

Claim: for a node, its rewrite function and tag and the context flags of the memo key (context.rs:312-318), all
one-line results over all shapes are the same text. A shape enters a rewrite only through budgets
(fits tests), indent strings (written only after a newline), and the choice among layouts; a
one-line result has no newline, so it is the construct's flat template over its children's one-line
texts. By construction:

| construct | one-line text | holds | reason |
|---|---|---|---|
| path, ident, literal | source or normalised source | yes | literals are copied as written: `format_strings` is a constant `false` (config.rs:484, 660; expr.rs:1282-1314) |
| call, method arguments, tuple, array, struct literal | `ident` `(` items joined by `", "` `)` | yes | items are rendered once at `nested_shape` and reused in any tactic (overflow.rs:577-591); a horizontal list has no trailing comma except with `trailing_comma = Always` or inside a macro whose source list ends with one (lists.rs:166-175, expr.rs:1316-1329), both shape-independent |
| chain | rewrites joined with `""` | yes | root absorption depends on `shape.offset` (chains.rs:765) but changes only grouping, not the joined text (chains.rs:698-735) |
| binary, cast, unary, index, range, paren | operands with fixed infixes | yes | pairs.rs:185-208 |
| closure | `\|params\| body` | yes, per rewrite function | `format_expr`'s path renders a forced block body only through the visitor, never on one line (closures.rs:160-176); the overflow path's `rewrite_last_closure` gives the same node a one-line expression form (closures.rs:402-411), so a width is kept per rewrite function, not per node |
| block | `{ expr }` | yes | expr.rs:577-600 |
| `if`/`else` | `if c { a } else { b }` | yes | expr.rs:864-908 |
| block comment inside a node | `a /* c */ b` | yes | joined on one line only when it fits (comment.rs:131) |
| `#[rustfmt::skip]` | the source snippet | yes | every shape (expr.rs:132-133) |
| macro call | formatted, or the source snippet when `rewrite_macro` fails at that shape | **no** | `or_else(wrap_str(snippet))` (expr.rs:249-260) |
| any expression whose candidate lost a comment | the source snippet | **no** | `recover_comment_removed` (expr.rs:300, comment.rs:1199-1213) |

- On already formatted input the snippet of a one-line node is its formatted text, so the two
  exceptions agree there (**hypothesis**); on unformatted input they can differ. A search with
  `fmt_stdin` (4 inputs, `max_width` 14–60) found no output with the fallback text inside an
  otherwise formatted statement (**measured**); the path is established from code only.
- Options that would break uniqueness in rustfmt are constants in chloro: `format_strings`,
  `overflow_delimited_expr`, `struct_lit_single_line`, `wrap_comments`, `normalize_comments`
  (config.rs:476-512, 655-690).

Whether the rewrite at shape `s` is one line is not a function of the first-line width alone (read):
- per-construct limits from `use_small_heuristics` (config.rs:262-300) cap the flat width:
  `fn_call_width` through `min(one_line_width, item_max_width)` (overflow.rs:340, lists.rs:297),
  `chain_width` (chains.rs:628-633), `struct_lit_width`, `array_width`,
  `single_line_if_else_max_width`, `single_line_let_else_max_width`, `attr_fn_like_width`,
  `struct_variant_width` — constants for a run;
- items are rendered at `nested_shape`, block indent + `tab_spaces` with `max_width`
  (overflow.rs:709-724): an item is one line there only if it fits `max_width − indent −
  tab_spaces − 1`, which can be narrower than the list's one-line budget when the shape's width is
  not tied to its indent (`infinite_width`, expr.rs:2097-2101, used for enum discriminants at
  items.rs:1901-1908; `Shape::legacy` shapes);
- a block is one line only if the visitor's multi-line result has ≤ 3 lines (expr.rs:693);
- `one_line_width == 0` forces a vertical list (overflow.rs:530).

### The pass (design; hypothesis)

Bottom-up over expression, pattern and type nodes, per flag set in use:
- `W(n)`: width of the unique one-line text, or `None` ("unknown": not modelled, contains a comment,
  a multi-line literal or a macro, or exceeds a construct limit);
- `R(n)`: the deepest block indent at which every item that `n` renders at a nested shape still fits
  there; for a list, `R = min over items i of min(max_width − tab_spaces − 1 − W(i), R(i) −
  tab_spaces)`, `∞` for leaves;
- `P(n, s)` = `W(n)` known ∧ `W(n) ≤ s.width` (the call's ident and delimiters are inside `W`) ∧
  `s.block().indent.block_indent ≤ R(n)` ∧ the construct's own shape-independent conditions.
- Exactness needs only: `W(n) = Some(w)` ⇒ (the rewrite at `s` is one line ⇔ `P(n, s)`) and its
  width is `w`. Where `W` is `None`, rustfmt's control flow runs unchanged, so the pass can cover
  constructs one at a time; a debug assertion in every rewrite checks both properties on both
  corpora.
- Cost: 15,051 expression, 2,818 pattern and 2,692 type nodes in the sample's file trees
  (**measured**, `dump_tree`; macro-argument fragments not included) at 100–300 Ir per node:
  2–6 M, 0.7–2.0% (**hypothesis**).

Decisions the pass takes over with identical output, if `P` is exact and the § 1 guards hold:
- chain: `rw` one line and within `one_line_budget` ⇒ `new_rw` cannot change the outcome, since
  chains.rs:660-690 keeps `rw` for any `new_rw` with ≥ 1 line or `None` — a dead candidate; this one
  needs no `W` at all (**read**);
- closure veto: `¬P(body, body_shape)` and the veto applies ⇒ `None` without rendering
  (closures.rs:201-205);
- `if`/`else` one line: `P` on both branches (expr.rs:864-908);
- `choose_rhs` and match bodies: `¬P(e, same-line shape) ∧ P(e, next-line shape)` ⇒ the next-line
  candidate is one line and fits, `filtered_str_fits` passes and `prefer_next_line` is true
  (expr.rs:2141, matches.rs:556) ⇒ the same-line candidate need not be rendered;
- overflow: `P(last, overflow shape)` gives the placeholder width of a one-line candidate;
- list tactics: `definitive_tactic` from `W` and `P` at the nested shape (lists.rs:278-314);
- literals: `filtered_str_fits` on a literal reads three widths of its text (utils.rs:271-298);
  1,964 calls from `rewrite_literal` cost 4.67 M (1.49%) for 1,595 `LITERAL` nodes (**measured**),
  because paths and literals bypass the memo (expr.rs:113-117); widths computed once per literal
  remove the repeats, ≤ (1,964 − 1,595)/1,964 × 4.67 M = 0.88 M (0.28%) (**derived**).

Decisions it cannot take over, because they read a rendered multi-line candidate: chain line counts
when `rw` is multi-line (chains.rs:674); `prefer_next_line` on two multi-line candidates
(expr.rs:2142-2145); the match body's slow path (matches.rs:553-570); blocks of ≤ 3 lines
(expr.rs:693); last closures of ≤ 7 lines (closures.rs:405); the placeholder of a multi-line
overflow candidate (overflow.rs:420-424) and `count_newlines(overflowed) == 1` (overflow.rs:502);
results produced through a source fallback.

Bound (**derived**, uniform-cost estimates of § 1 at the points taken over, chain excluded): closures
7.57 + 0.55, `if`/`else` 0.71, overflow ≤ 1.92, `choose_rhs` same-line ≤ 0.71, match same-line
≤ 1.50 = 12.96 M (4.14%) gross; 2.1–3.4% after the pass's own cost.

### Measure-then-render printers

Oppen's printer (1980), Wadler's "A prettier printer" (2003), Leijen's `wl-pprint` and Prettier's IR
build a document of text, breaks, nesting and groups; a group prints flat if its flat width plus the
text up to the next possible break fits the remaining width, otherwise broken, so widths are
measured once and printing is linear. Prettier adds `conditionalGroup` (alternatives in order, the
first whose first line fits) and an "expanded last argument" state; Bernardy (2017) replaces the
greedy choice by a search for layouts optimal in (height, width).

- Expressible with exact equivalence (**derived** from the uniqueness table): the one-line half — a
  group per construct whose flat form is the unique one-line text, with a per-group width cap (the
  `use_small_heuristics` limits) and the `R` condition as an extra fits test.
- Not expressible, by construct (**read**):
  1. choices between two broken layouts by line count or first-line ending — chains
     (chains.rs:660-690), `choose_rhs`/`prefer_next_line` (expr.rs:2135-2146), match bodies
     (matches.rs:553-570), blocks ≤ 3 lines (expr.rs:693), last closures ≤ 7 lines
     (closures.rs:405), overflow's single-newline re-render (overflow.rs:502-513): a group or a
     conditional group selects by whether a flat form or a first line fits, never by the height of a
     broken alternative, and Bernardy's global height criterion differs from rustfmt's local one
     (`choose_rhs` keeps a taller same-line layout unless the next-line one is 2 lines shorter,
     expr.rs:2142);
  2. failure as control flow: `?` on 149 `sub_width`/`offset_left`/`checked_sub` sites and on child
     rewrites; "try A, else B" (closures.rs:74-77, `choose_rhs`'s `(None, Some)` arm, a `None`
     placeholder) has no counterpart in an algebra whose layouts always exist;
  3. attempts that change context for the subtree: `force_one_line_chain` is set for the overflow
     attempt of a method-call argument (overflow.rs:449-477) and read by `join_rewrites`
     (chains.rs:704-706), so one subtree renders differently in the attempt and in the fallback;
  4. effects of discarded attempts read later (§ 1);
  5. source fallbacks (the uniqueness table's two exceptions).
- An exactly equivalent document algebra would need choice by arbitrary predicates on rendered
  alternatives, which is rustfmt's control flow itself; the algebra's fragment is the width pass.

## 3. Exact reuse across shapes

Rule under test (formatter-work-measurements.md, X9c's starting point): reuse a result when the
first attempt was one line and its widest line fits the new shape. For a one-line result `T` at
`s1` reused at `s2` (**derived** from § 2):
1. by uniqueness, the rewrite at `s2` is `T`, multi-line, or `None`;
2. it is `T` iff the construct's one-line test passes at `s2`: the first-line width (the rule's
   condition); the `use_small_heuristics` limits (constants, passed at `s1`); the nested-shape fit
   `R`, which the rule does not check; the block's ≤ 3-line test;
3. tag and flags equal (in the key today);
4. `T` was not produced by a source fallback;
5. effects: the attempts rustfmt makes inside the rewrite at `s2` can differ from those at `s1`, so
   replaying `s1`'s effects is exact only outside snippet runs.

Counterexamples (read, derived):
- **Indent-dependent sub-layouts.** `f("…58 columns…")` is one line at block indent 0 (the item fits
  the nested width 100 − 4 − 1 = 95; 58 ≤ `fn_call_width` 60). At a shape with block indent 40 and
  infinite width (expr.rs:2097-2101) the nested width is 100 − 44 − 1 = 55 < 58: the literal fails
  `wrap_str` (expr.rs:1312), the list item is `None`, `write_list` returns `None` (lists.rs:356) and
  the call fails, while the rule returns `T`. Condition 2's `R` closes the case.
- **`max_width` and `use_small_heuristics`.** For one-line results the limits are run constants
  (config.rs:262-300). For multi-line results the binding term of `min(one_line_width,
  item_max_width)` (lists.rs:297) switches with the shape, so a wider shape can select horizontal
  where a narrower one selected vertical (X9c finding 1).
- **Trailing commas.** One-line lists carry one only under `Always` or a macro source comma — the
  same at both shapes; vertical lists carry one under the default `Vertical` (lists.rs:170), so a
  vertical result reused where rustfmt picks horizontal adds a comma (X9c's
  `Some(\n current_section_idx,\n)`).
- **Comments.** A block comment stays on the line when it fits (comment.rs:131); a candidate that
  lost a comment becomes the source snippet (comment.rs:1199-1213), excluded by condition 4;
  multi-line comments are re-indented with the shape, as multi-line literals are (X9c finding 3).

Bound from X9's breakdown (scaled as in § 1, **derived**): identical results 1.47%; of the 10.4
points of differing results, the indentation class (10.2% here) is multi-line by definition — a
one-line result has no indentation to differ in — and lies outside the rule; the layout class
(6.47%) contributes only its one-line repeats. Exactly reusable under conditions 1–5: ≤ 1.47% plus
the one-line part of 6.47%, ≤ 7.9%. X9c's inexact rules covered 8.7% (fixtures) and 2.2%
(registry) of `format_source`, with 0.62% and 0.10% of attempts disagreeing (**measured**, X9c).

Multi-line reuse with slack intervals (**hypothesis**): during a rewrite, record the least slack over
every width test it makes (budget − width where the test passes, width − budget where it fails):
98 explicit width comparisons in 18 files and 149 `sub_width`/`offset_left`/`checked_sub` sites
(**measured**, grep). A result at `s1` is valid at `s2` = `s1` shifted by Δ if |Δ| is below the
slack and the node holds no multi-line literal or comment: every decision inside is then the same,
so the text is `T` with continuation lines re-indented by Δ. Bound ≤ the indentation class, 10.2%,
which contains `choose_rhs`'s discarded next-line renders (8.85% estimated) where the same-line
candidate's slack exceeds the 4-column step plus the first-line budget difference. One unrecorded
comparison makes the scheme inexact.

## 4. A shared sink instead of `Option<String>`

### Design (paper)

- Signature: `fn rewrite_into(&self, ctx, shape, out: &mut Sink) -> Option<Range>`; a failing
  rewrite leaves `out` as it found it (`out.truncate(mark)`); `Sink` holds one `String` per run.
- Width queries on a result read its range (`first_line_width`, `last_line_width`,
  `contains('\n')`, line counts), at today's cost, or from per-range stats kept on every push —
  newline count, first, last and widest line width, ASCII flag — which compose in O(1) under
  concatenation (the "widths carried with strings" lever of scanning-and-strings.md).
- Two live candidates: A at `[m, a)`, B at `[a, b)`; keep A by `truncate(a)`, keep B by
  `copy_within(a..b, m)` and truncate; a prefix such as `choose_rhs`'s indent is written before
  rendering the candidate that needs it.
- Lists: items are rendered in sequence with their ranges; the tactic reads their stats; the joined
  list is written after them and moved down over them — one copy per list, as `write_list`'s copy
  today (lists.rs:353-596), without the per-item `String`s.
- Memo: results copied into a side arena (`Vec<u8>` plus ranges, cleared with `clear_memo`); a hit is
  one `extend_from_slice`, without the `clone` allocation of context.rs:356.
- Nested visitors (`rewrite_block_with_visitor`, expr.rs:613-650) write into the same sink.
- Control flow stays rustfmt's, so the module-to-rustfmt map stays readable (protocol §1.4).

Reads of candidate text a sink has to keep (**read**, counts by grep over `formatter/`): 105
`contains('\n')` in 17 files, 57 `last_line_width`, 15 `first_line_width`, 9 `count_newlines`, 6
`lines().count()`, 3 `first_line_ends_with`, 30 `combine_strs_with_missing_comments` calls (insert a
separator or comment between two rendered strings); transformations: `trim_tries`
(chains.rs:820-848), the source swap of `recover_comment_removed` (comment.rs:1199-1213),
`nop_block_collapse` (matches.rs:611-622), `rhs.trim_start()` (expr.rs:2052), the placeholder's
first line (overflow.rs:420-424), `write_list`'s `last_line_width(&result)` on its own partial
output (lists.rs:499, 545). Each operates on a range; replacements are truncate-and-write.

### Bound (measured, derived)

| item | figure | source |
|---|---|---|
| allocator arcs from formatter frames and the string helpers they call (inclusive, with realloc's internal copy) | 32,545,212 (10.41%) | **measured**, `final.pkl` arcs into `malloc`/`free`/`realloc`/`RawVec` growth |
| same, parser, macro-argument parsing and `cst` | 19,098,539 (6.11%) | **measured** |
| same, two `Rc<Cell<bool>>` per `RewriteContext` | 1,105,163 (0.35%) | **measured** |
| `String` blocks at this head | 176,786 − 40,278 = 136,508 | **derived**: `String` new + growth at `8d418ee` (profile-flat.md §3) minus E3's removed `malloc` + `realloc` calls (EXP-003 totals), all of them `String` sites by EXP-003's lever list |
| formatter blocks | 237,991 − 30,468 − 2,055 − 353 = 205,115; `String` share 66.6% | **derived** (DHAT totals and categories, `dhatsum_final.md`) |
| allocator Ir per block | 47,003,376 / 237,991 = 197.5 | **derived** |
| `String` allocation cost | A: 32.55 M × 0.666 = 21.66 M (6.93%); B: 136,508 × 197.5 = 26.96 M (8.62%) | **derived** |
| formatter `memcpy` (realloc's internal copies excluded) | 3,927,404 (1.26%) | **measured** |
| `fmt` machinery, exclusive (`format_inner`, `fmt::write`, `write_str`, `pad`) | 2,007,208 (0.64%) | **measured** |
| **sink bound** | 27.60–32.90 M (8.83–10.52%) | **derived**: A or B + `memcpy` + `fmt` |

- The bound counts every `String` block and every formatter copy; a sink keeps the moves of list
  re-assembly, candidate swaps and memo copies, so the removable part lies below it
  (**hypothesis**). profile-flat.md's lever 2 was ≤ 10.7% at `8d418ee` by method B.
- Nested visitors allocate twice: `FmtVisitor::from_context` calls `FmtVisitor::new`, which reserves
  twice the file's size (visitor.rs:102), then replaces the buffer with 256 bytes (visitor.rs:145):
  2,290 DHAT blocks and 68,614,171 of the run's 123,468,129 allocated bytes, ≤ 1,376,432 Ir (0.44%)
  inclusive (**measured**); rustfmt keeps the doubled buffer (rustfmt visitor.rs:803-833). The
  first reservation is removable alone, exact by construction.
- Cost: 191 `-> Option<String>` signatures in 19 files (expr.rs 40, items.rs 39, types.rs 24,
  chains.rs 12, macros.rs 11, others ≤ 8), 38 `impl Rewrite` blocks, 20 `-> String` and 10 tuple
  returns (**measured**, grep); every module (3,000–6,000 changed lines, **hypothesis**).
- Conformance: none if the bytes written are the same; the risk is implementation error, which the
  golden hashes on both corpora detect.

## 5. Memo extensions

- Final head (**measured**): 10,229 `memoize` calls, 9,838 misses, 391 hits (3.82%), as at
  `8d418ee`. Overhead: `memoize` exclusive 3,197,209 + `reserve_rehash` 580,122 + the result clone's
  `malloc` 634,176 and `memcpy` 305,372 (10,133 calls) + the frees at `clear_memo` (visitor.rs:414),
  ≈ 9,742 × 107.3 Ir = 1.05 M (**derived** from `visit_item`'s 10,817 frees for 1,160,572 Ir) =
  5.76 M (1.84%), against about 5.6 M (1.44%) at `8d418ee` (profile-flat.md).
- The memo saves 19 M net (memo on/off at `6b07131`, profile-baseline.md lever 11), so removing it
  is not a lever while repeats remain; its clone and free part (≤ 1.99 M, 0.63%) falls under § 4.
- Exact-shape hits stay rare because repeats come at other shapes: X9 counted 916,291 uncached
  rewrites of 558,884 nodes on the fixtures (**measured**, X9).
- A shape-free key for one-line results — `(tree, index, tag, flags)` → `T`, `W`, `R`, a fallback
  bit and the effects — answers at `s2` when `W ≤ s2.width`, `s2`'s block indent ≤ `R`, no
  fallback was used and the run is not a snippet run (§ 3 conditions). Bound ≤ X9's identical
  class, 1.47%, plus the one-line repeats of the layout class (≤ 6.47%) (**derived**); X9 places
  the identical class on nodes of low cost per rewrite (paths, short calls) and a lookup costs
  about 313 Ir (3,197,209 / 10,229, **derived**), so the net part of the 1.47% is unknown
  (**hypothesis**).

## Ranked bounds

Shares of 312,733,055, ranked by the upper end of the removable share. "Bound" is the instruction
count if the lever removes all it addresses.

| # | lever | addresses | removable, with argument | bound on total | cost | exactness / conformance risk | smallest experiment and falsifier |
|---|---|---|---|---|---|---|---|
| 1 | shared sink, rollback by truncation | formatter `String` allocation + copies + `fmt`, 27.60–32.90 M | ≤ 8.83–10.52% (moves of lists and candidates stay) | ≥ 279.8–285.1 M | 191 signatures, 38 `Rewrite` impls, 19 files, every module | exact by construction (none beyond bugs) | `write_list` appending to `wrap_items`'s buffer in `overflow::Context`: predicts −10,709 blocks ≈ −2.12 M Ir at 197.5 Ir/block; falsified if `Ir` falls < 1.06 M or a golden hash changes |
| 2 | slack-interval reuse of multi-line results | X9 indentation class 10.2% (scaled), incl. `choose_rhs` next-line renders 8.85% (est.) | ≤ 10.2% | ≥ 280.8 M | ~250 width-test sites in 18 files, every rewrite records slack | exact in principle; one unrecorded test breaks it (high) | shadow mode as X9c: reuse at \|Δ\| < slack, compare with fresh renders on both corpora; falsified by one disagreement |
| 3 | one-line memo without shape | X9 identical class 1.47% + layout one-line repeats | ≤ 1.47% (+ ≤ 6.47%) minus ~313 Ir per lookup | ≥ 308.1 M (≥ 288.0 M) | context.rs memo (~150 lines), fallback bits in expr.rs, comment.rs, macros.rs | conditionally exact (§ 3, 1–5); untested (medium) | shadow mode on one-line results with the `R`, fallback and run guards; falsified by one disagreement on either corpus |
| 4 | width pass (`W`, `R`, `P`) for one-line decisions | discarded first attempts at points decidable from widths, 12.96 M (4.14%) est. | 2.1–3.4% net of the pass | ≥ 299.8 M gross; 302.0–306.0 M net | width model per construct (1,500–3,000 lines), decision sites in expr, closures, overflow, matches, pairs | exact by model: `W` must equal the renderer (medium-high) | `W` for paths, literals and calls plus the debug assertion of § 2 on both corpora and `config_matrix`; falsified by one assertion failure |
| 5 | chain dead candidate: skip `new_rw` when `rw` is one line within `one_line_budget` | `new_rw` site 9.48 M (3.03%) | ≤ 3.03%; 0.75–2.79% (one-line `rw` events among 1,175, hypothesis) | ≥ 303.3 M | chains.rs:654-690 (~10 lines) + run-level snippet bit (~10 lines) | exact by argument outside snippet runs and 1.8.0 (low) | guard + count of skipped events; falsified by a golden, `conform -i` (1.9.0, 1.8.0) or `config_matrix` change, or `Ir` falling < 0.1% |
| 6 | nested visitor's doubled buffer | `from_context` 1.38 M (0.44%), 68.6 MB of 123.5 MB allocated | ≤ 0.44% | ≥ 311.4 M | visitor.rs:96-149 (~5 lines) | exact by construction | as row 5 |
| 7 | literal widths once per literal | `filtered_str_fits` from `rewrite_literal` 4.67 M (1.49%) | ≤ 0.88 M (0.28%) exact; more needs a cheaper exact scan | ≥ 311.9 M | expr.rs:1282-1314 (~40 lines) | exact by construction | as row 5's gates; falsified if `Ir` falls < 0.1% |
| — | memo overhead | 5.76 M (1.84%) | not removable while repeats remain (memo saves 19 M net) | — | — | — | — |
| — | speculation volume (reference) | discarded candidates and repeats | 15.6–18.1% (three estimates, § 1) | ≥ 256.1 M | — | — | — |

- Rows 2, 3, 4 and 5 act on the same speculation volume, so their union is at most that volume,
  15.6–18.1% by § 1's three estimates (the compound below takes X9's scaled 16.6–18.1%); the sink
  acts on the strings of whatever work remains (**derived**).
- Largest compound bound: all speculation removed exactly (rows 2 + 3 + 4 + 5 at their union) and
  the sink on the rest: 312.73 M × (1 − 0.181) × (1 − 0.1052) = 229.2 M, or with the low ends
  312.73 M × (1 − 0.166) × (1 − 0.0883) = 237.8 M — a formatter at 229–238 M instructions on the
  sample, −24% to −27% (**derived**; hypothesis: string volume shrinks in proportion to the work
  removed). Parsing (`formatting::parse`, 102.09 M inclusive, **measured**) is untouched, so the
  formatter-side work, 208.33 M (`format_text` 310.41 M − parse), would fall by 36–40%.
- Compound of the levers exact by construction or argument without a width model (rows 1, 5, 6,
  7): 312.73 M × (1 − 0.0303) × (1 − 0.1052) × (1 − 0.0028) × (1 − 0.0044) ≈ 269.3 M at the upper
  ends; 312.73 M × (1 − 0.0075) × (1 − 0.0883) × (1 − 0.0028) × (1 − 0.0044) ≈ 281.0 M at the low
  ends (**derived**).

## What this settles and what it leaves open

- Speculation inside a rewrite, which X9 left unmeasured, is measured per decision point: about one sixth
  of the run (15.6–18.1% by three methods), half of it at `choose_rhs`, whose next-line render of a
  multi-line right-hand side is discarded in 160 of 177 cases (8.85% estimated) (**measured**
  counts, **derived** shares). X9's repeat share and inside-call speculation are the same work.
- Within one call, rustfmt's lists never write an unchosen tactic, and fn signatures render once
  (**measured**, **read**); the discarded work consists of whole candidates, not of list assembly.
- A node's one-line text is the same at every shape except through source fallbacks (macro failure,
  lost comments), which agree on already formatted input (**read**, hypothesis for the input claim);
  whether it is chosen depends on construct limits, nested-shape fits (`R`) and, for blocks, a
  rendered line count (**read**).
- rustfmt's layout procedure is not an Oppen/Wadler/Prettier document with exact equivalence:
  choices compare rendered multi-line alternatives, failure is control flow, attempts change
  context and leave effects that snippet runs read (**read**). The one-line fragment is expressible
  and is what a width pass computes.
- One skip is exact from the code alone: the chain's `new_rw` when `rw` is one line and fits, with a
  run-level guard (snippet runs, `rustfmt_version = 1.8.0`) because discarded attempts' effects reach
  macro-body output (**read**); its size between 0.75% and 3.03% is open.
- The shared sink is the largest lever exact by construction, ≤ 8.83–10.52%, at the cost of every
  rewrite signature (**derived**); its per-block model is testable on `write_list` alone.
- Open: per-event costs at each decision point (an instrumented build that times kept and discarded
  candidates separately would replace the uniform-cost estimates); the one-line fraction of the
  chain's 1,175 `rw`-kept decisions; whether slack intervals hold on the corpora; the width pass's
  real cost; `String` blocks at this head counted directly (DHAT without line tables cannot type
  inlined `with_capacity` sites); the scaled X9 classes, which assume repeats fell with
  `memoize::<format_expr>`'s inclusive cost.
