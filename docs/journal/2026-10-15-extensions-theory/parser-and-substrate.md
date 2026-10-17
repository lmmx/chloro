# 2026-10-15: The parse floor and the substrate (theory)

Part of [../2026-10-15-extensions-theory.md](../2026-10-15-extensions-theory.md). Labels: measured, read, derived, hypothesis. No code was changed.

## Question

What the lexer, the parser, the trivia interspersal, the flat tree build, `rustc_compat` and
the macro-argument reparse cost at the final head; how far each of these can be cut or moved
off the critical path (parallel parsing on the flat array, per-item and streaming substrates,
an own lexer or parser, fused passes, the tree's own layout); and what remains if every lever
lands. Each bound states the arithmetic, the exactness risk, the cost and the smallest
experiment that would refute it.

## Method

`$P` = `/tmp/claude-0/-home-user-chloro/9623d6dc-d933-5682-9d83-7762284f5c7f/scratchpad/profile-final`
(the final-head profile, `bench -n 1` on the 49-file sample, 312,733,055 Ir, code of
`bd9c768` (was `bdfb9d1`)); `$S` = `.../scratchpad/theory-parser`; `$B` = `target/release/examples`;
`RA` = `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ra_ap_parser-0.0.307/src`;
`LX` = `.../ra-ap-rustc_lexer-0.137.0/src` (the version in `Cargo.lock`). The five large files
are the formatted fixture copies of M3's stdin workloads (`lints.rs` 639,771 B, `nodes.rs`
358,382 B, `hir/src/lib.rs` 236,066 B, `hover/tests.rs` 210,967 B, `config.rs` 169,196 B).

```text
callgrind_annotate --inclusive=yes $P/callgrind.1.out                     # $S/incl.txt
callgrind_annotate --inclusive=yes --tree=both $P/callgrind.1.out         # $S/tree.txt (callers, callees, call counts)
python3 grouping of $P/annotate-all.txt (1,211 functions) by name prefix  # exclusive Ir per component
callgrind_annotate --show=Ir,D1mr,Bi,Bim,Bc,Bcm --sort=Bim|D1mr $P/callgrind.cachesim.out
$B/dump_tree FILE | wc / item script                                       # elements, nodes, tokens, top-level items
valgrind --tool=callgrind $B/bench -n 1 --root $S/files/<f>   (×5, in parallel)   # $S/cg.<f>.out
valgrind --tool=massif $B/fmt_stdin < nodes.rs | lints.rs                  # $S/massif.<f>.out
$B/alloc_count $S/files/nodes | lints                                      # allocation size histogram
rustc -O $S/sz.rs                                                          # size_of Event (copied layout), Elem, Elem + parent
```

Total compute: about 4 minutes (five callgrind runs and two massif runs in parallel, the
rest seconds). Load average was 3.6–4.4 (other sessions' builds), so no wall time was taken;
every time figure below is M3's (`../2026-10-14-autoresearch/measurements/m3-quiet-host.md`,
2.80 GHz host) or derived from it.

## 1. Where the parse floor's instructions go

### By caller path, inclusive (measured, `$S/tree.txt`)

| path | calls | inclusive Ir | share of 312,733,055 |
|---|---|---|---|
| `formatting::parse` (the files) | 55 lexes, 53 parses | 102,088,548 | 32.64% |
| — `LexedStr::new` | 55 | 25,363,749 | 8.11% |
| — `LexedStr::to_input` | 53 | 4,282,083 | 1.37% |
| — `TopEntryPoint::parse` | 53 | 36,044,457 | 11.53% |
| —— grammar (`entry::top::source_file`) | 53 | 21,724,765 | 6.95% |
| —— `event::process` (derived: 36.04 M − 21.72 M) | 53 | ≈14,320,000 | 4.58% |
| — `Tree::from_steps_at` (`intersperse_trivia` + `TreeBuilder`) | 53 | 30,356,488 | 9.71% |
| — `rustc_compat::subtree_rejected_by_rustc` (`walk`) | 46 | 5,716,661 | 1.83% |
| — `rustc_syntax::reparse` | 9 | 261,934 | 0.08% |
| macro arguments: `MacroTokens::new` (lexes each token tree) | 702 | 2,338,783 | 0.75% |
| macro arguments: `parse_macro_arg` | 995 | 15,530,086 | 4.97% |
| — `prefix_parse` (lex, `to_input`, `PrefixEntryPoint`) | 995 | 6,946,086 | 2.22% |
| — `build` (lex again, `to_input`, `TopEntryPoint`, tree, `rustc_compat`) | 955 | 8,357,896 | 2.67% |
| **parse-related total** (the three roots) | | **119,957,417** | **38.36%** |

- Every call of `LexedStr::new`, `TopEntryPoint::parse` and `intersperse_trivia` in the profile
  descends from one of the three roots (measured, caller lists in `$S/tree.txt`).
- A macro argument's text is lexed three times: once in `MacroTokens::new`, once per window in
  `prefix_parse`, once in `build` (read, `chloro-core/src/formatter/macro_args.rs:125,200,314`),
  and parsed twice, by a `PrefixEntryPoint` and then a `TopEntryPoint`
  (`macro_args.rs:202,316`).

### By component, exclusive (measured, `$P/annotate-all.txt`)

| component | functions | exclusive Ir | share |
|---|---|---|---|
| lexer cursor | `ra_ap_rustc_lexer::*` (`advance_token` 8.70 M, `eat_while` 3.15 M, …) | 13,272,864 | 4.24% |
| token conversion | `LexedStr::new` 7.75 M, `Converter::push` 6.02 M, `frontmatter` | 13,867,514 | 4.43% |
| parser input | `LexedStr::to_input` | 4,019,992 | 1.29% |
| grammar | `ra_ap_parser::grammar::*` | 13,612,156 | 4.35% |
| parser machinery | `Marker::complete` 4.11 M, `expect`, `nth_at`, `precede`, … | 5,972,547 | 1.91% |
| event replay | `event::process` | 15,166,361 | 4.85% |
| trivia interspersal | `intersperse_trivia` 13.78 M, `Builder::eat_n_trivias` 3.54 M, `range_text` 4.02 M | 21,343,578 | 6.82% |
| other parser | `grow_one` instances, `text_range` | 151,026 | 0.05% |
| = parser and lexer (the orchestrator's 87.4 M) | | 87,406,038 | 27.95% |
| tree build | `TreeBuilder::steps::{closure#0}` 10.69 M, `new` | 10,741,797 | 3.43% |
| `rustc_compat` | `subtree_rejected_by_rustc` 5.69 M, `doc_token`, … | 5,947,498 | 1.90% |
| `macro_args` own code | | 1,184,607 | 0.38% |
| `rustc_syntax` | `owned_tree` closure, `reparse`, … | 61,201 | 0.02% |
| allocator, `memcpy`, `free` under the three roots (derived: 119.96 M − 105.34 M) | | ≈14,616,000 | 4.67% |

- Vector growth inside the parser crate, counted in the allocator row of the top-level table
  (measured, `grow_one` inclusive by caller): `RawVec<Event>` 6,351,660 (5,948 calls),
  `RawVec<u32>` from `event::process` 2,025,132 and from `Converter::push` 1,268,335,
  `RawVec<SyntaxKind>` 1,128,408, `RawVec<u64>` from `to_input` 264,256; total 11,037,791
  (3.53%).
- Per unit (derived; the sample lexes 103,559 tokens in all calls, `Converter::push` calls;
  `intersperse_trivia` makes 263,020 callbacks = tokens + node enters + node exits; the
  sample's 42 parsing files hold 170,126 elements, 89,688 tokens, 62,115 non-trivia tokens,
  80,438 nodes by `dump_tree`; the other 7 files hold 372 bytes):

| stage | cost |
|---|---|
| lexing, main files | 25,363,749 / 650,213 B = 39.0 Ir/byte; ≈ 282 Ir/token at ≈ 90 K tokens |
| lexing, all calls | 30,407,936 / 103,559 = 294 Ir/token, of which cursor 128 and conversion 134 |
| `to_input`, main files | 4,282,083 / 650,213 B = 6.6 Ir/byte; ≈ 48 Ir/token |
| `event::process` | 15,166,361 / 265,301 events = 57 Ir/event (event count = its indirect-branch count, one jump-table dispatch per event; hypothesis that nothing else in it branches indirectly) |
| `intersperse_trivia` without the sink | 21,343,578 / 263,020 = 81 Ir/callback |
| `TreeBuilder` sink | 10,741,797 / 263,020 = 41 Ir/callback |
| `from_steps_at`, main files | 30,356,488 / 170,126 elements = 178 Ir/element |
| `rustc_compat` walk, main files | 5,716,661 / 170,126 = 34 Ir/element |

## 2. Parallel parsing on the flat substrate

### What `ra_ap_parser` offers (read)

- `TopEntryPoint` has `SourceFile, MacroStmts, MacroItems, Pattern, Type, Expr, MetaItem`
  (`RA/lib.rs:78-88`); `PrefixEntryPoint` has `Vis, Block, Stmt, Pat, PatTop, Ty, Expr, Path,
  Item, MetaItem` (`RA/lib.rs:140-151`).
- `TopEntryPoint::MacroItems` runs `items::mod_contents(p, false)` under a `MACRO_ITEMS` root
  (`RA/grammar.rs:122-126`); `SourceFile` runs `p.eat(SHEBANG)`, `p.eat(FRONTMATTER)` and the
  same `items::mod_contents(p, false)` under `SOURCE_FILE` (`RA/grammar.rs:95-110`). The item
  subtrees come from the same function, `item_or_macro` (`RA/grammar/items.rs:20-26,48-106`).
  `PrefixEntryPoint::Item` calls `item_or_macro(p, true, true)` (`RA/grammar.rs:82-85`), which
  differs on `}` (`stop_on_r_curly`), so `MacroItems` is the entry that matches.
- `mod_contents` begins with `attributes::inner_attrs`, which accepts `#![..]`
  (`RA/grammar/attributes.rs:5-9`); in a whole file an inner attribute after an item is an
  error ("expected an item", `RA/grammar/items.rs:93-105`), in a `MacroItems` fragment that
  starts with one it is accepted.

### Trivia at item boundaries (read, `RA/shortcuts.rs`)

- After an item's last token the parser's `Exit` is deferred (`State::PendingExit`,
  `shortcuts.rs:160-166`); the next `Enter` emits it first, then counts the trivia run from
  the current position and attaches `n_attached_trivias(kind, run reversed)` of them inside
  the new node (`shortcuts.rs:136-158`). `n_attached_trivias` depends only on the item kind and
  that run (`shortcuts.rs:239-275`): comments attach, a blank line stops the walk unless an
  outer doc comment precedes it, an inner comment stops it.
- Consequence (derived): if fragment *k* is the text from just after item *k−1*'s last token to
  item *k*'s last token, the item's `Enter` in the fragment sees the same trivia run as in the
  whole file; the unattached part lands under `MACRO_ITEMS` instead of `SOURCE_FILE`. The last
  fragment ends at end of file, so trailing trivia lands inside its root as in the file
  (`shortcuts.rs:89-95`). Splitting changes neither the doc-comment nor the blank-line
  attachment.

### Exactness conditions (derived from the reads above; each a hypothesis until the corpus check)

The concatenation — fragment 0 parsed with `SourceFile`, fragments 1..n with `MacroItems`,
each fragment's root element dropped except fragment 0's, `end` indices shifted by the
fragment's base index, `start` already absolute through `from_steps_at`'s `base`
(`chloro-core/src/cst/mod.rs:132-142`) — equals the whole-file tree element by element when:

1. Every boundary follows a `}` or `;` token at bracket depth 0 (counting `(`, `[`, `{`; `;`
   and `}` inside `[u8; 3]` or `{ N }` generic arguments are at depth > 0 or end a fragment
   that then fails to parse). No item continues past its terminal `}`/`;` without error:
   `macro_call` and `macro_rules` take `;` only after `(..)`/`[..]` (`RA/grammar/items.rs:364-383,
   447-456`), and a `;` after a complete item is "expected item, found `;`"
   (`items.rs:54-59`). A false boundary inside an item leaves that item without its mandatory
   continuation (`>`, `;`, `,`), so the fragment reports an error (hypothesis, from the item
   grammar; the corpus check below tests it).
2. Any `Error` step in any fragment sends the file to today's sequential path — files with
   errors and `rustc_syntax::reparse` (which needs the whole-file `Output`,
   `chloro-core/src/formatter/formatting.rs:45-58`) never use the split.
3. Lexing a fragment gives the whole file's tokens: `LexedStr::new` restarts the tokenizer at
   every token (`RA/lexed_str.rs:52-60`), so lexing from any token boundary reproduces the
   following tokens, except the shebang and frontmatter checks that run on the text's start
   (`lexed_str.rs:40-50`). A fragment *k* > 0 that starts with `#!` or a `---` fence is refused
   by the scan (in a whole file both are errors there).
4. A fragment *k* > 0 whose first non-trivia tokens are `#` `!` is refused (inner attribute,
   accepted by `MacroItems` and rejected in a file, see above).
5. Jointness: `to_input` marks a token joint with the next punctuation (`RA/shortcuts.rs:28-62`);
   a fragment's last `}`/`;` loses that mark. No composite token starts with `}` or `;`
   (`RA/parser.rs:69-93`), so the grammar never reads it.
6. `rustc_compat`: the walker carries three booleans in source order (`prev_colon`,
   `inner_doc_ok`, `pending_outer_doc`; `rustc_compat.rs:474-561`). After a `}` or `;` all three
   are false unless the file is already rejected (`rustc_compat.rs:478-481,506,548-549`). Walking
   fragment *k* > 0 from that state with `first` at the fragment root
   (`subtree_rejected_by_rustc`, `rustc_compat.rs:641-658`, already used per macro fragment) and
   or-ing the answers, with `inner_doc_ok = true` for fragment 0 only and the end-of-file
   `pending_outer_doc` check on the last fragment, gives the file's answer: the ancestor rules
   stop at kinds inside an item and never test `SOURCE_FILE` (`rustc_compat.rs:81-130,268-288,
   375-395,430-435`).

Verification that would prove equality on the corpora: for every file of the 1238 fixtures and
the 20,850 registry files, build the whole-file `Tree` and the concatenated one and compare the
`Elem` sequences field by field (kind, `is_node`, `end`, `start`, `len`), as EXP-010's step 2
specifies for its builder and as EXP-007b compared the flat tree with rowan; compare
`rejected_by_rustc` with the or of the per-fragment answers; count the files that fell back.
One mismatch on a file that did not fall back refutes the conditions above.

### The sequential part and the bound (measured shares, derived times)

Measured per file (`$S/cg.<f>.out`; parse = `formatting::parse` inclusive; lex = its
`LexedStr::new`; items and largest item from `dump_tree`):

| file | elements | top-level items | largest item (elements) | parse / file Ir | lex / parse | lex Ir/byte |
|---|---|---|---|---|---|---|
| `lints.rs` | 120,397 | 11 | 44.8% | 27.7% | 29.6% | 32.2 |
| `nodes.rs` | 246,522 | 1,960 | 1.2% | 45.8% | 23.0% | 85.3 |
| `hir/src/lib.rs` | 127,339 | 261 | 15.1% | 32.8% | 23.5% | 69.9 |
| `hover/tests.rs` | 25,560 | 261 | 2.5% | 27.6% | 26.3% | 18.1 |
| `config.rs` | 49,439 | 116 | 32.7% | 33.6% | 29.8% | 50.6 |

Amdahl on M3's unpinned means, `head --threads 1` (T1) and E1's 4-thread `head` (TE1): lints
39.0/27.6 ms, nodes 51.8/36.0, hir-lib 46.7/26.0, hover 11.0/7.3, config 19.5/13.5. Assumed
(hypothesis): the parse's share of time equals its share of Ir. P = parse ms = share × T1.
Sequential after the split: the lex (lex / parse × P) plus a token scan and the concatenation,
at 5 Ir/token and 4 Ir/element (hypothesis; ≈ 0.2–0.4% of a file's Ir). Parallel parse time
on 4 cores ≥ max(¼, largest item share) × the rest.

| file | P (ms) | TE1 − P (ms) | (a) parse phase parallel, then E1 | (b) each worker parses its own units, + E1's residual | TE1 if the parse cost 0 |
|---|---|---|---|---|---|
| `lints.rs` | 10.8 | 16.8 | 23.5 ms (1.17x) | 23.4 ms (1.18x) | 16.8 ms (1.64x) |
| `nodes.rs` | 23.7 | 12.3 | 22.6 ms (1.59x) | 22.3 ms (1.61x) | 12.3 ms (2.93x) |
| `hir/src/lib.rs` | 15.3 | 10.7 | 17.4 ms (1.50x) | 17.2 ms (1.51x) | 10.7 ms (2.44x) |
| `hover/tests.rs` | 3.0 | 4.3 | 5.7 ms (1.29x) | 5.6 ms (1.30x) | 4.3 ms (1.71x) |
| `config.rs` | 6.5 | 7.0 | 10.5 ms (1.29x) | 10.4 ms (1.30x) | 7.0 ms (1.94x) |

- (b) is seq-lex + max((T1 − seq-lex)/4, largest share × (T1 − seq-lex)) plus E1's residual,
  the measured TE1 minus E1's own model P + max((T1 − P)/4, largest × (T1 − P)): 4.2, 5.3,
  2.8, 2.3, 2.7 ms (derived; start-up, stitching, contention). (a) and (b) agree within 0.02x:
  with units this small, overlapping parse and format adds nothing over a parallel parse
  phase; with the residual left out, (b) reads 1.43x–2.18x.
- Interaction with E1 (derived): the parse is the largest part of E1's sequential path on
  `nodes.rs` and `hir/src/lib.rs` (23.7 of 36.0 ms; 15.3 of 26.0 ms); after the split the
  sequential part is the lex, 3.2–5.5 ms on these files. `lints.rs` and `config.rs` stay
  bounded by their largest item (the 287,600-byte and 42,860-byte items), which no top-level
  split divides.
- Scope (read, E1): the CLI splits only files of 128 KiB and more
  (`../2026-10-14-autoresearch/experiments/EXP-001-parallel-items.md`); a crate's other files
  already run in parallel at file level, so the gain is on the largest file's tail, not on
  `bench`'s single-thread Ir, which rises by the scan and concatenation (+0.5–0.8%, hypothesis
  at the rates above on the sample's ≈ 90 K tokens and 170 K elements, plus one `LexedStr`,
  `Input` and `Output` per item if each fragment is lexed again: 969 top-level items in the
  sample × ≈ 1.1–1.5 K Ir per parse call, from `build`'s 955 calls: `free` 538 Ir/call plus
  allocations, hypothesis).
- The lex is the next sequential floor: 23–30% of the parse. Lexing chunks in parallel from
  guessed token boundaries and checking the junctions removes it except for a byte scan
  (hypothesis; no experiment exists).
- Cost (read and hypothesis): a boundary scan over `LexedStr` kinds, a `MacroItems` parse per
  fragment on the worker threads, the concatenation, a per-fragment `rustc_compat` walk; about
  300–500 lines in `formatting.rs`/`parallel.rs`/`cst/mod.rs`; no fork when each fragment is
  lexed again (`LexedStr` cannot be sliced: fields private, `RA/lexed_str.rs:24-29`), or no
  re-lex with a range variant of EXP-010's own builder.

## 3. Per-item CST and streaming (options 2-per-item and 4)

Elements per byte (measured by `dump_tree`; 16 bytes per `Elem`, `rustc -O $S/sz.rs`): sample
170,126 / 650,213 = 0.262 (4.2 tree bytes per source byte); `lints.rs` 0.188 (3.0), `nodes.rs`
0.688 (11.0), `hir/src/lib.rs` 0.539 (8.6), `hover/tests.rs` 0.121 (1.9), `config.rs` 0.292 (4.7).
The builder reserves `2 × lexed tokens + 8` elements (`cst/mod.rs:139`), 32 bytes per lexed
token.

Peak heap is the parser's event vector, not the tree (measured, massif, `fmt_stdin`):

| file | peak heap | at the peak: `Vec<Event>` | `Output` (`Vec<u32>`) | heap after the parse (max of later snapshots) |
|---|---|---|---|---|
| `nodes.rs` | 16,652,600 B | 12,582,912 B (75.6%) | 2,621,440 B | 7.56 MB (first snapshot after the peak), 5.5–5.7 MB while formatting |
| `lints.rs` | 9,531,608 B | 6,291,456 B (66.0%) | 1,572,864 B | 7.41 MB, 3.2–7.4 MB while formatting |

- `Event` is 24 bytes (measured on a copy of the enum, `$S/sz.rs`; its `Error { msg: String }`
  variant sets the size, `RA/event.rs:16-84`); the vector starts empty (`RA/parser.rs:35-36`)
  and doubles, so 12,582,912 B is a 524,288-event capacity; `alloc_count` shows one allocation
  of 8–16 MB on `nodes.rs` (measured).
- Per-item parsing caps `Vec<Event>` and `Output` at the largest item (1.2% of `nodes.rs`'s
  elements, 44.8% of `lints.rs`'s): peak heap falls to the post-parse maximum, 16.65 → ≤ 7.56
  MB on `nodes.rs` (−55%) and 9.53 → ≤ 7.41 MB on `lints.rs` (−22%) (derived from the table).
  M3's peak RSS 14 and 10 MiB for these files would fall by up to the same bytes (hypothesis:
  RSS tracks the heap's touched pages).
- Streaming (format item *k*, drop its tree) removes the tree too: 3.94 MB on `nodes.rs` (246,522
  × 16), leaving source, output, `LexedStr` (6 bytes per lexed token, `lexed_str.rs:24-29`),
  `Input` and one item's structures (derived).
- Time: the parser runs over the whole file either way, so Ir does not fall; per-item calls add
  ≈ 1.1–1.5 K Ir each (+0.3–0.5% on the sample, hypothesis above). The event buffer of one item
  stays in cache between the grammar and `event::process`: one pass over 265,301 events × 24 B
  is 99,488 lines, 56% of `event::process`'s 176,191 simulated L1 read misses (derived); the
  simulator has no prefetcher, so the hardware effect of a sequential pass is smaller
  (hypothesis).
- What neither changes (read): lexing and parsing of every byte before any output counts (a
  syntax error anywhere returns the file unchanged, `formatting.rs:35-58`); reorder groups,
  which format several items as one unit (`parallel.rs:9-12`); the gaps between items, which
  read the output so far (`parallel.rs:19-25`); `forward_parent`, which delivers an enclosing
  node's kind after its first child (`RA/event.rs:25-55`), so a stream needs at least a
  statement's events before it can lay out the statement.
- A fork that sizes `Vec<Event>` from the token count and shrinks `Event` (errors out of line,
  8 bytes) cuts the same peak without per-item trees: 524,288 × 24 → ≈ 330 K × 8 B ≈ 2.6 MB on
  `nodes.rs` (hypothesis: event count from `Output`'s 2,621,440-byte capacity, 327 K–655 K steps).

## 4. An own lexer or parser (option 5), and parser-level savings

### `ra_ap_parser`'s design (read)

- Token based: `LexedStr::new` runs `rustc_lexer::tokenize` once per token and converts its
  `TokenKind` to a `SyntaxKind` (`RA/lexed_str.rs:37-63,193-312`), keyword lookup included
  (`SyntaxKind::from_keyword`, `lexed_str.rs:226-229`), literal escapes checked for errors
  (`lexed_str.rs:314-`).
- `to_input` makes a second pass: trivia dropped, contextual keywords looked up again for
  every `IDENT` (`from_contextual_keyword`, `RA/shortcuts.rs:36-41`), joint bits set
  (`shortcuts.rs:28-62`).
- Grammar: recursive descent over `Input`, 5,195 lines in `RA/grammar*` (measured, `wc -l`),
  emitting a `Vec<Event>` with `Start`/`Finish`/`Token`/`Error` and `forward_parent` for
  left-recursive nodes (`RA/parser.rs:173-291`, `RA/event.rs:16-84`).
- `event::process` resolves `forward_parent` chains and encodes each step as one `u32` in
  `Output` (`RA/event.rs:91-138`, `RA/output.rs:14-112`); `intersperse_trivia` decodes it again
  and attaches trivia (`RA/shortcuts.rs:65-99`). No macro expansion and no semantic pass.
- Public surface a replacement can use without a fork: `Input::with_capacity/push/push_ident/
  was_joint` (`RA/input.rs:24-56`), `SyntaxKind::from_keyword/from_contextual_keyword`
  (`RA/syntax_kind/generated.rs:879,937`), `Output::iter` (`RA/output.rs:51`). Not public:
  `LexedStr`'s constructor from parts, `Parser`, `Event`, `Output`'s constructor.

### What a formatter-only parser could drop (read, derived)

- Error recovery: not droppable. chloro returns the input on any lexer or parser error but must
  detect every one (`formatting.rs:35-58`), and `rustc_syntax` rebuilds trees from the error
  steps of 9 sample files (`rustc_syntax.rs:213-245`). Recovery code runs only on error paths;
  `err_and_bump`, `err_recover` and `error_block` total 6,952 exclusive Ir on the sample
  (measured; `Parser::error` has no symbol of its own), so dropping recovery saves nothing
  measurable.
- The `Output` encode/decode, the `StrStep` layer, the second keyword lookup and the vector
  growth are what a replacement removes; the grammar's decisions (grammar 13.61 M + parser
  machinery 5.97 M = 19.58 M, 6.26%) are the same work in any recursive descent over the same
  grammar (hypothesis).
- The lexer against a lower bound: 39.0 Ir/byte on the main files (cursor ≈ 17.6, conversion
  ≈ 20.3, apportioned by the all-calls split, derived). A single-pass table-driven lexer costs
  about 3–4 Ir/byte in identifier and whitespace runs (load, class table, compare, increment)
  plus ≈ 20 Ir per token for the kind and offset pushes and a keyword hash, at 7.2 bytes per
  token ≈ 5–7 Ir/byte (hypothesis): 3.3–4.6 M on the sample's 650,213 bytes against 25.36 M.
- Per token or element today (§ 1 table): `to_input` ≈ 48 Ir/token, `event::process` 57
  Ir/event, interspersal 81 and tree sink 41 Ir per callback.

### Conformance risk (read)

- rustc acceptance: 15 of the 24 cases in the conformance ledger are rust-analyzer/rustc parser
  or lexer gaps (cases 4–14, 16–18 and 20,
  `../2026-10-14-autoresearch/conformance-ledger.md`), each handled in `rustc_compat.rs` or
  `rustc_syntax.rs` against rust-analyzer's exact behaviour. An own parser re-opens all of them
  and adds its own; an own lexer re-opens the lexer cases (case 18, unknown literal prefixes;
  `c"` by edition, `rustc_compat.rs:533-537`; guarded string prefixes, `lexed_str.rs:237-246`).
- A conversion layer over `ra-ap-rustc_lexer`'s `TokenKind` (keeping its cursor) carries the
  low risk class: the mapping is a pure function of `TokenKind` and token text
  (`lexed_str.rs:193-312`), checkable token by token.

### Smallest experiment

An example binary that, for every file of both corpora and every macro-argument window, builds
`(kind, start)` arrays and an `Input` with a chloro-side conversion over `rustc_lexer::tokenize`,
and compares them with `LexedStr::new` + `to_input` (kinds, offsets, error presence, contextual
kinds, joint bits). One difference refutes the port; then the Ir of the two front ends on the
sample, with the falsification at < 10 M saved (half the 20.17 M addressed).

## 5. Fusing passes

| fusion | addressed (measured) | bound | argument |
|---|---|---|---|
| trivia interspersal + tree build (EXP-010, no fork) | `intersperse_trivia` inclusive 31,524,439 (10.08%) | likely 7.74 M (2.47%), ceiling 27.95 M (8.94%) | EXP-010's figures (`experiments/EXP-010-fused-tree-build-feasibility.md`), rescaled from its 390.5 M total to 312.7 M; the absolute Ir are unchanged at this head (31.52 M vs 31.34 M) |
| `rustc_compat::walk` during the build | 5,947,498 (1.90%) | ceiling 1.90%; hypothesis ≤ 1.1% | token rules run as tokens are emitted (source order is the build order); node rules (`node_rejected`, `rustc_compat.rs:397-449`) need the node's children, so they run at `finish_node`; ancestors come from the builder's `open` stack (`cst/mod.rs:208-264`) instead of the backward search (`rustc_compat.rs:65-79`); the inner-attribute and token-tree states (`rustc_compat.rs:569-619`) become a counter and a saved flag. The rule checks remain |
| `to_input` removed (the lexer writes `Input`) | `to_input` inclusive 5,317,345 (1.70%) | ≤ ≈ 4.9 M (1.57%) | `Input`'s pushes remain (≈ 5 Ir per non-trivia token, hypothesis); the second keyword lookup goes (`shortcuts.rs:36-41` after `lexed_str.rs:226-229`). Needs chloro's own conversion, since `LexedStr` cannot be built from parts |
| `event::process` fused with the builder (fork) | `event::process` inclusive 18,037,091 (5.77%) | ≤ ≈ 13–15 M (4.2–4.8%), overlapping EXP-010's decode share | the `forward_parent` walk stays (≈ 3–5 M at 12–20 Ir/event, hypothesis); the `Output` encode, its growth (2.31 M) and the decode in `intersperse_trivia` go. `event` and `parser` are private (`RA/lib.rs`), so this needs a fork |
| macro arguments parsed once | prefix lex + `to_input` + build lex + `to_input` + `TopEntryPoint` = 8,311,410 (2.66%) | ceiling 2.66%; exact part 6,076,839 (1.94%) | for `Expr` and `Ty`, `PrefixEntryPoint` runs the same grammar function as `TopEntryPoint` (`RA/grammar.rs:73-78` vs `141-165`; the top entry adds a marker it abandons at EOF, which `event::process` drops), so when the prefix parse consumes its whole window (`macro_args.rs:247-248`, `n == window`) the build's lex, input and parse reproduce the prefix's; 943 of 955 builds are `Expr`, 22 `Type` (measured call counts). `Pat` (`pattern_single` vs `pattern`) and `Item` differ and keep the second parse |
| `rustc_syntax` rebuilds | `reparse` 261,934 (0.08%), 9 calls on 49 files, 7 `owned_tree` | ≤ 0.08% | measured; the 9 files are rust-analyzer's `test_data/*/err` fixtures and similar |

Branch prediction (measured counts from callgrind's simulator, which models a simple
predictor; `perf` is absent in this container):

| function | indirect branches | mispredicted | what dispatches |
|---|---|---|---|
| `intersperse_trivia` | 553,541 | 206,312 | the `Output` step tag and the `dyn FnMut` sink call (`shortcuts.rs:72-87,186-190`) |
| `TreeBuilder::steps::{closure#0}` | 263,020 (one per callback) | 193,687 (73.6%) | the `match step` on `StrStep` (`cst/mod.rs:271-286`) |
| `event::process` | 265,301 | 146,229 | the `match` on `Event` (`RA/event.rs:96-134`) |
| `LexedStr::new` | 132,881 | 113,793 (85.6%) | the `TokenKind` → `SyntaxKind` match (`lexed_str.rs:200-308`) |
| `Cursor::advance_token` | 141,260 | 93,438 | first-character dispatch (`LX/cursor.rs`, `LX/lib.rs`) |
| `to_input`, `prefix_parse` | 19,470 + 16,083 | 12,791 + 6,721 | |
| parse functions, total | | 772,971 of 973,446 (79.4%) | |

- A fused builder (EXP-010) removes the second dispatch on the same step, the closure's
  263,020 indirect branches and 193,687 mispredicts (19.9% of all), derived from the table;
  the sink call has one target and is predicted (its count sits in `intersperse_trivia`'s
  row).
- A lookup table for the punctuation arms of the `TokenKind` match replaces jump-table
  dispatch by a load for those tokens (hypothesis: most of the 113,793); the lexer's
  first-character dispatch and the Enter/Exit/Token split stay data-dependent in any design.
- At 15–20 cycles per mispredict (hypothesis), 193,687 mispredicts are 2.9–3.9 M cycles and
  the parser's 772,971 are 11.6–15.5 M cycles, against ≈ 194 M cycles for the sample (650,213
  B at M3's 9.4 MB/s and 2.80 GHz, derived): 1.5–2% and 6–8%. These are effects beyond Ir.

## 6. The cost of the tree on the formatter side

- Visible share (measured): the orchestrator's flat-tree row is 16.69 M (5.4%); `TreeBuilder`
  is 10.74 M of it, so reads and generated casts not inlined are ≈ 5.95 M (1.9%), led by
  `Expr::cast` 0.83 M, `Pat`/`Type` rewrites and casts. `kind`, `children`, `Children::next`
  and `NodeChildren::next` are `#[inline]` (`cst/mod.rs:473-514,643-686`), so their Ir sit in
  the formatter functions that call them and cannot be separated in this profile
  (hypothesis on the inlining, from the 19-Ir `Children::next` row, measured).
- Parent search (measured): `parent()` scans backwards over the preceding siblings' subtrees
  (`cst/mod.rs:187-201`). `visit_assoc_item` calls it twice per associated item
  (`chloro-core/src/formatter/visitor.rs:575-579`), so an `impl` with *n* items costs O(n ×
  impl size): 1,962,731 Ir (0.63%) and 45,871 L1 read misses (5.9% of all) on the sample;
  2,757,265 (0.95%) of `nodes.rs`, 9,738,627 (4.55%) of `hir/src/lib.rs`, 5,667,020 (6.62%) of
  `config.rs` (exclusive, `$S/cg.<f>.out`). Six other `parent()` calls (`nodes.rs:486`,
  `types.rs:661,665`, `rustc_syntax.rs:402,454,472`) and eight `Ancestors` uses
  (`rustc_compat.rs:61-550`) search the same way.
- A parent column: `Elem` grows from 16 to 20 bytes (measured, `$S/sz.rs`), elements per
  64-byte line from 4 to 3.2, tree memory +25% (sample 2.72 → 3.40 MB; `nodes.rs` 3.94 → 4.93
  MB), plus one store per element in the builder (≈ 2 Ir × 263,020 ≈ 0.5 M, hypothesis). It
  removes the backward searches: ≤ 0.63% on the sample, ≤ 4.55–6.62% on the two files. Passing
  `in_trait` from the caller that iterates the item list removes the `visit_assoc_item` part
  with no layout change (read: the caller holds the list).
- First-child/next-sibling: the preorder array already encodes both — first child at `idx + 1`
  (`cst/mod.rs:501-508`), next sibling at `end` (`cst/mod.rs:648-650`) — so that layout adds
  nothing; only the parent link is absent, and no previous-sibling query exists in the
  formatter (`grep`, read).
- Cache (derived): a `children` scan reads one 16-byte element per child, each after the
  previous child's subtree, so children whose subtrees span ≥ 4 elements sit on distinct
  lines. A full preorder pass touches each line once: `rustc_compat`'s 42,370 L1 read misses
  match 170,126 elements × 16 B / 64 B = 42,532 lines within 0.4%; fusing the walk into the
  build (§ 5) turns those into hits in the simulator; on hardware a sequential pass is
  prefetched (hypothesis). Last-level misses are negligible (orchestrator's table), so the tree
  fits the cache.

## Ranked bounds

Bounds are on the sample's 312,733,055 Ir unless stated; "likely" figures are hypotheses
with their argument in the section named. Risk classes: none (same events or same booleans by
construction), low (a port checked element by element), medium (lexical rules), high/very
high (rustc acceptance). Gates are `protocol.md` § 4.

| # | lever | share addressed | removable, argument | bound on total Ir | effect beyond Ir | risk | cost | smallest experiment; falsified if |
|---|---|---|---|---|---|---|---|---|
| P1 | trivia interspersal fused with the tree build (EXP-010) | 31.52 M (10.08%) | `StrStep` layer, `range_text`, sink calls; element writes stay (§ 5) | likely 2.47%, ceiling 8.94% | −193,687 indirect mispredicts (19.9%) | low | ≈ 150–250 lines in `cst/mod.rs`, no fork | EXP-010 steps 1–3; any `Elem` mismatch, or Ir saved < 7.74 M |
| P2 | own token conversion over `rustc_lexer` and direct `Input` (replaces `LexedStr::new` conversion and `to_input`) | 20.17 M (6.45%) incl. their vector growth | conversion 134 Ir/token and `to_input` ≈ 48 Ir/token down to ≈ 30 Ir/token (hypothesis) | ≤ 5.5% net (6.45% − ≈ 3.1 M new cost) | fewer `TokenKind` jump-table mispredicts (≤ 113,793) | low | ≈ 300 lines; needs P1's builder (no `LexedStr`); `ra-ap-rustc_lexer` as a direct dependency (gate: measured reason) | § 4's token-equality binary; any difference, or Ir saved < 10 M |
| P3 | fork: parser vectors sized up front (`Vec<Event>`, `Output`, `LexedStr`, `Input` bits) | 11.04 M (3.53%), in the allocator row | all but one allocation per vector per call (≈ 1.2 M stays, hypothesis) | ≤ 3.1% | peak heap −55% / −22% on `nodes.rs` / `lints.rs` with compact `Event` (§ 3) | none | ≈ 10 lines in 4 files of a vendored `ra_ap_parser` 0.0.307 via `[patch.crates-io]`; a pinned fork to maintain | patched build: gates 2–4 and massif on `nodes.rs`; Ir saved < 5 M or peak heap not below 8 MB |
| P4 | table-driven lexer replacing the `rustc_lexer` cursor | 13.27 M (4.24%) | cursor 128 Ir/token down to 3–4 Ir/byte (§ 4, hypothesis) | ≤ 3.6% (beyond P2; 13.27 M − 2.0–2.6 M) | first-character dispatch becomes loads (93,438 mispredicts) | medium | 600–1000 lines and XID tables, no fork; on top of P2 | differential lexing against `rustc_lexer` over both corpora and random byte strings; any token difference, or Ir saved < 5 M |
| P5 | macro arguments parsed once (reuse the prefix parse's `LexedStr`/`Output` when it consumed its window) | 8.31 M (2.66%) | exact for `Expr`/`Ty` when `n == window` (§ 5) | ≤ 1.94% exact part, ceiling 2.66% | — | low | ≈ 80–150 lines in `macro_args.rs`, no fork | fragment `Elem` equality against today's `build` on both corpora; any mismatch, or Ir saved < 3 M |
| P6 | fork: `event::process` writes the tree directly (no `Output`, no decode) | 18.04 M (5.77%) | the `forward_parent` walk stays | ≤ 4.2–4.8%, overlapping P1 and P3 | −146,229 or fewer process mispredicts only if the event match goes; the event stream's L1 misses stay | none (same events) | ≈ 200 lines in a fork | after P1: element equality, Ir saved beyond P1 < 4 M refutes |
| P7 | `rustc_compat` run inside the build | 5.95 M (1.90%) | walk loop, element reloads, backward ancestor searches | ≤ 1.1% (hypothesis), ceiling 1.90% | −42,370 simulated L1 misses | low | ≈ 150 lines in `rustc_compat.rs`/`cst/mod.rs` | the boolean on both corpora and the rustc-gap tests unchanged; any change, or Ir saved < 1 M |
| P8 | `in_trait` passed down (or a parent column) instead of `parent()` in `visit_assoc_item` | 1.96 M (0.63%) | the whole backward search | ≤ 0.63%; ≤ 4.55% of `hir/src/lib.rs`, ≤ 6.62% of `config.rs` | −45,871 L1 misses | none | ≈ 10 lines (column: +25% tree bytes) | gate 2 and per-file Ir on the two files; saved < half the bound |
| P9 | parallel parse of top-level items (E1 extension) | parse of files ≥ 128 KiB | parse/4 or the largest item, lex sequential (§ 2) | +0.5–0.8% (cost) | E1 4-thread wall 1.17x–1.61x on the five largest files (upper bound); nothing on smaller files or file-parallel crates | low with fallback | 300–500 lines, no fork | the § 2 element-equality harness; then hyperfine ≥ 10 interleaved runs against E1 with the pinned single-core figure; any mismatch without fallback, or < 1.1x on `nodes.rs` and `hir/src/lib.rs` |
| P10 | per-item CST / streaming | memory | event and tree buffers per item | +0.3–0.5% (cost) | peak heap −55% / −22% (`nodes.rs` / `lints.rs`), more with streaming | low | high: formatter over many trees (the arena already holds fragment trees, `cst/mod.rs:17-22`) | massif on the two files; peak not below the post-parse maximum (7.56 / 7.41 MB) |
| P11 | own typed parser (option 5) | 119.96 M parse-related | beyond P1–P7: the event layer's residual (≈ 3–5 M) and any difference in decision cost (unknown) | ≈ 1–1.6% beyond P1–P7 (hypothesis) | — | very high (rustc acceptance; 15 ledger cases re-opened) | 5,195 grammar lines to re-own | none smaller than P1+P3+P6, whose residual bounds it |

### The irreducible part and the single-thread ceiling (derived; the floor terms are hypotheses)

| stage | today (inclusive, all callers) | floor if every lever lands |
|---|---|---|
| lexing, once per byte (P2, P4, P5) | 30.41 M | 3.3–4.6 M (5–7 Ir/byte × 650,213 B) |
| parser input (P2) | 5.32 M | 0.3–0.7 M |
| grammar and parser machinery, fragments once (P3, P5) | 25.92 M incl. `Vec<Event>` growth | ≈ 17.5 M (19.58 M exclusive − the second fragment parse, 2.13 M) |
| event replay (P6) | 18.04 M | 3–5 M |
| interspersal and build (P1) | 31.52 M | ≈ 5.4 M (3.39 M element writes + attachment) |
| `rustc_compat` (P7) | 5.95 M | 2–3 M |
| `macro_args` own code, `rustc_syntax`, rest | ≈ 2.8 M | ≈ 1.4 M |
| **parse-related** | **119.96 M (38.36%)** | **≈ 33–38 M (10.6–12.1%)** |

- Removable from the parse side if every lever lands: ≈ 82–87 M (26–28% of today's total).
  Under the assumption that wall time scales with Ir (hypothesis; the parser's mispredict
  share argues its time share exceeds its Ir share), M3's 9.4 MB/s at 312.7 M becomes 9.4 ×
  312.7 / (312.7 − 82..87) = **12.7–13.0 MB/s** single-threaded, with the formatter's 192.8 M
  as the remainder.
- The likely subset without an own lexer or parser (P1 7.74 M, P5 6.08 M, P3 ≈ 9 M, P2 ≈ 10 M,
  P7 ≈ 2 M, P8 1.96 M ≈ 37 M, hypothesis) gives 9.4 × 312.7 / 275.7 ≈ 10.7 MB/s.
- With a parse that cost nothing, the ceiling is the formatter's: 9.4 × 312.7 / 192.8 = 15.2
  MB/s. The parse floor alone, with a formatter that cost nothing, corresponds to 77–89 MB/s
  (9.4 × 312.7 / 33..38).

## What this settles and what it leaves open

- The parse-related path is 119.96 M Ir, 38.36% of the final head (measured), more than the
  28.0% parser-and-lexer row: the tree build (3.43%), `rustc_compat` (1.90%), the macro
  reparse's own code and ≈ 4.67% of allocator and `memcpy` sit under the same three roots; the
  parser's vector growth alone is 11.04 M (3.53%) of the allocator row (measured).
- Peak heap on the largest files is the parser's `Vec<Event>` (24-byte events, doubling from
  empty), 75.6% of `nodes.rs`'s 16.65 MB peak, not the 16-byte-element tree (measured); the tree
  is 3.0–11.0 bytes per source byte (measured).
- Splitting a file at top-level items and parsing the pieces with `TopEntryPoint::MacroItems`
  reproduces the whole-file tree under six conditions read from the parser and the walker
  (§ 2), with a sequential fallback on any error; the corpus comparison has not been run. On
  E1's 4-thread path it bounds the five largest files at 1.17x–1.61x (derived); the lex (23–30%
  of the parse) is the next sequential floor.
- 79.4% of the simulated indirect mispredicts are in the parser's dispatch, and one pass over
  the event array explains 56% of `event::process`'s L1 read misses (derived); both are
  simulator counts, with no hardware counters in this container.
- The `parent()` search in `visit_assoc_item` costs 4.55% and 6.62% of two large files'
  instructions (measured); the tree layout needs no change to remove it.
- Open: the corpus equality check of § 2; the per-call fixed cost of a parse (estimated from
  `build`'s `free`, not measured); the cycle cost of the mispredicts and misses on hardware;
  the lexer's lower bound (no table-driven lexer has been measured); whether an own parser's
  decisions cost less than rust-analyzer's grammar; and every wall-time figure here, which
  rests on M3's means and the assumption that time follows Ir.
