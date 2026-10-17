# EXP-016: scanning and string levers, one per commit

Ledger row E16 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md); the levers are rows S1–S16 of section 7 ("Ranked bounds")
of [../../2026-10-15-extensions-theory/scanning-and-strings.md](../../2026-10-15-extensions-theory/scanning-and-strings.md),
taken in the record's order of bound without S6, S9 and S14. Labels: **measured** (a tool's
output on the builds below), **read** (from source), **derived**.

## Identity

| field | value |
|---|---|
| branch | `20261015-scanner-paths` from `25b6e17` (was `f2a25dd`) (integration branch `claude/chloro-perf-research-swarm-nrscj8`), worktree `/home/user/wt/scanner-paths` |
| base BKG | `397159b` (was `92fdef0`), whose `chloro-core` is the code of `bd9c768` (was `bdfb9d1`); `25b6e17` (was `f2a25dd`) adds only documentation after it |
| kept commits | `92004c5` (was `ca71f90`) (S3), `52a56fd` (was `4358bb9`) (S1), `a4b99f9` (was `6e6deca`) and `802ccad` (was `fb48651`) (S2), `2af3547` (was `4499fbc`) (S4), `c52fd82` (was `d80cdfd`) (S7), `ef2532e` (was `70f75e5`) (S8), `1a5d2d1` (was `8832233`) (S5), `af9ae1b` (was `5950f4c`) (S12), `919bf3c` (was `b092f47`) (S15), `d26b9ef` (was `5cb66ff`) (S13); then this file and the ledger row |
| rejected, reverted | S12 in the study's form, S10, S16 (two forms), a second round of S7; details under *Rejected* |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, valgrind 3.22.0; `cargo build --release -p chloro-core --examples` (the workspace's fat-LTO, one-codegen-unit profile) |
| host | 4 vCPU Xeon @ 2.80GHz, 15 GB; other experiments built and ran on the host throughout (load average 2.3–7.5). `Ir` is unaffected; the wall-clock pairs below are **measured, provisional** (protocol § 6) |
| `Ir` command | `benchmarks/profile.sh --core 3 --runs 1 --sample /tmp/chloro-bench/scanpth1`: the script's sample at a private path of the default's length (`sample49` is shared with other agents and deleted by each run) |
| `Ir` of the base | 312,997,148 for `25b6e17` (was `f2a25dd`) built in this worktree, against the ledger's 312,734,032 for the same code: the count depends on the length of the binary's path (E3, *Identity*), and every figure below is from this worktree's path |
| golden hashes | `scratchpad/golden/fixtures-e8d87c2.tsv` (1238 fixtures) and `registry-e8d87c2.tsv` (20,850 recorded registry files; the registry directory holds 25,024 files tonight, the 4,174 others are reported as not recorded and ignored) |

## Hypothesis and falsification

The local levers of the scanning record remove about 12% of the instructions on the 49-file
sample (its compound bound without S6, S9 and S14: 37.8 M of 312.7 M) with byte-identical
output. Each lever is falsified on its own if `Ir` does not fall by more than 0.1% of the
start (313 K) or any output changes; the record's own per-row falsifier is reported next to
each result.

## Levers (read, measured)

Each kept commit's message states its exactness argument, its test and its `Ir`. Where the
implementation departs from the study's description, the departure is named.

### S3: list gaps (`92004c5` (was `ca71f90`))

- `extract_pre_comment` returns `(None, None)` when the snippet has no `/` (each of its three
  tests needs one); `extract_post_comment` returns `None` when `snippet[..comment_end]` has no
  `/`; `get_comment_end` with a one-byte separator or terminator and no `/` in the snippet
  takes `find` of the byte and the newline rule; `has_extra_newline` walks bytes while they are
  ASCII. The full functions stay as `*_full` for every other input.
- Departure: the study's condition was "no `/`, `"`, `'`". `CharClasses` yields string and
  raw-string characters as `InString` and character-literal characters as `Normal`, and
  `find_uncommented` matches a pattern character in both kinds, so only comments, which need a
  `/`, can hide a separator. The comparison test passes with the narrower condition and fails
  when the `/` test is dropped.

### S1: `filtered_str_fits` (`52a56fd` (was `4358bb9`))

- `lines_fit` walks the text once, finding the next `\n`, `\r` or non-ASCII byte eight bytes
  at a time; a line's width is its length when ASCII and its `unicode_str_width` otherwise; a
  text with `\r` takes the line-by-line form (`str_fits_by_lines`, rustfmt's).

### S2: newline counts (`a4b99f9` (was `6e6deca`), `802ccad` (was `fb48651`))

- `a4b99f9` (was `6e6deca`): `push_str` no longer counts. `line_number()` counts the buffer's `\n` from a cached
  prefix count, less `uncounted_newlines` (a worker's leading newline; the ABI text that
  `format_foreign_mod` writes without `push_str`). The removed-import `pop` keeps rustfmt's
  `saturating_sub`. Debug builds keep the running count and assert equality at each read; the
  parallel tests reach the assertion.
- `802ccad` (was `fb48651`): `count_newline_bytes` sums exact per-byte flags over up to 255 words before
  reducing; `count_newlines(overflowed) == 1` becomes `has_one_newline`, which stops at a
  second `\n`. `prefer_next_line`'s comparison keeps two counts.

### S4: `find_uncommented` (`2af3547` (was `4499fbc`))

- For a one-byte pattern other than `/`: search for the pattern byte and `/`; the pattern
  first is the answer, the end is `None`, a `/` before neither `*` nor `/` is passed over, any
  other `/` runs the existing search from the start.
- Departure: the study's stop set was the pattern and `/ " ' r`. For the same reason as in
  S3 only a comment opener can hide a pattern byte; E3's per-character comparison test
  (extended with the patterns `#`, `*`, `\`, `\n`, `'`) confirms it and fails when the
  `/`-before-`/` stop is dropped.

### S7: `format!` to `concat` (`c52fd82` (was `d80cdfd`))

- The `format!` calls with the largest inclusive cost (`ControlFlow::rewrite_cond`,
  `rewrite_match`, `wrap_struct_field`, `rewrite_macro`'s empty forms and `;`, use-tree
  segments, lists and lines, `identify_comment`, `rewrite_missing_comment`,
  `light_rewrite_comment`, `-> ` return types, parenthesised bounds, `for<..>`): 25 calls in
  7 files. E3's invariant: string arguments under bare `{}` are written unchanged; no type in
  the crate derefs to `str`; `?` arguments are bound first, in `format!`'s order.

### S8: ASCII-first `trim` (`ef2532e` (was `70f75e5`))

- `utils::trim` finds the first and last bytes outside tab, `\n`, `\x0B`, `\x0C`, `\r` and
  space (`char::is_whitespace` on ASCII; `trim_ascii` omits `\x0B`) and returns `str::trim`'s
  result if either is non-ASCII; an all-whitespace text gives the empty slice at its start.
  16 call sites: the blank-gap tests of `format_missing_inner`, `process_missing_code`,
  `trim_tries`, `rewrite_chain_from`, `close_block`, `push_rewrite_inner`,
  `rewrite_missing_comment`, `trimmed_last_line_width`, `rewrite_match_body`.

### S5: sixteen-byte search (`1a5d2d1` (was `8832233`))

- `find_any_byte` (used by `skip_plain`, `skip_comment` and the searches of S2–S4) compares
  sixteen bytes per step with SSE2 and reads the final sixteen bytes of a slice once, shifting
  out the flags before `pos`; slices under sixteen bytes and non-x86-64 targets keep the
  eight-byte search. One `unsafe` block under `cfg(all(target_arch = "x86_64", target_feature
  = "sse2"))`, with its safety argument in the code (protocol gate 4); the search test gains
  20,000 sparse strings of up to 79 bytes.
- Departure: the study placed S5 after S4 inside `skip_plain`/`skip_comment`; the change is in
  their shared search, so the S2–S4 searches gain too. The `r` lookback the study proposed
  was not taken.

### S12: path allocation (`af9ae1b` (was `5950f4c`))

- Departure: the 9,269 `reserve` calls attributed to `rewrite_path`'s 32-byte buffer come
  from `single_ident_path`, inlined into it, which pushed onto `String::new()`. It now copies
  the first token with `to_owned`. The study's form was measured and rejected (below).

### S15: indent strings (`919bf3c` (was `b092f47`))

- `to_string` and `to_string_with_newline` are `#[inline]` and return the `INDENT_BUFFER`
  slice when there are no hard tabs and the indent fits; everything else calls the previous
  body, `#[inline(never)]`.

### S13: line table (`d26b9ef` (was `5cb66ff`))

- `line_of` extends the table by `find_any_byte` for each `\n` instead of a byte loop.

## Results (measured)

`Ir` on the 49-file sample, one callgrind run per build, all gates on every kept commit's
binary. "Change" is against the previous commit; "share of base" against 312,997,148. (The
commit messages give one of the two percentages, not always the same one.)

| commit | lever | `Ir` before | `Ir` after | change | share of base | study's bound; falsifier | decision |
|---|---|---|---|---|---|---|---|
| `92004c5` (was `ca71f90`) | S3, list gaps | 312,997,148 | 303,767,669 | −9,229,479 (−2.95%) | −2.95% | 11.1 M; < 7.8 M refutes | kept |
| `52a56fd` (was `4358bb9`) | S1, `filtered_str_fits` | 303,767,669 | 300,212,806 | −3,554,863 (−1.17%) | −1.14% | 5.7 M; < 3 M refutes | kept |
| `a4b99f9` (was `6e6deca`) | S2, `line_number` on demand | 300,212,806 | 294,668,603 | −5,544,203 (−1.85%) | −1.77% | 6.5 M for S2; < 4 M refutes | kept |
| `802ccad` (was `fb48651`) | S2, word count, `== 1` | 294,668,603 | 292,628,791 | −2,039,812 (−0.69%) | −0.65% | (S2 together −7.58 M) | kept |
| `2af3547` (was `4499fbc`) | S4, `find_uncommented` | 292,628,791 | 288,311,366 | −4,317,425 (−1.48%) | −1.38% | 4.5 M; < 2 M refutes | kept |
| `c52fd82` (was `d80cdfd`) | S7, `format!` → `concat` | 288,311,366 | 284,093,442 | −4,217,924 (−1.46%) | −1.35% | 2.8 M; < 1 M refutes | kept |
| `ef2532e` (was `70f75e5`) | S8, `trim` | 284,093,442 | 282,855,651 | −1,237,791 (−0.44%) | −0.40% | 1.1–1.5 M; < 0.5 M refutes | kept |
| `1a5d2d1` (was `8832233`) | S5, sixteen-byte search | 282,855,651 | 281,021,083 | −1,834,568 (−0.65%) | −0.59% | 1.5 M after S4; < 0.5 M refutes | kept |
| `af9ae1b` (was `5950f4c`) | S12, single-identifier path | 281,021,083 | 280,341,736 | −679,347 (−0.24%) | −0.22% | 1.0 M; < 0.3 M refutes | kept |
| `919bf3c` (was `b092f47`) | S15, indent slice inline | 280,341,736 | 278,965,452 | −1,376,284 (−0.49%) | −0.44% | 1.1 M; < 0.3 M refutes | kept |
| `d26b9ef` (was `5cb66ff`) | S13, line table | 278,965,452 | 278,125,378 | −840,074 (−0.30%) | −0.27% | 0.9 M; < 0.3 M refutes | kept |
| **total** | | 312,997,148 | 278,125,378 | **−34,871,770 (−11.14%)** | | 37.8 M (12.1%) | |

The head measured again after the final rebuild: 278,125,519 (141 from the S13 run, the
script's run-to-run spread).

## Rejected (measured)

| attempt | measurement | mechanism | decision |
|---|---|---|---|
| S12 as the study gives it: `rewrite_path`'s buffer `max(32, len + len / 4)` from the path's source length | 281,021,083 → 281,226,778 (+205,695) | the 9,269 `reserve` calls stayed (they are `single_ident_path`'s); `_int_realloc`'s copies +0.22 M | reverted; replaced by `af9ae1b` (was `5950f4c`) |
| S10: `from_context` without the file-sized buffer of `new` (a `with_buffer` constructor) | 278,965,452 → 279,447,330 (+481,878) | `malloc`/`_int_malloc`/`free` −0.55 M as expected, but `memcpy` called from `_int_realloc` +1.44 M (16,456 calls against 16,300): without the large blocks freed back into the top chunk, blocks that grew in place now move | reverted |
| S16: `contains('\r') && contains("\r\n")` in `normalize_src` | 278,125,378 → 278,458,658 (+333,280) | `is_contained_in` −400,304, but the inlined `char` search over the whole file costs 735,273 in `normalize_src` | reverted |
| S16 with `find_any_byte` for `\r` | 278,125,378 → 278,175,940 (+50,562) | a sixteen-byte search of 650 KB costs about what the substring search did | reverted |
| S7, second round: 18 more sites (`rewrite_pat_expr`, the one-line `if`/`else`, `else` keyword, `rewrite_assignment`, `format_struct_struct`, `rewrite_pair`, `rewrite_macro`'s brace and `vec![a; n]` forms, `format_visibility`) | 278,125,378 → 279,309,580 (+1,184,202) | the `fmt` machinery −0.46 M and `join_generic_copy` +0.13 M, but `memcpy` from `_int_realloc` +1.54 M at the same call count (16,291 against 16,300) | reverted |
| S5 without intrinsics (a sixteen-bit flag loop for the compiler to vectorise) | 282,855,651 → 290,418,234 (+7,562,583) | the search was no longer inlined into `skip_plain` and its closure ran out of line | replaced by the SSE2 form before commit |
| S6, S9, S14 | not attempted | formatter-wide (S6, S9) or after S3 and S4 (S14), excluded by the experiment's scope | — |

## Totals (measured)

| | base `25b6e17` (was `f2a25dd`) | head `d26b9ef` (was `5cb66ff`) | change |
|---|---|---|---|
| `Ir`, 49-file sample (`profile.sh`, this worktree) | 312,997,148 | 278,125,378 | −34,871,770 (−11.14%) |
| `memcpy` from `_int_realloc` (calls) | 5,507,672 (18,737) | 3,546,886 (16,300) | −1,960,786 (−2,437 calls, S7) |

The `Ir` drop is 92% of the study's compound bound for these rows (37.8 M). By lever against
its bound: S2 (7.58 M against 6.5), S7 (4.22 against 2.8), S5 (1.83 against 1.5) and S15
(1.38 against 1.1) exceed it; S3 (9.23 against 11.1), S1 (3.55 against 5.7), S4 (4.32 against
4.5), S12 (0.68 against 1.0) and S13 (0.84 against 0.9) stay below it; S8 is inside its range;
S10 and S16 lost.

## Wall clock (measured, provisional)

`bench -n 5` on the 1238 fixtures (13.9 MB), the base binary (`25b6e17` (was `f2a25dd`), built in this
worktree before the first change) and the head binary interleaved, three pairs,
`taskset -c 2`, load average 2.7–4.2 while other experiments ran.

| pair | base, best of 5 | head, best of 5 | base / head |
|---|---|---|---|
| 1 | 1.510 s, 9.21 MB/s | 1.347 s, 10.32 MB/s | 1.121 |
| 2 | 1.515 s, 9.18 MB/s | 1.355 s, 10.26 MB/s | 1.118 |
| 3 | 1.497 s, 9.29 MB/s | 1.348 s, 10.32 MB/s | 1.111 |

The ranges are 9.18–9.29 MB/s and 10.26–10.32 MB/s and do not overlap; the 11–12% matches
the `Ir` change. A quiet-host run is still owed (protocol § 6).

## Gates (measured)

| gate | corpus | result |
|---|---|---|
| `cargo fmt --all --check`, `git diff --check` | | clean on every kept commit |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | clean on every kept commit (two commits fixed before commit: a byte-array literal, two needless borrows) |
| `cargo test -p chloro-core` | 183 library tests at the base rising to 187, conformance batches, integration | all pass on every kept commit |
| `bench --check`, sequential and `--threads 4` | 1238 fixtures against `fixtures-e8d87c2.tsv` | 0 changed; 0 differ between 4 threads and `format_source`; every kept commit |
| `bench --check` | 20,850 recorded registry files (25,024 present) against `registry-e8d87c2.tsv` | 0 changed, every kept commit |
| `conform -i`, `conform -i -V 1.8.0` | 1220 fixtures | 1220/1220 identical, 0 not idempotent, at both versions, every kept commit |
| `cargo test --workspace --all-features` | head | all pass, 0 failed |
| `config_matrix -n 400` | `max_width=80`, `newline_style=Windows`, `hard_tabs=true` | 343/343 each |
| `config_matrix -n 1220` (gate 6, all 31 values) | head | every value fully identical: 1022/1022, editions 844/844, 843/843, 838/838 (the BKG's counts) |

The registry `config_matrix` against live rustfmt (gate 7) was not run: no output changed on
either hash corpus.

## Tests added (read)

| test | file | compares |
|---|---|---|
| `list_gap_fast_paths_match_the_full_functions` | lists.rs | the four list-gap functions with their full forms on every string of up to five characters over `, ; ) ] } space \n / * " ' r #` and on strings with `\r`, `\x0B`, `\x0C`, U+00A0, U+3000, `é`, raw strings and character literals, every `comment_end` |
| `one_pass_fit_matches_the_line_by_line_fit` | utils.rs | `filtered_str_fits` with the line-by-line form on every string of up to five characters over `a`, space, `\n`, `\r`, U+3000, U+0301, U+FE0F, `/`, and longer strings, 50 shapes |
| `newline_counts_match_the_byte_filter` | utils.rs | `count_newline_bytes` and `has_one_newline` with a byte filter, lengths across the words and the 255-word blocks, `\n` beside `\x0B`, `\x08`, `\x8a` |
| `ascii_first_trim_matches_str_trim` | utils.rs | `utils::trim` with `str::trim`, text and pointer, every string of up to five characters over the six ASCII whitespace characters, a letter, U+0085, U+00A0, U+3000, `é` |
| `find_uncommented_matches_the_per_character_search` (extended) | comment.rs | five more patterns |
| `find_any_byte_finds_the_first_target` (extended) | comment.rs | 20,000 sparse strings of 0 to 79 bytes |
| debug assertion in `FmtVisitor::line_number` | visitor.rs | the on-demand count with rustfmt's running count, at every read in debug builds (reached by the parallel tests) |

Each new comparison was checked against a mutation of the code it covers (the `/` test of
S3 dropped, `\x0B` dropped from `has_extra_newline` and from `trim`, the `\r` fallback of S1
dropped, `find_uncommented`'s `/`-before-`/` stop dropped, the worker reset of S2 broken);
every mutation failed its test.

## What surprised

- Strings and character literals never hide a pattern from `find_uncommented`: it matches in
  `InString` as well as `Normal`, so only a comment opener matters. S3 and S4 need a scan for
  `/` alone, where the study assumed `/ " '` (and `r`).
- The 1.3 M of `reserve` the study gave to `rewrite_path`'s buffer was `single_ident_path`'s
  `push_str` onto `String::new()`; the buffer-sizing form measured worse.
- Three changes that remove work measured more instructions because of the allocator: S10
  (+0.48 M) and the second S7 round (+1.18 M) each removed 0.5 M of their own cost and added
  1.4–1.5 M of `memcpy` inside `_int_realloc` at an unchanged number of reallocations, that is,
  larger blocks moving instead of growing in place. The first S7 round went the other way
  (−1.96 M of the same `memcpy`, 2,437 fewer reallocations). The study notes that most of those
  copies are probably the parser's `Vec<Event>` chain; until that growth is reserved, `Ir`
  changes of about 1.5 M can come from heap layout rather than from the code changed.
- S5 without intrinsics lost 7.6 M because the search stopped being inlined; with SSE2 and
  `#[inline(always)]` it gained 1.8 M.

## Decision

Kept: the eleven commits under *kept commits*, −11.14% `Ir` with byte-identical output on
both hash corpora, `conform` at both versions and all 31 `config_matrix` values. Branch
`20261015-scanner-paths`, for the integrator's gates 1–5 on the merged tree. Rejected: S10,
S16, S12 in the study's form, a second S7 round.

## Follow-up

- Reserve the parser's event vector (or otherwise remove the large in-place `realloc` growth)
  and measure S10 and the second S7 round again: their own savings (0.55 M and 0.46 M) are
  real, their losses are `_int_realloc` copies.
- S6 (carried width facts) and S14 (comment index) are untouched; with S1–S4 in, S14's bound is
  at most the 0.6% the study gives after S3 and S4.
- Remaining scanning and string cost at the head (exclusive `Ir`, callgrind of `d26b9ef` (was `5cb66ff`)):
  `write_list` 6.68 M (with its inlined width code), `filtered_str_fits` 3.61 M (`lines_fit`
  inlined), `CharSearcher::next_match` 3.01 M, `CharClasses::next` 2.61 M, `memrchr` 2.42 M
  (`lines().last()`, `rsplit`), `skip_plain` 2.34 M, `find_uncommented` 1.77 M.
