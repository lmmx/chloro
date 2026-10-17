# 2026-10-15: Scanning, widths and string assembly (theory)

Part of [../2026-10-15-extensions-theory.md](../2026-10-15-extensions-theory.md). Labels: measured, read, derived, hypothesis. No code was changed.

## Question

How much of the final head's 312,733,055 instructions (the 49-file sample, `benchmarks/profile.sh`,
code of `bd9c768` (was `bdfb9d1`)) is per-byte and per-string work — scanning source text for comments and
separators, measuring built strings, assembling strings — and how much of it each further
lever can remove, with the exactness argument, the cost, and the experiment that would refute
the bound. 1% of the total is 3,127,331 instructions throughout.

## Method

- Inputs (**measured**, earlier): the final-head profile in the scratch directory
  `profile-final/` (`callgrind.1.out`, 312,733,055 Ir; `annotate-all.txt`), X1 and X8
  ([../2026-10-10-formatter-work-measurements.md](../2026-10-10-formatter-work-measurements.md)),
  E3 ([../2026-10-14-autoresearch/experiments/EXP-003-hot-paths.md](../2026-10-14-autoresearch/experiments/EXP-003-hot-paths.md)),
  the DHAT categories of [../2026-10-14-autoresearch/profile-flat.md](../2026-10-14-autoresearch/profile-flat.md) (at `fe34bd2` (was `8d418ee`)).
- New runs on the existing release binaries (`target/release/examples/bench`, sha256
  `fdfce77d2f43c19a`, the binary of `profile-final/`; `examples/fmt_stdin`), scratch
  `$S = .../scratchpad/theory-scanning/`, load average 3.5–4.8 (other builds running; no wall
  time is used below):

```text
callgrind_annotate --inclusive=yes profile-final/callgrind.1.out            > $S/inclusive.txt
callgrind_annotate --tree=caller --inclusive=yes profile-final/callgrind.1.out > $S/tree-caller.txt
python3 $S/cg.py profile-final/callgrind.1.out $S/cg.pkl ; python3 $S/q.py {f,callers,callees} <regex>   # exclusive Ir, calls, caller/callee edges
setarch -R valgrind --tool=callgrind --dump-instr=yes bench -n 1 --root /tmp/chloro-bench/sample49   # 312,838,147 Ir
python3 $S/instr.py cg_instr.out <fn> ; objdump -d bench   # per-instruction execution counts of skip_plain, skip_comment,
                                                           # find_uncommented, CharSearcher::next_match, line_of, push_rewrite_inner
setarch -R valgrind --tool=dhat bench -n 1 --root /tmp/chloro-bench/sample49   # 237,991 blocks, 123,468,129 bytes
python3 $S/dhatsum3.py dhat2.json   # blocks / bytes requested / bytes written by innermost formatter frame
python3 $S/bytestats.py samplelist.txt   # non-ASCII share, run lengths between scanner stop bytes (sample, 1238 fixtures)
for f in sample: fmt_stdin < f | wc -c   # sample output: 659,580 bytes (input 650,213)
```

- The `--dump-instr` run differs from `profile.sh`'s count by 105,092 Ir (0.03%, different
  environment); every call count used from it equals the one in `callgrind.1.out` (`skip_plain`
  56,321, `find_uncommented` 41,643, `next_match` 58,445). Per-instruction counts give the loop
  iteration counts below; Ir of a loop is iterations × the loop's instruction count.
- `callgrind_annotate --auto=yes` was not used: the binary has no line tables, so costs cannot be
  attributed to source lines; attribution below is by function and caller edge, and by
  instruction address where the disassembly makes the loop evident. Total compute: under 3 minutes.

## 1. Where the per-byte and per-string work is (measured)

Exclusive and inclusive Ir from `callgrind.1.out`; Ir/call = inclusive / calls; per-byte
figures from the per-instruction counts unless marked.

### Scanning source text

| function | exclusive | inclusive | calls | Ir/call | per byte or char |
|---|---|---|---|---|---|
| `find_uncommented` (comment.rs:445-485) | 3,506,124 | 10,218,591 | 41,643 | 245 | 1.07 loop iterations per call; 39,341 calls (94.5%) return at the first `next()` after the skip |
| `CharClasses::skip_plain` (comment.rs:684-728) | 7,125,378 | 7,125,378 | 56,321 | 127 | ≈ 10.4 Ir/byte over ≈ 0.68 MB (derived below); 3.75 Ir/byte in the 8-byte loop |
| `CharClasses::next` (comment.rs:797-936) | 4,471,015 | 4,605,592 | 96,962 | 47 | 46 Ir per character stepped |
| `CharClasses::skip_comment` (comment.rs:730-748) | 1,720,543 | 1,720,543 | 52,948 | 32 | 50,194 calls (94.8%) return at the state test |
| `recover_comment_removed` (comment.rs:1199-1213) | 2,666,160 | 6,717,429 | 24,235 | 277 | slow path (`UngroupedCommentCodeSlices`) 3,784 calls, 3,388,367 |
| list gaps: `get_comment_end` (lists.rs:731-776) | 897,211 | 5,562,183 | 15,218 | 366 | of it `find_uncommented` 3,434,403 |
| list gaps: `extract_post_comment` (lists.rs:684-729) | 912,837 | 3,333,691 | 15,218 | 219 | `lines().last()` 1,383,894 (9,775 calls) |
| list gaps: `extract_pre_comment` (lists.rs:652-682) | 702,364 | 1,924,826 | 20,052 | 96 | `trim` 1,220,202 |
| list gaps: `has_extra_newline` (lists.rs:780-802) | 1,393,487 | 1,948,912 | 15,218 | 128 | |
| `format_missing_indent` (missed_spans.rs:57-77) | 800,760 | 5,458,780 | 5,155 | 1,059 | `process_comment` 1.54 M (253 calls) |
| `combine_strs_with_missing_comments` (comment.rs:131-217) | 674,111 | 2,458,980 | 5,999 | 410 | |
| `rewrite_missing_comment` (comment.rs:356-369) | 567,693 | 1,249,889 | 9,011 | 139 | |
| `CommentCodeSlices::next` | 664,249 | 2,160,028 | 6,059 | 357 | |
| `chains::trim_tries` (chains.rs:820-840) | 574,966 | 1,601,317 | 6,102 | 262 | `CharClasses::next` per character |
| `RunState::line_of` (context.rs:159-177) | 1,258,826 | 1,338,796 | 776 | 1,725 | 230,118 bytes at 5 Ir/byte |
| `span_ends_with_comma` (expr.rs:1462) | 276,135 | 439,872 | 640 | 687 | E3's lever 6b took it from 8.78 M |

### Scanning built text

| function | exclusive | inclusive | calls | Ir/call | note |
|---|---|---|---|---|---|
| `filtered_str_fits` (utils.rs:271-297) | 3,123,995 | 8,227,679 | 5,563 | 1,479 | callers: `rewrite_literal` 4,667,745 (1,964 calls), `Chain::rewrite` 2,361,549 (3,124), `rewrite_assign_rhs_expr` 1,112,479 (226) |
| `last_line_width` (utils.rs:77-84) | 2,414,185 | 2,414,185 | 37,493 | 64 | |
| `first_line_width` (utils.rs:65-73) | 1,342,753 | 1,342,753 | 21,876 | 61 | |
| `multi_line_or_unicode_str_width` (utils.rs:148-180) | 1,680,502 | 1,680,502 | 2,745 | 612 | 1,576,685 of it from `total_item_width` |
| `total_item_width` (lists.rs:902-906) | 1,615,069 | 3,191,754 | 23,212 | 138 | `is_ascii_single_line` inlined |
| `ListItem::is_multiline` (lists.rs:227-231) | 1,627,608 | 1,627,608 | 28,461 | 57 | three `contains('\n')` |
| `trimmed_last_line_width` (utils.rs:97-102) | 143,704 | 547,216 | 1,933 | 283 | |
| `write_list` (lists.rs:316-598) | 6,522,802 | 11,045,104 | 12,932 | 854 | `lines().last()` 845,458; `contains("::")` 837,668 |
| `count_newlines`-shaped loops (utils.rs:257-259 inlined) | ≈ 7,878,692 | | 414,668 iterations | | 4.75 Ir/byte over ≈ 1.66 MB (below) |
| `Indent::to_string_inner` (shape.rs:80-102) | 1,536,285 | 1,537,500 | 45,976 | 33 | returns a slice of `INDENT_BUFFER` when spaces only |

The newline-count loop is the auto-vectorised `bytes().filter(|&b| b == b'\n').count()`: two
2-byte loads per iteration, widened to 64-bit lanes because the count is a `usize`, 19
instructions per 4 bytes (**measured**, `push_rewrite_inner` at 0x58c40–0x58c8e, 228,095
iterations). Its iterations by host function (**measured**; bytes = 4 × iterations, Ir = 19 ×
iterations, **derived**):

| host | iterations | bytes | Ir | source of the count |
|---|---|---|---|---|
| `push_rewrite_inner` | 228,186 | 912,744 | 4,335,534 | `push_str`'s `line_number` (visitor.rs:653-656) |
| `overflow::Context::rewrite` | 87,340 | 349,360 | 1,659,460 | `count_newlines(overflowed) == 1` (overflow.rs:502) |
| `prefer_next_line` | 45,774 | 183,096 | 869,706 | `count_newlines(orig_rhs) > count_newlines(next_line_rhs) + 1` (expr.rs:2142), 298 calls |
| `process_comment`, `visit_fn`, `format_missing_indent` (+ closure), `has_extra_newline`, `close_block`, others | 53,368 | 213,472 | 1,013,992 | `push_str` and missed-span counts |
| total | 414,668 | 1,658,672 | 7,878,692 | 2.52% of the run |

### `core::str` searchers, by chloro caller (`--tree=caller` edges, inclusive edge Ir)

| searcher (exclusive, calls) | callers |
|---|---|
| `CharSearcher::next_match` 7,292,092, 58,445 | `filtered_str_fits` 3,514,027 (20,206: `contains('\n')` and `lines()`), `has_extra_newline` 555,425, `identify_comment` 706,345 (two entries), `Chain::rewrite` 607,412 (both levels), `rewrite_missing_comment` 477,583 (`find('/')`), `get_comment_end` 457,663, `rewrite_block_inner` 434,606, `Lines::fold` 273,397, `overflow::Context::rewrite` 215,549, `prefer_next_line` 165,474 |
| `trim_matches::<is_whitespace>` 4,370,777, 88,150 | `extract_pre_comment` 1,220,202, `extract_post_comment` 916,378, `format_missing_indent` 855,741, `rewrite_chain_from` 350,519, `trim_tries` 253,878, `close_block` 239,752, `rewrite_missing_comment` 204,613, `trimmed_last_line_width` 106,712 |
| `memrchr` 2,843,815, 36,461 (via `next_match_back`, 2,018,547, 35,451) | `SplitInternal::next_back_inclusive` (`lines().last()`, `rsplit`) 2,328,335: `extract_post_comment` 1,383,894, `write_list` 845,458, `rewrite_fn_base` 357,753; direct: `rewrite_call` 1,108,658 (inlined `rewrite_with_parens`, expr.rs:1331-1346), `rewrite_path_segments` 557,986, `trimmed_last_line_width` 296,800, `rewrite_match_body` 213,738, `visit_block` 320,828 |
| `<&str as Pattern>::is_contained_in` 1,338,482, 8,798 | `write_list` 837,668 (`contains("::")`, lists.rs:593), `format_source` 401,140 (49 whole-file `contains("\r\n")`, formatting.rs:186), `rewrite_fn_base` 153,060 |
| `StrSearcher::new` 937,906, 7,095 | `get_comment_end`'s `find("/*")` 772,906 (5,443, 142 Ir each), `UseSegment::cmp` 231,744 |

`CharSearcher::next_match` scans ≈ 1.31 MB in its 58,445 calls (**measured**: 59,404 sixteen-byte
steps = 950,464 bytes, and byte loops of 66,306, 73,224 and 220,242 bytes): 5.6 Ir/byte and 22
bytes per call (**derived**). Its byte loops cost 1,700,613 Ir and its 16-byte loop ≈ 0.94 M; the
other ≈ 4.65 M is per-call work, ≈ 80 Ir per call (**derived**). The searchers are dominated by
per-call cost on short haystacks, not by bytes.

### What the scanners are given (derived)

- `skip_plain` in the `Normal` state runs 50,207 times: 100,257 eight-byte steps, of which 39,129
  find a stop byte; 56,122 bytes go through the byte loop for slices with fewer than eight bytes
  left; 63,197 stop evaluations, of which 17,819 are an `r` not before `#` or `"` and continue
  (**measured**, skip_plain at 0x68870–0x68966). Bytes skipped ≈ 61,128 × 8 + 39,129 × 3.5 +
  56,122 ≈ 0.68 MB, ≈ 13 bytes per call; 2.0 eight-byte steps per call.
- X1 on the 1238 fixtures (at `9202b42`, before E3): text handed to `CharClasses` 117.7 M bytes,
  of which 21.8 MB of source and 16.0 MB of built text read; `unicode_str_width` over 58.8 MB of
  built strings, 4.1x the output. Scaled to the sample's share of the output (659,580 /
  14.2 MB = 4.6%), ≈ 1.0 MB read by `CharClasses` and ≈ 2.7 MB measured by width (**hypothesis**:
  same mix as the fixtures; E3 changed the scans, not the texts handed over, except
  `span_ends_with_comma` and `find_uncommented`'s early return).
- The formatter's scanning of source is short-range: `find_uncommented` finds its needle on the
  first step 94.5% of the time and `next_match` averages 22 bytes; the cost is per call
  (prologue, state setup, dispatch), not per byte.

## 2. Carrying width and line facts with the string

### The facts and their consumers (read)

A `Rewritten { text, first_w, last_w, mid_max_w, total_w, newlines, flags }` with `flags =
{ first byte ASCII and not '\n', has '/', has '\r', all ASCII }` answers every consumer below
from fields; `mid_max_w` is the widest line strictly between the first and the last:

| consumer | answer from the fields |
|---|---|
| `first_line_width` (utils.rs:65-73), `last_line_width` (utils.rs:77-84) | `first_w`, `last_w` |
| `unicode_str_width` (utils.rs:137-143) | `total_w` |
| `count_newlines` (utils.rs:257-259), `is_single_line` (utils.rs:44-46), `ListItem::is_multiline` (lists.rs:227-231) | `newlines`, `newlines == 0` |
| `filtered_str_fits` (utils.rs:271-297) without `/` | `first_w ≤ shape.width` and, if `newlines > 0`, `max(mid_max_w, last_w) ≤ max_width` and `last_w ≤ used + width`; with `/`, today's code (`filter_normal_code` drops comment lines) |
| `last_line_used_width` (utils.rs:88-94) | from `newlines`, `last_w` |
| `trimmed_last_line_width` (utils.rs:97-102) | not a function of these fields (trims the last line); today's code |

### Composition (read: unicode-width 0.1.14, tables.rs:203-222, 401-411)

- `str_width` folds the characters right to left, each character's width depending on the
  `WidthInfo` of the characters to its right; an ASCII character other than `\n` returns
  `WidthInfo::DEFAULT`, `\n` returns `LINE_FEED`, and a `\r` before `\n` counts 0
  (tables.rs:216-223). A character's width never depends on characters to its left.
- So for `a ++ b` with `b` empty or starting with an ASCII byte other than `\n`, the widths of
  `a`'s characters are those `a` has alone, and: `newlines = a.n + b.n`; `total_w = a.total +
  b.total`; `first_w = a.first` if `a.n > 0`, else `a.total + b.first`; `last_w = b.last` if
  `b.n > 0`, else `a.last + b.total`; `mid_max_w` is the maximum of `a.mid`, `b.mid`, and the
  joined line `a.last + b.first` when it is interior. With `b` starting `\n`, the same holds with
  `total_w` reduced by one if `a` ends with `\r`, and the per-line facts unchanged (the consumers
  split at `\n` first, utils.rs:66-67). With `b` starting with a non-ASCII byte, the line holding
  the join is re-measured (the variation selectors U+FE0E/U+FE0F and emoji modifiers change the
  width of the character before them, tables.rs:204-214, 225-232).
- Substrings: consumers that measure a slice of a built string (`write_list`'s `item_last_line`,
  lists.rs:366-371; `trimmed_last_line_width`; `rsplit('\n')` in `last_line_width`'s non-ASCII
  branch) need the text; for them the fields bound or skip the scan only.
- Exactness: the fields are functions of the text, computed by the same width function; a debug
  build that recomputes every queried field from the text and compares makes divergence a test
  failure on both corpora. Non-ASCII lines are 0.30% of the fixtures' lines (§3), so the
  re-measure branch is rare (**measured**, **derived**).

### Removable share (derived)

The built-text measurement at the final head, without double counting (Ir, **measured** rows of
§1): `filtered_str_fits` 8,227,679 (includes its own `first`/`last_line_width` calls, 528,729);
`last_line_width` 2,253,301 and `first_line_width` 974,908 outside it; `multi_line_or_unicode_str_width`
1,640,229 outside it; `total_item_width` exclusive 1,615,069; `is_multiline` 1,627,608;
`trimmed_last_line_width` 547,216; `write_list`'s `lines().last()` 845,458; newline loops on
built text 2,529,166 (`overflow`, `prefer_next_line`) and in `push_str` ≈ 4.95 M. Sum ≈ 25.2 M,
8.06%. Levers S1 and S2 (§7) take the `filtered_str_fits` and newline-count parts without a type
change; the rest, 9.51 M (3.04%), needs the carried facts. Composing costs O(1) per push: with
the 215,844 direct `memcpy` calls as the count of pushes and ≈ 10 Ir per composition
(**hypothesis**), ≈ 2.2 M, leaving ≤ 7.3 M (2.3%) removable.

### Cost (read)

191 functions return `Option<String>` across 19 files (expr.rs 40, items.rs 39, types.rs 24,
chains.rs 12, macros.rs 11, others ≤ 8); there is no `RewriteResult` (rustfmt's) in chloro; the
`Rewrite` trait (context.rs:181-183) has 38 impls; 379 `push_str`, 154 `format!`, 105
`contains('\n')`, 77 `last_line_width`, 24 `first_line_width`, 22 `unicode_str_width`, 16
`count_newlines` call sites. An incremental path: (1) `Rewritten` derefs to `str` and caches the
fields lazily on first query (removes repeat queries of one string; code that only reads keeps
compiling); (2) a builder whose `push(&Rewritten)` composes and whose `push_str(&str)` scans only
the pushed literal (removes re-measurement of concatenations); (3) the `Rewrite` signature. The
sink of [decide-once-and-exact-reuse.md](decide-once-and-exact-reuse.md) (written in parallel)
writes into a shared buffer instead of returning strings; if it lands, these fields belong to
the sink's cursor and this lever is part of it, not additive to it.

## 3. ASCII and SIMD fast paths

### Non-ASCII share (measured, `bytestats.py`)

| corpus | bytes | non-ASCII bytes | lines | lines with a non-ASCII byte |
|---|---|---|---|---|
| sample (49 files) | 650,213 | 221 (0.034%) | 27,472 | 50 (0.18%) |
| fixtures (1238 files) | 13,901,559 | 5,409 (0.039%) | 475,339 | 1,445 (0.30%) |

### Widths: what E3 did and what remains

- E3 lever 8 (**read**, EXP-003): `19db4fe` (was `0f5f235`) tests eight bytes per step for ASCII without `\n`
  and returns the length (−2.75% of `fe34bd2` (was `8d418ee`) on replay); `35e1475` (was `4b5bb3a`) finds the first/last `\n` or
  non-ASCII byte eight bytes per step (−2.23%); `bfe5af3` (was `a383516`) counts `\n` per word on the
  out-of-line path (−0.53%). `unicode_str_width`'s out-of-line part is 26,511 Ir at the head.
- What remains is per-call and repeat cost, not per-byte cost: `last_line_width` 64 Ir and
  `first_line_width` 61 Ir per call on strings averaging 17 bytes (E3, at its base: 120,914
  width calls, 17 bytes average). A 16-byte step saves at most one 8-byte word test (≈ 7 Ir) per call on
  17-byte strings: ≤ 120,914 × 7 ≈ 0.85 M (0.27%, **derived**, upper bound); it needs an `unsafe`
  unaligned load (`_mm_loadu_si128`) under protocol gate 4. The larger remaining width cost is the
  repeat measurement of §2.

### `skip_plain` and `CharClasses::next`

- The state machine (comment.rs:797-936) costs 46 Ir per character stepped; `skip_plain` costs
  3.75 Ir/byte in its 8-byte loop (30 instructions per step, **measured**) plus ≈ 40 Ir per call
  and ≈ 10 Ir per byte in the byte loop that handles the last < 8 bytes of a slice.
- Why E3's per-byte lookahead lost (+783,573): before `259abb0` (was `16e2e3d`) every byte went through the byte
  loop, and the lookahead (`bytes.get(pos + 1)` and its tests) ran on every byte; the stop bytes
  are one in 20.4 bytes of fixture source (mean run between `r " ' /`, **measured**), so the
  lookahead added work to ≈ 95% of bytes to save work on ≈ 5%. `259abb0` (was `16e2e3d`) computes it at the stop
  bytes only.
- Run lengths (**measured**, source text; the scanned spans are slices of it, **hypothesis**
  that the distribution carries over): between the current stop set `r " ' /`, mean 20.4, median
  11, 40% of runs ≥ 16 bytes, 20% ≥ 32 (62% of bytes lie in runs ≥ 32). Of the 451,861 `r` bytes
  in the fixtures, 17,015 (3.8%) precede `#` or `"`; with the stop set `" ' / #` and a one-byte
  lookback for `r` at `"` and `#`, the mean run is 53.5 and the stops fall from 649,077 to
  250,584 (−61%). At run time 17,819 of 63,197 stop evaluations (28%) are a non-opening `r`
  (**measured**).
- Break-even of a 16-byte SSE2 step (**derived**): 5 targets need 1 load, 5 `pcmpeqb`, 4 `por`,
  1 `pmovmskb`, 2 test/branch, 3 loop instructions ≈ 16 Ir per 16 bytes (1.0 Ir/byte) against 60
  Ir per 16 bytes now; constant setup is comparable to today's (3 `movdqa` + 4 `movabs`, measured
  at 0x6881c–0x6885c). With one overlapping final load per call replacing the byte loop, a call
  over L bytes costs ≈ 16 ⌈L/16⌉ + 16 against ≈ 30 ⌈L/8⌉ + 10 · (L mod 8, when the slice ends):
  the SSE2 form is not slower for any L ≥ 1, and for the measured ≈ 13 bytes per call saves
  ≈ 30–45 Ir. Bound: the 8-byte loop (100,257 × 30 = 3.01 M) and byte loop (≈ 0.56 M) become
  ≈ 0.68 MB × 1.0 + 50,207 × 16 ≈ 1.48 M: ≤ 2.1 M from `skip_plain`, ≤ 0.2 M from `skip_comment`
  (16,853 steps × 21 Ir = 0.35 M now). A 32-byte AVX2 step gains nothing further at 13 bytes per
  call and needs runtime feature detection.
- What would make it pay more: longer scans. The spans are short because callers stop early
  (§1); SIMD changes the per-byte cost of a scan that is already ≈ 40% per-call overhead.

### Newline counts and the line table

- E3 rejected `filter().count()` of `\n` in `unicode_str_width` (+4,474,566) because the vector
  prologue dominates 17-byte strings, and rejected the line table through `match_indices('\n')`
  (+518,973) against a plain loop (**read**, EXP-003 "Rejected"). The second is consistent with
  this profile's `CharSearcher` cost: ≈ 80 Ir fixed per call against 25-byte lines (230,118
  bytes, 9,155 line starts in `line_of`, **measured**).
- What changes the first: the strings `push_str` counts average 222 bytes (912,744 bytes in
  4,114 calls, **derived**), where the existing vector form runs at 4.75 Ir/byte because the
  `usize` accumulator limits it to 4 bytes per step. A byte accumulator (SSE2 `psubb` of the
  compare mask, flushed by `psadbw` every 255 steps, ≈ 0.3 Ir/byte) or the word count of
  `bfe5af3` (was `a383516`) (≈ 1.1 Ir/byte) is a different code shape from the one rejected.
- What changes the second: an inline 8-byte search (`find_any_byte`, comment.rs:770-792) costs
  ≈ 10 Ir per 8 bytes and ≈ 6 per hit: 230,118 / 8 × 10 + 9,155 × 6 ≈ 0.34 M against 1.26 M
  (**derived**).

## 4. Comment handling

### What each function scans (read, measured)

| function | scans | per node / item | measured |
|---|---|---|---|
| `recover_comment_removed` | `snippet != new` (`memcmp`), then `contains('/')` on both, then both texts' comments (comment.rs:1199-1213, 1248-1280) | once per expression and statement rewrite (`format_expr_uncached` 21,329, `format_stmt` 2,886 calls) | 24,235 calls, 277 Ir/call; slow path 3,784 calls |
| `combine_strs_with_missing_comments` | `rewrite_missing_comment`: `snippet(span).trim().find('/')` (comment.rs:356-369) | per attribute/prefix join | 5,999 calls; 9,011 `rewrite_missing_comment` calls |
| `find_uncommented` | `CharClasses` from the start of a span to the needle | `span_after`/`span_before` and list separators | 41,643 calls, 94.5% one step |
| list gaps | `trim`, `find("/*")`, `find('/')`, `find('\n')`, `find_uncommented(sep)`, `lines().last()`, `count_newlines` (lists.rs:652-802) | per list item, twice per gap | 15,218 items, 839 Ir per item for the four functions |
| `format_missing_indent` / missed spans | `trim`, `CommentCodeSlices`, `count_newlines` (missed_spans.rs:57-120) | per gap between items and statements | 5,155 calls, 1,059 Ir/call |

- Comments are rare where they are looked for: of 15,218 list items, `extract_post_comment` builds
  3 comment strings and `extract_pre_comment` 32 of 20,052 calls (their `malloc` calls,
  **measured**); `get_comment_end` never calls `find_comment_end` (no block comment found in any
  post-snippet). The sample has ≈ 2,240 comment openers (`grep -o '//\|/\*'`, **measured**,
  including any inside strings).

### A per-file comment index on the flat array

- X8 (**read**) answered `contains_comment` and `find_uncommented` from a token index built
  during parsing: identical answers, +3% instructions — index building 5.1 M in the rowan
  build, and per query two binary searches over all token starts plus a thread-local access,
  21.3 M against 10.6 M of scanning; the scans it replaced end within a few bytes.
- On the flat array the comment tokens are already `Elem`s, so the index is a filter at tree
  build: a `Vec<u32>` of ≈ 2,240 comment starts, one compare per token (≈ 0.3 M for the sample's
  ≈ 150,000 tokens, **hypothesis**), and a query is one `partition_point` over 2,240 entries
  (≈ 11 steps, ≈ 40–50 Ir, **derived**). That removes X8's build cost and its all-token search.
- It does not remove the per-call cost that dominates `find_uncommented` (§1): the needle is
  found on the first step, and lever S4 (§7) takes that overhead with a byte test and no index.
  The index's own margin over S4 is the scan to the needle, ≤ 20–30 Ir per call × 41,643 ≤ 1.2 M
  (**derived**), at the price of a span-based API at every `find_uncommented` call site (they
  pass `&str` today).
- `recover_comment_removed`: the index can answer the source side (`snippet` has no comment) in
  O(log n), but the built side (`new`) is not in the index and is scanned anyway; the source-side
  `contains('/')` it replaces costs about what the query costs at ≈ 85 bytes per snippet (X1:
  64.8 MB in 764,312 calls). The slow path's source half (≈ half of 3,388,367, **hypothesis**) is
  what an index could take.
- Bound for the index: ≤ 1.2 M + 1.7 M + ≈ 0.5 M (`rewrite_missing_comment`'s `find('/')` and
  `trim`) − 0.3 M build ≈ 3.1 M (1.0%) before the API cost, and ≤ 1.9 M (0.6%) after S3 and S4
  take the list-gap and `find_uncommented` overhead (**derived**). The flat array changes the
  economics of building the index, not of the queries.

## 5. List and string assembly

### Allocation sites at the final head (DHAT, measured)

Total 237,991 blocks, 123,468,129 bytes requested, 44,071,009 bytes written (fe34bd2 (was 8d418ee):
278,743 blocks); 186,688 blocks and 94,958,536 bytes are attributed to formatter frames, 51,301
blocks to the parser and `cst` (`dhatsum3.py`; DHAT attributes a growth `realloc` to the frame
that grew the block). Allocator Ir per block: glibc family 47,003,376 / 237,991 = 197
(**derived**).

| innermost formatter frame | blocks | bytes requested | bytes written | site |
|---|---|---|---|---|
| `overflow::Context::rewrite` | 23,122 | 4,501,760 | 2,632,157 | `wrap_items` (overflow.rs:644), `rewrite_items` `collect` (overflow.rs:591) |
| `chains::rewrite_chain_from` | 14,789 | 2,847,936 | 1,478,926 | chain items, `make_subexpr_list` |
| `write_list` | 12,938 | 2,217,611 | 980,836 | lists.rs:353, E3's estimate: 2.26x the bytes written |
| `types::rewrite_path` | 12,751 | 222,312 | 158,681 | `with_capacity(32)` and its growth (types.rs:93), `do_reserve` 9,269 calls, 1,332,270 Ir |
| `FmtVisitor::get_context` | 12,074 | 289,776 | 446,799 | two `Rc<Cell<bool>>` per context |
| `ChainItem::rewrite` | 11,835 | 249,597 | 167,733 | |
| `memoize` | 10,331 | 2,946,543 | 3,220,709 | the stored `result.clone()` (context.rs:373) |
| `rewrite_path_segments` | 9,241 | 629,759 | 83,860 | `with_capacity(128)` (types.rs:242, 301): 13% of requested bytes written |
| `combine_strs_with_missing_comments` | 7,440 | 117,760 | 99,662 | comment.rs:152 |
| `FmtVisitor::from_context` | 2,290 | 68,614,171 | 861,552 | `new` reserves 2 × file size, then `from_context` replaces it with 256 bytes (visitor.rs:102, 145) |

- `from_context` runs 868 times and calls `malloc` 1,736 times for 1,124,168 Ir inclusive
  (**measured**): the 2 × file-size buffer (≈ 78 KB on average, 67.5 MB in all) is allocated and
  freed unused for every nested visitor.

### Copies (measured, derived)

- `__memcpy_avx_unaligned_erms` 9,229,309 Ir in 234,443 calls: 5,210,381 (18,599 calls) inside
  `_int_realloc`, i.e. growth that moved; the rest, ≈ 4.02 M in 215,844 calls (18.6 Ir per call),
  is direct copies (`push_str`, `clone`, `concat`): `write_list` 437,457, `overflow` 780,335 (both
  levels), `join_generic_copy` 465,236, `memoize` 305,372, `push_rewrite_inner` 237,302.
- `realloc` is reached from `finish_grow` (14,469,361 inclusive) through `grow_amortized`
  (10,275,329, of which the parser's `Vec<Event>` 6,298,128, `Vec<u32>` 3,295,524,
  `Vec<SyntaxKind>` 1,077,171) and `do_reserve_and_handle` (6,888,911: `String` growth, of which
  `format!`'s `write_str` 1,913,248 and `rewrite_path` 1,332,270). The profile does not split
  `_int_realloc`'s `memcpy` by caller; by DHAT's bytes (the `Vec<Event>` chain alone requests
  18.0 MB) most of the 5.21 M is the parser's (**hypothesis**), which
  [parser-and-substrate.md](parser-and-substrate.md) covers.
- Copy factor at the visitor level: the strings `push_str` counts total ≈ 1.04 MB (§1 newline
  loops, `push_rewrite_inner` 912,744 + `visit_fn`, `format_missing_indent`, `close_block`) for
  659,580 bytes of output, 1.6x (**derived**): nested block visitors build their text and the
  parent copies it. Below the visitor, DHAT's bytes written at formatter frames, 17.83 MB, bound
  the bytes built at ≤ 27x the output (**derived**; an upper bound: it includes `Vec` element
  writes and over-written data). X1's 4.1x for width-measured bytes is a lower figure of the same
  kind.
- `Indent::to_string_inner` (45,976 calls, 33 Ir each) allocates only for tabs or indents over
  80 columns (shape.rs:87-101); its cost is the call, not a string.
- What reserving exact capacity or writing into a sink removes: the formatter-attributed blocks
  at 197 Ir each are ≤ 36.8 M (11.8%, **derived**) plus the direct copies ≈ 4.0 M; that is the
  sink's territory ([decide-once-and-exact-reuse.md](decide-once-and-exact-reuse.md)) and is not
  re-derived here. Local, exact pieces that do not need the sink are rows S7, S9, S10 and S12
  in §7.

## 6. The `core`/`std` share

The parent entry groups 24,589,974 Ir (7.9%) as `core`/`alloc`/`std`; this record's grouping of
`callgrind.1.out` (names under `core::`, `alloc::`, `std::`, `<str>`, `<&str`, `<char`, without
`RawVec`) gives 26,608,351 (**measured**; the difference is drop glue and `Vec::from_iter`
instances). Its parts:

| part | exclusive | driven by (§1 tables) | lever (§7) | removable (derived) |
|---|---|---|---|---|
| `CharSearcher::next_match` | 7,292,092 | `filtered_str_fits` 46%, list gaps 13%, comments 16% (shares of its 7,619,582 inclusive) | S1, S3 | ≈ 4.5 M |
| `trim_matches::<is_whitespace>` | 4,370,777 | list gaps 49%, missed spans 20% | S3, S8 | ≈ 3.6 M |
| `memrchr` + `next_match_back` + `SplitInternal` | 5,545,133 | `lines().last()` in list gaps and `write_list`, `rewrite_call` | S3, S6 | ≈ 2.2 M |
| `is_contained_in` (`&str`) + `StrSearcher::new` | 2,276,388 | `contains("::")` in `write_list`, `find("/*")` in `get_comment_end`, `contains("\r\n")` per file | S3, S16 | ≈ 1.1 M |
| `fmt` (`format_inner`, `write`, `write_str`, `pad`) | 2,007,208 | 5,002 `format!` calls (`rewrite_cond`, `rewrite_match`, `wrap_struct_field`, `rewrite_macro`, imports) | S7 | in S7 |
| `join_generic_copy` | 943,159 | `concat` (E3's `format!` replacement), 7,685 calls | — | 0 (one exact allocation already) |
| drop glue, iterators, `from_utf8`, paths | ≈ 4.2 M | | — | not bounded here |

- `char` patterns: `str::find(char)` and `contains(char)` go through `CharSearcher`, whose per-call
  cost (≈ 80 Ir) exceeds its per-byte cost on the 22-byte average haystack (§1). A `memchr`
  call does not change that (the per-call part is the searcher's state); removing the call does
  (S1, S3).
- `trim_start_matches`/`trim_end_matches` are 266,422 Ir together: not a lever.

## 7. Ranked bounds

Bounds on the 49-file sample, as a share of 312,733,055. "Addressed" is the measured Ir the lever
acts on; "removable" is what the argument leaves after the replacement's own cost; rows are
non-overlapping as stated in the argument column. Gates are protocol.md §4: Ir on `profile.sh`
must fall by more than 0.1% with 0 changed outputs on both hash corpora and `conform`.

| # | lever | addressed (share) | removable, argument | bound on total | exactness risk | cost | smallest experiment; falsified if |
|---|---|---|---|---|---|---|---|
| S3 | list gaps without `/`, `"`, `'`: one byte scan of the gap answers `extract_pre_comment` (None), `extract_post_comment` (None), `get_comment_end` (`find(sep)`/`find(term)` and the newline rule, lists.rs:756-771) and `has_extra_newline` | 12,769,612 (4.08%): the four functions inclusive | 12.77 M − 15,218 items × ≈ 100 Ir − 4,834 chain calls × 20 ≈ 11.1 M; comment-free share ≥ 99.8% (§4, measured) | ≤ 3.6% | low: equal results on gaps without the three bytes by construction; E3-style test over all strings ≤ 5 chars of `, ; ) ] } space \n / * " ' r #` | ≈ 60 lines, lists.rs | Ir after < Ir before − 7.8 M (2.5%), or any changed output |
| S2 | newline counts: `line_number` counted on demand (read only at visitor.rs:692 and parallel.rs:140), other counts by word or byte accumulator, early exit for `== 1` (overflow.rs:502) and `> 1` (lists.rs:801) | 7,878,692 (2.52%) | `push_str`'s 4.95 M becomes at most one count of the main buffer (nested visitors' counts are never read); the main buffer (0.66 MB) and the other counts (0.62 MB) at ≈ 1.1 Ir/byte ≈ 1.4 M remain: ≈ 6.5 M | ≤ 2.1% | low–medium: needs `line_number == '\n' count of the buffer` (+ the worker's uncounted leading newline, imports.rs:49; items.rs:1542, 1551 push text without `\n`) | ≈ 40 lines, visitor.rs, utils.rs | Ir drop < 4 M |
| S1 | `filtered_str_fits` in one pass: newline positions and per-line ASCII flags eight bytes per step; line width = length for ASCII lines without `\r`; other lines and `/` texts take today's path | 8,227,679 (2.63%) | keeps `contains_comment`/`LineClasses` 1.0 M and ≈ 0.8 MB × 1.5 Ir + 5,563 × 50 ≈ 1.5 M (bytes: **hypothesis** from 20,206 line searches at ≈ 40 bytes): ≈ 5.7 M | ≤ 1.8% | low: E3 lever 8's invariant; compare with today's function on generated strings and both corpora | ≈ 50 lines, utils.rs | Ir drop < 3 M |
| S6 | `Rewritten` facts carried and composed (§2) | 9.51 M (3.04%) residual after S1, S2 | − ≈ 2.2 M composition (**hypothesis**): ≈ 7.3 M | ≤ 2.3% | low on output (facts are functions of the text; re-measure at non-ASCII joins; debug recompute); medium for implementation | high: 191 fns, 38 impls, 379 `push_str`; staged behind `Deref<Target = str>` | stage 1 (lazy cached facts on `Rewrite` results only): Ir drop < 1 M refutes the repeat-query premise |
| S4 | `find_uncommented` with a one-byte ASCII needle not in `/ " ' r`: one inline `find_any_byte([needle, '/', '"', '\'', 'r'])`; needle first → return; else today's loop (also drops `skip_comment`'s 50,194 no-op calls) | 6,784,188 (2.17%): 10.22 M less `get_comment_end`'s 3.43 M (S3) | 26,425 calls × (257 − ≈ 70) Ir − fallbacks ≈ 4.5 M | ≤ 1.4% | low: before the first state byte `CharClasses` yields `Normal` (comment.rs:856-890); E3's `find_uncommented_matches_the_per_character_search` test extends | ≈ 30 lines, comment.rs | Ir drop < 2 M |
| S7 | remaining `format!` with string arguments → `concat` (E3 converted 35 of them) | 4,289,477 (1.37%): `format_inner` inclusive, 5,002 calls, 858 Ir each | 5,002 × (858 − ≈ 300) ≈ 2.8 M if every remaining call has only `{}` string arguments (**hypothesis**; 154 sites to read) | ≤ 0.9% | low: E3's invariant and `concat_matches_format_on_strings` | per site, ≈ 150 lines over 20 files | Ir drop < 1 M |
| S9 | memo returns a shared string instead of storing a clone (context.rs:356, 373) | 10,331 blocks × 197 + `memcpy` 0.31 M ≈ 2.3 M (0.74%) | ≈ 2.3 M | ≤ 0.7% | low | needs a shared result type: part of S6 or the sink | with S6 stage 1 |
| S5 | `skip_plain`/`skip_comment` 16-byte steps with an overlapping final load; `r` found by lookback at `"`/`#` | 3.57 M of loops + 0.77 M false-`r` restarts (1.39%) | ≈ 2.3 M + ≤ 0.5 M; after S4, which removes 41,651 of the 56,321 calls, ≈ 1.5 M (**derived**: × 3.58 M / 7.13 M of non-`find_uncommented` callers) | ≤ 0.5% after S4 | low: E3's skip tests; one `unsafe` load per step (gate 4: safety argument) | ≈ 80 lines, comment.rs | Ir drop < 0.5 M after S4 |
| S8 | ASCII-first `trim` (the `char::is_whitespace` ASCII set includes `\x0B`, so `trim_ascii` is not equivalent; stop at a byte ≥ 0x80 and finish with `trim()`) | 4,370,777 (1.40%); 2.23 M after S3 | 36,000 calls × (50 − 20) ≈ 1.1–1.5 M | ≤ 0.5% | low | helper + ≈ 15 sites | Ir drop < 0.5 M |
| S12 | `rewrite_path` sized from the path's text, no growth (types.rs:93) | 1,332,270 (0.43%) `do_reserve` from it | ≈ 1.0 M | ≤ 0.3% | none on output | ≈ 5 lines | Ir drop < 0.3 M |
| S15 | `Indent::to_string_inner` inlined to its slice case | 1,536,285 (0.49%) | 45,976 × (33 − ≈ 8) ≈ 1.1 M | ≤ 0.35% | none | ≈ 5 lines | Ir drop < 0.3 M |
| S10 | `from_context` without the 2 × file-size buffer (visitor.rs:102, 145) | 1,124,168 + 161,124 (0.41%) | 868 large `malloc`/`free` ≈ 1.0 M | ≤ 0.3% | none (capacity only) | ≈ 5 lines | Ir drop < 0.3 M |
| S13 | `line_of` by 8-byte search (context.rs:169-173) | 1,258,826 (0.40%) | 1.26 − 0.34 ≈ 0.9 M | ≤ 0.3% | none (E3's `line_of` test) | ≈ 15 lines | Ir drop < 0.3 M |
| S16 | `contains("\r\n")` per file after `contains('\r')` (formatting.rs:186) | 401,140 (0.13%) | ≈ 0.35 M | ≤ 0.1% | none | 1 line | below the 0.1% gate on its own |
| S14 | comment index over the flat array's comment `Elem`s (§4) | ≤ 3.1 M (1.0%) | ≤ 1.9 M after S3, S4, before the span-API cost | ≤ 0.6% | medium (X8's two `CharClasses` behaviours, comment.rs:783-799, 816) | span-based API at ≈ 65,000 query calls' sites | not worth an experiment before S3 and S4 land |

Compound bound (**derived**, rows S1–S16 except S14, overlaps removed as stated):

- Addressed, if every row removed all it addresses: 12.77 + 8.23 + 7.88 + 6.78 + 9.51 + 4.29
  + 2.30 + 2.18 (S5 after S4: 4.34 × 3.58 / 7.13) + 2.23 + 1.33 + 1.54 + 1.29 + 1.26 + 0.40 =
  62.0 M (19.8%): an instruction count ≥ 250.7 M.
- Removable on the arguments above: 11.1 + 6.5 + 5.7 + 7.3 + 4.5 + 2.8 + 2.3 + 1.5 + 1.3 +
  1.0 + 1.1 + 1.0 + 0.9 + 0.35 = 47.4 M (15.1%): ≈ 265.4 M.
- Without the formatter-wide rows S6 and S9 (local changes only): 37.8 M (12.1%): ≈ 275.0 M.
- The levers do not touch the allocator's 36.8 M at formatter frames beyond S9, S10, S12 and
  S7's allocations; that, and the 4.0 M of direct copies, is the sink's
  ([decide-once-and-exact-reuse.md](decide-once-and-exact-reuse.md)); if the sink lands it
  absorbs S6 and S9 and part of S7.

## What this settles and what it leaves open

- The per-byte work left after E3 is small; the per-call work is not: `find_uncommented` costs
  245 Ir per call and finds its needle on the first step in 94.5% of 41,643 calls;
  `CharSearcher::next_match` spends ≈ 80 Ir per call on 22-byte haystacks; `skip_comment`
  returns at its state test in 94.8% of calls (**measured**, per-instruction counts). Fast
  paths that skip the call (S3, S4) bound larger than wider vectors (S5).
- List-gap comment handling costs 839 Ir per list item, 12.77 M (4.08%) in all, and finds 3
  post-comments and 32 pre-comments in the sample (**measured**); an exact byte test of the gap
  removes ≈ 11 M of it (**derived**).
- Newline counting runs at 4.75 Ir/byte over 1.66 MB, 7.88 M (2.52%), most of it maintaining
  `line_number` for nested visitors whose count is never read (**measured**, **read**); this is
  not the short-string vector form E3 rejected.
- Width and line facts compose exactly over concatenation when the right part starts with an
  ASCII byte, because unicode-width 0.1.14 measures right to left (**read**); 9.51 M of
  re-measurement (3.04%) remains for a carried-facts type after S1 and S2 (**derived**).
- A comment index over the flat array removes X8's build cost but not the per-call cost that
  dominates the queries; bound ≤ 0.6% after S3 and S4 (**derived**).
- Open: the bytes `filtered_str_fits` reads on the sample (S1's residual rests on an estimate);
  whether all 154 remaining `format!` sites take only string arguments (S7); the split of
  `_int_realloc`'s 5.21 M of `memcpy` between parser and formatter (no per-caller data); the
  fraction of S6 that a lazily cached first stage removes (the repeat-query count per string is
  not measured); every row's figure is a bound until its experiment runs.

## Current State

- The record bounds sixteen levers on per-byte and per-string work at the final head, from the
  final-head profile, a per-instruction callgrind run and a DHAT run of the same binary
  (scratch `theory-scanning/`)
- No formatter code was changed for this record

## Missing

- An `Ir` measurement of any lever in the ranked table
- A per-caller split of `_int_realloc`'s `memcpy`, and a final-head DHAT classification of
  `String` against `Vec` blocks (the binary has no debug info; profile-flat.md's categories are
  from `fe34bd2` (was `8d418ee`))
