# EXP-010: fused tree build over `Output` (feasibility, no code)

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md); examines lever 3
of [../profile-flat.md](../profile-flat.md) ("steps decoded twice"). Labels: **measured**
(callgrind data of that profile, `$S/cg/prof.out`), **read** (from source), **hypothesis**.
`P` = `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ra_ap_parser-0.0.307/src`;
`intersperse_trivia` lives in `P/shortcuts.rs`, not in `lexed_str.rs`.

## Pipeline as read

- `TopEntryPoint::parse` (`P/lib.rs:91-127`): grammar → `p.finish()` → `event::process(events)`
  (`P/lib.rs:105`), which resolves `forward_parent` chains and encodes each step as one `u32`
  in `Output` (`P/event.rs:91-138`, `P/output.rs:81-112`).
- `mod event;` and `mod parser;` are private (`P/lib.rs:28,34`); `Event` is `pub(crate)`
  (`P/event.rs:16`). No public API yields events before `process`, so `event::process`
  (15.17 M exclusive) is not reachable without a fork.
- `Output::iter` (`P/output.rs:51-79`) is the only public reader of `Output` (fields private,
  `P/output.rs:22-23`). `intersperse_trivia` (`P/shortcuts.rs:65-99`) is its single consumer
  walk: it decodes each step once and calls `sink` with a `StrStep` (`P/shortcuts.rs:20-25`).
  There is no second walk over `Output`; the second representation is the `StrStep` stream.
- chloro: `TreeBuilder::steps` (`chloro-core/src/cst/mod.rs:269-288`) is that `sink`; it uses
  `text.len()` only and treats any `Error` as failure. Callers: `Tree::parse` (`:99-107`),
  `parse_with_errors` (`:112-119`), `from_steps_at` (`:132-142`), `formatter/formatting.rs:50`,
  `formatter/macro_args.rs:297,318`. `formatter/rustc_syntax.rs:163` (`owned_tree`, rejected
  files only) reads `Error { msg, pos }` and keeps `intersperse_trivia`.

## Public API a fused builder needs (all present in 0.0.307; no fork)

- `Output::iter` → `Step` (`P/output.rs:27-33,51`), re-exported at `P/lib.rs:51`.
  `Step::Token { kind, n_input_tokens }` covers `n_input_tokens` lexed tokens (glued tokens
  such as `>>=`); `FloatSplit { ends_in_dot }`; `Enter { kind }`; `Exit`; `Error { msg }`.
- `LexedStr::len` (`P/lexed_str.rs:87`), `kind(i)` (`:95`, asserts `i < len`), `text(i)` (`:100`,
  needed for `FloatSplit` only), `text_start(i)` (`:118`, asserts `i <= len`), `errors` (`:134`).
  Token length = `text_start(pos + n) - text_start(pos)`, which replaces `range_text`
  (`:104-110`, a `&str` slice with char-boundary checks).
- `SyntaxKind::is_trivia` (`P/syntax_kind.rs:29-31`: `WHITESPACE | COMMENT`).
- Private, so copied into chloro (about 150 lines): the `Builder` state machine
  (`P/shortcuts.rs:102-237`), `n_attached_trivias` (`:239-275`), `is_outer` (`:277-282`),
  `is_inner` (`:284-286`).

## Rules `intersperse_trivia` applies (read), which a fused builder reproduces

1. The first `Enter` is emitted with no trivia handling (`:138-143`); leading trivia of the
   file lands inside the root node.
2. `Exit` is deferred: it sets `PendingExit` (`:160-166`); a second `Exit` while pending emits
   the first. Every `Token`, `FloatSplit` and `Enter` emits a pending `Exit` first
   (`:119,129,144`). Result: trivia after a node's last token never lands inside that node;
   it lands in the parent or the next node.
3. `Token`: after the pending `Exit`, all trivia up to the next non-trivia token is emitted
   one token each at the current depth (`eat_trivias`, `:168-176`), then the token with the
   text of `n_input_tokens` lexed tokens (`do_token`, `:186-190`).
4. `Enter { kind }` (not first): count the trivia run ahead (`n_trivias`, `:148-150`); call
   `n_attached_trivias(kind, run reversed)`; emit `n_trivias - n_attached` trivia before the
   `Enter` (in the parent), then `Enter`, then `n_attached` trivia inside the node (`:151-157`).
5. `n_attached_trivias` (`:239-275`) is non-zero only for `CONST ENUM FN IMPL MACRO_CALL
   MACRO_DEF MACRO_RULES MODULE RECORD_FIELD STATIC STRUCT TRAIT TUPLE_FIELD TYPE_ALIAS UNION
   USE VARIANT EXTERN_CRATE`. Walking back from the node: whitespace without `"\n\n"` is
   skipped; whitespace with `"\n\n"` stops the walk unless the trivia before it is an outer
   doc comment (`///`, `/**`, not `////`, `/***`; `:252-261`); an inner comment (`//!`, `/*!`)
   stops it (`:263-265`); any other comment, plain `//` included, extends the attached run to
   include it (`res = i + 1`, `:266`). Attributes are not trivia: the item marker starts
   before `outer_attrs` (`P/grammar/items.rs:49-50`), so comments above `#[..]` attach inside
   the item, before its `ATTR` child.
6. `FloatSplit` (`:126-134,192-236`): pending `Exit`, trivia, then the one `FLOAT_NUMBER`
   lexed token split at `.`: `Enter NAME_REF`, `INT_NUMBER` (left), `Exit`, `Exit` (the
   parser's `Finish` after the hack is removed in `P/event.rs:129-133`), `DOT` (len 1); if not
   `ends_in_dot`: `Enter NAME_REF`, `INT_NUMBER` (right), `Exit`, state `PendingExit`; else
   state `Normal`. No `.` in the text (`1e0`): `Error "illegal float literal"`, `Enter ERROR`,
   `FLOAT_NUMBER`, `Exit`, `Exit`, state by `ends_in_dot`. `pos` advances by 1.
7. `Error { msg }` (`:82-85`) emits an error at `text_start(pos)` and leaves the state as is.
   For `TreeBuilder` this is a flag (`cst/mod.rs:285`).
8. End: state is `PendingExit`; remaining trivia is emitted inside the root, then the root's
   `Exit` (`:89-95`). The returned `pos == len` flag is ignored by chloro (`cst/mod.rs:271`).

## Cost split (measured, 49-file sample, total 390,496,251 Ir)

| item | Ir | % |
|---|---|---|
| `intersperse_trivia` inclusive (1,018 calls, files and fragments) | 31.34 M | 8.03 |
| — exclusive (decode via inlined `Output::iter`, state machine, trivia scan) | 13.74 M | 3.52 |
| — `TreeBuilder::steps::{closure#0}` called directly (242,198 calls) | 9.86 M | 2.52 |
| — `Builder::eat_n_trivias` (166,268 calls; holds the other 20,822 callbacks, 0.83 M) | 5.13 M | 1.31 |
| — `LexedStr::range_text` (66,909 non-inlined calls) | 2.47 M | 0.63 |
| callback total (263,020 calls) | 10.69 M | 2.74 |
| — closure entry and `match step` (`cst/mod.rs:271`) + closure exit (`:287`) | 3.16 + 2.10 = 5.27 M | 1.35 |
| — element writes: `Vec::push` 1.90 + `ptr` 1.06 + `RawVec` 0.43 | 3.39 M | 0.87 |
| `event::process`, inside `TopEntryPoint::parse` | 15.17 M | 3.88 |

## Upper bound

- Not reachable: `event::process` 15.17 M, which lever 3's 28.91 M (7.4%) contains.
- Ceiling: 31.34 M inclusive − 3.39 M element writes = 27.95 M (7.16%), with the `Output`
  decode, state machine, trivia scan and `kind` asserts at zero; a fused builder runs them all.
- Attributable to the `StrStep` layer: closure entry/dispatch/exit 5.27 M + `range_text`
  2.47 M = 7.74 M (1.98%). Hypothesis: the fused saving lies near this figure plus the part
  of the 13.74 M exclusive and 5.13 M `eat_n_trivias` spent building `StrStep` values and
  `&str` slices, and below 27.95 M. No new `unsafe` (protocol.md:22): the asserts remain.

## Risk

- Any divergence from rules 1-8 moves trivia across a node boundary: `Elem.start`/`len`/`end`
  change, so `text_range`, `first_token`, `children_with_tokens` and comment lookups change,
  and so does formatted output; rules 5 (doc comments, blank lines) and 6 (floats) included.
- The copy is pinned to 0.0.307: a bump that changes `P/shortcuts.rs` changes rust-analyzer's
  attachment and not chloro's (nor `owned_tree`'s, which keeps `intersperse_trivia`).

## Smallest experiment

1. Write `TreeBuilder::steps_fused(&mut self, lexed: &LexedStr<'_>, output: &Output) ->
   Option<()>` in `chloro-core/src/cst/mod.rs`: one loop over `output.iter()` with `pos: usize`
   and a pending-exit flag, ports of `n_attached_trivias`/`is_outer`/`is_inner`, token
   lengths from `text_start`, calling `token`/`start_node`/`finish_node` directly.
2. Equivalence check (decides correctness before timing): build each tree with `steps` and
   `steps_fused`, compare the `Elem` sequences field by field (`Elem` derives no `PartialEq`,
   `cst/mod.rs:38`) over the fixture trees, the registry corpus and every macro fragment
   `macro_args.rs` re-parses. One mismatch rejects.
3. Switch `from_steps_at` and `macro_args.rs:297` to `steps_fused`; run gates 1, 2 and 4
   (protocol.md:13,16,22) and callgrind Ir on the 49-file sample in the `target/prof2`
   configuration. The Ir delta against 7.74 M (attributable) and 27.95 M (ceiling) decides.
