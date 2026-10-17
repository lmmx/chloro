# EXP-003: hot paths, one lever per commit

Ledger row E3 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md); the levers come from the lever table and the allocation
sites of [../profile-baseline.md](../profile-baseline.md). Labels: **measured** (a tool's
output on the builds below), **read** (from source).

## Identity

| field | value |
|---|---|
| branch | `20261014-hot-paths` |
| bases | `8b93cba` (was `4cd6336`) (formatter code of `884f9c9`); merged with the integration branch at `966a7d5` (was `aa1e058`) (E1, E2, E4, merge `4e72e10` (was `c2c0934`)) and at `fe34bd2` (was `8d418ee`) (E7b, E8, merge `49edb17` (was `adcacf2`)) |
| kept commits | `19db4fe` (was `0f5f235`), `d74d6c6` (was `b6a249d`), `259abb0` (was `16e2e3d`), `a58d88b` (was `d46aa59`), `6406b73` (was `9b77c23`), `e0235a6` (was `350e6e4`), `a3afd18` (was `6f7c386`), `a96dfd5` (was `abdc10c`) (on `8b93cba` (was `4cd6336`)); `5838c14` (was `83dd37c`) (half), `860514d` (was `887674b`), `35e1475` (was `4b5bb3a`), `bfe5af3` (was `a383516`), `db2cea1` (was `e2128cf`), `9ad28d4` (was `0178efa`), `a02ae1b` (was `c28d403`) (on `966a7d5` (was `aa1e058`)); `5932986` (was `ed4303a`), `bc89f41` (was `a8e9cdf`), `216ac08` (was `ff250c6`), `dfb540e` (was `f99d772`) (on `fe34bd2` (was `8d418ee`)) |
| superseded by E7b | `02ca1dd` (was `3c2cd30`) (`node_text` borrow) and the `single_ident_path` half of `5838c14` (was `83dd37c`), dropped in the merge `49edb17` (was `adcacf2`) in favour of E7b's code |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0`, valgrind 3.22.0 |
| host | 4 vCPU Intel Xeon @ 2.80GHz, 15 GB; builds, tests, hash checks, `conform` and callgrind pinned with `taskset -c 0`; other agents ran on cores 1–3 (load average 1–9 during the work) |
| build | `cargo build --release -p chloro-core --examples`: default release profile before `4e72e10` (was `c2c0934`), the workspace's fat-LTO, one-codegen-unit profile (E2) after it |
| `Ir` command, before `4e72e10` (was `c2c0934`) | `setarch $(uname -m) -R taskset -c 0 valgrind --tool=callgrind bench -n 1 --root SAMPLE`, sample of [../profile-baseline.md](../profile-baseline.md) (49 files copied as `NN_<basename>.rs`) |
| `Ir` command, from `4e72e10` (was `c2c0934`) on | `benchmarks/profile.sh --core 0 --sample /tmp/chloro-bench/hotpath3`: the script's sample and environment at a private path of the default's length (`/tmp/chloro-bench/sample49` is shared with other agents and deleted by each run) |
| golden hashes | `scratchpad/golden/fixtures-884f9c9.tsv` (1238 fixtures) and `registry-884f9c9.tsv` (registry corpus, recorded once with the unmodified `884f9c9` binary at the start of this experiment) |

Two properties of the measurement (measured):

- The callgrind count depends on the length of the benchmark binary's path as well as the
  sample path: the `966a7d5` (was `aa1e058`) binary measures 519,451,512 at
  `/home/user/wt/hot-paths-base/target/release/examples/bench` and 518,954,466 when
  copied to `/home/user/wt/hot-paths/target/release/examples/bencB` (same length as this
  branch's binary path), against 518,951,890 in the ledger. Base binaries are therefore
  measured as copies at `bencB`, and the replay below runs in a worktree whose path has the
  length of this one's (`/home/user/wt/hot-chain`).
- The registry record holds 20,593 files. The registry directory grew to 20,847 and then
  20,850 files during the work (other sessions fetching crates); every check below compares
  the 20,593 recorded files and lists the others as not recorded.

## Hypothesis and falsification

The slow paths that the profile names shrink with identical output when the step they
compute is computed with less work: fewer allocations, no rescans, no per-character
iterator where a byte scan answers the same question. Expected 5–10% of `Ir` in total.
A lever is kept when `Ir` drops by more than 0.1% with 0 changed outputs; otherwise it is
reverted and its number recorded.

## Levers (read, measured)

Each kept commit's message states its invariant and its measured `Ir`; the sections here
summarise them. "Branch" is the measurement at the time of the commit, on the base it was
written against; "replay on `fe34bd2` (was `8d418ee`)" is the same commit replayed in order onto the current
integration head (section *Replay on fe34bd2 (was 8d418ee)*).

### Lever 8: `unicode_str_width` (`19db4fe` (was `0f5f235`), `35e1475` (was `4b5bb3a`), `bfe5af3` (was `a383516`))

- Invariant: under unicode-width 0.1.14 an ASCII string is as wide as its length minus its
  `\n` bytes minus its `\r\n` pairs; an ASCII line without `\n` is as wide as it is long.
- `19db4fe` (was `0f5f235`): `is_ascii_single_line` tests eight bytes per step with an exact zero-byte test
  (`(y & 0x7f..) + 0x7f..` never carries between bytes) and returns the length; every other
  string takes the old code, moved out of line (the inlined `str::width` path made every
  call save and restore six registers). The sample's 120,914 calls average 17 bytes; 2,695
  contain `\n`.
- `35e1475` (was `4b5bb3a`): `first_line_width` and `last_line_width` find the first or last byte that is
  `\n` or non-ASCII, eight bytes per step (forward and backward), and return the line
  length when the line is ASCII; a `\r` before the `\n` stays in the line and counts one
  column, as in the split form.
- `bfe5af3` (was `a383516`): the out-of-line path counts `\n` per word (exact flag, shifted to one bit per
  byte, summed by a multiplication) when the string has no `\r`.

### Lever 8a: `filter_normal_code` (`d74d6c6` (was `b6a249d`))

- Invariant: without a comment, every line `LineClasses` yields has a kept kind, so the
  loop rebuilds the snippet unless a line ends with `\r`. A snippet with `/`, no `\r` and no
  comment is borrowed; 432 of the 461 snippets past the `/` check on the sample take this
  path. The existing `/` check stays as it is.

### Lever 6: `CharClasses` scanning (`259abb0` (was `16e2e3d`), `6406b73` (was `9b77c23`), `e0235a6` (was `350e6e4`), `a3afd18` (was `6f7c386`), `216ac08` (was `ff250c6`), `dfb540e` (was `f99d772`))

- Invariant: a skip moves only past characters that `next` would yield with the same state
  and a kind the caller treats as the skipped characters' kind; a test checks every skip
  against a full scan on every string of up to five characters over the bytes that drive
  the state machine.
- `259abb0` (was `16e2e3d`): `skip_plain` no longer stops at `r` (unless before `#` or `"`), `/` (unless
  before `*` or `/`) or `'` (unless it can open a character literal), and skips string and
  raw-string contents.
- `6406b73` (was `9b77c23`): `skip_comment` skips comment interiors; `UngroupedCommentCodeSlices` and
  `find_uncommented` (needle reset) use it.
- `e0235a6` (was `350e6e4`): both skips find the next state-changing byte eight bytes per step
  (`find_any_byte`; the lowest flag of the borrow-based zero test is exact).
- `a3afd18` (was `6f7c386`): `CommentCodeSlices` skips in both slice kinds; at the end of the text a comment
  skip is stepped through, because the skipped spaces would decide `first_whitespace`.
- `216ac08` (was `ff250c6`): `find_uncommented` returns when the last pattern character matches; the old
  loop returned the same position one character later or at the end of the text.
- `dfb540e` (was `f99d772`): `LineClasses` keeps its own peeked item and skips up to the next `\n`; the
  kind of the skipped characters follows from the unchanged state. `skip_comment` gains a
  stop byte.

### Lever 6a: `recover_comment_removed`'s slow path (`6406b73` (was `9b77c23`), `5932986` (was `ed4303a`))

- `6406b73` (was `9b77c23`) (above) takes most of the per-character work out of `UngroupedCommentCodeSlices`.
- `5932986` (was `ed4303a`): `changed_comment_content` compares the comments of both texts as text, pairwise;
  equal texts have equal payloads, and the payload streams are compared only from the first
  pair that differs. A test compares the result with the full payload comparison on
  generated strings and eight edited variants of each.

### Lever 6b: `span_ends_with_comma` (`a58d88b` (was `d46aa59`))

- Invariant: the loop's result depends only on the last two characters that are neither
  whitespace nor in a comment (`,` last, or `,` and then one closing delimiter). Without a
  comment in the snippet, the loop runs over the tail starting at the second-to-last
  non-whitespace character; the one case where the shorter scan reads a comment the full
  scan does not (`/` directly followed by `*` or `/`) gives `false` for both. A test
  compares the two on every string of up to five characters over `, ) ] space / * " ' r # \n a`.

### Lever 15: the line-start table (`a96dfd5` (was `abdc10c`))

- Invariant: `line_of(pos)` counts the line starts at or before `pos`, each of which
  follows a `\n` before `pos`; the table is extended only up to the asked position.
  `walk_reorderable_items` asks for the lines of import and module groups, near the start
  of a file.

### Lever 4a: `node_text` (`02ca1dd` (was `3c2cd30`), superseded)

- Borrowed the token text of one-token nodes as a `Cow`: 478,903,051 → 475,166,846
  (−0.78%) on `966a7d5` (was `aa1e058`). E7b's `node_text` returns a slice of the source for every node,
  so the merge `49edb17` (was `adcacf2`) takes E7b's function and callers.

### Lever 4c: `single_ident_path`, `rewrite_method_call` (`5838c14` (was `83dd37c`))

- `single_ident_path` walked borrowed green nodes and allocated only when the name fit
  (superseded by E7b's flat-tree version in `49edb17` (was `adcacf2`)); `rewrite_method_call` builds
  `.name::<types>` in one buffer sized for its three parts (kept).

### Allocation row 2: `combine_strs_with_missing_comments` from `format_expr_uncached` (`860514d` (was `887674b`))

- Invariant: without outer attributes, `attrs_str` and `attrs_span` are empty, so the
  combination finds no comment, pushes no separator and returns a copy of `expr_str`;
  `format_expr_uncached` returns `expr_str`. 21,333 of the function's 27,329 calls on the
  sample came from this site.

### Allocation rows 5 and 16: `write_list`, `join_rewrites` (`9ad28d4` (was `0178efa`), `db2cea1` (was `e2128cf`), `bc89f41` (was `a8e9cdf`))

- `9ad28d4` (was `0178efa`): `write_list` starts its buffer at the larger of rustfmt's 128 bytes and the
  summed item, comment, separator, newline and indent lengths (3,097 reallocations on the
  sample before).
- `db2cea1` (was `e2128cf`): `join_rewrites` allocates the joined chain once instead of cloning the first
  rewrite and growing it.
- `bc89f41` (was `a8e9cdf`): `write_list`'s `contains("::")` (rustfmt's nested-import test, every item) is
  preceded by `contains(':')`, a `memchr`; 15,172 substring searches cost 2.74 M Ir.

### `format!` with string arguments (`a02ae1b` (was `c28d403`))

- Invariant: `format!` with literal text and bare `{}` placeholders whose arguments are
  strings writes the pieces unchanged; `utils::concat` joins them with one exact
  allocation. 35 single-line calls in closures.rs, expr.rs, matches.rs and attr.rs are
  converted; the compiler checks that every argument coerces to `&str`, and no type in the
  crate derefs to `str`. `format!` cost about 885 Ir per call on the sample (11.04 M in
  12,463 calls).

## Results on the branch (measured)

`Ir` on the 49-file sample, one callgrind run per build, gates on every kept commit's
binary.

| commit | lever | `Ir` before | `Ir` after | change | decision |
|---|---|---|---|---|---|
| `19db4fe` (was `0f5f235`) | 8, ASCII single-line width | 587,212,338 | 578,299,701 | −8,912,637 (−1.52%) | kept |
| `d74d6c6` (was `b6a249d`) | 8a, `filter_normal_code` | 578,299,701 | 577,151,808 | −1,147,893 (−0.20%) | kept |
| `259abb0` (was `16e2e3d`) | 6, `skip_plain` coverage | 577,151,808 | 571,637,469 | −5,514,339 (−0.96%) | kept |
| `a58d88b` (was `d46aa59`) | 6b, `span_ends_with_comma` | 571,637,469 | 563,223,712 | −8,413,757 (−1.47%) | kept |
| `6406b73` (was `9b77c23`) | 6, 6a, `skip_comment` | 563,223,712 | 559,915,459 | −3,308,253 (−0.59%) | kept |
| `e0235a6` (was `350e6e4`) | 6, eight-byte skips | 559,915,459 | 556,580,733 | −3,334,726 (−0.60%) | kept |
| `a3afd18` (was `6f7c386`) | 6, `CommentCodeSlices` | 556,580,733 | 552,089,041 | −4,491,692 (−0.81%) | kept |
| `a96dfd5` (was `abdc10c`) | 15, line table | 552,089,041 | 548,286,879 | −3,802,162 (−0.69%) | kept |
| `4e72e10` (was `c2c0934`) | merge of `966a7d5` (was `aa1e058`) (`profile.sh` from here on) | 518,954,466 (`966a7d5` (was `aa1e058`)) | 478,903,051 | −40,051,415 (−7.72%) | — |
| `02ca1dd` (was `3c2cd30`) | 4a, `node_text` | 478,903,051 | 475,166,846 | −3,736,205 (−0.78%) | kept, superseded by E7b |
| `5838c14` (was `83dd37c`) | 4c | 475,166,846 | 473,503,959 | −1,662,887 (−0.35%) | kept (callee half after E7b) |
| `860514d` (was `887674b`) | row 2, expressions without attributes | 473,503,959 | 464,822,802 | −8,681,157 (−1.83%) | kept |
| `35e1475` (was `4b5bb3a`) | 8, first and last line widths | 464,822,802 | 456,123,059 | −8,699,743 (−1.87%) | kept |
| `bfe5af3` (was `a383516`) | 8, multi-line newline count | 456,123,059 | 453,996,189 | −2,126,870 (−0.47%) | kept |
| `db2cea1` (was `e2128cf`) | row 16, `join_rewrites` | 453,996,189 | 452,727,354 | −1,268,835 (−0.28%) | kept |
| `9ad28d4` (was `0178efa`) | row 5, `write_list` buffer | 452,727,354 | 451,010,174 | −1,717,180 (−0.38%) | kept |
| `a02ae1b` (was `c28d403`) | `format!` → `concat` | 451,010,174 | 446,063,206 | −4,946,968 (−1.10%) | kept |
| `49edb17` (was `adcacf2`) | merge of `fe34bd2` (was `8d418ee`) | 390,690,267 (`fe34bd2` (was `8d418ee`)) | 323,459,825 | −67,230,442 (−17.21%) | — |
| `5932986` (was `ed4303a`) | 6a, comment texts first | 323,459,825 | 320,989,066 | −2,470,759 (−0.76%) | kept |
| `bc89f41` (was `a8e9cdf`) | row 5, `:` before `::` | 320,989,066 | 320,173,109 | −815,957 (−0.25%) | kept |
| `216ac08` (was `ff250c6`) | 6, `find_uncommented` return | 320,173,109 | 318,131,523 | −2,041,586 (−0.64%) | kept |
| `dfb540e` (was `f99d772`) | 6, `LineClasses` | 318,131,523 | 316,301,682 | −1,829,841 (−0.58%) | kept |

The pre-merge rows use the profile baseline's sample command (587.2 M for the base); the
rows from `4e72e10` (was `c2c0934`) on use `benchmarks/profile.sh` (the two differ by about 0.13% on the
same binary, from the sample's paths). Before `4e72e10` (was `c2c0934`) the build is the default release
profile; after it, fat LTO with one codegen unit.

## Rejected and not attempted (measured, read)

| attempt | measurement | decision |
|---|---|---|
| `unicode_str_width`: `filter().count()` of `\n` (vectorised) | 587,212,338 → 591,686,904 (+0.76%): the vector prologue dominates 17-byte strings | rejected |
| the same count on the multi-line path, before the out-of-line split | 580,632,846 against 580,475,497 for the form without it (+157,349) | rejected |
| eight-byte `\n` count on the out-of-line path, default profile | 578,357,869 against 578,301,408 (+56,461) | rejected then; the same count on the LTO build after `35e1475` (was `4b5bb3a`) measured −2,126,870 and is `bfe5af3` (was `a383516`) |
| `skip_plain` with the lookahead computed for every byte | 577,151,808 → 577,935,381 (+783,573) | rejected; `259abb0` (was `16e2e3d`) computes it only at the four special bytes (−980,070 for the `Normal` part alone) |
| line table through `match_indices('\n')` over the whole source | 552,089,041 → 552,608,014 (+518,973) | rejected |
| line table as a plain loop over the whole source | 552,089,041 → 550,701,672 (−1,387,369) | superseded by the on-demand table of `a96dfd5` (was `abdc10c`) |
| `write_list` estimate of items and comments plus 16 bytes | 452,727,354 → 451,086,960 (−1,640,394) | superseded by the fuller estimate of `9ad28d4` (was `0178efa`) (−1,717,180) |
| lever 4b, memo result as `Rc<str>` | not measured | not attempted: `format_expr` returns `String`, so storing an `Rc<str>` copies the result once, as the clone does; removing the copy needs `Rewrite` to return a shared string (row 4 of the lever table) |
| lever 11, memo without the stored node | not measured | superseded: E7b keys the memo by (tree id, index) and stores no node |
| row 7, `wrap_items` | read | not changed: chloro already sizes the buffer from its parts |
| row 20, `Rc` per `RewriteContext::new` | read | not attempted: the `Rc<Cell<bool>>` is rustfmt's shared `inside_macro` flag; removing the allocation changes how contexts share it |
| X8's token index for comment queries | — | not rebuilt (refuted earlier at +3%) |

## Replay on fe34bd2 (was 8d418ee) (measured)

The kept commits replayed in order onto the integration head (`385815c` (was `60eb8e0`), docs-only after
`fe34bd2` (was `8d418ee`)) in the worktree `/home/user/wt/hot-chain` on the local branch `scratch-e3-chain`
(not pushed): `02ca1dd` (was `3c2cd30`) is left out, `5838c14` (was `83dd37c`) contributes its chains.rs half, and the
conflicts of `a96dfd5` (was `abdc10c`) (context.rs) and `a02ae1b` (was `c28d403`) (closures.rs) take this branch's
resolved files, which differ from `fe34bd2` (was `8d418ee`) only by those commits. The replayed tree's
library code equals this branch's head. One callgrind run per commit.

| replayed commit | branch commit | lever | `Ir` before | `Ir` after | change | share of base |
|---|---|---|---|---|---|---|
| `385815c` (was `60eb8e0`) (`fe34bd2` (was `8d418ee`) and docs) | — | base | — | 390,690,354 | — | — |
| `18b0122` | `19db4fe` (was `0f5f235`) | 8, ASCII single-line width | 390,690,354 | 379,936,350 | −10,754,004 (−2.75%) | −2.75% |
| `c13d977` | `d74d6c6` (was `b6a249d`) | 8a, `filter_normal_code` | 379,936,350 | 378,804,336 | −1,132,014 (−0.30%) | −0.29% |
| `830cdb3` | `259abb0` (was `16e2e3d`) | 6, `skip_plain` coverage | 378,804,336 | 373,406,577 | −5,397,759 (−1.42%) | −1.38% |
| `432e564` | `a58d88b` (was `d46aa59`) | 6b, `span_ends_with_comma` | 373,406,577 | 365,131,665 | −8,274,912 (−2.22%) | −2.12% |
| `d70336d` | `6406b73` (was `9b77c23`) | 6, 6a, `skip_comment` | 365,131,665 | 361,904,825 | −3,226,840 (−0.88%) | −0.83% |
| `3d70fb5` | `e0235a6` (was `350e6e4`) | 6, eight-byte skips | 361,904,825 | 358,787,905 | −3,116,920 (−0.86%) | −0.80% |
| `daa79f7` | `a3afd18` (was `6f7c386`) | 6, `CommentCodeSlices` | 358,787,905 | 354,277,314 | −4,510,591 (−1.26%) | −1.15% |
| `58712ca` | `a96dfd5` (was `abdc10c`) | 15, line table | 354,277,314 | 350,372,410 | −3,904,904 (−1.10%) | −1.00% |
| `860de17` | `5838c14` (was `83dd37c`) (chains.rs half) | 4c, callee buffer | 350,372,410 | 349,431,561 | −940,849 (−0.27%) | −0.24% |
| `80ffe40` | `860514d` (was `887674b`) | row 2, expressions without attributes | 349,431,561 | 341,065,422 | −8,366,139 (−2.39%) | −2.14% |
| `aec532b` | `35e1475` (was `4b5bb3a`) | 8, first and last line widths | 341,065,422 | 332,365,786 | −8,699,636 (−2.55%) | −2.23% |
| `3d4498d` | `bfe5af3` (was `a383516`) | 8, multi-line newline count | 332,365,786 | 330,301,335 | −2,064,451 (−0.62%) | −0.53% |
| `9056b61` | `db2cea1` (was `e2128cf`) | row 16, `join_rewrites` | 330,301,335 | 328,961,812 | −1,339,523 (−0.41%) | −0.34% |
| `a9eda13` | `9ad28d4` (was `0178efa`) | row 5, `write_list` buffer | 328,961,812 | 326,653,624 | −2,308,188 (−0.70%) | −0.59% |
| `cd2c47b` | `a02ae1b` (was `c28d403`) | `format!` → `concat` | 326,653,624 | 323,459,873 | −3,193,751 (−0.98%) | −0.82% |
| `b6c039b` | `5932986` (was `ed4303a`) | 6a, comment texts first | 323,459,873 | 320,989,129 | −2,470,744 (−0.76%) | −0.63% |
| `1f0e917` | `bc89f41` (was `a8e9cdf`) | row 5, `:` before `::` | 320,989,129 | 320,173,057 | −816,072 (−0.25%) | −0.21% |
| `ebc3c20` | `216ac08` (was `ff250c6`) | 6, `find_uncommented` return | 320,173,057 | 318,131,586 | −2,041,471 (−0.64%) | −0.52% |
| `eb310be` | `dfb540e` (was `f99d772`) | 6, `LineClasses` | 318,131,586 | 316,301,692 | −1,829,894 (−0.58%) | −0.47% |
| **total** | | | 390,690,354 | 316,301,692 | **−74,388,662 (−19.04%)** | |

Each step builds `examples/bench` and runs `profile.sh` once; the gates ran on the branch
commits, not on the replayed ones. Every lever removes more than 0.1% on this base; the
smallest is the callee buffer of `5838c14` (was `83dd37c`) (−0.27%). The replay's last point and this
branch's head measure 10 instructions apart.

## Totals (measured)

| | `fe34bd2` (was `8d418ee`) | head `dfb540e` (was `f99d772`) | change |
|---|---|---|---|
| `Ir`, 49-file sample (`profile.sh`) | 390,690,267 | 316,301,682 | −74,388,585 (−19.04%) |
| `malloc` calls (callgrind) | 233,281 | 211,520 | −21,761 (−9.3%) |
| `realloc` calls (callgrind) | 45,794 | 27,277 | −18,517 (−40.4%) |

Largest changes of exclusive `Ir` by function, `fe34bd2` (was `8d418ee`) → head: `CharClasses::next`
27.75 M → 4.47 M, `unicode_str_width` 20.69 M → 0.03 M (`first_line_width`,
`last_line_width`, the out-of-line path and `unicode_str_width` take 5.47 M together, and
the inlined `is_ascii_single_line` runs inside callers such as `total_item_width`,
1.62 M), `UngroupedCommentCodeSlices::next` 5.97 M → 0.36 M, the line table
4.76 M → `line_of` 1.26 M, `CharSearcher::next_match` 10.84 M → 7.29 M, `memrchr`
6.07 M → 2.84 M, `CommentReducer::next` 2.04 M → 0, `format_inner` 1.76 M → 0.67 M;
`skip_plain` (7.13 M) and `skip_comment` (1.72 M) are new.

## Wall clock (measured)

`bench -n 5` on the 1238 fixtures (13.9 MB), the `fe34bd2` (was `8d418ee`) binary and the head binary
interleaved, three pairs, `taskset -c 0`, load average 2.4–2.9 (other agents on cores 1–3).

| pair | `fe34bd2` (was `8d418ee`), best of 5 | head `dfb540e` (was `f99d772`), best of 5 | difference |
|---|---|---|---|
| 1 | 1.778 s, 7.82 MB/s | 1.464 s, 9.50 MB/s | +21.5% |
| 2 | 1.812 s, 7.67 MB/s | 1.494 s, 9.30 MB/s | +21.3% |
| 3 | 1.811 s, 7.68 MB/s | 1.482 s, 9.38 MB/s | +22.1% |

The ranges are 7.67–7.82 MB/s and 9.30–9.50 MB/s and do not overlap.

## Gates (measured)

| gate | corpus | result, every kept commit and both merges |
|---|---|---|
| `cargo fmt --check` | | no differences |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | no warnings |
| `cargo test -p chloro-core` | 174 tests at `19db4fe` (was `0f5f235`) rising to 215 at `dfb540e` (was `f99d772`) (library, conformance batches, integration) | all pass |
| `bench --check` | 1238 fixture files against `fixtures-884f9c9.tsv` | 0 changed |
| `bench --check` | 20,593 registry files under `~/.cargo/registry/src` against `registry-884f9c9.tsv` | 0 changed |
| `conform -i` | 1220 rust-analyzer fixtures, `rustfmt_version` 1.9.0 | 1219/1220 identical, 0 not idempotent |
| `conform -i -V 1.8.0` | the same fixtures under `rustfmt_version` 1.8.0, from `49edb17` (was `adcacf2`) on | 1219/1220 identical, 0 not idempotent |
| workspace tests | `cargo test --workspace --all-features` at the head (`dfb540e` (was `f99d772`) with the docs merge) | 233 passed, 0 failed, 1 ignored |

`config_matrix` (gate 6) and the registry `config_matrix` run against live rustfmt (gate 7)
were not run: no output changed on either hash corpus.

## Tests added (read)

| test | file | compares |
|---|---|---|
| `ascii_width_fast_path_matches_unicode_width` | utils.rs | `unicode_str_width` with `str::width`, lengths 0–39, one special character at every position |
| `line_widths_match_the_split_lines_and_unicode_width` | utils.rs | first and last line widths with the split forms, and `unicode_str_width` with `str::width`, two special characters |
| `concat_matches_format_on_strings` | utils.rs | `concat` with `format!` |
| `filter_normal_code_fast_paths_match_line_classes` | comment.rs | the borrowed result with the line loop |
| `skip_plain_skips_only_unchanged_normal_or_string_characters`, `skip_comment_skips_only_unchanged_comment_characters` | comment.rs | every skip with a full scan |
| `find_any_byte_finds_the_first_target` | comment.rs | the eight-byte search with a plain search on 20,000 generated byte strings |
| `comment_code_slices_match_the_per_character_scan` | comment.rs | `CommentCodeSlices` with a copy of the per-character loop |
| `changed_comment_content_matches_the_whole_payload_comparison` | comment.rs | the pairwise comparison with the full payload comparison |
| `find_uncommented_matches_the_per_character_search` | comment.rs | the search with rustfmt's per-character loop, eleven patterns |
| `line_classes_match_the_per_character_scan` | comment.rs | `LineClasses` with a copy of the `Peekable` loop |
| `comma_scan_tail_gives_the_full_scan_result` | expr.rs | the loop over the tail with the loop over the whole string |
| `no_attributes_combine_to_the_expression` | expr.rs | `combine_strs_with_missing_comments` with an empty first string and span |
| `line_of_counts_the_lines_up_to_the_position` | context.rs | `line_of` with a count of `\n` bytes, positions in mixed order |

Each comparison test was checked against a mutation of the code it covers (a dropped stop
byte, a dropped guard, an off-by-one position); every mutation failed its test.

## Interpretation

- The total exceeds the hypothesis's 5–10%: −19.04% on `fe34bd2` (was `8d418ee`), where E7b had already
  removed the red-node and green-tree costs, so the string and comment scans these levers
  act on are a larger share of what is left.
- On `fe34bd2` (was `8d418ee`) the four largest steps are the ASCII single-line width (−2.75% of the
  base), the first and last line widths (−2.23%), expressions without attributes (−2.14%)
  and `span_ends_with_comma` (−2.12%); the last three remove a scan or a copy rather than
  speed one up.
- `CharClasses::next` keeps its state machine and costs 4.47 M Ir on the head; the skips
  answer the stretches where the state cannot change.
- The eight-byte newline count lost on the default-profile build and gained 2.13 M on the
  LTO build after the line-width change: the same code measured in a different build and
  call mix.

## Decision

Kept: the commits listed under *kept commits*. Branch `20261014-hot-paths` at the head
named in the ledger row, ready for the integrator's gates 1–5 on the merged tree.

## Follow-up

Remaining costs on the head (exclusive `Ir`, read from the head's callgrind profile):
`rustc_compat::walk` 9.38 M (lever 10), `CharSearcher::next_match` 7.29 M (20,206 calls
from `filtered_str_fits`'s `contains('\n')` and `lines()`), `skip_plain` 7.13 M,
`overflow::Context::rewrite` 7.73 M, `write_list` 6.52 M (with the inlined width code),
`trim_matches(char::is_whitespace)` 4.37 M (`extract_pre_comment`, `extract_post_comment`),
`find_uncommented` 3.51 M, `filtered_str_fits` 3.12 M, `memrchr` 2.84 M.
