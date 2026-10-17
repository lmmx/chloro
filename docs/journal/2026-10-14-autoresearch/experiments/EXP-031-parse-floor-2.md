# EXP-031: the parse floor, second pass (P7, P2, P4)

Ledger row E31 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md); the levers are P7, P2 and P4 of the ranked bounds in
[../../2026-10-15-extensions-theory/parser-and-substrate.md](../../2026-10-15-extensions-theory/parser-and-substrate.md),
after [EXP-017](EXP-017-parse-floor.md) took P8, P5 and P1. At the M5 head the parse-only phase
is 39.2% of the fixture sample's instructions and 46.6% of the registry sample's (the fork
trigger of the unmined-tricks plan, 40%, is crossed on the registry sample); forking
`ra_ap_parser` (P3, P6, P11) is the owner's decision and was not done: no vendored or patched
parser, no `[patch]`. Labels: **measured** (a tool's output on the builds below), **read**
(from source), **derived** (arithmetic on measured figures).

## Identity

| field | value |
|---|---|
| branch | `20261015-parse-floor-2` from `4ad3b54` (was `03c61be`) (integration head; code of `a26b460` (was `e96b530`), the best-known-good), worktree `/home/user/wt/parse-floor-2` |
| commits | `600763d` (was `1b44ee5`) (P7 attempt) and `e1c2eec` (was `eabb600`) (its revert); `89e4c88` (was `29f8fb5`) (P2, kept); `7ab5f8d` (was `7ce5658`) (P4, kept); then this record |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, valgrind 3.22.0, `ra_ap_parser` 0.0.307, `ra-ap-rustc_lexer` 0.137.0, `rustc-literal-escaper` 0.0.4, `rustfmt 1.9.0` |
| host | 4 vCPU Intel Xeon @ 2.80GHz, shared with E29 and E30 throughout (load average 6–14 when measured; the session was interrupted on 2026-10-15 by a rate limit and resumed on 2026-10-17 at 06:31 UTC after a host reboot; binaries were checked by sha256 before reuse). `Ir` is unaffected by load; the wall-clock figures below are provisional |
| build | `cargo build --release -p chloro-core --examples` (fat LTO, one codegen unit) |
| binaries (sha256, first 16) | base `5965903ba274b0cb` (`4ad3b54` (was `03c61be`)), P7 attempt `70e1ccaac9ef0617`, P7 inline variant `scratchpad/e31/p7b/bench`, P2 `be0fd0c8112a6eec` (`89e4c88` (was `29f8fb5`)), P4 `8703798680fa9b65` (`7ab5f8d` (was `7ce5658`); a build before `cargo fmt` of the same code, `41ffb5de2bfac53f`, measured within 8 K of it on every figure) |
| `Ir` commands | `benchmarks/profile.sh --core N --runs 2 --per-file --bench B` (fixture sample) and the same with `--sample-list benchmarks/samples/registry49.txt` (registry sample), each also with `--phase parse`; the four ran in parallel on cores 0–3 (`Ir` does not depend on the core). Figures are the minimum of the two runs; per-file figures are net of the empty sample |
| attribution | `callgrind_annotate --inclusive=yes` and `--tree=caller` (call counts) on the `--phase parse` profiles; element and token counts of the sample trees by `examples/dump_tree` |
| golden hashes | `scratchpad/golden/fixtures-e8d87c2.tsv` and `registry-e8d87c2.tsv` |
| equality builds | `CARGO_TARGET_DIR=target/verify CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true CARGO_PROFILE_RELEASE_LTO=off CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 cargo build --release -p chloro-core --example bench --example lex_equality`, run as `bench -n 1 --check GOLDEN --root R` over both corpora; the counts below came from an uncommitted `eprintln!` per checked tree, lexed text or fused check, removed after use |
| scratch | `/tmp/claude-0/-home-user-chloro/9623d6dc-d933-5682-9d83-7762284f5c7f/scratchpad/e31/` (profiles `prof-<build>-<fx,reg,fxp,regp>`, harness outputs `lex_equality.out`, `verify-p2/`, `verify-p4/`, gate outputs `gates-p2/`, `gates-p4/`, the P7 diff `p7-attempt2.patch.now`) |

## Hypothesis and falsifiers

The exact, fork-free parser-side levers remove a measurable part of the parse floor with
byte-identical output: P7 (`rustc_compat` inside the tree build; bound ≤ 1.1%, ceiling 1.90%),
P2 (an own token conversion over `rustc_lexer` producing the parser's `Input` directly; bound
≤ 5.5% net), P4 (a table-driven lexer replacing the `rustc_lexer` cursor; ≤ 3.6% beyond P2).
Keep rules (the table's refutation column): P7 at least 1 M `Ir` saved per file on the fixture
sample, P2 at least 10 M, P4 at least 5 M beyond P2; each with no registry regression and zero
harness mismatches. Falsifiers: zero levers kept; any harness mismatch; any output change; a
new crate in the runtime tree without a measured reason of at least 3% per file.

## The attribution re-measured at the base (measured)

`4ad3b54` (was `03c61be`), `--phase parse`. Token counts are `Converter::push` calls (every token
`LexedStr::new` lexes, in every file including those that do not parse); non-trivia tokens and
elements are the sample trees' (`dump_tree`: fixtures 42 files that parse alone, 80,438 nodes,
91,576 tokens of which 64,003 not trivia; registry 49 files, 105,268 nodes, 180,388 tokens of
which 120,708 not trivia).

| stage (inclusive) | fixture sample | per unit | registry sample | per unit |
|---|---|---|---|---|
| parse phase total | 86,532,162 (39.16% of 220,959,314) | | 139,892,609 (46.64% of 299,933,954) | |
| lexer cursor (`Cursor::advance_token`) | 12,039,994 | 127.4 Ir/token (94,482) | 22,261,546 | 119.7 Ir/token (186,013) |
| token conversion (`LexedStr::new` minus the cursor) | 12,969,051 | 137.3 Ir/token | 28,508,455 | 153.3 Ir/token |
| — of which `Converter::push` | 6,213,745 | 65.8 | 12,351,163 | 66.4 |
| `LexedStr::to_input` | 4,271,079 | 45.2 Ir/token | 8,109,589 | 43.6 Ir/token |
| grammar (`entry::top::source_file`) | 19,406,349 | 303 Ir/non-trivia token | 25,026,947 | 207 |
| `event::process` | 14,042,755 | 219 Ir/non-trivia token | 19,679,778 | 163 |
| tree build (`Tree::from_steps_at`, interspersal fused, E17) | 16,320,180 | 94.9 Ir/element (172,014) | 25,290,730 | 88.5 Ir/element (285,656) |
| `rustc_compat::subtree_rejected_by_rustc` | 5,700,935 | 33.1 Ir/element | 9,015,613 | 31.6 Ir/element |
| parser vector growth (`RawVec<Event>`, `<u32>`, `<SyntaxKind>` `grow_one`) | 2,987,150 | | 3,787,788 | |

- The lexer front end (cursor, conversion, `to_input`) is 29.28 M of the fixture sample's
  parse phase (33.8%) and 58.88 M of the registry sample's (42.1%): on the registry sample it
  is the largest stage, larger than the grammar and `event::process` together (44.71 M).
- Lever bounds at this head (derived): P2 addresses conversion + `to_input` = 17.24 M
  (fixtures) and 36.62 M (registry); P4 the cursor, 12.04 M and 22.26 M; P7 the walk, 5.70 M
  and 9.02 M.
- The format phase lexes macro-argument windows too: `prefix_parse_window` and `build` call
  `LexedStr::new` for 1.70 M and `to_input` for 0.57 M on the fixture sample, 14.97 M and
  2.36 M on the registry sample (measured, `--tree=caller` on the format-phase profiles).

## P7: `rustc_compat` inside the tree build (`600763d` (was `1b44ee5`), reverted in `e1c2eec` (was `eabb600`))

- Change: a `BuildHook` trait lets `TreeBuilder::steps_with` report every token, `Enter` and
  `Exit`; `CompatHook` applies the walker's token rules as tokens are laid out (ancestors from
  the open-node stack instead of `Ancestors`' backward search), keeps the walk's token-tree
  state (a depth counter; the first element after a top-level `TOKEN_TREE` gets the full
  rules, the rest only the `:::` rule) and inner-attribute state (saved at the attribute's
  `Enter`, restored at its `Exit` unless an inner attribute nested in it ended first, which is
  when the walk's single slot is overwritten), and collects the nodes whose kind has a node
  rule, checked on the finished tree (node rules read only the tree; the answer is an `or`).
  Trees from `rustc_syntax::reparse` keep the walk.
- Exactness (measured): debug builds assert the fused answer equals the walk's on every file;
  the debug-assertion build over both corpora compared 1,142 fixture files (23 rejected by
  rustc's rules) and 30,599 registry files (25 rejected), the file-level trees without parser
  errors, 0 disagreements, 0 outputs changed; a unit test covers nested inner attributes in
  attribute values, `if` blocks, doc comments around token trees, `:::`, and four editions
  by two rustfmt versions.
- Measured, hook out of line: fixture 220,959,314 → 229,867,799 (+8,908,485; per file net
  220,755,444 → 229,730,702), registry 299,933,954 → 314,565,745 (+14,631,791), parse phase
  86,532,162 → 94,950,658 and 139,892,609 → 154,259,242. `CompatHook::token` alone cost 46 Ir
  per token, and the generic build no longer inlined `trivias` and `n_attached_trivias`.
- Measured, the common cases inline in the build loop (whitespace, tokens inside token trees,
  tokens no rule reads, nodes without a rule; the rest out of line): fixture 221,377,024
  (+417,710), registry 300,720,695 (+786,741), parse phase 86,577,619 (+45,457) and
  140,566,286 (+673,677), one run each. The fused hook's work is 33 Ir per element, the
  walk's cost: the token and node decisions are the walk, and they stay in any fused form;
  what the fusion removes (the loop and sequential element loads, 42 K simulated L1 misses in
  the theory record) costs few instructions.
- Decision: rejected by its keep rule (at least 1 M saved per file); the theory bound
  (≤ 1.1%) is refuted at this head: the removable part is below the measurement's resolution.

## P2: own token conversion and the parser's input built directly (`89e4c88` (was `29f8fb5`), kept)

- Change: `cst::lex::lex(edition, text)` returns `Tokens` (kinds and starts with an `EOF`
  entry, as `LexedStr` holds them) and the parser's `Input`, or `None`; the tree build is
  generic over a `Lexed` trait (`LexedStr` or `Tokens`). Used by `formatting::parse`,
  `Tree::parse`, and the macro-argument windows and builds (`WindowLex`). On a parser error
  `reparse` gets `LexedStr::new` with the output of the new input. Keyword lookups skip
  identifiers that cannot be keywords (every strict and contextual keyword is 2–15 bytes and
  starts with a lowercase letter, except `Self`; debug builds assert the skip).
- Exactness argument (read, `lexed_str.rs`, `shortcuts.rs`, `frontmatter.rs`, `rustc_lexer`
  `lib.rs`): with `FrontmatterAllowed::No` the cursor reads nothing but the remaining text
  (`prev` exists only for debug assertions), so a cursor continued over the text equals one
  started per token; the guarded string prefix before 2024 (`#"`, `##`, cut to `#`) restarts
  after the `#` as `LexedStr` does; `ScriptSource::parse` emits a shebang or frontmatter token
  only after `#!` not followed by `[`, or at a `-` after Pattern_White_Space, and `lex`
  answers `None` for those texts; `lex` answers `None` at the first token `LexedStr` attaches
  an error message to (every condition of `extend_token` and `extend_literal`; an escape
  error counts when `err_to_msg` gives it a non-empty message; a string without a backslash
  has no such error, since the lexer ends it at the first `"`), so every text with a lexer
  error, and its error tokens, go through `LexedStr` itself; the input is `to_input`'s loop
  run as each token is pushed.
- Dependencies (measured, `cargo tree -p chloro-core -e normal`): `ra-ap-rustc_lexer 0.137.0`
  and `rustc-literal-escaper 0.0.4` were already in the runtime tree through `ra_ap_parser`;
  the lockfile diff adds the two names to chloro-core's dependency list and no package; `cargo
  tree -d` shows no duplicate. No crate added (protocol § 1.4 gate not triggered).
- Token-equality harness (measured, `examples/lex_equality.rs`, release build): every token's
  kind and start, the sequence of `Input` calls (`push`, `push_ident` with its contextual
  kind, `was_joint`) against a transcription of `to_input`, and the parser's output over both
  inputs, step by step; every `None` checked to be a lexer error or a script prefix:

| inputs | texts | tokens | lexed by `lex` | left to `LexedStr`: lexer errors | script prefix | mismatches |
|---|---|---|---|---|---|---|
| 1,238 fixture files × editions 2021, 2024 | 2,476 | 5,455,612 | 2,327 | 137 | 12 | 0 |
| 25,077 registry files × 2021, 2024 | 50,154 | 211,794,633 | 50,001 | 141 | 12 | 0 |
| 10,000 random texts × 2015, 2018, 2021, 2024 | 40,000 | 667,779 | 12,595 | 26,979 | 426 | 0 |
| 10,000 fixture mutations × four editions | 40,000 | 37,808,520 | 30,312 | 9,440 | 248 | 0 |

  The random texts are made of pieces of every lexical class (unterminated literals, raw
  strings with up to four hashes, `#!`, `---` fences, byte and char literals, lifetimes and
  `'` ambiguities, Unicode identifiers, combining marks, emoji, nested block comments, doc
  comments, every Pattern_White_Space character, NUL, BOM), half of them from whole tokens so
  that they reach the own path; mutations insert such pieces, delete ranges or cut windows.
- Element equality (measured, E17's harness: the debug-assertion bench over both corpora; each
  tree is also built by `intersperse_trivia` and compared element by element, and each text
  lexed by `lex` is compared with `LexedStr::new`):

| corpus | texts lexed by `lex` | left to `LexedStr` | file-level trees (with parser errors) | their elements | fragment trees | their elements | mismatches | outputs changed |
|---|---|---|---|---|---|---|---|---|
| fixtures (1238 files) | 18,794 | 209 | 1,330 (188) | 5,046,996 | 14,579 | 138,566 | 0 | 0 of 1238 |
| registry (25,077 files) | 885,774 | 2,055 | 34,104 (3,505) | 172,148,648 | 628,070 | 6,311,290 | 0 | 0 of 25,077 |

- Measured:

| sample | base | P2 | change |
|---|---|---|---|
| fixtures, whole | 220,959,314 | 210,432,382 | −10,526,932 (−4.76%) |
| fixtures, per file net | 220,755,444 | 210,220,247 | −10,535,197 |
| registry, whole | 299,933,954 | 271,239,973 | −28,693,981 (−9.57%) |
| registry, per file net | 298,834,458 | 271,227,472 | −27,606,986 |
| parse phase, fixtures | 86,532,162 | 75,687,643 | −10,844,519 (−12.5%) |
| parse phase, fixtures, per file net | 87,402,884 | 78,062,019 | −9,340,865 |
| parse phase, registry | 139,892,609 | 119,784,124 | −20,108,485 (−14.4%) |
| parse phase, registry, per file net | 140,268,207 | 121,249,786 | −19,018,421 |

- Where it went (measured, parse phase): `lex::lex` 18,618,268 inclusive on the fixture sample
  (cursor 12,060,842 unchanged; conversion and input 6.56 M = 69 Ir per token, from 182) and
  38,394,825 on the registry sample (cursor 22,261,546; conversion and input 16.13 M = 87 Ir
  per token, from 197); `LexedStr::new`, `Converter::push`, `to_input` and the `LexedStr`
  vectors' growth are gone from both but for the texts left to `LexedStr`. The tree build
  over `Tokens` costs 16.99 M against 16.32 M (+0.67 M; the merged `(start, kind)` layout and
  the generic instance; a follow-up).
- Decision: kept (−10.53 M per file on the fixture sample against the 10 M rule, −27.6 M on the
  registry sample, zero mismatches).

## P4: the common tokens by a byte scan (`7ab5f8d` (was `7ce5658`), kept)

- Change: in `lex`, table-driven scans decide whitespace (Pattern_White_Space's ASCII
  members), ASCII identifiers and keywords, one-byte punctuation (straight into the token
  vector and the input) and line comments (`//` to the next line feed, with their doc style);
  a cursor started at the token lexes every other token (literals, lifetimes, block
  comments, non-ASCII text, `#"`/`##`). Not a full replacement of the cursor: 2,549 of the
  fixture sample's 94.5 K tokens and 6,931 of the registry sample's 186 K still go to it
  (measured call counts).
- Exactness argument (read, `rustc_lexer` `advance_token`, `whitespace`,
  `ident_or_unknown_prefix`, `c_or_byte_string`, `line_comment`, `is_whitespace`): each scan
  gives up where the byte after it could change the cursor's answer: whitespace before a
  non-ASCII byte (five Pattern_White_Space characters are not ASCII); an identifier before
  `#`, `"`, `'` (raw identifiers, raw and prefixed literals, unknown prefixes) or a non-ASCII
  byte (XID_Continue, emoji); `#` before `"` or `#`; `/` before `*`. The one-symbol tokens of
  the cursor take no lookahead. No `unsafe`.
- Differential lexing (measured): debug builds compare every scanned token's kind and length
  with the cursor's. `lex_equality` in the debug-assertion build over the same inputs as P2
  (both corpora in two editions, 10,000 random texts and 10,000 mutations in four editions):
  0 mismatches, no check failed, the same counts as the table above; the debug-assertion bench
  over both corpora: 0 of 1238 and 0 of 25,077 outputs changed, no check failed.
- Measured:

| sample | P2 | P4 | change |
|---|---|---|---|
| fixtures, whole | 210,432,382 | 203,150,467 | −7,281,915 (−3.46%) |
| fixtures, per file net | 210,220,247 | 202,936,968 | −7,283,279 |
| registry, whole | 271,239,973 | 255,677,158 | −15,562,815 (−5.74%) |
| registry, per file net | 271,227,472 | 255,651,412 | −15,576,060 |
| parse phase, fixtures | 75,687,643 | 68,605,771 | −7,081,872 (−9.36%) |
| parse phase, fixtures, per file net | 78,062,019 | 70,970,224 | −7,091,795 |
| parse phase, registry | 119,784,124 | 106,889,214 | −12,894,910 (−10.77%) |
| parse phase, registry, per file net | 121,249,786 | 108,343,572 | −12,906,214 |

- Where it went (measured, parse phase): the cursor 12.06 M → 0.89 M (fixtures) and 22.26 M →
  2.48 M (registry); `lex::lex` inclusive 18.62 M → 11.54 M (122 Ir per token all in) and
  38.39 M → 25.50 M (137 Ir per token).
- Decision: kept (−7,283,279 per file beyond P2 against the 5 M rule; no registry regression).

## The head against the base (measured)

| sample | base `4ad3b54` (was `03c61be`) | head `7ab5f8d` (was `7ce5658`) | change |
|---|---|---|---|
| fixtures, whole | 220,959,314 | 203,150,467 | −17,808,847 (−8.06%) |
| fixtures, per file net | 220,755,444 | 202,936,968 | −17,818,476 (−8.07%) |
| registry, whole | 299,933,954 | 255,677,158 | −44,256,796 (−14.76%) |
| registry, per file net | 298,834,458 | 255,651,412 | −43,183,046 (−14.45%) |
| parse phase, fixtures (share of the whole) | 86,532,162 (39.16%) | 68,605,771 (33.77%) | −17,926,391 (−20.72%) |
| parse phase, registry (share of the whole) | 139,892,609 (46.64%) | 106,889,214 (41.81%) | −33,003,395 (−23.59%) |

Remaining parse phase at the head (measured, inclusive): fixtures `TopEntryPoint::parse`
32,539,402 (grammar 18,736,922, `event::process` 13,800,506), tree build 16,988,572, lexer
front end 11,536,782, `rustc_compat` 5,700,935, `RawVec<Event>` growth 1,208,395; registry
44,190,861 (24,744,786 and 19,444,017), 26,188,243, 25,504,056, 9,015,613, 1,538,553.

## Gates (measured)

| gate | scope | P2 `89e4c88` (was `29f8fb5`) | P4 `7ab5f8d` (was `7ce5658`) |
|---|---|---|---|
| `cargo fmt --all --check` | | clean | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | no warnings | no warnings |
| `cargo test --workspace --all-features` | | not run at this commit (`cargo test -p chloro-core --lib`: 208 passed, before the `is_empty` default method clippy asked for) | 308 passed, 0 failed, 1 ignored |
| `bench -n 1 --check fixtures-e8d87c2.tsv` | 1238 fixtures, sequential and `--threads 4` | 0 of 1238 changed; 0 differ between 4 threads and `format_source` | the same |
| `bench -n 1 --check registry-e8d87c2.tsv` | 25,077 registry files | 0 changed | 0 changed |
| `conform -i`, `conform -i -V 1.8.0` | 1220 fixtures | 1220/1220 identical, 0 not idempotent, both versions | the same |
| `config_matrix -n 1220` | 31 option values against live rustfmt 1.9.0 | not run (no output changed) | every value identical on every file: 1022/1022 for the 28 option values, editions 2021 844/844, 2018 843/843, 2015 838/838 |
| harnesses | | token equality and element equality: 0 mismatches | differential lexing and element equality: 0 mismatches |
| `git diff --check` | every commit | clean | clean |

## Wall clock (measured, provisional)

`taskset -c 1 bench -n 5` over the 1238 fixtures, base and head interleaved, three pairs, load
average 3.3–4.3 with E29 and E30 running (a first triple on core 3, shared with another
agent's pinned runs, read 6.3–12.9 MB/s and is discarded): base 13.01, 13.03, 13.01 MB/s; head
13.70, 13.55, 13.50 MB/s; ranges disjoint, 1.04–1.05x. A quiet-host replay is needed for a
figure to cite; the instruction counts are the metric.

## Decision

Kept: `89e4c88` (was `29f8fb5`) (P2) and `7ab5f8d` (was `7ce5658`) (P4), on the branch for the integrator's gates 1–5 on the
merged tree. Rejected: P7 (`600763d` (was `1b44ee5`), reverted in `e1c2eec` (was `eabb600`)), measured above. None of the
falsifiers holds: two levers kept, no harness mismatch, no output change, no crate added.

What the fork would add beyond these levers (derived from the head's parse-phase profiles; the
owner's decision, not done here):

| fork lever | addressed at the head | bound on the head's `Ir` | argument |
|---|---|---|---|
| P3: parser vectors sized up front | `RawVec<Event>` growth 1.21 M + `Output` growth 0.13 M + `memcpy` 1.10 M (fixtures); 1.54 + 0.16 + 1.97 M (registry) | ≤ 2.44 M (1.2%) fixtures, ≤ 3.67 M (1.4%) registry | `memcpy` includes other copies, so these are upper bounds; the lexer's own vectors are already sized (P2) |
| P6: `event::process` writes the tree (no `Output`, no decode) | `event::process` 13.80 M (fixtures), 19.44 M (registry) | ≤ 8.8–10.8 M (4.3–5.3%) fixtures, ≤ 12.4–15.4 M (4.9–6.0%) registry | the `forward_parent` walk stays (3–5 M on the fixture sample by the theory record's 12–20 Ir per event; 4–7 M scaled to the registry sample, hypothesis); the build's decode of `Output` steps also goes, overlapping |
| P3 + P6 | | ≈ 11–13 M (5.5–6.5%) fixtures, ≈ 16–19 M (6.3–7.5%) registry | the registry parse share would fall from 41.81% to about 37–38% (fixtures from 33.77% to about 29–30%) |
| P11: own parser | grammar 18.74 M (fixtures), 24.74 M (registry) | beyond P3 + P6, only a difference in decision cost (the same work in any recursive descent over the same grammar; the theory record's ≈ 1–1.6% of its total) | very high risk: 15 conformance-ledger cases re-opened |

Merged at `de9dcc9` (was `2ecc8a3`) after the integrator reproduced the gates on the merged tree: 0 of 1238
fixture hashes changed (sequential and 4 threads), 0 of 25,077 registry hashes; `conform -i`
1220/1220 at 1.9.0 and 1.8.0; `Ir` 203,151,226 and 203,151,309 on the fixture sample (per file
202,934,484 net) and 255,677,420 and 255,677,661 on the registry sample (per file 255,654,373); parse
phase 68,607,540 and 106,890,987; fmt, clippy, 308 tests, the option matrix identical on every
value; `lex_equality --root <fixtures> --root <registry>` with the default 10,000 random texts
and 10,000 mutations: 0 mismatches; the debug-assertion `bench` over both corpora against the
golden hashes: 0 changed, no assertion fired. The merge was conflict-free.

## What surprised

- P7 bought nothing. The walk's 33 Ir per element is the rules' cost, not the loop's: fused
  into the build, the same decisions cost the same, and a hook that is not inlined costs more
  than the walk (+8.9 M). The theory bound assumed most of the walk was loop overhead.
- The lexer front end, not the grammar, was the registry sample's largest parse stage (42%);
  registry code is denser in tokens (3.9 bytes per token against 6.9 in the fixtures), so a
  per-token saving counts about twice there: P2 and P4 together save 2.5x as much on the
  registry sample as on the fixture sample (44.26 M against 17.81 M).
- `rustc_lexer`'s cursor cost 120–127 Ir per token; a byte scan with a fallback took 96–97% of
  the tokens away from it with a short exactness argument per scan, without replacing it.

## Follow-up

- `lex` sizes its vectors for a token per four bytes; the registry sample still grows
  `RawVec<Token>` (0.57 M): a denser estimate (a token per three bytes) is a small gain.
- The tree build over `Tokens` costs 0.67 M more than over `LexedStr` (fixtures); the layout of
  `Token` (a `(start, kind)` pair) against two arrays is the first suspect.
- The remaining scans: number literals and lifetimes still go to the cursor (most of the 2,549
  and 6,931 cursor calls); a scan for decimal integers without suffix is the next piece.
- `lex` and P1's interspersal port are pinned to `ra_ap_parser` 0.0.307 and
  `ra-ap-rustc_lexer` 0.137.0: a parser bump must rerun `lex_equality` and the debug-assertion
  bench over both corpora.
- The fork levers (P3, P6) are the next parse-side step if the owner chooses to take it; the
  bounds are above.
