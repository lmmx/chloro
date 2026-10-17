# EXP-017: the parse floor, three exact levers

Ledger row E17 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md); the levers are P8, P5 and P1 of the ranked bounds in
[../../2026-10-15-extensions-theory/parser-and-substrate.md](../../2026-10-15-extensions-theory/parser-and-substrate.md)
(the parse floor is 38.4% of the sample's instructions), P1 as specified by
[EXP-010-fused-tree-build-feasibility.md](EXP-010-fused-tree-build-feasibility.md). Labels:
**measured** (a tool's output on the builds below), **read** (from source).

## Identity

| field | value |
|---|---|
| branch | `20261015-parse-floor` |
| base | `7001c6a` (was `d179b50`) (integration head; `chloro-core` is the code of `bd9c768` (was `bdfb9d1`), the best-known-good's formatter) |
| commits | `c9a8863` (was `d8b066e`) (P8), `f9c7799` (was `0479cbc`) (P5), `a908a7b` (was `d14d259`) (P1), then this record |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, valgrind 3.22.0, `ra_ap_parser` 0.0.307 |
| host | 4 vCPU Intel Xeon @ 2.80GHz; other experiments ran on the host throughout (load average 3.4–7.4). `Ir` is unaffected by load; the wall-time figures below are not quiet-host figures |
| build | `cargo build --release -p chloro-core --example bench --example conform` (the workspace's fat-LTO, one-codegen-unit profile) |
| `Ir` command | `benchmarks/profile.sh --core 3 --runs 1 --sample /tmp/chloro-bench/pfloor49` (a private sample path of the default's length; the base binary measured as a copy at `target/release/examples/bencB`, a path of the same length as the head's) |
| per-file `Ir` | `taskset -c 3 env -i setarch $(uname -m) -R valgrind --tool=callgrind bench -n 1 --root DIR`, `DIR` holding one fixture file: `rust-analyzer/hir/src/lib.rs` (227,843 B) or `rust-analyzer/rust-analyzer/src/config.rs` (167,117 B). These are the raw fixtures, not the formatted copies the theory record measured, so the figures are close to but not the same as its 4.55% and 6.62% |
| golden hashes | `scratchpad/golden/fixtures-e8d87c2.tsv` and `registry-e8d87c2.tsv` (the best-known-good's) |
| equality builds | `CARGO_TARGET_DIR=target/verify CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true cargo build --release -p chloro-core --example bench`, with an uncommitted `eprintln!` per checked tree or reuse for the counts, run as `bench -n 1 --root R --check GOLDEN` over both corpora; removed after use |

## Hypothesis and falsification

Three exact, fork-free levers from the theory record remove a measurable part of the parse
floor on the 49-file sample (312,732,995 `Ir` at the base, this run) with byte-identical
output: P8 (bound 0.63%), P5 (exact part 1.94%, ceiling 2.66%), P1 (likely 2.47%, ceiling
8.94%). Each lever is falsified on its own if `Ir` does not fall by more than 0.1% of the
start, or if any output or tree element changes; P1 additionally needs element-by-element
equality of the fused tree with the current build over every file of both corpora and
every macro fragment before any output gate.

## Commits (measured)

| commit | lever | `Ir` before | `Ir` after | change | of the start |
|---|---|---|---|---|---|
| `c9a8863` (was `d8b066e`) | P8: `in_trait` passed to `visit_assoc_item` | 312,732,995 | 310,877,081 | −1,855,914 | −0.59% |
| `f9c7799` (was `0479cbc`) | P5: Expr/Ty fragments built from the prefix parse | 310,877,081 | 304,645,699 | −6,231,382 | −1.99% |
| `a908a7b` (was `d14d259`) | P1: the tree built from `Output` in one pass | 304,645,699 | 290,493,311 | −14,152,388 | −4.53% |
| total | | 312,732,995 | 290,493,311 | −22,239,684 | **−7.11%** |

Per file (P8's own gate, then the head):

| file | base | after P8 | head (P8+P5+P1) |
|---|---|---|---|
| `hir/src/lib.rs` | 213,902,439 | 204,281,608 (−4.50%) | 192,036,537 (−10.22%) |
| `config.rs` | 85,447,231 | 79,833,401 (−6.57%) | 74,950,567 (−12.29%) |

### P8: `visit_assoc_item` (`c9a8863` (was `d8b066e`))

- Before: `visit_assoc_item` called `parent()` twice (item → `ASSOC_ITEM_LIST` → `TRAIT` or
  `IMPL`); `parent()` scans backwards over the preceding siblings' subtrees, so an impl with
  *n* items cost O(n × impl size) (theory record § 6).
- Exactness (read): it has two callers. `visit_impl_items`, called only by `format_impl`
  with the items of `iimpl.assoc_item_list()`, whose grandparent is `IMPL` (false);
  `format_trait`'s loop over `trait_.assoc_item_list()`, whose grandparent is `TRAIT`
  (true). Each now passes that constant. A debug assertion recomputes the search on every
  associated item the debug tests (and the debug-assertion equality builds of P5 and P1
  over both corpora) format.
- Measured: −1,855,914 against a bound of 1,962,731 (95%); the two large files lose
  4.50% and 6.57%, against bounds of 4.55% and 6.62% on their formatted copies.

### P5: macro arguments parsed once (`f9c7799` (was `0479cbc`))

- Before (read, `macro_args.rs`): each argument window was lexed and prefix-parsed to
  find its end, then the consumed tokens were lexed, converted to parser input and parsed
  again with the `TopEntryPoint` to build the fragment tree.
- Change: `prefix_parse_window` keeps the window's `LexedStr` and `Output`; `build` takes
  them when `PrefixParse::matches_top_parse` holds. All three Expr build sites
  (`parse_fragment`, `parse_expr`, `parse_lazy_static`) pass them.
- Exactness (read, `ra_ap_parser` `grammar.rs`, `parser.rs`, `event.rs`): reuse only for
  `Expr` and `Ty`, without a parser error, when the prefix parse consumed the whole window.
  Then the fragment's text is the window's text and lexes the same; the top entry opens a
  marker, runs the same grammar function (`expressions::expr`, `types::type_`) and abandons
  the marker at EOF; the abandoned marker is a tombstone `Start` that `event::process` drops,
  and `forward_parent` offsets are relative, so the two outputs hold the same steps. `Pat`
  (`pattern_single` against `pattern`) and `Item` keep the second parse.
- Equality gate (measured, debug-assertion build: every reuse re-parses with the top entry
  and asserts the same steps): fixtures 14,258 reused builds (14,110 Expr, 148 Ty), 212
  parsed again; registry 461,182 reused (459,451 Expr, 1,731 Ty), 163,865 parsed again
  (158,391 Expr whose prefix parse stopped before the end of its window, 4,452 Ty, 709 Pat,
  313 Item); no difference, 0 outputs changed.
- Measured: −6,231,382, slightly above the record's exact part (6,076,839), which counted
  `parse_fragment`'s builds only; `parse_macro_arg` inclusive 15,530,086 → 9,168,802.
- Unit test `reused_prefix_parse_builds_the_same_tree`: eight arguments (split floats,
  closures, casts with a trailing comment, `>>`, ranges, references to arrays, `async`
  blocks), trees built both ways compared element by element.

### P1: interspersal fused with the tree build (`a908a7b` (was `d14d259`))

- Before (read): `TreeBuilder::steps` ran `LexedStr::intersperse_trivia` with a `dyn FnMut`
  sink; each step was decoded from `Output`, each token's text sliced (`range_text`, with
  char-boundary checks), wrapped in a `StrStep`, and matched again in the closure.
- Change: `steps` runs `steps_fused`, one loop over `Output::iter` that keeps the
  interspersal state machine and calls `token`/`start_node`/`finish_node` directly, token
  lengths from `text_start`. The old path stays as `steps_interspersed`. About 150 lines of
  `shortcuts.rs` are ported: the deferred `Exit` (`PendingExit`), the first `Enter` without
  trivia, trivia before tokens at the current depth, the run before an `Enter` split by
  `n_attached_trivias` (copied with `is_outer` and `is_inner`), split floats (with and
  without a trailing dot, and the dot-less error case), the trailing trivia inside the
  root, and the `unreachable!` states and float asserts. `steps` covers every tree the
  formatter builds: files, macro fragments, `parse_maybe_bound`'s synthetic node and
  `parse_with_errors`. `rustc_syntax::owned_tree` keeps `intersperse_trivia` (it reads
  error positions).
- Equality gate (measured, before the output gates): in debug builds `steps` also runs
  `steps_interspersed` on a copy of the builder and asserts equal elements (kind, class,
  `end`, `start`, `len`; `Elem` now derives `PartialEq`), open-node stack, offset and
  result. The debug-assertion release build over both corpora:

| corpus | file-level trees (id 0) | of which with parser errors | their elements | fragment trees | their elements | mismatches |
|---|---|---|---|---|---|---|
| fixtures (1238 files) | 1,330 | 188 | 5,046,996 | 14,579 | 138,566 | 0 |
| registry (25,024 files) | 34,008 | 3,488 | 171,363,172 | 625,316 | 6,269,462 | 0 |

  File-level trees outnumber files because the formatter also parses whole texts (macro
  bodies, code blocks in doc comments, re-parses); files that fail to parse are compared
  too. The first run of the gate passed; no rule needed a second attempt.
- Unit tests: `fused_build_equals_the_interspersed_build` (split floats `t.0.1`, `x.0 .1`,
  `x.0. bar`, `x.0. await` — asserted to contain a split ending in a dot — and `x.1e0`; a
  doc comment, a blank line, an item; `////` and `/***`; a plain comment after a blank
  line; an inner doc comment; trailing trivia at the end of a block and of the file; a
  comment above and between attributes; a `macro_rules!` body with comments; fields and
  variants with comments; Expr, Type and MacroItems fragments; a parse error; the empty
  file; the rowan test's sources) and
  `fused_build_equals_the_interspersed_build_on_the_fixtures` (every one of the 1238
  fixture files, parsing or not, 4.5 s in a debug build).
- Measured: −14,152,388, against the record's likely 7.74 M and ceiling 27.95 M;
  `intersperse_trivia` inclusive 31,523,220 → `steps_fused` 17,199,232 (−45%);
  `from_steps_at` 31,703,492 → 17,561,260; `formatting::parse` 102,158,956 → 88,441,360.

## Rejected levers

None. Each lever fell by far more than 0.1% of the start (312,733 `Ir`) with no output and
no tree element changed.

## Gates (measured, at each commit unless stated)

| gate | scope | result |
|---|---|---|
| `cargo fmt --all --check` | | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | no warnings |
| `cargo test -p chloro-core` | 183 library tests at P8, 184 at P5, 186 at P1, plus the integration batches | all pass |
| `bench -n 1 --check fixtures-e8d87c2.tsv` | 1238 fixtures, sequential and `--threads 4` | 0 of 1238 changed |
| `bench -n 1 --check registry-e8d87c2.tsv` | 25,024 registry files, 20,850 recorded (4,174 not recorded: crates fetched since) | 0 changed |
| `conform -i`, `conform -i -V 1.8.0` | 1220 fixtures | 1220/1220 identical, 0 not idempotent, both versions |
| `cargo test --workspace --all-features` | at the head | 267 passed, 0 failed, 1 ignored |
| `git diff --check` | every commit | clean |

`config_matrix` (gate 6) and the registry `config_matrix` against live rustfmt (gate 7)
were not run: no output changed on either hash corpus.

## `bench -n 5` pairs (measured, loaded host)

`taskset -c 3`, base binary built from `7001c6a` (was `d179b50`) (kept as `bencB`), interleaved, load
average 5.1–6.9 during the run, so the absolute figures are about half of M3's quiet-host
9.4 MB/s and are not comparable to them:

| pair | base | head | ratio |
|---|---|---|---|
| 1 | 4.45 MB/s | 4.88 MB/s | 1.097 |
| 2 | 4.44 MB/s | 4.74 MB/s | 1.068 |
| 3 | 4.43 MB/s | 4.89 MB/s | 1.104 |

Ranges 4.43–4.45 and 4.74–4.89 do not overlap. A quiet-host replay is needed for a
figure to cite.

## What surprised

- P1 saved nearly twice the record's likely figure. The record attributed 7.74 M to the
  `StrStep` layer and `range_text`; the fused loop also removed most of the decode and
  state-machine cost it had left in the ceiling, because the trivia scans and element
  writes now inline into one loop without a call per element. What remains of
  interspersal and build (17.2 M) is still about three times the record's floor estimate
  (≈ 5.4 M).
- On the registry, 158,391 Expr builds (a quarter of all fragment builds) are not reusable
  under P5's condition: the prefix parse stopped before the end of its window without an
  error (rustc's greedy parse accepts the argument and the remaining tokens become the next
  slot). Their build text is a prefix of the window, so its lexed tokens are a prefix of
  the window's; reusing them is not exact by the argument above (a different `LexedStr`,
  and the top entry would see EOF where the prefix parse saw more tokens).
- P8 reached 95% of its bound: the backward search was almost all of the cost.

## Decision

Kept: `c9a8863` (was `d8b066e`), `f9c7799` (was `0479cbc`), `a908a7b` (was `d14d259`). Branch `20261015-parse-floor`, for the integrator's
gates 1–5 on the merged tree; not merged.

## Follow-up

- P1's copy of `n_attached_trivias` and the interspersal state machine is pinned to
  `ra_ap_parser` 0.0.307: a parser bump must be followed by the equality tests (the debug
  check in `steps` runs on every tree the debug tests build, and the fixture test compares
  every fixture).
- Next on the same floor, by the theory record's ranking: P3 (parser vectors sized up
  front, a fork), P2 (own token conversion; it needs a builder that does not go through
  `intersperse_trivia`, and P1's reads the lexer's output only through `kind`,
  `text_start` and `text`), P7 (`rustc_compat` run inside the build, now that the build is one
  loop).
- A quiet-host `bench -n 5` triple against the best-known-good.
