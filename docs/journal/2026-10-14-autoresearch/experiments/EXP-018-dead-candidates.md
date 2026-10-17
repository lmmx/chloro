# EXP-018: dead candidates (chain `new_rw`, literal widths)

Experiment E18 of the [autoresearch programme](../../2026-10-14-autoresearch.md); procedure
and gates in [protocol.md](../protocol.md). It implements rows 5 and 7 of the ranked bounds in
the design study
[../../2026-10-15-extensions-theory/decide-once-and-exact-reuse.md](../../2026-10-15-extensions-theory/decide-once-and-exact-reuse.md)
(section 1, speculation per decision point), one gated commit per lever as in
[EXP-003](EXP-003-hot-paths.md). Row 6 (the nested visitor's doubled buffer) belongs to E16
and is not touched here. Labels: **measured**, **read**, **derived**.

## Identity

| field | value |
|---|---|
| branch | `20261015-dead-candidates`, worktree `/home/user/wt/dead-candidates` |
| base | `9929551` (was `aa44d4c`) (integration branch `claude/chloro-perf-research-swarm-nrscj8`; formatter code of the best-known-good) |
| commits | `25dd4a4` (was `70cbc93`) `perf(chains): skip the last child's vertical candidate when it cannot win`; this record |
| rejected | row 7, literal widths once per literal (reverted, not committed; diff kept as `scratchpad/e18/lit-v1.diff`) |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, valgrind 3.22.0; workspace release profile (fat LTO, one codegen unit), `cargo build --release -p chloro-core --examples` |
| base binary | `bench` built at `9929551` (was `aa44d4c`) in this worktree before any change, copied to `target/release/examples/bencB` (same path length as the head's `bench`, see EXP-003 on path-length sensitivity), sha256 `30a72a7c48262518` |
| `Ir` command | `benchmarks/profile.sh --core 3 --runs 1 --sample /tmp/chloro-bench/deadc018 [--bench target/release/examples/bencB]` (private sample path of the default's length) |
| host | 4 vCPU, shared with other experiments (load average 3.7–6.5 during the work); `Ir` is deterministic, wall times are not quiet-host figures |

The base measures 312,999,355 `Ir` under this command, against the ledger's 312,734,032 for the
same code: the 265 K difference is the binary and output paths; every figure below compares
binaries measured the same way.

## Hypothesis and falsification

**Row 5.** In `format_last_child` (chains.rs; rustfmt 1.9.0 chains.rs:712) the vertical
candidate `new_rw` is not rendered when the one-line candidate `rw` fits `one_line_budget` and
has at most one line, since rustfmt then keeps `rw` whatever `new_rw` is. Bound ≤ 3.03% (the
`new_rw` site, 9.48 M), estimate 0.75–2.79%.

**Row 7.** The width facts of a literal are computed once per literal instead of
`filtered_str_fits` over its text on every attempt. Bound 0.28% exact (0.88 M of the 4.67 M
under `rewrite_literal`).

Each lever is falsified on its own if `Ir` does not fall by more than 0.1% of the start
(0.31 M), or if any output changes.

## Row 5: chain dead candidate (kept, `25dd4a4` (was `70cbc93`))

### Exactness (read)

rustfmt's choice after rendering `new_rw` (chloro's port is line for line):

| `new_rw` | condition | kept |
|---|---|---|
| `Some` | `rw`'s first line misses the budget | `new_rw` |
| `Some` | `new_rw.lines().count() >= rw.lines().count()` | `rw` |
| `Some` | fewer lines | `new_rw` |
| `None` | — | `rw` |

`fits_single_line` becomes `could_fit_single_line && all_in_one_line` whenever `rw` is kept.
With `could_fit_single_line` and `line_count <= 1`, the first row cannot apply and the third
needs an empty `new_rw`. The children of a chain are method calls, fields, tuple fields,
`.await` (rendered from a leading `.` or ` .` plus `?`s) and comments (non-empty text), so a
child's rewrite is never empty. Every outcome then keeps `rw` with
`fits_single_line = all_in_one_line`. The `child_shape.sub_width(..)?` that precedes the render
is still evaluated, so a call that returned `None` there still does.

Effects of the skipped render on the run (read): `macro_rewrite_failure` (macros.rs:153, 195),
`lost_comment` (`recover_comment_removed`), `skipped_range` (`add_skipped_span`) and, under
`rustfmt_version = "1.8.0"`, `format_failure` (missed_spans.rs:190-200). The first three are
read only by `format_snippet` (formatting.rs:276-279); `format_failure` returns a 1.8.0 file
unchanged. The context cells a render touches (`force_one_line_chain`, `inside_macro`,
`is_if_else_block`, `is_loop_block`) are restored before it returns. The memo, the block and
macro-argument caches and fragment tree ids are identity caches and change no text.

Guard: `RewriteContext::may_skip_dead_candidates()` = `RunState::whole_file` (set by
`format_source_with_config` through `format_text(.., whole_file: true)` and by both run states
of `parallel.rs`; false in `format_snippet` and in any other `RunState`) and
`rustfmt_version == 1.9.0`.

### Firing count (measured)

A temporary `eprintln!` behind an environment variable (removed before the commit), on the
49-file sample:

| event | count |
|---|---|
| skip taken | 2,024 |
| `new_rw` rendered, dead but the guard is off (macro definition bodies) | 4 |
| `new_rw` rendered, `rw` misses the budget | 161 (78 of them with a one-line `rw`) |
| `new_rw` rendered, `rw` fits with 2–4 lines | 119 |
| all `new_rw` decisions | 2,308 |

The study's 1,276 + 1,183 call-site executions counted the outer and `'2` levels of
callgrind's recursion; this count covers every call.

### `Ir` (measured)

| binary | `Ir` | Δ |
|---|---|---|
| base `9929551` (was `aa44d4c`) (`bencB`) | 312,999,355 | |
| `25dd4a4` (was `70cbc93`) | 302,370,674 | −10,628,681 (−3.40%) |

The fall exceeds the study's 3.03% bound, which priced only the outer call site's `new_rw`
(9.48 M): dead `new_rw` renders nested inside outer `rw` renders (the `'2` level) were outside
that site.

### Tests

`chloro-core/tests/dead_candidates.rs`: a chain whose last child fits on one line, one wider
than `chain_width`, one whose `rw` misses the budget, one whose `rw` fits with three lines, and
chains in a macro definition body (the guard off), each at `rustfmt_version` 1.9.0 and 1.8.0,
with the expected outputs printed by `rustfmt 1.9.0` and `rustup run 1.88.0 rustfmt`.

## Row 7: literal widths once per literal (rejected)

Built on `25dd4a4` (was `70cbc93`): `StrWidths` (the first-line width, single-line flag, widest later line and
last-line width of `filter_normal_code(text)`, with `fits` equal to `filtered_str_fits` for every
`max_width` and shape, unit-tested over 11 literal texts × 48 × 48 × 3 shapes) and
`LiteralFacts` (plus the continuation test of `rewrite_string_lit`), cached per run by the text's
address and length for multi-line literals; one-line literals kept the old path. All its tests
passed (a multi-line string, a raw string, a byte string with `\r` escapes and a continued
string at `max_width` 100, 60 and 40, against rustfmt 1.9.0 and 1.8.0).

| binary | `Ir` | Δ against `25dd4a4` (was `70cbc93`) |
|---|---|---|
| row 7 on `25dd4a4` (was `70cbc93`) | 307,663,496 | +5,292,822 (+1.75%) |

`rewrite_literal`'s inclusive cost rose from 5.19 M to 8.30 M. A counter run found 1,448 cache
misses (235,691 bytes of literal text) and 58 hits (4,893 bytes): after row 5, a multi-line
literal is almost never rewritten twice, because the repeats the study counted
((1,964 − 1,595) literal calls) were mostly inside the dead `new_rw` renders that row 5 removes.
Each miss pays a full scan of every line for the widest later line, where `filtered_str_fits`
stops at the first failing test, plus the map insert. With 58 hits the lever's ceiling on this
head is far below 0.1%; a cheaper exact scan of long literals would be a different lever
(per-call cost, not repeats). Reverted; the rebuilt head binary is byte-identical to
`25dd4a4` (was `70cbc93`)'s (sha256 `78979a831bdb920a`).

## Gates (`25dd4a4` (was `70cbc93`))

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test -p chloro-core`; `cargo test --workspace --all-features` | all pass |
| `bench -n 1 --check fixtures-e8d87c2.tsv` | 0 of 1238 changed; with `--threads 4` 0 of 1238 changed and 0 differ from `format_source` |
| `bench -n 1 --root ~/.cargo/registry/src --check registry-e8d87c2.tsv` | 0 of 25,024 changed (20,850 recorded, 4,174 not recorded) |
| `conform -i` | 1220/1220, 0 not idempotent |
| `conform -i -V 1.8.0` | 1220/1220, 0 not idempotent |
| `config_matrix -n 400 max_width=80` | 343/343 |
| `config_matrix -n 400 chain_width=40` | 343/343 |
| `benchmarks/profile.sh --core 3 --runs 1` | 312,999,355 → 302,370,674 (−3.40%) |

## `bench -n 5`, in process (measured, loaded host)

Three interleaved pairs, `taskset -c 3`, 1238 fixtures, best of 5, load average 4.1–6.5:

| pair | base `bencB` | head `bench` |
|---|---|---|
| 1 | 4.46 MB/s | 4.54 MB/s |
| 2 | 4.49 MB/s | 4.49 MB/s |
| 3 | 4.44 MB/s | 4.59 MB/s |

Ranges 4.44–4.49 against 4.49–4.59 touch at 4.49: not disjoint on this loaded host (absolute
figures are half the quiet-host M3 values). The claim rests on `Ir`; a quiet-host triple is
left to the integrator.

## Decision

- Row 5 kept: −3.40% `Ir`, exact by argument with the run-level guard, every output unchanged
  on both corpora, both versions and the two width options. It is kept on its branch for the
  integrator.
- Row 7 rejected: +1.75% `Ir` on top of row 5; 58 repeats remain to be saved.
- Follow-up: the study's `choose_rhs` and match-body points (§ 1) are the next candidates for a
  dead-candidate test; any of them needs the same run-level guard.
