# 2026-10-15: The already-formatted fast path (theory)

Part of [../2026-10-15-extensions-theory.md](../2026-10-15-extensions-theory.md). Labels: measured, read, derived, hypothesis. No code was changed.

## Question

Whether a conformant formatter can decide that text is already rustfmt's output (`format(x) == x`)
with fewer instructions than computing `format(x)`, and what a verify mode costs. This is the open
question "Recognising canonical text" of
[2026-10-10-first-attempts-and-reuse.md](../2026-10-10-first-attempts-and-reuse.md). X12a
([2026-10-10-literals-and-canonical-text.md](../2026-10-10-literals-and-canonical-text.md)) bounded
a free, perfect recogniser of canonical top-level items at 42.9% of `format_source` cycles on the
registry and 21.9% on the fixtures, and found no class "where proving canonicality is cheaper than
formatting, beyond leaf tokens". This record bounds the cost of a recogniser that is not free.
Neighbouring records: [incremental-and-process.md](incremental-and-process.md) (item cache, `--check`
early exit, resident process), `decide-once-and-exact-reuse.md` (per-node one-line widths,
width-first decisions; written in parallel), `parser-and-substrate.md` (the parse floor),
`scanning-and-strings.md` (per-byte scanners).

## Method

Code read at `bdfb9d1` (`chloro-core` unchanged since); every code claim carries file:line under
`chloro-core/src/formatter/` unless another path is given. The proof of concept was read with
`git show c4d74ee:<path>`. Existing release binaries only; scratch directory
`$S = /tmp/claude-0/-home-user-chloro/9623d6dc-d933-5682-9d83-7762284f5c7f/scratchpad/theory-fastpath`.
Instruction counts are host-independent; no wall time was taken in this window.

```text
for f in sample49/**/*.rs: fmt_stdin < f | cmp - f; diff f <(fmt_stdin < f)      # is the sample formatted?
fmt_stdin < sample49/f > $S/fsample/f; then fmt_stdin on $S/fsample: 0 of 49 change   # formatted copy of the sample
valgrind --tool=callgrind bench -n 1 --root $S/fsample                              # 311,614,568 and 311,614,640 Ir
callgrind_annotate --inclusive=yes profile-final/callgrind.1.out, $S/cg-bench-*.out
python3 -I $S/prov.py  theory-incremental/fx | $S/fsample    # dump_tree: bytes by token class, nodes by kind and line-ness
python3 -I $S/slx.py   ...  # maximal single-line statements of each file into one `fn f() { .. }` per file ($S/sl-fx, 745 files)
fmt_stdin on $S/sl-fx (742 of 745 fixed points); valgrind --tool=callgrind bench -n 1 --root $S/sl-fx
python3 -I $S/tok.py, $S/mlshare.py                          # tokens per corpus; tokens under multi-line constructs
valgrind --tool=callgrind bench -n 1 --root <one of the five largest formatted fixtures>   # parse share per file
chloro --check --no-cache <registry>; xargs -P4 $S/eq.sh (fmt_stdin | cmp per file)       # canonical files on the registry
diff f <(fmt_stdin < f) on 400 non-canonical registry files (shuf with a fixed random source)
```

`theory-incremental/fx` is the 1238-fixture copy rewritten by `chloro --write --no-cache`;
`chloro --check --no-cache` on it exits 0 (measured). Total compute: about 6 minutes.

**The profiled sample is not already-formatted input** (measured). 31 of the 49 sample files change
under `fmt_stdin`; 361 of 27,423 lines are removed and 1,009 added. X12a found the fixtures formatted
under rust-analyzer's own `rustfmt.toml` (`use_small_heuristics=Max`), not rustfmt's defaults;
EXP-006 counted 702 of 1238 fixtures changed. The final profile (312,733,055 Ir, 650,213 bytes, 481.0
Ir/byte) is therefore of mostly-formatted input. The formatted copy of the same 49 files measures
311.6 M Ir for 659,580 bytes (472.4 Ir/byte, −1.8% per byte; run without `profile.sh`'s `env -i
setarch -R`, which moves counts by up to 0.1%, `benchmarks/profile.sh:64-67`). Both agree with M3:
chloro's cost does not depend on whether its input is formatted. Every share below that says
"formatted sample" uses the formatted copy.

## 1. What is copied and what is recomputed

**Provenance by code path (read).** Under the defaults chloro reproduces (`config.rs:476-520`:
`wrap_comments`, `normalize_comments`, `format_strings`, `format_macro_matchers` false,
`format_macro_bodies` true):

| output text | how it is produced | where |
|---|---|---|
| identifiers, paths, lifetimes | source slices through `snippet` / `node_text` | `context.rs:265`; X1: 4.45 M `snippet` calls, 174.8 MB, all source |
| number and char literals | source slice, then a fit check | `expr.rs:1288-1298` |
| string literals | source slice unchanged; continuation-escaped literals skip the fit check | `expr.rs:1301-1314` |
| comments | re-indented: continuation lines trimmed and re-prefixed with the indent, trailing whitespace trimmed | `comment.rs:228-235, 330-334, 410-436` |
| brace-delimited macro calls, unparsable macro calls | source slice, lines shifted to the new indent (`trim_left_preserve_layout`, `utils.rs:409`), or the slice as is | `macros.rs:146-176, 360-368` |
| `macro_rules!` matchers | source slice | `macros.rs:785-789` |
| `macro_rules!` bodies | formatted after renaming `$x` to identifiers (`format_macro_bodies` true) | `macros.rs:795-810` |
| `#[rustfmt::skip]` items | source slice | `visitor.rs:673-697, 702-705` |
| any statement or item whose rewrite returns `None` | source slice, trimmed | `visitor.rs:658-665` |
| keywords and punctuation | constant strings chosen by the rewrite code | e.g. `macros.rs:250-256` |
| whitespace, line breaks, indentation | layout decisions; blank lines from the source clamped to `[0, 1]` | `missed_spans.rs:112-139`, `shape.rs` |
| trailing commas, braces around arm and closure bodies, nested parens, `use` order and braces, merged derives, `extern "C"` | normalisations: tokens added, removed or reordered | `lists.rs:316` `write_list`, `matches.rs:372`, `closures.rs:87-120`, `expr.rs:1517-1530`, `imports.rs:465-545, 570`, `attr.rs:590-595` |

**Shares on formatted text (measured, `prov.py`; output equals input, so these are output shares).**

| class | fixtures, formatted (14,190,145 B) | formatted sample (659,580 B) |
|---|---|---|
| copied token text outside macro token trees: identifiers, literals, comments | 7,497,642 (52.8%) | 333,745 (50.6%) |
| of which string literals | 2,326,258 (16.4%) | 119,553 (18.1%) |
| of which comments | 1,046,960 (7.4%) | 70,454 (10.7%) |
| macro token trees (call arguments and `macro_rules!` bodies), all classes | 1,654,902 (11.7%) | 146,501 (22.2%) |
| keywords and punctuation outside token trees | 1,565,342 (11.0%) | 52,892 (8.0%) |
| whitespace outside token trees: containing a newline / inline | 3,079,545 (21.7%) / 373,403 (2.6%) | 114,519 (17.4%) / 11,551 (1.8%) |
| not classified (lines the parser does not print as tokens) | 19,311 (0.14%) | 372 (0.06%) |

- Non-trivia tokens: 1,879,881 on the formatted fixtures (0.132 per byte), 64,129 on the formatted
  sample (0.097 per byte); whitespace tokens 726,910 and 27,832 (measured). Between consecutive
  tokens there are about 1.88 M gaps on the fixtures, 61% of them empty (**derived**).
- Elements of the flat tree: 172,408 on the formatted sample (0.261 per byte) (**measured**).
- Lines over 100 columns: 2,554 of 497,115 on the formatted fixtures (0.51%), 89 of 28,120 on the
  formatted sample (**measured**); an upper bound on lines left as written by a failed rewrite
  (long strings and comments are counted too).

**Consequence.** On canonical input about three quarters of the bytes are token text that chloro
copies or emits from the token's kind; their content is fixed once the token sequence is fixed. A
verifier checks them by token identity, without a byte comparison, provided no normalisation adds,
removes or reorders a token (**derived** from the two tables). The decisions sit in the remaining
24.3% (whitespace, 0.73 M runs) and in the 1.15 M empty gaps, plus the normalisations; the
formatter, by contrast, rebuilds all 14.2 MB through candidate strings (X1: `Some` results total
50.2 MB at `9202b42`).

## 2. A decision procedure for canonicality

**Shape of rustfmt's decisions (read).** Most decisions try layouts in order and take the first
that passes a predicate. `definitive_tactic` (`lists.rs:278-313`) is width-only given the items'
widths and line-ness: vertical if any item has a line comment, horizontal if the total fits the
limit and no item is multi-line, else mixed or vertical. The width heuristics are fixed functions
of `max_width` (`config.rs:301-323`: `fn_call_width` 60, `attr_fn_like_width` 70,
`struct_lit_width` 18, `struct_variant_width` 35, `array_width` 60, `chain_width` 60,
`single_line_if_else_max_width` and `single_line_let_else_max_width` 50). The per-node one-line
width these predicates need is analysed in `decide-once-and-exact-reuse.md`; not re-derived here.

**Source-guided verification (design, hypothesis).** On canonical input the source already shows
which layout was chosen at each node. The verifier reads it instead of searching:

1. Shared gates as in the formatter: lex, parse, `rejected_by_rustc`, skip attributes, length
   (`formatting.rs:45-58`). Output: ACCEPT or UNKNOWN; UNKNOWN runs the formatter.
2. Token-sequence checks, no rendering: reject `((` nested parens (`expr.rs:1517-1530`), two
   adjacent `#[derive]` (`attr.rs:590-595`), an unsorted or unnormalised `use` group or list
   (`imports.rs:465-545`, `reorder.rs:65`), a leading `|` on an arm, a block arm body or closure
   body that rustfmt flattens (`matches.rs:372-405`, `closures.rs:87-120`), a trailing comma that
   disagrees with the list's line-ness, a missing ABI string, trailing whitespace, CRLF under
   `newline_style = Auto` mismatches.
3. Bottom-up: one-line width of every node, or "none" when it holds a comment, a multi-line string
   literal or a construct that is never one line.
4. Top-down: at each node take the source's layout L (one line if the span has no newline; else
   the family the newline positions show), check every alternative rustfmt tries before L is
   refuted, check that L rendered at the node's shape equals the source span (line breaks, indent
   of each line, separators), derive the children's shapes from L with the formatter's own `Shape`
   arithmetic, recurse. Abort at the first mismatch.

**Four classes of decision (read).** Whether step 4 needs a render depends on the class:

| class | examples (file:line) | what the verifier needs |
|---|---|---|
| A. width-only refutation of an earlier alternative | one-line call, array, struct literal, chain against `chain_width` (`chains.rs:628-638`), if-else (`expr.rs:1016-1022`), struct literal (`expr.rs:1753-1756`), list tactic (`lists.rs:278-313`), `where` on its own line (`items.rs:1216-1235`) | the one-line width from step 3 against the budget: no render |
| B. preserved from the source, clamped | blank lines (`missed_spans.rs:112-139`, `visitor.rs:373`, `lists.rs:790-801`), trailing comma inside macros (`expr.rs:1316-1328`), macro fallback style (`macros.rs:157-165`) | a range check on the source: no render |
| C. a rendered alternative absent from the source | chain last child overflowed against on its own line, by line count (`chains.rs:645-685`); closure forced block ≤ 7 lines (`closures.rs:405`); block single line if ≤ 3 lines (`expr.rs:686-697`); same-line RHS against next-line RHS by newline counts and first-line endings (`expr.rs:2059-2106, 2125-2146`); overflowed last argument with one newline (`overflow.rs:494-510`) | the chosen layout's line count is the source's (by induction the child already matches its span); the alternative not in the source has to be rendered — the formatter on that subtree |
| D. failure | a rewrite returning `None` copies the source (`visitor.rs:658-665`, `macros.rs:146-176`) | proving that every alternative fails; the verifier returns UNKNOWN |

Two found examples of class D on the formatted fixtures (measured): statements that are single line
in their file and change when moved into `fn f() { .. }` at indent 4 — 3 of 28,694 extracted
statements (`$S/sl-fx`). `hir-def/src/nameres/proc_macro.rs:93` (107 columns, left as written in
its match arm) and `ide-assists/src/handlers/generate_enum_projection_method.rs:165`, a 74-column
chain above `chain_width` that survives because the enclosing `acc.add_group(..)` call is copied
whole after its string argument fails to fit. Text inside a failed rewrite is canonical whatever it
contains, so a verifier cannot accept it from its shape alone.

**What still has to be computed, not rendered (read).** Comments: only the continuation-line
indent and trailing whitespace (`comment.rs:410-436`). String literals: untouched; their first,
widest-later and last line widths feed fits checks (X8b version 2 computes them from the token).
`use` order: one `version_sort` comparison per adjacent pair (`imports.rs:596-688`). Derives,
nested parens: token patterns. Macro calls: the arguments still have to be parsed to know whether
rustfmt formats them as a call (`macros.rs:268-283`); `macro_rules!` bodies are formatted through a
re-parse (`macros.rs:795-810`) and go to UNKNOWN (70,041 bytes of the fixtures, 0.49%; 1,098 bytes
of the sample; measured).

**Cost bound (formatted sample, 311.6 M Ir).**

| component | Ir | basis |
|---|---|---|
| parse, tree build, file-level `rejected_by_rustc` (`formatting::parse`, inclusive) | 102.5 M (32.9%) | measured |
| macro argument parsing (`parse_macro_arg`, inclusive) | 15.5 M (5.0%) | measured |
| `subtree_rejected_by_rustc` on macro arguments | 6.0 M (1.9%) | measured |
| one walk over 172,408 elements and 64,129 gaps at 10–40 Ir per element | 1.7–6.9 M | hypothesis on the per-element cost |
| floor (classes A and B only, no UNKNOWN) | 125.7–130.9 M = 0.40–0.42x | derived |
| class C renders, upper estimate: the formatter's cost on every subtree under a multi-line `let`, method chain, call, `match`, closure, macro call or struct literal | ≤ 50.7% of tokens (32,512 of 64,129, measured) × 206.6 M non-parse Ir = 104.7 M | derived; Ir proportional to tokens is a hypothesis |
| verifier on formatted input, range | 0.40x to 0.76x of today's Ir | derived |

- Multi-line `let` statements alone hold 16.7% of the formatted sample's tokens (measured); every
  one whose RHS sits on the `=` line needs the next-line render unless the RHS's one-line width
  fits the next-line budget, in which case the source is not canonical (`expr.rs:2068-2096`,
  **read**). Inclusive of all assignments, `rewrite_assign_rhs_expr` is 22.1% of the final
  profile (measured).
- The floor is the parse: no recogniser that needs the tree goes below 0.33x of the sample's Ir.
  Below it, only `parser-and-substrate.md`'s levers apply.
- On input that is not canonical the verifier adds its walk up to the first mismatch per unit,
  at most 1.7–6.9 M over the whole sample (≤ 2.2%), before the formatter runs (**derived**).

## 3. Verify mode in the current architecture

**(a) Format with early exit at unit granularity.** On formatted input every unit matches, so no
work is skipped: 0 (**derived**). On an edited file `--check` can stop at the first differing unit;
[incremental-and-process.md](incremental-and-process.md) §3 bounds this at ≤ 0.78 of a differing
file's time and lists its exactness conditions (`imports.rs:44-50` newline retraction,
`parallel.rs:168-172` whole-file fallback). For unchanged units the saving is the item cache's
(that record's §1; E15).

**(b) Copy-through of a single-line node inside the formatter.** Before rewriting a node at shape
`S`, test whether its source is one line, holds no comment, and is the text rustfmt's one-line
layout would produce; if so return the source slice. Exact when, and only when, all of these hold
(read):

| condition | rule and location |
|---|---|
| the one-line layout is the first rustfmt tries for this kind in this position | not for: `match` with arms (always vertical), a block with statements, `fn` bodies (`fn_single_line` false, `config.rs:492`), struct item fields (`items.rs:1729` passes no `one_line_width`, so the one-line budget is 0 at `items.rs:2737`), enum variant lists, trait and impl bodies, `where` on a `fn` (`items.rs:1216-1235`; one line only for impls and type aliases, `items.rs:1972, 2431`), an if-else outside expression position (`expr.rs:1016-1022`), a block expression unless `rewrite_single_line_block` applies (`expr.rs:577-594, 686-697`) |
| the one-line width fits `S` and every nested heuristic | `fn_call_width`, `chain_width` (`chains.rs:628-632`), `struct_lit_width` (`expr.rs:1753-1756`), `array_width`, `attr_fn_like_width`, if-else and let-else widths (`config.rs:301-323`, scaled when `max_width` > 100) |
| no normalisation fires | nested parens, trailing comma in a one-line list, `{ expr }` closure and arm bodies, `use` list order and sole-item braces, derives, leading arm `|`, ABI, empty block spacing; under non-default options also `use_field_init_shorthand`, `use_try_shorthand`, `force_explicit_abi=false`, `remove_nested_parens=false` |
| every gap equals the canonical one-line spacing | a table keyed by parent kind and token pair (binary against unary `-`, `&`, `*`; generics `<`; ranges without spaces; closures' `|`) — a one-line formatter written a second time |

- Line-ness of the formatted fixtures by kind (measured; count, share single-line): `PATH_EXPR`
  167,785, 100.0%; `METHOD_CALL_EXPR` 69,086, 77.4%; `CALL_EXPR` 35,816, 66.9%; `LITERAL` 31,622,
  63.6%; `EXPR_STMT` 29,418, 49.7%; `LET_STMT` 21,087, 66.9%; `MACRO_EXPR` 11,241, 66.8%;
  `CLOSURE_EXPR` 7,180, 59.5%; `RECORD_EXPR` 5,228, 15.0%; `IF_EXPR` 7,126, 1.4%; `MATCH_EXPR`
  4,802, 0.3%; `BLOCK_EXPR` 37,729, 1.7%. X9b's kinds with the largest first-attempt cost on the
  agreement corpus (`MACRO_EXPR`, `CALL_EXPR`, `LITERAL`, `ARRAY_EXPR`) are single-line in
  64–81% of instances here.
- Coverage by bytes (measured): lines that hold one whole single-line, comment-free statement,
  arm, field, attribute, `use` or other item: 20.05% of the formatted fixtures (`LET_STMT` 5.8%,
  `EXPR_STMT` 5.0%, `MATCH_ARM` 4.2%, macro calls 1.4%, attributes 1.3%, `use` 0.9%), 13.7% of the
  formatted sample (statements 6.1%). Maximal single-line comment-free statements and expressions,
  wherever they sit: 27.5% and 17.8%.
- Cost of formatting single-line statements (measured): 28,694 statements extracted from the
  formatted fixtures into 745 files (1,325,048 bytes, 325,882 tokens): 1,268.6 M Ir, parse 518.5 M,
  `format_text` 1,254.4 M, so 2,258 non-parse Ir per token, against 3,221 on the formatted sample
  (206.6 M / 64,129). Parse costs 1,591 and 1,598 Ir per token on the two corpora (measured): the
  per-token normalisation holds for the parser.
- **Bound** (derived; per-token rate carried over to the statements in place, hypothesis): the
  formatted sample's single-line statements hold 8,138 tokens → 8,138 × 2,258 = 18.4 M Ir = 5.9%
  of 311.6 M. Scaled by bytes to all whole-line candidates (90,517 / 40,375): 41.2 M = 13.2%; to
  all maximal single-line statements and expressions (117,409 / 40,375): 53.5 M = 17.2%. The
  check itself, one pass over the node's tokens with a table lookup per gap at 20–60 Ir per token
  (hypothesis), is 1–3% of the 2,258 Ir it replaces.
- The rule applies on any input, formatted or not, and on first runs; the shape comes from the
  parent rewrite, so the context-dependence shown by the 3 non-fixed extracted statements does not
  arise (**derived**: the rule only runs inside a rewrite with its real shape).

## 4. The proof of concept as evidence

- What it copied (X11a, measured on the fixtures): 55.8% of bytes by fallback (a `None` from an
  unsupported statement or expression propagates and the node is copied, `c4d74ee:
  chloro-core/src/formatter/node/block.rs:117-120`), 20.2% by construct policy (initialisers,
  parameters, `use` trees, closure bodies, arm patterns, macro calls, attributes), 4.2% leaf tokens
  and 2.1% comment text that rustfmt also copies. The shares are the same on perturbed input; no
  comparison of input with output exists in its sources (X11a, read). It laid out about a quarter
  of its input and two thirds of inter-token spaces survived perturbation (change-of-direction,
  Evidence 3: 64.9%).
- Where its instructions went (change-of-direction, Evidence 4, measured on the 49-file sample):
  299 M in total; `SourceFile::parse` 51.6% (≈ 154 M); `format_node` inclusive 144 M, of which
  73 M is `SyntaxText`'s `Display` — the copying itself, through a red-tree walk; its own
  formatting code 4.8 M (1.6%).
- Split (derived): of its 144 M formatting instructions, 73 M (51%) paid for copying canonical text
  and about 71 M for laying out the remaining quarter and the glue. The copy was not free on rowan:
  it cost more than its layout. On the flat tree a span copy is a `memcpy`; every `memcpy` in the
  final profile together is 9.2 M (2.95%, measured).
- Head against the proof of concept (derived): 312.7 M against 299 M Ir on the sample, 1.046x, with
  conform 1220/1220 against its 233/1220. Head spends 102.1 M on parsing (flat) against ≈ 154 M
  (rowan) and 210.6 M on formatting everything against 144 M for copying three quarters and laying
  out one quarter. Caveat: `benchmarks/profile.sh:10-12` records that the sample's original
  construction command is lost; the current construction gives 588.0 M at `884f9c9` against the
  595 M reported for `9202b42` on the earlier one.
- Extrapolation (hypothesis): 71 M for about 24% of the input scales to ≈ 296 M for all of it, above
  head's 210.6 M; the quarter it laid out is the code its dispatch supported (X11a: unsupported
  constructs fall back to copying), so the scaled figure is a lower estimate. Its instruction lead
  came from not laying out three quarters of the text, not from fewer instructions per laid-out
  byte.
- Recoverable part for a conformant formatter: the work head spends on text that is canonical and
  that a sound check accepts, at the check's cost — the verifier of §2 (0.40–0.76x) or the
  copy-through of §3 (5.9–17.2% of Ir). Copying without a check, as `c4d74ee` did, is not
  recoverable.

## 5. Real edits

**Per-file costs** (measured, callgrind of `bench -n 1` on each formatted file alone; M3 `stdin`
rows, one core, formatted input):

| file | bytes | top-level items | largest item, share of item bytes | Ir | parse Ir (P/T) | M3 ms |
|---|---|---|---|---|---|---|
| `ide-db/src/generated/lints.rs` | 639,771 | 11 | 45.0% | 251.1 M | 69.6 M (0.277) | 35.3 |
| `syntax/src/ast/generated/nodes.rs` | 358,382 | 1960 | 1.5% | 290.7 M | 133.1 M (0.458) | 47.7 |
| `hir/src/lib.rs` | 236,066 | 261 | 16.0% | 213.9 M | 70.2 M (0.328) | 42.8 |
| `ide/src/hover/tests.rs` | 210,967 | 261 | 3.1% | 52.5 M | 14.5 M (0.276) | 10.4 |
| `rust-analyzer/src/config.rs` | 169,196 | 116 | 25.5% | 85.6 M | 28.7 M (0.336) | 16.3 |

The parse share by time is below its share by instructions: `parse_floor flat` measured 21% of
`lints.rs`'s time and 37% of `nodes.rs`'s (incremental-and-process.md §1) against 27.7% and 45.8%
here.

**Model (derived; work proportional to item bytes is a hypothesis).** One unit `k` edited, share
`s_k` of the formatter's non-parse work, parse share `p`:

| scenario | cost / today | lints | nodes | hir lib | hover tests | config |
|---|---|---|---|---|---|---|
| today | 1 | 1 | 1 | 1 | 1 | 1 |
| item cache, all other units hit, median unit edited | ≈ p (+ hashing, gaps) | 0.28 | 0.46 | 0.33 | 0.28 | 0.34 |
| item cache, largest unit edited: p + (1 − p) s_max | | 0.60 | 0.47 | 0.44 | 0.30 | 0.51 |
| verifier only (no cache), edited unit not canonical: p + walk + (1 − p) s_k + R of the other units | ≥ the item-cache row | ≥ 0.28–0.60 | ≥ 0.46–0.47 | ≥ 0.33–0.44 | ≥ 0.28–0.30 | ≥ 0.34–0.51 |
| verifier, edited unit canonical (formatted by the editor) | p + walk + R | ≥ 0.28 | ≥ 0.46 | ≥ 0.33 | ≥ 0.28 | ≥ 0.34 |

- The item cache answers units seen before, canonical or not; the verifier answers units never
  seen, canonical only. Composed: `cost = P + hashing + gaps + Σ over missed units of
  min(format, verify-then-format)`. Both bottom out at the same `p`; they do not multiply
  (**derived**).
- Cases only the verifier reaches: a first run (fresh checkout, CI without a restored cache); a new
  chloro binary, which changes the cache key (`chloro/src/cli/cache.rs:237-320`, E12's stamp); an
  edited unit that is already canonical. Cases only the cache reaches: unchanged non-canonical units
  (`--check` on a tree that fails).
- Real input (measured): the registry in this container, 24,582 files and 381.9 MB as found (the
  protocol's 20,570 plus crates fetched since): 14,954 files (60.8%) are fixed points of chloro, but
  they hold 106.2 MB (27.8% of bytes; median size 2,523 bytes against 8,358 for the others). In 400
  non-canonical files chosen with a fixed random source, 26,046 of 285,132 lines (9.1%) change.
  X12a (5659 registry files, `d136f89`): 90.6% of top-level items canonical, carrying 42.9% of
  `format_source` cycles. A unit-granular verifier on the as-found registry saves at most
  42.9% × (1 − v) of `format_source`, where `v` is the verifier's cost relative to formatting a
  canonical item: 38.2% at v = 0.11, 15.4% at v = 0.64 (**derived**; v is the §2 verifier's non-parse
  part over the formatter's non-parse 206.6 M: floor (15.5 + 6.0 + 1.7–6.9) / 206.6 = 0.11–0.14,
  ceiling with class-C renders (that + 104.7) / 206.6 = 0.62–0.64; carrying the sample's ratio to
  the registry is a hypothesis).

## Ranked bounds

Bounds are on instructions unless marked; "today" is the final head (312.7 M on the sample as found,
311.6 M on its formatted copy).

| # | lever | addresses (scenario, Ir share) | bound with argument | exactness risk and proof obligation | cost and maintenance | smallest experiment; falsified if |
|---|---|---|---|---|---|---|
| F1 | copy-through of single-line nodes inside the formatter | any run, any input; formatter work in single-line statements and expressions | −5.9% (statements, measured rate × tokens) to −13.2% (whole-line candidates) to −17.2% (all maximal single-line nodes), less 1–3% of that for the check; §3 | medium-high: for each kind it accepts, prove one-line is rustfmt's first layout at that position, every nested width heuristic holds, no normalisation fires, spacing matches — under all 31 option values | spacing table, exception list and heuristics: a second one-line formatter beside `expr.rs`, `chains.rs`, `lists.rs`, `overflow.rs` (estimate 1–2k lines, hypothesis); every conformance fix to one-line output needs the twin change | shadow mode at `format_stmt` and `format_expr`: compute the rule's verdict and the rewrite, count accepts and disagreements on both corpora (raw and formatted copies), the 31 `config_matrix` values and the perturbation set below; falsified by any accept whose rewrite differs from the source, or by accepted-node Ir under 3% of the sample |
| V1 | source-guided verifier for `--check` and save on formatted input, first run | formatted input without cache; the 67% non-parse Ir | 0.40–0.42x if no class-C render is needed; ≤ 0.76x if every multi-line decision subtree needs one (§2) | high: soundness per decision class A–D; class D (failure) can only be UNKNOWN; source-dependent class B; all options | a second implementation of every layout decision of a 24,535-line formatter (`wc -l formatter/*.rs`), kept in lock-step with each conformance change; gated by the agreement harness on every merge | first measure R without building V1: a scratch patch counting, on the formatted corpora, the class-C sites whose chosen result equals the source span and whose alternative is not width-refutable, with their inclusive Ir; falsified (V1 not pursued) if that Ir exceeds 0.3 of `format_source` (V1 ≥ 0.7x). Then V1 in shadow: falsified by one ACCEPT where `format(x) != x` on any file of either corpus, option value or perturbation |
| V2 | V1 at unit granularity on as-found code | registry-like input | ≤ 42.9% × (1 − v) of `format_source`: 15.4–38.2% (§5) | as V1, plus unit boundaries (E1's units) | as V1 | as V1, reported per unit |
| C1 | item cache (E15; incremental-and-process.md L1) | repeated runs, unchanged units | floor p = 0.28–0.46 per large file; 0 on a first run | medium (that record §1) | that record | that record |
| C1+V1 | cache first, verifier on misses | first runs and canonical edited units | same floor p; adds V1's range for missed canonical units | sum of both | sum of both | as V1 on the cache-miss path |
| E1 | unit early exit for `--check` | failing trees | 0 on formatted input; ≤ 0.78 of a differing file (incremental-and-process.md L7) | medium (that record §3) | ~60 lines | that record |
| F0 | whole-file fixed-point cache (E6, landed) | warm runs on unchanged files | formatting removed for hits; 60.8% of registry files would hit after one observation | none beyond the binary key | landed | landed |

**Perturbation set for V1 and F1** (each applied at k sites per file, on copies of the formatted
fixtures and the formatted registry; the oracle is `format(x) == x` from the formatter):

| class | operation | rule exercised |
|---|---|---|
| spacing | double one space; delete a space; insert a space in an empty gap | spacing table; `CharClasses` paths |
| line structure | join two lines at a gap; split a line at a gap; indent a line ±1 and ±4 | layout family and indent checks |
| blank lines | insert a second blank line; delete one | `missed_spans.rs:112-139` clamp |
| width thresholds | lengthen or shorten an identifier so a line or a list crosses 100, 60, 70, 50, 35, 18 by ±1 | class A predicates, `config.rs:301-323` |
| trailing commas | add one to a one-line list; remove one from a vertical list | `write_list` separator tactic |
| normalisations | `((e))`; split a derive; swap two `use` items and two names in a `use` list; `use a::{b}`; wrap an arm body and a closure body in `{ }`; leading `|`; `extern fn` | `expr.rs:1517-1530`, `attr.rs:590-595`, `imports.rs:465-688`, `matches.rs:372`, `closures.rs:87-120` |
| comments | re-indent a block comment's continuation line; trailing space in a comment; a line comment inside a one-line list | `comment.rs:410-436`, `lists.rs:289-296` |
| macros | change spacing inside a call-like macro, a brace macro, an unparsable macro, a `macro_rules!` body | `macros.rs:146-176, 268-368, 795-810` |
| failure regions | lengthen a string literal until its statement no longer fits | `visitor.rs:658-665` (class D) |
| class C sites | lengthen a chain's last call, a `let` RHS, a closure body by one line around the 3-, 5- and 7-line thresholds | `chains.rs:657-675`, `expr.rs:693, 2142`, `closures.rs:405` |
| file level | CRLF; a byte-order mark; an inner `#![rustfmt::skip]` | `formatter.rs:103-114`, `visitor.rs:702-705` |

Gates (protocol §1, §4): a false ACCEPT is a conformance failure whatever the speed; a false
UNKNOWN costs speed only and is reported as a rate. F1 changes the formatter's path and runs gates
1–8; V1 adds no output path for `--write` (UNKNOWN formats) and is judged on the agreement counts
and on `Ir` of the formatted sample.

## What this settles and what it leaves open

- chloro's cost does not depend on whether its input is formatted: 481.0 Ir/byte on the sample as
  found and 472.4 on its formatted copy (**measured**); the profiled sample is not formatted input
  (31 of 49 files change, **measured**), so the final profile describes mostly-formatted code.
- On formatted text, 75.5% of bytes are token text whose content the token sequence fixes (copied
  slices 52.8%, keywords and punctuation 11.0%, macro token trees 11.7% in which chloro both copies
  and formats); layout lives in 24.3% whitespace bytes and 1.88 M gaps (**measured**).
- Any recogniser that needs the syntax tree has the parse as floor: 32.9% of the formatted sample's
  Ir, 27.6–45.8% of the five largest files' (**measured**); a verifier adds macro-argument parsing
  and its walk, 0.40–0.42x in all (**derived**), and its ceiling is set by decisions that compare a
  rendered alternative absent from the source (class C), whose cost is not measured: ≤ 0.76x
  (**derived** upper estimate).
- Text inside a rewrite that rustfmt gave up on is canonical whatever it contains (two examples on
  the fixtures, **measured**); a verifier cannot accept it without proving the failure, so such
  regions always format.
- The proof of concept's lead was not doing work: its copying cost it 73 M of 144 M formatting Ir
  on rowan, and head now runs 1.046x its instructions while conforming (**derived**); copying
  without a check is not available to a conformant formatter.
- Copy-through of single-line statements bounds at 5.9% of Ir on the sample (**derived** from a
  measured rate) and at 13.2–17.2% with all single-line nodes (hypothesis on per-token rates).
- Open: R, the class-C render cost on formatted corpora (the first V1 experiment); the per-element
  cost of a verifier walk on the flat tree; whether the registry's per-token rates match the
  fixtures'; the hit rate of either lever on real edit histories; wall time for any row (no wall
  measurement was taken in this window).
