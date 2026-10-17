# EXP-022: the Ir-blind budget: branch and cache co-metric, hot/cold splits, kind dispatch, BOLT

Ledger row E22 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md); the brief is tricks T1 and T10 of
[../../2026-10-15-unmined-tricks.md](../../2026-10-15-unmined-tricks.md) §5, with the cache and
branch simulation of [../../2026-10-15-extensions-theory.md](../../2026-10-15-extensions-theory.md)
("The final head, measured") as its starting point. Labels: **measured** (a tool's output on the
builds below), **read** (from source or documentation), **derived** (arithmetic on measured
figures).

## Identity

| field | value |
|---|---|
| branch | `20261015-layout` from `8ebd3bb` (integration branch `claude/chloro-perf-research-swarm-nrscj8`), worktree `/home/user/wt/layout` |
| base BKG | `e640fbf`; `8ebd3bb` adds only documentation after it |
| kept commits | `cbbf081` (`profile.sh --sim`, the co-metric), `4365e53` (`steps_fused` without the per-step exit state); then this file and the ledger row |
| rejected, reverted | hot/cold splits of `write_list`; of `overflow::Context` and `ChainItem::rewrite`; a kind flag table in front of `Expr`/`Pat`/`Type::cast` (details under *Rejected*) |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, LLVM 22.1.6, linker LLD 22.1.6 (rustc's default on this target; `readelf -p .comment`); workspace profile fat LTO, one codegen unit |
| tools | valgrind 3.22.0 (callgrind's simulator: I1 and D1 32 KB 8-way 64 B lines, LL 34 MB 17-way, as detected); `llvm-bolt` 20.1.2 from Ubuntu's `bolt-20` package; hyperfine 2.0.0 |
| host | 4 vCPU Xeon @ 2.80GHz, Linux 6.18.44. Other experiments (E19–E21) built and ran throughout, load average 2–13; their callgrind runs are pinned to core 3, which is time-shared with them for most of the window. Wall figures are **measured, provisional** (protocol §6) and carry their load |
| `Ir` command | `benchmarks/profile.sh --core 3 --sample /tmp/chloro-bench/layout22 [--sim]`: the script's sample at a private path of the default's length (`sample49` is shared and rebuilt by each run) |
| `Ir` of the base | 248,197,787 for `8ebd3bb` built in this worktree (`target/release/examples/bench`), against the BKG's 248,202,094–248,203,822 at `/home/user/chloro`: the count moves with the length of the binary's path (E3, *Identity*); every comparison below is between binaries at paths of equal length |
| golden hashes | `scratchpad/golden/fixtures-e8d87c2.tsv` (1238 fixtures) and `registry-e8d87c2.tsv` (20,850 recorded registry files; 25,024 present, the others reported as not recorded) |

## Hypothesis and falsification

From §1 of the unmined-tricks entry (derived there, not measured): at IPC ≈ 1.6, the
simulator's 4.8 M mispredicted branches and 5.2 M L1 instruction-cache misses per 312.7 M
instructions are of the order of a third of the cycles, which the instruction count does not
see; code layout (hot/cold splitting, post-link ordering) and dispatch shape (flag tables
instead of compare chains and jump tables) move them.

Falsified for a lever if `Bcm + Bim` and `I1mr` on the sample do not fall, or fall without a
wall gain in three interleaved `bench` pairs. The brief's keep rule: a lever is kept only if
it moves `I1mr` or `Bcm` materially and does not raise `Ir`.

## Commands

```sh
# co-metric (two runs, pinned, env -i, setarch -R, as profile.sh always runs callgrind)
benchmarks/profile.sh --core 3 --runs 2 --sim --sample /tmp/chloro-bench/layout22 --out DIR [--bench BIN]
# per-instruction profile for the layout analyses below
taskset -c 3 env -i setarch x86_64 -R valgrind --tool=callgrind --cache-sim=yes --branch-sim=yes \
  --dump-instr=yes --callgrind-out-file=cg.out BIN -n 1 --root /tmp/chloro-bench/layout22
# the same object linked with its functions in a random order (lld), for the layout spread
cargo rustc --release -p chloro-core --example bench -- -C save-temps   # keeps the LTO object
cc -m64 <object> ... -fuse-ld=lld ... -Wl,--shuffle-sections='.text*=SEED'   # cargo's link line otherwise
# wall: order alternating per pair (base, head), (head, base), ...
taskset -c 3 BIN -n 5 --root chloro-core/tests/conformance/fixtures
```

The relink reproduces cargo's binary byte for byte without `--shuffle-sections` (checked for the
base and every candidate), so a shuffled binary differs from the shipped one only in the order
of its functions.

## The co-metric (T10, kept: `cbbf081`)

`benchmarks/profile.sh --sim` runs the same pinned `env -i`, `setarch -R` callgrind invocation
with `--cache-sim=yes --branch-sim=yes` and prints, per run, `Ir`, `Bc`, `Bcm`, `Bi`, `Bim`,
`I1mr`, `D1mr`, `D1mw` and the last-level misses, then the ten functions with the most `I1mr`
and the most `Bcm` (exclusive, `callgrind_annotate --sort`). The `Ir` parse takes the first
collected event, so `Ir` is the same with and without the option. Documented in
`benchmarks/README.md` (section `profile.sh`).

What the simulator models (read, Cachegrind manual, "Branch simulation specifics"):
conditional branches from 16,384 two-bit counters indexed by the branch address and recent
global history; indirect branches from a 512-entry table of last targets indexed by address
alone. Current x86 cores predict indirect branches from history as well, so `Bim` overstates
their cost on this host's hardware; `Bcm` and `I1mr` are closer models.

### Baseline (measured, `8ebd3bb`)

| run | binary path | `Ir` | `Bc` | `Bcm` | `Bi` | `Bim` | `I1mr` | `D1mr` | `D1mw` |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `/home/user/wt/layout/bench-base` | 248,196,061 | 41,122,953 | 3,229,833 | 1,923,315 | 713,287 | 4,863,020 | 676,015 | 527,483 |
| 2 | same | 248,195,902 | 41,122,927 | 3,229,831 | 1,923,314 | 713,287 | 4,863,020 | 676,015 | 527,483 |
| 3 | same | 248,196,073 | 41,122,953 | 3,229,835 | 1,923,314 | 713,287 | 4,863,020 | 676,015 | 527,483 |
| — | same, without `--sim` | 248,195,854 | | | | | | | |
| 1 | `target/release/examples/bench` (the path candidates are measured at) | 248,197,787 | 41,123,220 | 3,229,865 | 1,923,319 | 713,292 | 4,863,038 | 675,476 | 525,934 |

Rates: 7.85% of conditional branches and 37.09% of indirect branches mispredicted; `I1mr`
1.96% of `Ir`; last-level misses 11.6 K instruction, 12.0 K data read, 37.2 K data write
(negligible). Stability: three runs of one binary gave identical `I1mr`, `Bim`, `D1mr` and
`D1mw`, `Bcm` within 4, `Ir` within 171 (219 with the run without the simulator). One run takes 14 s against 5.6 s without the
simulator. The counts are as deterministic as `Ir`; what they are not is stable under a change
of code layout (next section).

Top ten (exclusive, base at the standard path):

| by `I1mr` | | by `Bcm` | | by `Bim` | |
|---|---|---|---|---|---|
| `write_list` | 252,951 | `memcpy` (libc) | 214,327 | `ra_ap_parser::event::process` | 140,655 |
| `overflow::Context::rewrite` | 187,488 | `TreeBuilder::steps_fused` | 142,047 | `TreeBuilder::steps_fused` | 135,115 |
| `overflow::Context::rewrite` (recursive) | 154,388 | `rustc_compat::subtree_rejected_by_rustc` | 122,502 | `LexedStr::new` | 111,396 |
| `_int_malloc` | 132,186 | `_int_malloc` | 122,186 | lexer `advance_token` | 91,723 |
| `rewrite_chain_from` | 124,481 | `write_list` | 104,421 | `Expr::cast` | 31,946 |
| `rewrite_fn_base` | 90,175 | `FnSig::from_fn` | 86,190 | `format_expr_uncached` (recursive) | 15,153 |
| `memoize::<format_expr>` (recursive) | 86,235 | `malloc` | 75,073 | `LexedStr::to_input` | 13,564 |
| `get_comment_end` | 85,755 | `LexedStr::to_input` | 70,180 | `CharClasses::next` | 11,220 |
| `Chain::rewrite` | 82,734 | `_int_free` | 67,114 | `join_generic_copy` | 7,901 |
| `rewrite_chain_from` (recursive) | 81,585 | `ra_ap_parser::event::process` | 59,756 | `Pat::cast` | 7,679 |

The parser and lexer (a dependency) hold 344 K of the 713 K `Bim`; the tree build
(`steps_fused`) 135 K; chloro's typed layer (`Expr`/`Pat`/`Type::cast`) 46 K.

## What the baseline says about the levers (measured, derived)

**The instruction-cache misses are capacity misses of a large hot footprint, not cold code
inside hot lines.** From the per-instruction profile (`--dump-instr=yes`, every executed
64-byte line of the binary and libc):

| share of `Ir` | in the hottest | those lines hold, of `I1mr` |
|---|---|---|
| 50% | 150 lines (9 KB) | 6.5% |
| 80% | 778 lines (49 KB) | 42.5% |
| 90% | 1,539 lines (96 KB) | 65.4% |
| 95% | 2,472 lines (154 KB) | 80.7% |
| 99% | 4,591 lines (287 KB) | 95.5% |

11,673 lines (730 KB) are touched at all; the L1I holds 32 KB. Within the six functions the
brief names, lines are classified by how often their most-executed instruction runs relative to
the function's entry count:

| function (size) | lines run on ≥ 50% of calls: count, executed-byte density, `I1mr` | lines run on < 1% of calls: count, `I1mr` | never-run lines |
|---|---|---|---|
| `write_list` (10,144 B) | 48, 89%, 229,611 of 252,951 | 47, 548 | 39 |
| `overflow::Context::rewrite` (11,051 B) | 64, 89%, 158,947 of 187,489 | 22, 246 | 28 |
| `rewrite_chain_from` (9,658 B) | 86, 84%, 118,609 of 124,482 | 4, 8 | 41 |
| `Chain::rewrite` (7,578 B) | 63, 89%, 75,536 of 82,732 | 0, 0 | 27 |
| `rewrite_fn_base` (13,868 B) | 93, 84%, 76,318 of 90,175 | 3, 3 | 64 |
| `memoize::<format_expr>` (2,574 B) | 28, 92%, 85,048 of 86,235 | 1, 5 | 6 |

The misses sit in lines that run on most calls and are already 84–92% dense; the rarely-run
lines carry under 1% of each function's misses. `write_list` misses 23 lines per call out of
the 48–66 it runs: its own hot lines are evicted between calls by the rest of the 150 KB working
set. Splitting cold arms out can at best make the hot lines denser (the 8–16% of their bytes
that do not run), a bound of about 10% of these functions' misses (1.12 M with their recursive
instances), about 2% of the total, before any effect on the rest of the layout.

**The layout spread is larger than that bound.** The same object linked with its functions in
four random orders (`lld --shuffle-sections`, seeds 1–4) gives the base `I1mr` 4,921,258 to
5,035,112 (mean 4,995,118, s.d. 51 K, about 1%), `Bcm` 3,247,597 to 3,287,087 (s.d. 17 K) and
`Bim` 687,305 to 699,741 (s.d. 5 K): which functions share cache sets and predictor entries
moves `I1mr` by ±1% and `Bcm` by ±0.5% with no code change. Cargo's own order is better than
random for the base (`I1mr` 4,863,031, 2.6% under the random mean), and any code change
re-draws it: a candidate measured in cargo's order alone compares one draw with another. Every
candidate below is therefore also measured in the same four random orders (independent draws:
seed `k` gives unrelated orders for different objects), and a change is material only if it
exceeds that spread.

**Kind tests are a small part of the mispredicts.** Every load or compare of `Elem.kind` (the
16-bit field at offset 12 of a 16-byte element) in the binary, with the conditional jumps in
the 16 instructions after it: 102 sites, `Bc` 430 K (1.4% of all), `Bcm` 46.4 K (1.8% of the
binary's 2.62 M). The largest, `outer_attributes`/`has_leading_attrs` (17 K), is a loop's
node-or-token and whitespace-or-comment control flow, which a table does not remove. Chains of
three or more compares on one register run 208 K times in all; the only hot one is in the
parser's `LexedStr::to_input` (98 K runs, 47 K `Bcm`, a dependency). chloro's own `SyntaxKind`
predicates compile to one or two compares or a bit test; `matches!` over many kinds becomes a
range check and bit test, not a chain. What remains is the typed layer's `cast`: `Expr::cast`
is a 36-way `match` that LLVM compiles to a jump table whose arms each store the variant's tag
(the switch-to-lookup-table transform does not fire on the `sret` stores); all 44,553 calls take
the indirect jump (31,946 mispredicted), 10,966 of them to return `None`. With `Pat::cast` and
`Type::cast`: 46 K `Bim`. Bound of the lever on the simulator: about 2.3% of `Bcm + Bim` (the
46 K `Bcm` at kind tests and the 46 K `Bim` of the casts), less the control flow a table cannot
remove.

**The tree build's mispredicts are its exit state.** In `steps_fused` the indirect jump over
the step kind (234 K executed, 129 K mispredicted) and four conditional branches mispredicted
at 21–49% (32.6 K, 31.8 K, 24.8 K, 21.8 K) were the `PendingExit` tests of the ported
`intersperse_trivia` state machine at each token, `Enter` and `Exit` (read from the
disassembly).

## Levers (measured)

All figures on the 49-file sample. "Cargo order" is the object linked as cargo links it (the
relink without `--shuffle-sections`, byte-identical to cargo's binary), "random orders" the mean
of the same object linked in four shuffled orders; all at relink paths of equal length (the
base's file name is one character longer, worth about 100 `Ir`). Differences are against the
base, kf1's against st1, on which it was built.

| candidate | `Ir` cargo order / random mean | `Bcm` cargo / random | `Bim` cargo / random | `I1mr` cargo / random | decision |
|---|---|---|---|---|---|
| base `8ebd3bb` | 248,196,781 / 248,202,186 | 3,229,913 / 3,265,552 | 713,287 / 694,345 | 4,863,031 / 4,995,118 | — |
| wl1: `write_list`'s pre-comment, horizontal and vertical post-comment arms in three `#[cold] #[inline(never)]` helpers | 247,873,146 / 247,879,436 (−0.13%) | 3,230,317 / 3,246,370 (−19 K) | 707,339 / 701,092 (+7 K) | 4,888,011 / 4,963,926 (−31 K) | rejected |
| oc1: `overflow::Context`'s special-macro/mixed vertical tactic and control-flow last argument, `ChainItem::rewrite`'s parenthesised parent, as cold helpers | 248,268,061 / 248,266,828 (+0.03%) | 3,227,440 / 3,265,325 (0) | 687,257 / 705,434 (+11 K) | 4,938,437 / 5,028,695 (+34 K) | rejected |
| st1 `4365e53`: `steps_fused` finishes a node at its `Exit`, defers only the root's | 247,936,645 / 247,955,053 (−0.10%) | 3,132,768 / 3,198,762 (**−67 K, −2.0%**) | 725,663 / 697,610 (+3 K) | 5,003,944 / 4,994,183 (−1 K) | **kept** |
| kf1 on st1: `kind_flags.rs` table answers non-members before `Expr`/`Pat`/`Type::cast`'s jump table | 247,997,967 / 248,021,858 (+0.027%) | 3,145,900 / 3,199,776 (+1 K) | 705,176 / 676,576 (**−21 K, −3.0%**) | 5,018,891 / 4,976,652 (−18 K) | rejected (`Ir` up) |

At `target/release/examples/bench` (`profile.sh`, the gate's path): base 248,197,787, wl1
247,870,419, st1 247,936,009 (247,935,964 and 247,936,083 for the committed head).

Spread of the random-order means (s.d. of four orders): `Bcm` 11–22 K, `Bim` 1–7 K, `I1mr`
16–51 K. Against the base's mean: wl1's `I1mr` −31 K is one standard error of the difference
(32 K) and its `Bcm` −19 K under two (11 K); oc1 is flat or worse on every count; st1's `Bcm`
−67 K is 4.8 standard errors (14 K) and lower in each of the four orders (−104 K, −55 K, −38 K,
−70 K against the base's orders); kf1's `Bim` −21 K is lower in every order by 17–24 K, at
+67 K `Ir` (one more test in each of 67 K `cast` calls).

### Hot/cold splitting (rejected)

- **wl1** moved `write_list`'s own misses as the analysis predicts (252,951 → 219,475, −13%;
  the helpers 462 together) and lowered `Ir` by 0.13%, but
  the program's `I1mr` in cargo's order rose 0.5% (+58 K in functions it did not touch) and its
  random-order mean fell 31 K, inside the ±51 K spread. `Bcm` likewise. Its own `Bcm` rose
  (104 K → 117 K). The wall pairs (four, order alternating, core 3, load 4.5–5.3) gave per-pair
  ratios 1.020, 0.994, 0.922, 1.010 with overlapping ranges; an earlier run of three pairs with
  the base always first (load 4.4–5.1) had the head 3.7–5.2% slower in each, which the
  alternating order did not reproduce: every pair series below alternates the order. Not material on the co-metric; reverted.
  The `Ir` gain (−0.13%) is real and recorded for whoever owns `write_list` next (E20 rewrites
  its body).
- **oc1** raised `Ir` (+65 K) and left `Bcm` and `I1mr` where they were. The cold arms of
  `overflow::Context` that are rare by construction (special-macro callees, short simple
  arrays, a control-flow last argument) were not in its hot lines; the analysis above says
  why. Reverted.
- `rewrite_fn_base`'s rare arms are the pre-2024 style-edition branches (a `push` each) and
  the comment between return type and body (a few pushes); `memoize`'s two paths (hit and
  insert) both run on most calls; `_int_malloc` is libc. No candidate was large enough to
  measure against the spread, so none was built.

### `steps_fused` (kept, `4365e53`)

The ported state machine deferred every `Exit` to the next step. Nothing is written between an
`Exit` and the next step's handling of the deferred exit, and trivia is only written by a token,
an `Enter`, a split float or the end, each after the deferred exit is taken; so finishing a node
at its own `Exit` (and the unbalanced node a split float leaves open, at the split) writes the
same elements. Only the root's `Exit` waits for the trailing trivia; the root is the node opened
first by this call, since the builder may hold nodes of its own around it (`parse_maybe_bound`
opens three; the first draft compared the depth with 1 and panicked on the fixtures, which the
golden check caught at once). A step after the root's `Exit` finishes it first, as the original
does (`#[cold]` helper); the parser never produces one. Debug builds still run
`steps_interspersed` on every parse and assert equal elements: the 196 library tests and the
conformance tests passed under that assertion.

`steps_fused`'s own counts: `Bcm` 142,047 → 97,243, `Bc` 3,154,012 → 3,024,424, `Ir`
16,892,828 → 16,631,955; its indirect jump over the step kind stays (`Bim` 135,115 → 134,320).
Program: `Ir` −0.105%, `Bcm` −3.0% in cargo's order and −2.0% over random orders, `I1mr`
unchanged over random orders. In cargo's order the head's `I1mr` is 2.9% above the base's
(5,003,912 against 4,863,038): the base's cargo order is a lucky draw (2.6% under its random
mean) and the head's is an average one (5,003,944 against its random mean 4,994,183).

## BOLT trial (measured)

**Installation.** `apt-get update && apt-get install -y bolt-20` (Ubuntu noble's
`bolt-20 1:20.1.2-0ubuntu1~24.04.3` with `libbolt-20-dev`; `bolt-15` to `bolt-20` are all in
the archive), under 5 minutes in all, through the configured proxy; binaries
`/usr/bin/llvm-bolt-20`, `merge-fdata-20`, `perf2bolt-20`, the instrumentation runtime
`/usr/lib/llvm-20/lib/libbolt_rt_instr.a`. BOLT 20 rewrites LLVM 22 output without complaint.

**Build.** The base code (`8ebd3bb`, exported to a scratch directory so the worktree could keep
changing) with `RUSTFLAGS="-C link-args=-Wl,--emit-relocs" benchmarks/pgo.sh <root> cli`
(13 raw profiles merged, 4.1 MB, as `pgo.sh` without `--target`): `chloro` and
`examples/bench`, profile-guided, with `.rela.text` kept. `-Wl,-q` was not needed.

```sh
llvm-bolt-20 chloro.pgo -instrument -o chloro.inst \
  -instrumentation-file=$PWD/chloro.fdata -instrumentation-file-append-pid     # 5.8 s (bench 4.9 s)
./chloro.inst --check --no-cache chloro-core/tests/conformance/fixtures       # 3.5 s, exit 1 (unformatted)
./bench.inst -n 1 --root chloro-core/tests/conformance/fixtures               # 4.3 s
merge-fdata-20 chloro.fdata.*.fdata > chloro.fdata                             # one file each, 10 MB
llvm-bolt-20 chloro.pgo -o chloro.bolt -data=chloro.fdata -reorder-blocks=ext-tsp \
  -reorder-functions=hfsort+ -split-functions -split-all-cold -icf=1 -use-gnu-stack   # 3.3 s
```

BOLT's report for `chloro`: 621 of 1,506 functions profiled, 40 of them not optimised, top-5%
CFG discontinuity 0.00%, blocks reordered in 474 functions, splitting "947,925 hot bytes from
411,002 cold bytes", ICF folded 7 functions. BOLT 20 reports `hfsort+` deprecated in favour of
`cdsort`, and a numeric `-icf` value deprecated; with `-icf` (no value) it folds 56 functions
instead of 7 (seen in the `-use-old-text` builds below, not timed). The training is the PGO training (the CLI on the
fixtures, `bench -n 1`), so the BOLT profile and the PGO profile come from the same workload.

**Identity.** `bench.bolt --check`: 0 of 1238 fixtures changed, sequential and `--threads 4`;
0 of 25,024 registry files changed. `chloro.bolt` against `chloro.pgo` on a copy of the
fixtures: print mode `--no-cache --threads 1` identical stdout (14,190,145 bytes, same sha256),
`--check --no-cache -v` sorted reports identical, exit status 1 for both; on the three formatted
crates below `--check --no-cache` exits 0 for both.

**Co-metric** (`profile.sh --sim`, `bench.pgo` against `bench.bolt`, same directory and name
length):

| binary | `Ir` | `Bcm` | `Bim` | `I1mr` | `D1mr` |
|---|---|---|---|---|---|
| PGO | 205,911,294 | 3,355,870 | 738,173 | 3,886,806 | 732,662 |
| PGO + BOLT | 205,747,918 (−0.08%) | 3,240,856 (−3.4%) | 741,951 (+0.5%) | 3,533,361 (−9.1%) | 738,053 |

**Size.**

| binary | file | stripped | `text` (size) |
|---|---|---|---|
| `chloro` PGO (with relocations) | 4,287,768 | 2,668,640 | 2,637,346 |
| `chloro` PGO + BOLT | 6,946,976 | 6,590,048 | 4,624,982 |
| `bench` PGO | 3,911,448 | 2,427,824 | 2,403,337 |
| `bench` PGO + BOLT | 6,848,736 | 6,561,608 | 4,268,772 |

BOLT keeps the original code (`.bolt.org.text`) and places the new hot text at a 2 MB boundary
("padding code to 0x600000 to accommodate hot text"), so the stripped CLI is 2.47 times as
large. `-use-old-text` did not help: "original .text too small to fit the new code using
0x200000 alignment" (with `-align-text=64` too).

**Wall.** Pinned hyperfine (`-N -w 2 -r 15`, two rounds, order reversed in the
second), `chloro --check --no-cache <tree>` on the formatted copies under
`/tmp/chloro-bench/formatted/`, merged rounds (n = 30 per binary), min / mean ms:

| window | crate | PGO | PGO + BOLT | BOLT/PGO mean | min |
|---|---|---|---|---|---|
| 09:43–09:44 UTC, core 3, load 2.0–2.7 | `fx-hir-ty` (122 files) | 288.1 / 360.6 | 287.0 / 351.6 | 0.975 | 0.996 |
| | `reg-regex-syntax` (33) | 214.8 / 265.4 | 215.2 / 253.4 | 0.955 | 1.002 |
| | `fx-parser` (28) | 32.5 / 39.4 | 31.8 / 37.3 | 0.945 | 0.978 |
| 10:44–10:46, core 3 time-shared with other agents' callgrind runs, load 8.4–9.2 | `fx-hir-ty` | 585.8 / 719.4 | 603.7 / 720.8 | 1.002 | 1.031 |
| | `reg-regex-syntax` | 408.3 / 491.0 | 351.7 / 449.9 | 0.916 | 0.861 |
| | `fx-parser` | 55.5 / 78.8 | 62.0 / 77.8 | 0.988 | 1.117 |
| 10:48–10:49, core 2, load 4.8–5.1 | `fx-hir-ty` | 290.4 / 342.1 | 296.8 / 343.9 | 1.005 | 1.022 |
| | `reg-regex-syntax` | 217.7 / 260.4 | 220.8 / 249.2 | 0.957 | 1.014 |
| | `fx-parser` | 33.3 / 36.6 | 32.3 / 39.9 | 1.090 | 0.969 |
| 10:53–10:54, core 0 (idle before the block), load 4.5–5.1 | `fx-hir-ty` | 285.3 / 332.7 | 291.2 / 333.1 | 1.001 | 1.021 |
| | `reg-regex-syntax` | 214.9 / 259.7 | 218.7 / 236.9 | 0.912 | 1.018 |
| | `fx-parser` | 33.4 / 36.8 | 32.0 / 35.6 | 0.966 | 0.961 |

Every min–max range overlaps; σ is 5–26% of the mean (36 cells of the first three windows).
`bench -n 5`, four pairs with the order alternating (MB/s, PGO then PGO + BOLT): core 3, load
9.2–10.6: 5.57/6.90, 5.29/6.59, 4.98/4.46, 4.77/4.65 (ratios 1.239, 1.246, 0.896, 0.975); core
2, load 5.1–5.8: 10.83/10.52, 10.25/10.92, 9.43/9.29, 10.34/10.47 (0.971, 1.065, 0.985, 1.013);
core 0, load 4.7–5.1: 10.12/8.50, 9.18/7.55, 9.57/8.11, 9.83/10.35 (0.840, 0.822, 0.847, 1.053,
BOLT slower in three of four); ranges overlap. In the quieter windows the CLI minima of the two
binaries are within 4% of each other in both directions: no wall gain beyond that is visible,
and none smaller can be resolved on this host today. The simulator's own cycle estimate (below)
puts BOLT at −1.6% against PGO.

## Wall clock (measured, provisional)

`bench -n 5` on the 1238 fixtures (13.9 MB), base (`8ebd3bb`, copied before the first
change) and head (`4365e53`), four pairs per window with the order alternating, MB/s:

| window | base | head | head/base per pair | ranges |
|---|---|---|---|---|
| 10:13–10:16 UTC, core 3, load 6.4–12.5 | 2.53, 3.76, 3.92, 4.26 | 2.62, 2.70, 4.34, 4.28 | 1.036, 0.718, 1.107, 1.005 | overlap |
| 10:40–10:42, core 3, load 4.7–10.6 | 3.10, 5.30, 5.20, 4.76 | 5.05, 5.93, 5.66, 6.27 | 1.629, 1.119, 1.088, 1.317 | overlap |
| 10:46–10:47, core 2, load 5.8–8.4 | 8.26, 6.79, 7.02, 8.63 | 7.96, 8.12, 8.35, 9.16 | 0.964, 1.196, 1.189, 1.061 | overlap |
| 10:51–10:52, core 0 (idle before the block), load 4.7–4.8 | 8.17, 7.36, 8.97, 9.00 | 8.59, 8.81, 9.55, 8.87 | 1.051, 1.197, 1.065, 0.986 | overlap |

Core 3 ran at a third to two thirds of its usual 9–11 MB/s, time-shared with other agents'
pinned profiling for most of the window (its `/proc/stat` busy share was 100% in the samples
taken at 10:40 and 10:49); cores 2 and 0 were less loaded but not idle. The head is ahead in 13
of 16 pairs, by ratios far larger than its `Ir` (−0.1%) and the simulator's cycle estimate (−0.26%,
next section) can explain: the windows resolve nothing below about 10%. No wall claim is made; a
quiet-host measurement (M5) is owed, as for every wave-3 result.

### A cycle estimate from the simulator (derived)

`CEst = Ir + 10 (Bcm + Bim) + 10 (I1mr + D1mr + D1mw) + 100 (ILmr + DLmr + DLmw)`, the weights of
KCachegrind's default cycle estimate (an illustration of scale, not a model of this CPU):

| build | `CEst` | `Ir` share | mispredicts | L1 misses | last-level |
|---|---|---|---|---|---|
| base, cargo order | 354,358,937 | 70.0% | 11.1% | 17.1% | 1.7% |
| head, cargo order | 354,631,969 (+0.08%) | 69.9% | 10.9% | 17.5% | 1.7% |
| base, random-order means | 355,861,096 | | | | |
| head, random-order means | 354,952,193 (−0.26%) | | | | |
| PGO (base code) | 305,475,394 | 67.4% | 13.4% | 17.0% | 2.2% |
| PGO + BOLT | 300,607,118 (−1.6%) | 68.4% | 13.2% | 16.1% | 2.2% |

Mispredicts and L1 misses are 28% of the estimate at these weights, in line with §1's "of the
order of a third"; the kept change moves the estimate by −0.26% (random-order means), BOLT on
top of PGO by −1.6%.

## Gates (measured)

| gate | corpus | result |
|---|---|---|
| `cargo fmt --all --check` | | clean at `4365e53` |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | clean at `4365e53` |
| `cargo test --workspace --all-features` | | all pass at `4365e53` (lib 196, every integration and doc test; debug builds compare `steps_fused` with `steps_interspersed` on every parse) |
| `bench --check`, sequential and `--threads 4` | 1238 fixtures, `fixtures-e8d87c2.tsv` | 0 changed; 0 differ between 4 threads and `format_source` (`4365e53`, and every candidate sequentially) |
| `bench --check` | 25,024 registry files (20,850 recorded), `registry-e8d87c2.tsv` | 0 changed (`4365e53`; the BOLT binary as well) |
| `conform -i`, `conform -i -V 1.8.0` | 1220 fixtures | 1220/1220 identical, 0 not idempotent, at both versions (`4365e53`) |
| `benchmarks/profile.sh` | 49-file sample | head 247,935,964 and 247,936,083 (two runs) against 248,197,787 for the base at the same path (−0.105%); the BKG range 248,202,094–248,203,822 is at another path length |
| `config_matrix` | | not run: no output changed on either hash corpus |

## Rejected (measured)

| attempt | measurement | why | decision |
|---|---|---|---|
| wl1, `write_list` cold arms | `Ir` −0.13%; `I1mr` cargo +0.5%, random mean −31 K (spread ±51 K); `Bcm` −19 K (spread ±17 K); own `I1mr` −13%, own `Bcm` +12% | hot lines already 89% dense; misses are capacity misses | reverted; `Ir` gain noted for E20 |
| oc1, `overflow`/`chains` cold arms | `Ir` +65 K; `Bcm`, `I1mr` flat or up | the arms were not in hot lines | reverted |
| kf1, flag table before `Expr`/`Pat`/`Type::cast` | `Bim` −21 K (−3.0%) in every order; `Ir` +67 K; `Bcm`, `I1mr` flat | one more test per call; the simulator's history-less indirect predictor overstates the gain on current cores | reverted (keep rule: `Ir` must not rise) |
| first draft of st1 (root depth taken as 1) | panicked in `parse_fragment` on the fixtures | builders opened by `parse_maybe_bound` hold three nodes around the parse | fixed before commit |
| flag tables for kind predicates in general | not built: kind tests hold 1.8% of the binary's `Bcm` and compile to one or two compares or a bit test | below the layout spread | — |

## Interpretation

- The derived third-of-the-cycles estimate holds up as a scale (28% of the simulator's cycle
  estimate at KCachegrind's weights), but its two parts are not where the levers of T1 reach.
  The I1 misses are spread over a 150 KB working set (80% of `Ir` in 49 KB, 95% in 154 KB,
  against a 32 KB L1I) and sit in dense hot lines; function order alone moves them by ±1%, and
  hand splitting the named functions cannot move them more than that. The conditional
  mispredicts are spread too: the largest source in chloro's own code was the tree build
  (4.4% of `Bcm`, mostly its exit state), which the kept change reduced by a third.
- The simulator is deterministic per binary but not per source change: a change re-draws the
  layout, and a cargo-order measurement then compares two draws. Shuffled relinks (seconds per
  variant once the LTO object is kept) turn the co-metric into a measurement of the change.
- BOLT, which orders and splits the whole binary from a profile, reaches −9.1% `I1mr` and
  −3.4% `Bcm` on top of PGO in the simulator, −1.6% by the simulator's cycle estimate; on this
  host its wall effect is inside ±4% on the CLI minima, while the stripped CLI grows 2.47 times.

## Decision

Kept on the branch, for the integrator: `cbbf081` (the co-metric; a harness change) and
`4365e53` (`steps_fused`: `Ir` −0.105%, `Bcm` −2.0% over random orders and −3.0% in cargo's,
output identical on both corpora, conform at both versions). It is kept on the primary metric
and the co-metric; no wall gain is claimed: the brief's falsifier ("fall without a wall gain in
three interleaved pairs") cannot be applied today, since the pairs on this host resolve nothing
below about 10% and the expected gain is 0.1–0.3%.

The hypothesis as a whole is refuted for this formatter on the levers T1 names: hand hot/cold
splitting moves neither `I1mr` nor `Bcm` beyond the layout spread, and flag tables on chloro's
kind predicates have a bound of about 2% of the mispredicts (the one built removes 3% of `Bim`
and raises `Ir`). BOLT on top of PGO is not proposed for the release workflow: identical output and
−9.1% `I1mr` in the simulator, but no wall gain resolved (CLI minima within ±4%; `bench` slower
in three of four pairs in the quietest window), a simulator cycle estimate of −1.6%, and a 2.47
times larger stripped binary. The Clang precedent's 12% does not transfer to a 150 KB hot
working set that PGO has already laid out. Three attempts on code layout (wl1, oc1, BOLT) gave
gains inside the noise band, so under the plateau rule (protocol §7) the layout lever is closed
at these ceilings until a quiet-host measurement says otherwise.

## Follow-ups

- Use shuffled relinks (`--shuffle-sections`, four or more seeds) for any layout or
  dispatch candidate; a cargo-order co-metric alone cannot tell a change from a re-draw of the
  layout. A `--shuffle N` option for `profile.sh` would need the LTO object and cargo's link
  line (`cargo rustc ... -C save-temps --print link-args`).
- wl1's `Ir` −0.13% (`write_list`'s comment arms out of line) is available to E20's rewrite.
- `Expr::cast` without the jump table needs the enum's tag from a table, which needs `unsafe`
  (a `#[repr(u8)]` typed enum built from its tag and node); bound 46 K `Bim` in the simulator,
  less on hardware with history-based indirect prediction.
- `steps_fused`'s remaining indirect jump (134 K `Bim`) is over the parser's step kinds, a
  data-dependent sequence; the parser's own decode (`event::process`, `LexedStr::new`, the
  lexer) holds 344 K `Bim`, behind the fork question (T9).
- BOLT can be re-measured on a quiet host in minutes from the commands above (the tool is one
  `apt-get install`); if M5 resolves a gain, a workflow step would run after the action's PGO
  build on `x86_64-unknown-linux-gnu` only (`--emit-relocs` in `RUSTFLAGS`, instrument, train
  as `pgo.sh` does, optimise), and would have to strip `.bolt.org.*` or accept the size.
