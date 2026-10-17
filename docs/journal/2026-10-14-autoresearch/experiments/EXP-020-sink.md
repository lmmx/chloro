# EXP-020: the transactional sink on `write_list`, chains and overflow

Ledger row E20 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md). The brief is trick T6 of
[../../2026-10-15-unmined-tricks.md](../../2026-10-15-unmined-tricks.md) § 5; its bound is row 1
of the ranked bounds in
[../../2026-10-15-extensions-theory/decide-once-and-exact-reuse.md](../../2026-10-15-extensions-theory/decide-once-and-exact-reuse.md)
(8.83–10.52% of the instructions at the 312.7 M head) with the width bookkeeping of S6/S9 in
[../../2026-10-15-extensions-theory/scanning-and-strings.md](../../2026-10-15-extensions-theory/scanning-and-strings.md).
Labels: **measured** (a tool's output on the builds below), **read** (from source),
**derived**.

## Identity

| field | value |
|---|---|
| branch | `20261015-sink` from `73f8a85` (was `8ebd3bb`) (integration branch `claude/chloro-perf-research-swarm-nrscj8`), worktree `/home/user/wt/sink` |
| base BKG | `f4be63e` (was `e640fbf`); `73f8a85` (was `8ebd3bb`) adds only documentation and `benchmarks/compare_rustfmt.sh` after it (`git diff --stat e640fbf 8ebd3bb -- chloro-core/src chloro-core/examples` is empty) |
| kept commits | `b9a279b` (was `8df88c3`) (step 2, `write_list_into`), `0f83db7` (was `7290457`) (step 3, chain entry lists), `804c400` (was `e6fe9c0`) (step 4, overflowed argument moved), `5ddfd19` (was `c40a441`) (the two per-item scans of `write_list`, where its cost was); then this file and the ledger row |
| rejected, reverted | step 3's chain rewrites in one buffer (two forms), the placeholder's first line written in place (a no-op), `write_list_into` through `&mut String` and the by-value form; details under *Rejected* |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, valgrind 3.22.0; `cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples` (fat LTO, one codegen unit) |
| host | 4 vCPU Xeon @ 2.80GHz, 15 GB; E19, E21 and E22 built and ran on the host throughout (load average 4.7–12.7, callgrind runs of other agents and E21's resident process on core 3). `Ir` is unaffected; every wall-clock figure is **measured, provisional** (protocol § 6) |
| `Ir` command | `benchmarks/profile.sh --core 3 --bench target/release/examples/bench --sample /tmp/chloro-bench/sinkxx49` (the script's sample at a private path of the default's length; `sample49` is shared and deleted by each run) |
| `Ir` of the base | 248,202,217 for `73f8a85` (was `8ebd3bb`) built in this worktree (the brief's range for the head: 248,202,094–248,203,822) |
| golden hashes | `scratchpad/golden/fixtures-e8d87c2.tsv` (1238 fixtures) and `registry-e8d87c2.tsv` (25,024 registry files present, one not recorded) |
| attribution builds | the same code with `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only` into a scratch target directory; `valgrind --tool=callgrind --dump-instr=yes` under the script's environment; each instruction's cost attributed through `addr2line -i` to the innermost (or outermost) inlining frame in `chloro-core/src/formatter/` (`e20/scripts/outer.py`); the line-table builds count within 0.01% of the release builds (248,204,351 against 248,202,217 at the base) |

## Hypothesis and falsification

rustfmt's protocol returns `Option<String>` from every rewrite, so every level builds a string,
measures it and copies it into its caller's. A shared growing buffer with transactional writes
(`fn ..._into(.., out: &mut String) -> bool`: a checkpoint `out.len()`, truncation on failure)
removes that assembly exactly; the `Option<String>` entry points stay as wrappers, so each step
is incremental and exact. Order: `write_list` and its item assembly; `rewrite_chain_from` and the
chain assembly; `overflow::Context::rewrite` and the argument-list assembly.

Falsified for a step if `Ir` does not fall by at least half of the step's measured site cost
(then the step is reverted and the record says where the cost was), or any output changes. The
brief's falsifier for the experiment: `write_list` converted and `Ir` not down by its measured
string-assembly share.

## Commands

```text
S=.../scratchpad/e20
callgrind_annotate --inclusive=yes --threshold=99.5 $S/prof-base/callgrind.1.out > $S/incl-base.txt   # and without --inclusive
benchmarks/profile.sh --core 3 --bench target/release/examples/bench --sample /tmp/chloro-bench/sinkxx49 --out $S/prof-<tag>
python3 -I $S/cg.py $S/prof-<tag>/callgrind.1.out $S/<tag>.pkl          # exclusive Ir, calls, caller->callee arcs
python3 -I $S/scripts/diff.py $S/<a>.pkl $S/<b>.pkl                     # exclusive Ir and call changes by function
python3 -I $S/scripts/sites.py $S/<tag>.pkl [CALLER_RX]                 # allocator and memcpy arcs by caller
python3 -I $S/scripts/allocsplit.py $S/<tag>.pkl                        # allocator arcs: formatter / parser / cst
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release -p chloro-core --example bench --target-dir $S/target-dbg
taskset -c 3 env -i setarch x86_64 -R valgrind --tool=callgrind --dump-instr=yes --callgrind-out-file=$S/cg-line-<tag>.out \
    $S/bin/bench-dbg-<tag> -n 1 --root /tmp/chloro-bench/sinkxx49
INNER=1 python3 -I $S/scripts/outer.py $S/cg-line-<tag>.out $S/bin/bench-dbg-<tag> FN_RX 'formatter/' # per source line
python3 -I $S/scripts/cmpsrc.py ...                                     # two builds' per-line costs aligned by source text
$S/gates.sh <tag>                                                       # protocol gates 1-4, log in $S/gates-<tag>.log
$S/pairs.sh /home/user/wt/sink/bench-base $S/bin/bench-<tag> 3          # taskset -c 3 bench -n 5, order alternating
```

## The fresh profile at the base (measured)

### Totals

| quantity | base `73f8a85` (was `8ebd3bb`) | head `5ddfd19` (was `c40a441`) |
|---|---|---|
| `Ir` | 248,202,217 | 238,033,138 |
| glibc allocator family, exclusive (`malloc`, `free`, `realloc`, `_int_*`, consolidation) and the `RawVec` growth helpers | 40,896,404 | 36,545,455 |
| allocator arcs from formatter frames (inclusive, `realloc`'s copies included) | 32,174,037 (12.96%), 342,041 calls | 27,913,034 (11.73%), 316,670 calls |
| allocator arcs from the parser and lexer | 8,001,623 | 7,730,299 |
| `memcpy`, exclusive | 7,290,158 (217,312 calls), of it 3,437,653 inside `_int_realloc` | 6,850,396 |

At this head the formatter's allocator arcs plus the `memcpy` outside `_int_realloc` (≈ 3.85 M,
most of it at formatter frames) are ≈ 36 M (14.5%): the pool decide-once row 1 bounds
(27.60–32.90 M at the 312.7 M head) is about the same in absolute terms and a larger share now
(**derived**).

### `write_list` (lists.rs; 10,952 calls; exclusive 6,150,510, inclusive ≈ 10.1 M)

Per source line of the base, outermost formatter frame, exclusive plus the calls made there.

| site (base line) | `Ir` | kind |
|---|---|---|
| result `String::with_capacity(128.max(estimate))` (353) | 427,840 + `malloc` 577,245 = 1,005,085 | assembly |
| capacity estimate over the items (339–352) | 327,236 | assembly |
| `push_str(inner_item)` (475), separator (492–493), other `push`/`push_str` | ≈ 0.40 M + `memcpy` 421,951 (24,087 calls) | assembly (bytes written once in any design) |
| the list string's `free` in the callers | 764,742 in `overflow` (8,841) + ≈ 0.18 M elsewhere | assembly |
| the list string's copy into `wrap_items`'s buffer (overflow.rs:660, 664) | 174,054 | assembly |
| **string assembly, all** | **≈ 3.27 M (1.32%)**; removable by a sink at the overflow site (8,842 of the 10,952 lists): with_capacity and `malloc` 0.81 M, `free` 0.76 M, copy 0.17 M = **1.75 M** | |
| `item_last_line_width` (367–374): `is_multiline` 968,167 + 70,855, `lines().last()` 845,458 + 75,693, `unicode_str_width` 718,545, `starts_with` 63,442 + `memcmp` 125,827, 49,295 | **2,917,282** | scan of the items, read only for 3 post-comments |
| `prev_item_is_nested_import` (593): `contains(':')` 1,114,349, `contains("::")` 829,543 | **1,943,892** | scan of the items, read only when `formatting.nested` |
| loop control (668,499 at the `while let`), tactic arms, `indent_str`, comments | the rest, ≈ 3.1 M of the ≈ 10.1 M inclusive | |

`write_list`'s cost is per-item scanning of the items' text, not string assembly: 4.86 M against
3.27 M, of which a sink removes 1.75 M (**measured**, **derived**).

### `overflow::Context::rewrite` (overflow.rs; both recursion levels; 8,842 calls)

| site (base line) | `Ir` |
|---|---|
| `wrap_items` result `with_capacity` (644) | `malloc` 475,554 |
| copies of callee, `(`, items, `)` into it (647, 648, 660, 664, 668) | `memcpy` 555,859 |
| the list string's `free` (675) | 764,742 |
| `overflowed.to_owned()` and the original's drop (514) | `malloc` 176,160 + `free` 167,741 + `memcpy` 61,078 = 404,979 |
| placeholder's first line (421) | `__rust_alloc` 128,912 + `memcpy` 29,135 (2,033) |
| `list_items` collect (591) | `malloc` 309,059 (6,859) |
| drop of `list_items` and the argument strings (618) | `free` 1,463,116 (16,923) |
| `last_line_width(ident)` (622), `first_line_width(items)` (634) | 580,508, 423,285 |

### Chains (chains.rs; 3,010 chains)

| site (base line) | `Ir` |
|---|---|
| `make_subexpr_list` growth of `vec![top]` (465) | `grow_one` 1,403,715 (3,179) |
| `rev_children` and `children` growth (348, 432) | `grow_one` 399,107 + 371,917 |
| end of `Chain::rewrite`: the rewrites' strings and vector (531) | `free` 824,780 (9,556) |
| `rewrites` vector (555) | `malloc` 149,596 |
| join's result and copies (727–740) | `__rust_alloc` 178,384 (2,549), `memcpy` 96,751, code ≈ 0.28 M |
| `format_root` growth on absorbed children (787–789) | 193,709 |
| child strings in `ChainItem::rewrite` | `malloc` 0.37 M, `do_reserve` 0.25 M, `__rust_alloc` 0.10 M, `memcpy` 0.07 M |
| `wrap_str`'s `filtered_str_fits` over the chain (utils.rs:328) | 1,466,505 |
| `last_line_width` of the root (620); `lines().count()` and `first_line_width` of the first candidate (658, 659) | 170,590; 592,550 + 171,808 |

## Steps (read, measured)

### Step 2: `write_list_into` (`b9a279b` (was `8df88c3`))

- `write_list_into(items, formatting, out) -> bool` appends the list to `out` or truncates it
  back and returns `false` where `write_list` returns `None`. Its reads of the list's own text
  (`!result.is_empty()` before a vertical item; `last_line_width(&result)` for a post-comment's
  overhead and alignment) read `result[start..]`. `write_list` is the wrapper, with rustfmt's
  at-least-128-byte buffer; the estimate is `write_list_capacity`.
- `overflow::Context::rewrite` reserves once for the callee, the delimiters, the list and both
  indents, writes the callee and the opening delimiter, writes the list in place, and
  `wrap_items` decides `force_single_line` from `result[items_start..]` exactly as from the list
  string, inserting the nested indent before the items when the list goes on its own lines.
- Forms measured before the commit: `out.reserve` on an empty `String` took
  `do_reserve_and_handle` (8,898 calls at ≈ 134 `Ir` against `with_capacity`'s path):
  248,202,217 → 247,320,662; `with_capacity` in the wrapper: 247,001,267; the buffer moved into
  a local while the list is written (`std::mem::take`): 246,674,459 (kept); a by-value form
  (`write_list_with(.., String) -> Option<String>`): 246,700,028. The loop in every form is
  0.17–0.43 M dearer than in `write_list` (spill and reload code at the call boundary, by
  `cmpsrc.py`), so the step keeps 87% of its site cost.

### Step 3: the chain's entry lists (`0f83db7` (was `7290457`))

- `make_subexpr_list` starts with room for eight entries instead of `vec![top]`; the item list
  with room for every sub-expression; the child list with room for every item after the parent.
  Capacities only.
- The chain's string assembly in one buffer was measured and rejected (below).

### Step 4: the overflowed argument (`804c400` (was `e6fe9c0`))

- `try_overflow_last_item` matches `placeholder` by value and moves the overflowed rewrite into
  the list item instead of `overflowed.to_owned()`.
- The placeholder's first line written into the item's own buffer measured as a no-op: the last
  item is never rewritten by `itemize_list` (`leave_last: true`), so its `item` is always `None`
  there (2,033 of 2,033 calls took the allocating branch). Reverted before the commit.

### Where `write_list`'s cost was (`5ddfd19` (was `c40a441`))

- `item_last_line_width` is computed in the one branch that reads it (a post-comment outside a
  horizontal list), from the same item, `item_sep_len` and indent; `prev_item_is_nested_import`
  is `formatting.nested && ..`, the only condition under which it is read. Both are pure
  functions of the item and the formatting.
- This is not the sink: it is the per-item width and newline scanning that T6's bookkeeping
  would answer from the buffer; here nothing needs answering, because the answers are read
  for 3 of 14,171 items and for nested import lists only. A separate commit, so that it can be
  taken without the rest.

## Results (measured)

`Ir` on the 49-file sample, one callgrind run per build. "Site cost" is the base's measured
cost of what the change removes; "parser growth" is the arcs from `ra_ap_*` into `Vec` growth,
which move with the heap layout the formatter leaves (EXP-016), and "net of it" is the change
without that term.

| commit | step | `Ir` before | `Ir` after | change | parser growth change | net of it | site cost | net / site | decision |
|---|---|---|---|---|---|---|---|---|---|
| `b9a279b` (was `8df88c3`) | 2, `write_list_into` | 248,202,217 | 246,674,459 | −1,527,758 (−0.62%) | +354,255 | −1,882,013 | 1.75 M | 108% (observed 87%) | kept |
| `0f83db7` (was `7290457`) | 3, chain entry lists | 246,674,459 | 244,224,959 | −2,449,500 (−0.99%) | −408,892 | −2,040,608 | 2.17 M | 94% | kept |
| — | 3, chain rewrites in one buffer | 244,224,959 | 246,227,944 | +2,002,985 (+0.82%) | +3,466,641 | −1,463,656 | ≈ 1.6 M | — | rejected |
| — | 3, the same, buffer sized from the chain's source | 244,224,959 | 246,720,522 | +2,495,563 (+1.02%) | +4,717,283 | −2,221,720 | ≈ 1.6 M | — | rejected |
| `804c400` (was `e6fe9c0`) | 4, overflowed argument moved | 244,224,959 | 243,593,585 | −631,374 (−0.26%) | −294,294 | −337,080 | 0.40 M | 83% | kept |
| `5ddfd19` (was `c40a441`) | `write_list`'s two scans | 243,593,585 | 238,033,138 | −5,560,447 (−2.28%) | 0 | −5,560,447 | 4.86 M | 114% | kept |
| **kept, total** | | 248,202,217 | 238,033,138 | **−10,169,079 (−4.10%)** | −348,931 | −9,820,148 | | | |

The gate runs of the committed builds read 246,675,864, 244,223,113, 243,593,647 and
238,033,124 (the script's run-to-run spread).

Against the brief's falsifier: `write_list` converted (step 2) moved `Ir` by 1.53 M, 87% of the
1.75 M of its string assembly a sink can remove and 47% of all its string assembly (3.27 M); the
"several percent for step 2 alone" expectation does not hold at this head, because
`write_list`'s cost is scanning (4.86 M), not assembly. The sink as a whole (steps 2–4, kept)
removed 4.61 M (1.86%), against the 8.8–10.5% bound.

### The heap-layout term (measured)

| build | parser `Vec` growth arcs | `grow_amortized` → `finish_grow` | `memcpy` inside `_int_realloc` (calls) |
|---|---|---|---|
| base | 11,230,572 | 6,485,228 | 3,437,653 (13,571) |
| step 2 | 11,584,827 | 6,691,373 | 3,630,477 (13,089) |
| step 3 (entry lists) | 11,175,935 | 6,422,102 | 3,346,613 (9,823) |
| step 3, chain buffer | 14,642,576 | 8,180,904 | 5,075,481 (12,752) |
| step 3, chain buffer sized | 15,893,218 | 8,820,854 | 5,714,363 (12,720) |
| step 4 | 10,881,641 | 6,305,201 | 3,175,654 (9,857) |
| head | 10,881,641 | 6,305,201 | 3,175,654 (9,857) |

The parser makes the same 20,209 growth calls in every build; what changes is whether its
event vectors grow in place. Changes to the formatter's allocation pattern move that cost by
−0.4 to +4.7 M, more than some of the steps themselves.

## Rejected (measured)

| attempt | measurement | mechanism | decision |
|---|---|---|---|
| step 3: the chain's rewrites in one buffer. `ChainFormatterShared` holds one `String` and the rewrites' ends instead of `Vec<String>`; the root's string becomes the buffer; absorbed, middle and last children are written in place through `ChainItem::rewrite_into` → `rewrite_call_into` → `overflow::rewrite_with_parens_into` → `Context::rewrite_into` (transactional at every level); the last child's first candidate is written in place and replaced by truncation when the second wins; the join returns the buffer itself when the chain fits on one line (no comment child is possible then: every write of `fits_single_line` is conjoined with `all_in_one_line`) and rebuilds with connectors otherwise. 0 of 1238 outputs changed. | 244,224,959 → 246,227,944 (+2,002,985) | net of the parser term −1.46 M (child strings, the join's allocation and the rewrites' frees gone), but the buffer grows by `realloc` as children are appended: `do_reserve_and_handle` from `Context::rewrite_into` +1.37 M (3,009 calls), the realloc copies, and the parser's growth +3.47 M | reverted; patch in `scratchpad/e20/s3b2.patch` |
| the same with the buffer reserved after `format_root` for the children's source length plus 64 bytes per child | 244,224,959 → 246,720,522 (+2,495,563) | the chain's own growth falls (Context's reserve +0.30 M), the reservation costs +1.04 M (1,955 `realloc`s), the parser term +4.72 M | reverted |
| step 4: the placeholder's first line written into the item's own string | 243,587,200 against 243,593,585 for the move alone (−6,385: the match's own code; no allocation removed) | the item is `None` there (`leave_last`), every call allocated as before | reverted |
| step 2 through `&mut String` without moving the buffer into a local, `write_list_capacity` out of line | 247,001,267 against 246,674,459 | +0.33 M together: the capacity helper as a call (614,287 for 10,952 calls, ≈ 0.2 M above its inlined form) and the loop's buffer accesses through the pointer (`write_list_into` 5.95 M against 5.83 M) | replaced |
| step 2 with `out.reserve` on an empty buffer | 247,320,662 | `do_reserve_and_handle` instead of `with_capacity`, ≈ +0.7 M | replaced |
| step 2 by value (`write_list_with(.., String) -> Option<String>`) | 246,700,028 | no better than the moved local; cannot leave a caller's buffer as it was on failure | replaced |

## Wall clock (measured, provisional)

`pairs.sh`: `taskset -c 3 bench -n 5` on the 1238 fixtures (13.9 MB), `bench-base` (the base
binary of this worktree, copied before the first change) and each kept step's binary, three
pairs, order alternating. Other agents' callgrind and builds ran on the same core and host
throughout; none of these ranges is a speedup claim.

| binary (`Ir` change against base) | window, UTC, load average (1, 5, 15 min) at start | base, MB/s per pair | head, MB/s per pair | head / base per pair | ranges |
|---|---|---|---|---|---|
| step 2, `b9a279b` (was `8df88c3`) (−0.62%) | 10:47–10:50, 5.93 7.90 8.75 | 3.03, 3.01, 3.12 | 2.74, 3.03, 4.24 | 0.904, 1.007, 1.359 | overlap |
| step 3, `0f83db7` (was `7290457`) (−1.60%) | 10:50–10:51, 5.05 6.70 8.16 | 4.41, 4.20, 4.13 | 4.23, 4.16, 4.26 | 0.959, 0.990, 1.031 | overlap |
| step 4, `804c400` (was `e6fe9c0`) (−1.86%) | 10:51–10:53, 4.70 6.18 7.82 | 2.64, 3.59, 5.06 | 4.33, 4.29, 7.67 | 1.640, 1.195, 1.516 | overlap |
| head, `5ddfd19` (was `c40a441`) (−4.10%) | 10:53–10:54, 5.45 5.93 7.52 | 8.42, 7.59, 7.82 | 8.16, 8.04, 8.88 | 0.969, 1.059, 1.136 | overlap |
| head again | 10:54–10:56, 4.81 5.61 7.30 | 8.02, 7.35, 7.81 | 7.32, 7.55, 7.94 | 0.913, 1.027, 1.017 | overlap |

The same base binary read 2.64–8.42 MB/s within ten minutes (M4's quiet host: 11.22–11.79 for
this code), while E21's resident process and other agents' callgrind runs shared core 3 and
four `rustc` processes the other cores. A −4.1% `Ir` change is inside that spread; the
wall-clock effect of the branch is not measured here, and a quiet-host run is owed
(binaries: `scratchpad/e20/bin/bench-{s2,s3a,s4,s5}`, `bench-base` in the worktree).

## Gates (measured)

| gate | corpus | result |
|---|---|---|
| `cargo fmt --all --check` | | clean on every kept commit |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | clean on every kept commit |
| `cargo test --workspace --all-features` | | 291 passed, 0 failed, on every kept commit (one test added by step 2) |
| `bench --check`, sequential and `--threads 4` | 1238 fixtures against `fixtures-e8d87c2.tsv` | 0 changed; 0 differ between 4 threads and `format_source`; every kept commit |
| `bench --check` | 25,024 registry files against `registry-e8d87c2.tsv` (one not recorded) | 0 changed, every kept commit |
| `conform -i`, `conform -i -V 1.8.0` | 1220 fixtures | 1220/1220 identical, 0 not idempotent, at both versions, every kept commit |
| `benchmarks/profile.sh` | 49-file sample | the *Results* table |

`config_matrix` (gates 6 and 7) was not run: no output changed on either hash corpus.

## Tests added (read)

| test | file | compares |
|---|---|---|
| `write_list_into_appends_write_list_or_leaves_the_buffer` | lists.rs | `write_list_into` after four prefixes (empty, `f(`, one ending a line, 120 columns) with `write_list`, over six lists (comments before and after, a multi-line item, an empty first item, a failing item), five tactics, three trailing-separator tactics, two separator places, three widths and the flags: equal text after the prefix, or the prefix alone where `write_list` fails |

The test was checked against four mutations: the vertical emptiness test reading the whole
buffer, either last-line width reading the whole buffer (the alignment one needed a
`SpecialMacro(2)` list with a narrower last item), and the truncation on failure dropped. Each
fails it.

## Interpretation

- The sink works where a string is built only to be copied once into the next level and the
  receiving buffer is allocated once at a known size: `write_list` into `wrap_items`'s buffer
  removed 87% of its site cost (one allocation, one copy and one free per call-like list).
- It loses where the receiving buffer has to grow: appending a chain's children to the root's
  string replaces small exact allocations by `realloc`s of a growing buffer, which are dearer
  and, through the heap layout they leave, make the parser's event vectors move on the next
  file. With glibc, one exact allocation per level and one copy is cheaper than one buffer
  grown by `realloc`, unless the final size is known before the first byte is written.
- The width and newline bookkeeping T6 attaches to the sink addresses scans whose answers are
  mostly not read: in `write_list` the last-line width is read for 3 of 14,171 items and the
  nested-import test only in import lists. Computing them where read removed 5.56 M, more than
  the sink steps together.
- A change of ±0.3–4.7 M in the parser's `Vec<Event>` growth accompanies any change to the
  formatter's allocation pattern. Allocation levers below about 2 M cannot be judged from total
  `Ir` alone until that growth is made layout-independent (P3 of the parser record, a fork).

## Decision

Kept: `b9a279b` (was `8df88c3`), `0f83db7` (was `7290457`), `804c400` (was `e6fe9c0`), `5ddfd19` (was `c40a441`), −10,169,079 `Ir` (−4.10%) with
byte-identical output on both hash corpora and `conform` at both versions; of it −4.61 M the
sink steps and −5.56 M the two scans of `write_list`, which the integrator can take or drop
separately. Rejected: the chain's rewrites in one buffer (two forms). The hypothesis holds for
the one-copy site (`write_list` into the call's buffer) and fails for incremental appends to a
growing buffer under glibc; on this evidence the 8.8–10.5% bound is not reachable by
converting signatures alone (**derived** from the two sites measured).

Merge note: E22 may put `#[inline(never)]` or `#[cold]` on `write_list`; on this branch
`write_list` is a three-line wrapper and the body is `write_list_into`/`write_list_at`, so such
an attribute belongs on `write_list_into`.

## Follow-up

What is left of the sink's pool at the head (formatter allocator arcs 27.91 M, 11.73%), by
site (**measured**, `sites.py` on the head):

- The argument strings themselves: `overflow`'s drop of `list_items` frees 16,924 blocks
  (1.46 M) that `itemize_list` allocated, one per argument, each copied once into the list.
  Removing them needs list items as ranges of a shared buffer (rendered in order, the layout
  decided from their ranges, the list written after them and moved down), a change to
  `ListItem` and its fifteen users, with the growth problem above.
- `types::rewrite_path_segments` 1.51 M and `rewrite_path` 0.99 M, `push_rewrite_inner` 0.99 M,
  `memoize`'s stored clone 0.71 M + 0.21 M (S9), `macros::rewrite_macro` 0.86 M,
  `attr::rewrite_attrs` 0.75 M, `closures::rewrite_closure_fn_decl` 0.64 M, the chain's strings
  (`Chain::rewrite` 1.49 M, `ChainItem::rewrite` 1.14 M), `FmtVisitor::from_context` 1.18 M
  (S10, rejected in E16 for the same layout reason), `get_context`'s `Rc` pair 0.56 M.
- Before any further allocation lever: make the parser's vector growth independent of the
  formatter's heap layout (size `Vec<Event>` from the token count in a patched `ra_ap_parser`,
  P3), or report `Ir` net of the parser growth arcs next to the total, as the *Results* table
  does, so that levers of 0.5–2 M can be judged.
- The scans that remain in the list and chain paths: `wrap_str`'s `filtered_str_fits` over
  every chain (1.47 M), `last_line_width(ident)` and `first_line_width(items)` in `wrap_items`
  (1.00 M), the candidates' `lines().count()` in `format_last_child` (0.59 M); each needs its
  own read-site argument like the two removed here.
