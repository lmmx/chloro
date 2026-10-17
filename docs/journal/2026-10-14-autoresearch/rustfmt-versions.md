# rustfmt versions: where 1.7.0, 1.8.0 and 1.9.0 differ

Part of [../2026-10-14-autoresearch.md](../2026-10-14-autoresearch.md). Labels as in that
entry: **measured** (run on this host), **read** (from source or git history),
**hypothesis** (not established). No chloro source file was changed for this document.

Four separate things are named here and kept apart:

- **edition**: the Rust language edition given to the parser (`--edition`, `edition`);
- **style edition**: the rule set inside one rustfmt (`style_edition` 2015/2018/2021/2024,
  and the unstable 2027; `version = One/Two` before rustfmt 1.8.0);
- **rustfmt release**: 1.7.0, 1.8.0, 1.9.0;
- **toolchain**: the rustc release that ships a rustfmt build, with its own rustc parser.

## Toolchains (measured, `--version`)

| toolchain | rustc | rustfmt |
|---|---|---|
| 1.80.0 | `rustc 1.80.0 (051478957 2024-07-21)` | `rustfmt 1.7.0-stable (0514789 2024-07-21)` |
| 1.85.0 | `rustc 1.85.0 (4d91de4e4 2025-02-17)` | `rustfmt 1.8.0-stable (4d91de4e48 2025-02-17)` |
| 1.88.0 | `rustc 1.88.0 (6b00bc388 2025-06-23)` | `rustfmt 1.8.0-stable (6b00bc3880 2025-06-23)` |
| stable | `rustc 1.97.0 (2d8144b78 2026-07-07)` | `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)` |

The hash in the rustfmt version string is the rust-lang/rust commit. Below, "1.8.0 (1.85)"
and "1.8.0 (1.88)" name the two 1.8.0 builds.

**What source a toolchain ships (read).** rust-lang/rust carries rustfmt as a subtree;
rustfmt's own commits reach a toolchain only through a sync, and rust-side commits reach
rustfmt's repository through the `subtree-push-nightly-*` merges. The first parent of each
such merge in `/home/user/ref/rustfmt-git` is the rust-side tree at that nightly:
`ba8ee715` (nightly-2025-01-02), `19636413` (nightly-2025-04-02), `ab08cc99`
(nightly-2026-02-19). `git merge-base --is-ancestor` reports that none of the three
contains the rustfmt-side commits `5f48fe9b` (style_edition 2027), `2d049af8` (version
sort for `mod`), `a43eef16` (#6425 raw labels), `5619b644` (#6468 closure labels),
`334670e0` (#6573 `use self::self`), `a57ce886` (pre-2024 import order), `74931504` (#5739)
or `f371e165`. The shipped 1.8.0 builds are therefore the rustfmt tree of the
2024-09-19 sync (`28beb813`, `v1.8.0` era) plus rust-side commits up to each beta branch:
the effect of `7de0ca8b` "Disable overflow_delimited_expr in edition 2024" (2025-01-30)
is present in both (probe `ode.rs`: all four toolchains print `do_thing(x, Bar {..})` with the struct on its
own lines), and `266da427` "Do not remove super keyword from super-let" (2025-05-12) is in
neither (class F4). The tags `v1.8.0` and `v1.9.0` are rustfmt-repository states; line
citations below use the worktrees `/home/user/ref/rustfmt-v1.{7,8,9}.0`, and a `v1.8.0`
citation shows the behaviour the shipped 1.8.0 builds have unless stated otherwise.

## Method (measured)

- Binaries: `~/.rustup/toolchains/<T>-x86_64-unknown-linux-gnu/bin/rustfmt` (the file
  `rustup run <T> rustfmt` executes, without the proxy). Every file is formatted from
  stdin: `timeout 120 taskset -c 1 $RUSTFMT <args> < file > out 2> err`, exit status
  recorded (`bin/run.sh`). No `rustfmt.toml` exists in the working directory or its
  ancestors.
- Scratch root `S = /tmp/claude-0/-home-user-chloro/9623d6dc-d933-5682-9d83-7762284f5c7f/scratchpad/versions/`:
  outputs in `S/out/<run>/`, exit codes in `S/out/<run>.rc`, unified diffs and lists in
  `S/diffs/<runA>__<runB>/`, probes in `S/probes/` (outputs of every probe on every
  toolchain and edition in `S/probes/res/`). Driver scripts: `S/bin/fixtures_all.sh`,
  `S/bin/fixtures_more.sh`, `S/bin/registry_all.sh` (stopped by the session's 30 min
  background limit during `reg-1.9.0-e2021`; the rest ran from `S/bin/registry_rest.sh`,
  which resumed that run at the file it had reached), comparison `S/bin/cmp.sh <A> <B>`,
  import-only classifier `S/bin/strip_use.py`. The fixture runs started before the 120 s
  timeout was added to `run.sh`; every fixture invocation ended with exit 0, 1 or 101.
- Fixtures: the 1220 files under `chloro-core/tests/conformance/fixtures/rust-analyzer`,
  sorted (`S/fixtures.list`). The run `fx-1.9.0-e2024` equals the tracked snapshots
  `snapshots/ra/**.rustfmt.rs` byte for byte for all 1004 files rustfmt accepts, and the
  216 files it rejects equal their inputs in the snapshots.
- Registry: all 20,593 `.rs` files under `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`
  at the time of the run (`S/registry_all.list`, `LC_ALL=C sort`; the protocol's figure of
  20,570 predates this listing). The subset `S/registry_q.list` holds lines 1, 5, 9, ...
  of that list (every 4th file starting with the first, 5149 files).
- chloro line citations are against `HEAD` = `c4ed735` (was `fbe8685`); another session had uncommitted
  edits in `imports.rs`, `missed_spans.rs`, `reorder.rs` and `visitor.rs` while this was
  written, which shift some of those lines in the working tree.
- "Accepted" means exit status 0. Two runs are compared on files both accept; a file one
  run accepts and the other rejects is counted as an exit-status mismatch.

## Step 1: what each release accepts (measured)

Probe `fn main(){let x=1;}` and `use a::{b10, b9, B, a};` (`S/probes/t*.rs`):

| setting | 1.7.0 | 1.8.0 (1.85, 1.88) | 1.9.0 |
|---|---|---|---|
| `--edition 2015/2018/2021/2024` | all accepted | all accepted | all accepted |
| `--config style_edition=2015..2024` | rejected: ``invalid key=val pair: `style_edition=2024` `` | accepted, applied | accepted, applied |
| `--config style_edition=2027` | rejected | rejected | accepted and applied on the CLI; in `rustfmt.toml` refused: ``can't set `style_edition = Edition2027`, unstable variants are only available in nightly channel`` |
| `--style-edition 2024` (CLI flag) | `Unrecognized option` | accepted; prints three spurious ``can't set `overflow_delimited_expr = false` `` style warnings | accepted |
| `--config version=Two` | accepted and applied | accepted with deprecation warning, = `style_edition=2024` | accepted with warning naming `style_edition="2024"` |
| `version = "Two"` in `rustfmt.toml` | refused on stable: ``can't set `version = Two`, unstable features are only available in nightly channel`` | (not probed) | (not probed) |
| import order under `--edition 2024` | `{a, b10, b9, B}` (Version One) | `{B, a, b9, b10}` | `{B, a, b9, b10}` |

- 1.7.0 ignores the edition when choosing rules: `--edition 2024` parses as 2024 and
  formats with Version One. Version Two is reachable on stable only through `--config`.
- `style_edition` is declared unstable in the `v1.8.0` tag (`src/config/mod.rs:152`,
  stability flag `false`); the shipped 1.8.0 builds carry rust-side `ba8ee715` "Stabilize
  style_edition 2024 in-tree" (flag `true` at `src/config/mod.rs:152` in `ba8ee715` and
  `19636413`), and `--config style_edition=2024` behaves the same on 1.85 and 1.88 (read +
  measured).

## Compatibility matrix: fixtures (measured)

Comparisons are against 1.9.0 with the same arguments unless the row says otherwise.
"both" = files both accept; "rc" = exit-status mismatches.

| run | accepted /1220 | both | identical | differing | rc | classes |
|---|---|---|---|---|---|---|
| 1.9.0 `--edition 2024` (reference) | 1004 | | | | | |
| 1.9.0 `--edition 2021` (reference) | 826 | | | | | |
| 1.8.0 (1.88) `--edition 2024` | 1001 | 1001 | 1000 | 1 | 3 | F4; P2 |
| 1.8.0 (1.88) `--edition 2024 --config style_edition=2021` vs 1.9.0 same | 1001 | 1001 | 1000 | 1 | 3 | F4; P2 |
| 1.8.0 (1.88) `--edition 2021` | 999 | 823 | 821 | 2 | 179 | F4, P1 (1 diff + 176 rc); P2 |
| 1.8.0 (1.88) `--edition 2021 --config style_edition=2024` vs 1.9.0 same | 999 | 823 | 821 | 2 | 179 | same as above |
| 1.8.0 (1.85) `--edition 2024` | 999 | 997 | 997 | 0 | 9 | P2, P3, P4, P5, P6 |
| 1.8.0 (1.85) `--edition 2021` | 997 | 819 | 818 | 1 | 185 | P1–P6 |
| 1.7.0 `--edition 2024` | 984 | 981 | 441 | 540 | 26 | all style-edition-2024 rules absent (Version One), F8–F11; P2–P7 |
| 1.7.0 `--edition 2024` vs 1.9.0 `--edition 2024 --config style_edition=2021` | 984 | 981 | 976 | 5 | 26 | F8–F11; P2–P7 |
| 1.7.0 `--edition 2024 --config version=Two` | 984 | 981 | 446 | 535 | 26 | F7 (528 files import-only), F12, F8–F11 |
| 1.7.0 `--edition 2021` | 982 | 809 | 803 | 6 | 190 | F8–F11, P1 (1 diff + 170 rc), P2–P7 |
| 1.8.0 (1.85) vs 1.8.0 (1.88), `--edition 2024` | 999 / 1001 | 997 | 997 | 0 | 6 | P3, P4, P5, P6 |
| 1.8.0 (1.85) vs 1.8.0 (1.88), `--edition 2021` | 997 / 999 | 995 | 995 | 0 | 6 | P3, P4, P5, P6 |
| scale: 1.9.0 `--edition 2024` vs 1.9.0 `--edition 2024 --config style_edition=2021` | 1004 | 1004 | 458 | 546 | 0 | (style edition, not release) |

- The 528 import-only files of the `version=Two` row were classified by
  `S/bin/strip_use.py`: with every `use` item reduced to the sorted multiset of its tokens,
  the two outputs are equal; the remaining 7 files are listed under F8–F12.
- The 1.88-vs-1.9.0 formatting difference on the fixtures at `--edition 2024` is one file
  (`parser/test_data/parser/inline/ok/let_stmt.rs`, F4, unstable syntax).

## Compatibility matrix: registry (measured)

Comparisons against 1.9.0 with the same arguments. The subset is `S/registry_q.list`
(5149 files); "all" is `S/registry_all.list` (20,593 files).

| run | files | accepted (1.9.0 / other) | both | identical | differing | rc | classes |
|---|---|---|---|---|---|---|---|
| 1.8.0 (1.88) `--edition 2024` | all | 20,285 / 20,283 | 20,282 | 20,280 | 2 | 5 | F13 `facet-core-0.30.0/src/macros.rs`; F4 `ra_ap_parser-0.0.307/.../let_stmt.rs`; rc: P2 ×3 (`ra_ap_parser` test data), P8 `syn-1.0.109/tests/test_item.rs`, and `pest_meta-2.9.2/src/grammar.rs` (no output on either within the limit: the 1.9.0 process was stopped by hand after 2 min 41 s, the 1.88 one by the 120 s timeout) |
| 1.8.0 (1.88) `--edition 2021` | subset | 5088 / 5093 | 5088 | 5081 | 7 | 5 | F5 ×7 (`event-listener-5.4.2/src/lib.rs`, `nix-0.31.3/src/sys/termios.rs`, `winapi-0.3.9/src/um/sapi.rs`, four `winnow` files); rc: P1 ×5 (all `let chains are only allowed in Rust 2024 or later` from 1.9.0) |
| 1.8.0 (1.85) `--edition 2024` | subset | 5078 / 5078 | 5078 | 5078 | 0 | 1 | rc: `pest_meta` grammar.rs (both time out) |
| 1.8.0 (1.85) vs 1.8.0 (1.88), `--edition 2024` | subset | 5078 / 5078 | 5078 | 5078 | 0 | 0 | |
| 1.7.0 `--edition 2021` | subset | 5088 / 5090 | 5085 | 5049 | 36 | 8 | F10 ×25 (`zerocopy-0.8.63/benches/*`, `#[unsafe(no_mangle)]` → `#[no_mangle]`); F5 ×7 (the same files as the 1.88 row); F14 ×2 (`syn-{2.0.119,3.0.9}/tests/test_generics.rs`); F8 ×1, F9 ×1 (`ra_ap_parser` test data); rc: P1 ×4, P7 ×3 (`use<..>` ×2, `unsafe` field ×1), and `pest_meta` grammar.rs, which 1.7.0 formats within 120 s and 1.9.0 does not |

- The 1.8.0 (1.88) runs show no file of classes F1, F2, F3 or F6 in the registry; the
  classes the registry adds over the fixtures are F5 (7 of 5149 files at edition 2021),
  F13 (1 of 20,593) and P8 (1 of 20,593).
- 1.7.0 at `--edition 2024` was not run on the registry.

## Difference classes

F = formatting classes: both releases parse the input, the text differs. P = acceptance
classes: one toolchain's rustc parser rejects the file (exit 1, rustfmt prints nothing and
the expected output is the input) where the other formats it; these follow the toolchain,
not rustfmt's code. Counts are fixture files (`--edition 2024` unless stated) and registry
files (from the registry rows above). Reproducers are in `S/probes/`; outputs per
toolchain and edition in `S/probes/res/<probe>.<toolchain>.e<edition>.{out,rc,err}`.

### F1 raw labels lose `r#` in `break`/`continue` (1.8.0; 1.7.0 rejects the syntax)

- Reproducer `raw_label.rs`: `'r#if: { break 'r#if; }`. 1.8.0 (1.85, 1.88), editions 2021
  and 2024: `break 'if;` (the label no longer resolves). 1.9.0: `break 'r#if;`. 1.7.0:
  exit 1, ``expected `while`, `for`, `loop` or `{` after a label``.
- Counts: fixtures 0; registry 0 (both 1.88 runs).
- CHANGELOG 1.9.0 Fixed #6411; commit `a43eef16` (PR #6425).
- rustfmt: `v1.9.0 src/expr.rs:225-244` take `context.snippet(label.ident.span)`;
  `v1.8.0 src/expr.rs:201-210` format `label.ident` (rustc's `Ident` drops `r#`).
- chloro branch point: `chloro-core/src/formatter/expr.rs:222-235` (`ContinueExpr`,
  `BreakExpr`), which print the label's source text.

### F2 labelled block as closure body (1.7.0, 1.8.0)

- Reproducer `closure_label.rs`: `bar(|| 'b: { 1 })` → 1.7.0/1.8.0 `bar(|| 1)` (label
  dropped); 1.9.0 `bar(|| 'b: { 1 })`. Inside a macro call, `foo!(|| 'a: { break 'a 1; })`
  → 1.7.0/1.8.0 print `foo!(|| {        a: {        break 'a 1;` followed by `});`, which
  does not parse; 1.9.0 prints the closure with its label on three lines.
- Counts: fixtures 0; registry 0 (both 1.88 runs).
- CHANGELOG 1.9.0 Fixed #6465; commit `5619b644` (PR #6468).
- rustfmt: `v1.9.0 src/closures.rs:121-137` (`needs_block` takes the label, `|| label.is_some()`
  at :136) and `:75` passes `body` to `rewrite_closure_block`; `v1.8.0 src/closures.rs:123`
  (`needs_block` without label) and `:77` passes the inner `block`, whose span starts after
  the label, so the snippet arithmetic produces the broken text (hypothesis: the span
  arithmetic was not traced).
- chloro branch points: `closures.rs:120-132` (`needs_block`'s `has_label`) and
  `closures.rs:71-74` (`rewrite_closure_block(&body, ..)`). Reproducing the broken macro
  text needs the 1.8.0 span offsets (see Design: partial support).

### F3 `use self::self;` reduced to `use self;` (1.7.0, 1.8.0)

- Reproducer `use_self_self.rs`: 1.7.0/1.8.0 `use self;`; 1.9.0 `use self::self;`.
- Counts: fixtures 0; registry 0 (both 1.88 runs).
- CHANGELOG 1.9.0 Fixed #6558; commit `334670e0` (PR #6573).
- rustfmt: `v1.9.0 src/imports.rs:574-584` keep a `self` preceded by `self`;
  `v1.8.0 src/imports.rs:572` normalises unconditionally.
- chloro branch point: `imports.rs:465-476`.

### F4 `super let` loses `super` (1.8.0 (1.88) only; unstable syntax)

- Reproducer `super_let.rs`: `super let x = 1;` → 1.8.0 (1.88) `let x = 1;`; 1.9.0
  unchanged (statement not formatted); 1.7.0 and 1.8.0 (1.85) exit 1 (rustc parser has no
  `super let`, class P6).
- Counts: fixtures 1 (`parser/test_data/parser/inline/ok/let_stmt.rs`); registry 1 (`ra_ap_parser-0.0.307`, the same test file).
- No CHANGELOG entry; rust-side commit `266da427` (2025-05-12), after the 1.88 beta branch.
- rustfmt: `v1.9.0 src/items.rs:68-71` returns `SkipFormatting` when `self.super_` is set;
  `v1.8.0 src/items.rs:49` (`impl Rewrite for ast::Local`) has no such check (the 1.88 AST
  has the field, the formatter ignores it).
- chloro branch point: `items.rs:135-138` (`has_token(.., T![super])` → `None`).

### F5 pre-2024 import order of `_`-prefixed names (1.7.0, 1.8.0; style edition ≤ 2021)

- Reproducer `use_underscore2.rs`, `--edition 2021`: `use b::{Ab, _x};` → 1.7.0/1.8.0
  `use b::{Ab, _x};`, 1.9.0 `use b::{_x, Ab};`. `use a::{ab, _x, Ab};` gives
  `{_x, ab, Ab}` on all releases. `use_underscore3.rs` (the registry pattern):
  `use libc::{NCCS, _POSIX_VDISABLE};` stays as written on 1.7.0/1.8.0 and becomes
  `{_POSIX_VDISABLE, NCCS}` on 1.9.0; `use crate::Parser;` sorts before
  `use crate::_tutorial::chapter_7;` on 1.7.0/1.8.0 and after it on 1.9.0. Under style
  edition 2024 the releases agree (version sort).
- Counts: fixtures 0; registry 7 of the 5149-file subset at `--edition 2021`, for 1.8.0 (1.88) and for 1.7.0
  alike; 0 at edition 2024 (version sort applies).
- CHANGELOG 1.9.0 Fixed #6668 (panic when sorting imports starting with `_`); commit
  `a57ce886`.
- rustfmt: `v1.9.0 src/imports.rs:953-960` sort by the key `(is_upper_snake_case,
  starts_with_uppercase, ident)`; `v1.8.0 src/imports.rs:945-958` compare with early
  returns, which is not transitive (`Ab < _x < ab < Ab`): the result depends on the sort
  algorithm and the input order, and CHANGELOG #6668 reports a panic from it.
- chloro branch point: `imports.rs:574-583` (the `else` arm of the `style_edition >=
  Edition2024` test).

### F6 Unicode whitespace in a blank-line gap: panic (1.7.0, 1.8.0)

- Reproducer `uws2.rs` (`// compile-flags: x\n\n\n\u{2029}\n`): 1.7.0, 1.8.0 (1.85, 1.88)
  exit 101, `panicked at src/tools/rustfmt/src/missed_spans.rs:216:36` (`:215:36` on 1.7.0):
  `byte index 23 is not a char boundary`; 1.9.0 exit 0. U+2028/U+2029/U+200E between
  items without the preceding comment (`unicode_ws.rs`) format identically on 1.8.0 (1.88)
  and 1.9.0.
- Counts: fixtures 0; registry 0 (both 1.88 runs).
- CHANGELOG 1.9.0 Fixed #5739; commit `74931504`.
- rustfmt: `v1.9.0 src/missed_spans.rs:210` `line_start = offset + subslice.len()`;
  `v1.8.0 src/missed_spans.rs:209` `offset + lf_count + crlf_count * 2`.
- chloro branch point: `missed_spans.rs:150-153` (already the 1.9.0 arithmetic).

### F7 import version sorting under style edition 2024 (1.7.0)

- Reproducer `t2.rs`: `use a::{b10, b9, B, a};` → 1.7.0 (any edition, also `version=Two`)
  `{a, b10, b9, B}`; 1.8.0/1.9.0 with style edition 2024 `{B, a, b9, b10}`.
- Counts: fixtures 528 files differ only in `use` items between 1.7.0 `version=Two` and
  1.9.0 `--edition 2024`; registry: not measured for 1.7.0 at edition 2024.
- CHANGELOG 1.8.0 Changed #6284; commits `1681c91a`, `29641726` (2024-07-09).
- rustfmt: `v1.9.0 src/imports.rs:935,951,973` (`version_sort`); `v1.7.0
  src/imports.rs:893-929` (Version::Two branches without version sort).
- chloro branch points: `imports.rs:556,566,572,594` (four `style_edition >= Edition2024`
  tests); for 1.7.0 the style edition itself is never 2024 (Design).

### F8–F11 newer syntax 1.7.0 formats differently (1.7.0)

The five files of the row "1.7.0 `--edition 2024` vs 1.9.0 `style_edition=2021`"
(`S/diffs/fx-1.9.0-e2024-se2021__fx-1.7.0-e2024/`), reproduced in `type_alias_where.rs`,
`extern_safe.rs`, `unsafe_attr.rs`, `misc17.rs`:

| class | input | 1.7.0 | 1.8.0 / 1.9.0 | fixtures | source |
|---|---|---|---|---|---|
| F8 type alias with trailing `where` | `type Foo<T>   =   Vec<T>   where   T:Copy ;` | unchanged (item not formatted) | `type Foo<T>` / `    = Vec<T>` / `where` / `    T: Copy;` | 1 | CHANGELOG 1.8.0 Added #5887, `df06d7eb`; `v1.9.0 src/items.rs:1742` `rewrite_ty`, `:1803`, `:1848` |
| F9 `safe`/`unsafe` statics in `extern` blocks | `pub safe static A: i32;` | `pub static A: i32;` | unchanged | 1 | CHANGELOG 1.7.1 #6204, `30cdc2b3`; `v1.9.0 src/items.rs:3491` `format_safety` |
| F10 unsafe attributes | `#[unsafe(no_mangle)]`, `#[unsafe(export_name = "x")]` | `#[no_mangle]`, `#[export_name = "x"]` (list form `#[unsafe(a(b))]` kept) | unchanged | 1 | `8abbcad9`; `v1.9.0 src/attr.rs:374`, `v1.8.0 src/attr.rs:381` |
| F11 generic const items (unstable `generic_const_items`) | `const C<T>: u32 = 0 where T: Copy;` | `const C: u32 = 0;` | unchanged (not formatted) | 2 | `d03d3386` "Do not format generic consts"; `v1.9.0 src/items.rs:1988,2022` |

chloro locations: `items.rs:3166` (`rewrite_ty`), `items.rs:3317-3381` (`StaticParts`,
`rewrite_static`), `attr.rs:517`, `items.rs:3331` (`StaticParts::from_const`).

### F12 Version Two fixes absent from 1.7.0 (1.7.0 with `--config version=Two` only)

| fix | reproducer | 1.7.0 `version=Two` | 1.9.0 style 2024 | fixtures | source |
|---|---|---|---|---|---|
| comment containing `=>` after a match arm's `=>` | `arm_comment_arrow.rs` | arm body left as written (`foo(  a  );`) | `foo(a);` | 1 (`hir-ty/src/infer/coerce.rs`) | CHANGELOG 1.7.1 #5998, `9580747a`; `v1.9.0 src/matches.rs:443-447`; chloro `matches.rs:445` |
| inner attributes in `match` indented | `arm_inner_attr.rs` | 2nd and later `#![..]` at the `match` indentation | all at arm indentation | 1 | CHANGELOG 1.7.1 #6147; `v1.9.0 src/matches.rs:115`; chloro `matches.rs:122` |
| `trace!` formatted like `debug!` | `trace.rs` | one argument per line | format string, then `alpha, beta, gamma_delta` on one line | 0 | CHANGELOG 1.7.1 #5987; `v1.9.0 src/overflow.rs:66-69`; chloro `overflow.rs:212` |

1.9.0 with `style_edition=2021` keeps the old behaviour of all three (they are style-edition
gated), so they matter only for 1.7.0 with `version=Two`.

### F13 `macro_rules!` body that is one `const { .. }` block (1.7.0, 1.8.0)

- Reproducer `macro_const_block.rs` (shape of `facet-core-0.30.0/src/macros.rs`): 1.7.0
  and 1.8.0 (1.85, 1.88) leave the macro body as written; 1.9.0 formats it, with the
  mixed indentation the chloro journal describes (`Some(` at 12 columns inside a block at
  16).
- Counts: fixtures 0; registry 1 of 20,593 (`facet-core-0.30.0/src/macros.rs`).
- No CHANGELOG entry; rust-side commits `a0dc8e3c` (2025-11-22, "Audit
  `AllowConstBlockItems`") and `d7923b14` (2025-11-26, "rustfmt: support const block
  items"): rustc's parser reads `const { .. }` in item position as `ItemKind::ConstBlock`
  and `v1.9.0 src/visitor.rs:539` formats it; the 1.8.0 parser has no such item and the
  macro body fails to format as a file.
- chloro: const block items are not implemented (Missing in
  [../2026-10-10-performance-checkpoint.md](../2026-10-10-performance-checkpoint.md));
  `target/release/chloro` (built 2026-10-14 14:44 from this checkout) prints the 1.8.0
  (1.88) output byte for byte on the reproducer.

### F14 `?for<> Trait` inside macro arguments (1.7.0)

- Input `quote!(?for<> Trait)` (in `syn-2.0.119/tests/test_generics.rs`): 1.7.0 prints
  `quote!(?Trait)`; 1.8.0 (1.88) and 1.9.0 print it unchanged (`--edition 2021`). The
  rustc 1.80 parser reads the argument as a type and its rewrite drops the empty binder
  (hypothesis, from the output; the 1.7.0 bound rewrite was not read); rustc 1.97 rejects
  `for<>` after `?` (P7'), so the macro call is left as written. chloro rejects the same
  bound (`rustc_compat.rs`, `TYPE_BOUND` with `FOR_BINDER` and `?`). Registry: 2 files of
  the subset; fixtures 0.

**chloro on the reproducers (measured).** `target/release/chloro` gives rustfmt 1.9.0's
output (`--edition 2024`) on every probe of F1–F12 (`raw_label`, `closure_label`,
`use_self_self`, `super_let`, `use_underscore2`, `arm_comment_arrow`, `type_alias_where`,
`extern_safe`, `unsafe_attr`, `misc17`, `trace`, `uws2`, `t2`, `mods`) and at
`edition=2021` on `letchain_macro` and `use_underscore2`; it differs on
`macro_const_block` only (outputs in `S/probes/chl/`).

### P1–P7 acceptance classes (toolchain parser)

| class | construct | 1.7.0 | 1.8.0 (1.85) | 1.8.0 (1.88) | 1.9.0 | fixture files |
|---|---|---|---|---|---|---|
| P1 | `if a && let ..` with edition ≤ 2021 | formats | formats | formats | exit 1 `let chains are only allowed in Rust 2024 or later` | 176 (1.88 vs 1.9.0 at 2021); plus `ide-diagnostics/src/handlers/json_is_not_rust.rs`, where the let chain sits in `vec![..]` arguments: 1.9.0 leaves the macro call as written, 1.7.0/1.8.0 format it (`letchain_macro.rs`) |
| P2 | `[const]` trait bounds | exit 1 | exit 1 | exit 1 | formats | 3 |
| P3 | frontmatter (`---` block) | exit 1 | exit 1 | formats | formats | 3 |
| P4 | `const { .. }` patterns | formats | formats | exit 1 | exit 1 | 1 |
| P5 | `S { #[attr] .. }` | formats | formats | exit 1 | exit 1 | 1 |
| P6 | `super let` | exit 1 | exit 1 | formats (F4) | formats (skips stmt) | 1 |
| P7 | `use<..>` bounds; return-type notation `(..)`; field default values; `unsafe` fields | exit 1 | formats | formats | formats | 12; 2; 1; 1 |
| P7' | `for<'a> ?Trait` | formats | exit 1 | exit 1 | exit 1 | 1 |
| P8 | `impl !Trait {}` (negative inherent impl; probe `neg_impl.rs`) | formats | formats | formats | exit 1 `inherent impls cannot be negative` | 0 (registry: `syn-1.0.109/tests/test_item.rs`) |

chloro expresses rustc's acceptance in `rustc_compat.rs`: P1 at `rustc_compat.rs:63-71`
(`let_expr_allowed`, `edition.at_least_2024()`), P4/P5/P7'/P8 at `rustc_compat.rs:128-170`
(`node_rejected`: `CONST_BLOCK_PAT`, `REST_PAT`, `TYPE_BOUND`, `IMPL` with `!` and no
`for` at `:133-138`); P2, P3, P6, P7 have no rule there because rust-analyzer and
rustc 1.97 both accept them.

## Step 4: 1.8.0 from 1.85.0 and from 1.88.0 (measured)

- Fixtures: no file both builds accept is formatted differently (997/997 at 2024,
  995/995 at 2021). The six exit-status mismatches are P3 (3 files), P4, P5 and P6 (one
  each).
- Registry subset at `--edition 2024`: 5078 files both builds accept, all identical, no
  exit-status mismatch.
- The F4 text difference (`super let`) is reachable only on 1.88, because 1.85 rejects the
  syntax.

## Behaviours that are not release differences (read + measured)

- Style edition 2027 (unstable; on stable 1.9.0 accepted from `--config` and
  `--style-edition`, refused from `rustfmt.toml`): version-sorted `mod` and `extern crate`
  order (`v1.9.0 src/reorder.rs:28-67`; probe `mods.rs`: `mod a9;` before `mod a10;` only
  with 2027), long `if let` patterns (#6202, `src/expr.rs` `rewrite_let`), `impl Trait`
  parameter wrapping (#6381, `src/types.rs:1013-1019`), `lazy_static::lazy_static!`
  (#6287), loop-body trailing semicolons with `trailing_semicolon` (#6711). chloro's
  `Config::set` rejects `style_edition=2027` (`config.rs:394-401`).
- Nightly-only options added in 1.9.0: `match_arm_indent` (default `true`, the behaviour
  chloro reproduces), `float_literal_trailing_zero` (default `Preserve`). Unstable options
  stay at their defaults in chloro (`config.rs`, `UNSTABLE_DEFAULTS`).
- Unstable syntax that 1.9.0 leaves unformatted or formats newly: `super let` (skipped),
  generic const items (skipped), `use ||` closures and `.use` (`ergonomic_clones`,
  `cdccf235`), heterogeneous `try` blocks (`b883eeb8`), const block items (`d7923b14`),
  `&pin` patterns (`8f1be83b`), guard patterns, `unsafe` fields, field default values
  (`c6a272b2`). The registry and fixture inputs that use these are counted in the classes
  above only where a stable toolchain also parses them.
- `version` sorting of `mod` declarations: the `v1.8.0` tag has none; `2d049af8` added it
  for 2024 on the rustfmt side and `a3e7a640` moved it to 2027 before any sync, so no
  shipped release sorts `mod` by version under 2024 (probe `mods.rs`: 1.85, 1.88, 1.9.0
  give `mod B; mod a10; mod a9;`). The same holds for `a7e0c15d`/`d21c8f34` (zero-argument
  functions: deferred to 2027, then reverted).
- `overflow_delimited_expr`: the `v1.8.0` tag defaults it to `true` under style edition
  2024 (CHANGELOG 1.8.0 #6260); no installed toolchain has that default (probe `ode.rs`).
  1.8.0 builds in rust 1.83 and 1.84 are not installed (Open questions).

## Design: a `rustfmt_version` knob

### Configuration

- `Config` gains one field outside rustfmt's option table:
  `pub rustfmt_version: RustfmtVersion`, an enum `{ V1_7_0, V1_8_0, V1_9_0 }` deriving
  `Copy, Ord`, default `V1_9_0`. `Config::set("rustfmt_version", "1.8.0")` parses it; the
  CLI takes it through the existing `--config`. rustfmt has no such option, so the doc
  comment names it a chloro option and `rustfmt.toml` files never carry it.
- `V1_8_0` reproduces the 1.88.0 build (`6b00bc3880`). The 1.85.0 build differs from it
  only in acceptance (P3–P6); a fourth variant `V1_8_0_Rust1_85` covers that if wanted
  (Open questions).
- `Settings` gains `rustfmt_version()` next to `style_edition()` (`config.rs:373-379`).
  For `V1_7_0`, `Settings::style_edition()` returns `Edition2015` whatever `edition` says
  (1.7.0's Version One under `--edition 2024`), and `Config::set` rejects `style_edition`
  as 1.7.0 does; `version=Two` support is left out because stable 1.7.0 refuses it from
  `rustfmt.toml`.
- Each class gets one branch at the call site named below, written as the `style_edition`
  branches are (`if context.config.rustfmt_version() < RustfmtVersion::V1_9_0 { old }
  else { new }`), with a comment naming the rustfmt commit. On the default path the cost
  is one load and compare of a `Copy` enum in code that runs only for the construct
  (labels, labelled closure bodies, `use` normalisation, `super let`, the pre-2024 import
  comparator, blank-line gaps). The import comparator runs O(n log n) times per `use`
  list, and only under style edition ≤ 2021.

### Branch points and size (estimate, from the module map in `formatter.rs:1-19`)

| class | chloro location | change | lines | support |
|---|---|---|---|---|
| F1 raw labels | `expr.rs:222-235` | strip `r#` from the label text | ~6 | exact |
| F2 labelled closure body | `closures.rs:71-74`, `:120-132` | ignore the label in `needs_block`; pass the inner block | ~10 | exact outside macros; inside macro calls the 1.8.0 output is unparsable text from span offsets, reproduced only by porting those offsets (~30 more lines) |
| F3 `use self::self` | `imports.rs:465-476` | old normalisation | ~5 | exact |
| F4 `super let` | `items.rs:135-138` | format as `let` without `super` | ~3 | exact |
| F5 `_` import order | `imports.rs:574-583` | old early-return comparator | ~15 | partial: the order is not transitive, so the output depends on the sort algorithm and its std version; a panic in 1.8.0's sort means "file unchanged" |
| F6 Unicode gap panic | `missed_spans.rs:150-153` | compute 1.8.0's `line_start`; when it is not a char boundary, fail the file | ~10 | exact for the panic; other wrong offsets that land on a char boundary are not modelled |
| P1 let chains ≤ 2021 | `rustc_compat.rs:63-71` | accept chained `let` before 2024 | ~2 | exact: the 176 fixture files 1.88 formats at `--edition 2021` equal 1.9.0's `--edition 2024 --config style_edition=2021` output byte for byte (measured), and chloro matches that configuration on 1021/1022 files (option matrix) |
| P8 negative inherent impl | `rustc_compat.rs:133-138` | accept `impl !T {}` before 1.9.0 | ~2 | exact |
| F13 `const { .. }` macro body | none | chloro already prints the 1.8.0 output (const block items Missing) | 0 | exact for 1.8.0; the 1.9.0 behaviour is the open Missing item |
| P2 `[const]` bounds | `rustc_compat.rs:128-170` | reject `TYPE_BOUND` with `[const]` | ~4 | exact |
| P3–P6 (1.85 only) | `rustc_compat.rs:128-170`, `patterns.rs:50`, `items.rs:135` | reject frontmatter and `super let`; accept const block patterns and `#[attr] ..` | ~10 + formatting of const block patterns | partial until the two accepted constructs are formatted |
| 1.7.0 style | `config.rs:373-379` | style edition 2015 always | ~3 | exact |
| F8–F11 | `items.rs:3166`, `items.rs:3317-3381`, `attr.rs:517`, `items.rs:3331` | leave type alias unformatted; drop `safe`/`unsafe` on statics; drop `unsafe(..)` around path/name-value metas; drop const generics and `where` | ~25 | exact on the probes; F11 is unstable syntax |
| P7, F14 (1.7.0) | `rustc_compat.rs:128-170`, `types.rs` (bound rewrite) | reject `use<..>`, `(..)` RTN, field defaults, `unsafe` fields; accept `for<'a> ?Trait` and drop an empty `for<>` | ~20 | exact on the probes |

- 1.9.0: exact (the current state, 1219/1220).
- 1.8.0 (1.88): the fixtures need F4 and P2 at edition 2024, and P1 at edition 2021; the
  registry adds F5 (edition ≤ 2021), F13 and P8. F1–F4, P1, P2 and P8 are exact; F5 and F6 are partial for the reasons in the table; F2 is partial inside
  macro calls. Several of these 1.8.0 outputs change program meaning or validity (F1, F2,
  F3, F4), so supporting 1.8.0 means emitting that text on purpose.
- 1.7.0: the style mapping plus F1–F6, F8–F11, F14 and P1–P8; without `version=Two`. Larger and
  less tested (fixtures exercise F8–F11 with one or two files each).

### Reference snapshots

- Layout: `chloro-core/tests/conformance/snapshots/ra-1.8.0/<crate>/<case>.rustfmt.rs`,
  written only for cases whose 1.8.0 output differs from `snapshots/ra/<crate>/<case>.rustfmt.rs`,
  with the input as content where 1.8.0 rejects the file (the existing convention). The
  expected output for 1.8.0 is the `ra-1.8.0` file when present, else the `ra` file. On the
  current fixtures `ra-1.8.0` holds 4 files (`let_stmt` and the three P2 files); the
  registry corpus has 6 more cases at edition 2024 (F13, F4, P2 ×3, P8) and is checked
  live, not snapshotted, as now.
- Probes: the fixtures exercise few classes, so each reproducer in `S/probes/` becomes a
  case under `chloro-core/tests/conformance/fixtures/versions/` with one expected file
  per release and edition (`<probe>.<version>.e<edition>.rustfmt.rs`).
- Regeneration: `helpers/rustfmt.rs::format_with_rustfmt` takes the command from a
  `CHLORO_RUSTFMT_VERSIONS` variable (e.g. `1.8.0=1.88.0`), runs
  `rustup run 1.88.0 rustfmt --edition 2024`, compares with the 1.9.0 result, writes or
  removes the `ra-1.8.0` file, and compares chloro with `rustfmt_version=1.8.0` against the
  resolved expectation. Without the variable, or without the toolchain installed, the
  version cases are skipped. `cargo test -p chloro-core --test conformance batch` keeps its
  current behaviour for 1.9.0.
- Edition 2021: the snapshot suite runs `--edition 2024` only, where P1 never appears; the
  option matrix (`examples/config_matrix.rs`) gains a `--rustfmt <cmd>` argument so that
  `rustfmt_version=1.8.0` with `edition=2021` is compared with `rustup run 1.88.0 rustfmt`.

## Open questions

- rustfmt 1.8.0 as shipped in rust 1.83 and 1.84 (not installed): the `v1.8.0` tag sets
  `overflow_delimited_expr = true` under style edition 2024; whether those builds apply it
  with `--edition 2024` is not measured.
- Whether `RustfmtVersion` names releases (1.8.0) or toolchains (1.85, 1.88): the two 1.8.0
  builds differ only in acceptance on the measured corpora.
- F5's old comparator: whether chloro's std sort reproduces the order 1.8.0's std sort
  produced for every input, and whether it panics on the same inputs, is not measured.
- 1.7.0 at edition 2024 on the registry, and 1.7.0 with `version=Two` beyond the fixtures,
  are not measured.
