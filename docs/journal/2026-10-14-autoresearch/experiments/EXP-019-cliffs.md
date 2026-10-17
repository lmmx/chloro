# EXP-019: the cliff hunt and the repeated-work audit

Ledger row E19 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md); the brief is trick T8 and pass-2 points 2–3 of
[../../2026-10-15-unmined-tricks.md](../../2026-10-15-unmined-tricks.md) (§ 4, § 5). Labels:
**measured** (a tool's output on the builds below), **read** (from source), **derived**.

## Identity

| field | value |
|---|---|
| branch | `20261015-cliffs` from `73f8a85` (was `8ebd3bb`) (integration branch; its code is the BKG `f4be63e` (was `e640fbf`), the commits above it are documentation), worktree `/home/user/wt/cliffs` |
| kept commits | `de2c229` (was `d17cb74`) (else-if chain buffer, a cliff), `339c42a` (was `9c43d8c`) (use-segment comparison, a cliff), `ab342c2` (was `a4b51d5`) (`rustc_span` and leading attributes from the first child), `6a0fe4e` (was `774559d`) (`definitive_tactic` stops at the deciding item), `8704724` (was `f18dac3`) (`write_list` last-line width only for post-comments), `daee9b5` (was `25b1bf9`) (token-tree tokens from the tree, not lexed again); `6419b33` (was `f81f9ca`) (the generator) and this record |
| rejected, reverted | none of the commits; levers considered and not built are under *Not built* |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, valgrind 3.22.0, `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`; `cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples` (fat LTO, one codegen unit) |
| host | 4 vCPU Intel Xeon @ 2.80GHz, 15 GB; other experiments built and ran throughout (load average 2.7–17.8 during this run). `Ir` is unaffected by load; every wall-clock figure is **provisional** (protocol § 6) |
| `Ir` command | `benchmarks/profile.sh --core 3 --bench <copy> --sample /tmp/chloro-bench/cliffs49` (or `cliffs4b`): the script's sample under a private path of the default's length, the binary copied to `/tmp/chloro-bench/bin/bench-<x>` so that every build runs from a path of one length (the count moves with path lengths) |
| per-file `Ir` (co-metric, new) | the 49 sample files, each formatted by its own `bench -n 1` process under the same `valgrind` invocation, summed (script in the scratch directory, `tools/perfile.sh`; directory `/tmp/chloro-bench/1f-bench-<x>`). Each process starts with a fresh heap; see *Heap layout* for why it is reported |
| golden hashes | `scratchpad/golden/fixtures-e8d87c2.tsv` (1238 fixtures) and `registry-e8d87c2.tsv` (20,850 recorded registry files; 25,024 present, the others reported as not recorded) |

## Hypothesis and falsifiers

(a) At least one input shape makes chloro's instructions grow superlinearly with its size.
Falsified if every shape's fitted exponent lies within 1.0–1.1.

(b) Facts are recomputed many times per node (parent and subtree-boundary searches, fit
checks whose inputs are known, indentation strings, trivia relations, repeated scans of one
source region) at a cost a function profile does not attribute to a "recomputation".
Falsified if no recomputation costs more than 0.5% of the sample's instructions.

Both are **refuted** (the hypotheses hold): one shape has an exponent of 2.57 (3.6 in the
additive-constant fit) and two more exceeded 1.1 until fixed; five repeated or unneeded
computations of more than 0.5% each were found and removed.

## 1. The cliff hunt

### Method (measured)

`benchmarks/cliffs/gen.py` (standard library only) writes 27 shapes at scale factors 1, 2, 4,
8: the 12 of the brief plus parameters, a chain of `.map(|x| ..)` closures, multi-line string
arguments, `+` chains, array literals, `if`/`else if` chains, use lists, enums, nested generic
types, nested tuples, attributes and nested inline modules. Count shapes use `n = base x
scale` elements; depth shapes use depth `2 x scale` and repeat their nest 8 times
(`gen.py list` prints each). `benchmarks/cliffs/run.sh` checks every file against `rustfmt
--edition 2024` (exit status and byte equality of chloro's output), runs callgrind exactly as
`profile.sh` does on `bench -n 1 --root <directory holding that one file>`, and fits `Ir = a
n^k` by least squares on log–log. The fixed cost of a run (an empty file: 328,199 `Ir`) is
subtracted before the fit (`k (net)`), since a constant pulls the exponent towards 0; the fit
against chloro's output bytes is also given, because the depth shapes' output grows as the
square of the depth (every nested line carries the indentation of its depth). At the
coordinator's request (external research brief § 1.4) `gen.py fit` also fits `Ir = a + b
n^p` with the constant `a` fitted (the per-file fixed cost: process, settings, the tree's
base allocation), by relative least squares with `p` on a 0.001 grid, against the scale and
against output bytes; see *The additive-constant fit* for which exponent the decisions use.

```sh
benchmarks/cliffs/run.sh --core 3 --work /tmp/chloro-bench/cliffs-e19              # 27 x {1,2,4,8}
benchmarks/cliffs/run.sh --core 3 --work /tmp/chloro-bench/cliffs-big --no-compare \
    --scales 8,16,32,64 --shapes '<the 19 count shapes>'
benchmarks/cliffs/run.sh --core 3 --work /tmp/chloro-bench/cliffs-ce --no-compare \
    --scales 1,2,3,4,5,6,7,8 --shapes 'closures-expr nested-tuples nested-types nested-macros'
```

**Conformance (measured).** All 108 files (27 shapes × 4 scales) are accepted by rustfmt
(exit 0, nothing on stderr) and chloro's output is identical to rustfmt's on every one, at
the base and at the branch head. No conformance finding.

### Shapes at scales 1–8 (measured; base `73f8a85` (was `8ebd3bb`) and branch head `daee9b5` (was `25b1bf9`))

| shape | n = | Ir s1 | Ir s2 | Ir s4 | Ir s8 | k (raw) | k (net) | k vs output bytes | last local slope | head Ir s8 | head k (net) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| chain | 16 x scale | 678,081 | 981,083 | 1,607,887 | 2,821,607 | 0.688 | 0.947 | 0.956 | 0.962 | 2,722,753 | 0.943 |
| chain-closures | 8 x scale | 1,103,403 | 1,815,880 | 3,190,372 | 5,993,397 | 0.814 | 0.955 | 1.009 | 0.985 | 5,779,254 | 0.953 |
| closures-block | 2 x scale | 1,588,821 | 2,648,157 | 5,000,646 | 11,545,333 | 0.950 | 1.047 | 0.799 | 1.263 | 11,299,462 | 1.049 |
| **closures-expr** | 2 x scale | 1,032,625 | 1,473,195 | 16,221,022 | 115,094,935 | 2.386 | **2.584** | **1.663** | 2.852 | 107,511,580 | **2.566** |
| generic-calls | 2 x scale | 1,867,362 | 3,085,671 | 5,643,339 | 12,443,329 | 0.908 | 0.988 | 0.689 | 1.189 | 10,880,768 | 0.942 |
| match-arms | 32 x scale | 1,961,544 | 3,499,518 | 6,836,003 | 12,997,963 | 0.915 | 0.990 | 0.983 | 0.961 | 12,503,029 | 0.990 |
| args | 32 x scale | 561,255 | 760,033 | 1,164,461 | 1,939,999 | 0.598 | 0.932 | 0.941 | 0.947 | 1,824,687 | 0.920 |
| params | 16 x scale | 549,996 | 738,067 | 1,134,133 | 1,883,924 | 0.595 | 0.941 | 0.939 | 0.949 | 1,817,967 | 0.934 |
| struct-lit | 32 x scale | 789,794 | 1,190,919 | 2,001,553 | 3,610,761 | 0.733 | 0.945 | 0.935 | 0.972 | 3,373,965 | 0.937 |
| block-comment | 32 x scale | 473,990 | 567,323 | 759,434 | 1,147,197 | 0.425 | 0.832 | 0.835 | 0.925 | 1,188,210 | 0.836 |
| line-comments | 32 x scale | 488,696 | 602,268 | 835,504 | 1,296,537 | 0.470 | 0.867 | 0.868 | 0.933 | 1,297,978 | 0.864 |
| string-lit | 32 x scale | 497,469 | 594,104 | 788,678 | 1,331,174 | 0.467 | 0.849 | 0.865 | 1.123 | 1,344,545 | 0.848 |
| string-lines | 32 x scale | 545,393 | 688,204 | 974,234 | 1,715,296 | 0.546 | 0.887 | 0.896 | 1.102 | 1,563,624 | 0.879 |
| items | 32 x scale | 4,335,702 | 8,408,158 | 16,578,352 | 32,700,894 | 0.972 | 1.005 | 0.997 | 0.994 | 32,185,764 | 1.005 |
| nested-blocks | 2 x scale | 1,006,728 | 1,465,752 | 2,470,799 | 5,056,389 | 0.774 | 0.932 | 0.685 | 1.142 | 5,018,832 | 0.933 |
| where | 8 x scale | 713,155 | 1,073,786 | 1,751,430 | 3,120,798 | 0.709 | 0.951 | 0.912 | 0.972 | 3,037,008 | 0.948 |
| **nested-macros** | 2 x scale | 1,358,703 | 2,613,620 | 5,748,341 | 16,210,554 | 1.187 | **1.308** | 0.790 | 1.551 | 12,848,187 | **1.241** |
| macro-args | 32 x scale | 911,753 | 1,414,620 | 2,427,036 | 4,494,677 | 0.768 | 0.946 | 0.948 | 0.989 | 4,077,182 | 0.935 |
| binary | 16 x scale | 512,721 | 658,937 | 962,295 | 1,519,903 | 0.525 | 0.901 | 0.906 | 0.910 | 1,511,702 | 0.898 |
| array | 64 x scale | 819,012 | 1,279,147 | 2,201,359 | 4,032,214 | 0.768 | 0.973 | 0.918 | 0.984 | 3,663,060 | 0.958 |
| if-else | 8 x scale | 697,636 | 1,023,094 | 1,672,682 | 2,983,025 | 0.700 | 0.949 | 0.982 | 0.982 | 2,842,035 | 0.940 |
| **use-list** | 32 x scale | 1,144,367 | 2,107,441 | 4,215,061 | 9,246,214 | 1.004 | **1.148** | 1.138 | 1.198 | 4,346,540 | 1.081 |
| enum | 32 x scale | 851,598 | 1,368,692 | 2,369,784 | 4,361,142 | 0.786 | 0.981 | 0.969 | 0.982 | 4,206,813 | 0.977 |
| **nested-types** | 2 x scale | 677,748 | 918,327 | 2,002,366 | 5,153,892 | 0.991 | **1.287** | 0.767 | 1.527 | 4,289,047 | 1.207 |
| **nested-tuples** | 2 x scale | 741,669 | 930,767 | 1,865,479 | 5,219,835 | 0.945 | **1.204** | 0.730 | 1.670 | 3,676,014 | 1.038 |
| attrs | 16 x scale | 683,043 | 1,001,138 | 1,648,303 | 2,942,235 | 0.704 | 0.962 | 0.959 | 0.986 | 2,913,755 | 0.959 |
| nested-modules | 2 x scale | 896,801 | 1,206,081 | 1,823,909 | 3,088,379 | 0.595 | 0.761 | 0.603 | 0.884 | 2,997,379 | 0.756 |

Exponents below 1 at small scales are the per-file costs that do not scale with the shape
(the item around it, the file's tree root); the net fit removes only the empty file's cost.

### The additive-constant fit, and the exponent acted on (measured)

`Ir = a + b n^p`, `p` against the scale (`n` = 1, 2, 4, 8) and against output bytes, base
`73f8a85` (was `8ebd3bb`) and branch head `daee9b5` (was `25b1bf9`); the last column is the same fit over the longer series:
scales 8–64 for the count shapes (base, and head where a lever changed it), the eight depths
2, 4, …, 16 for the depth shapes measured there. `a` comes out at 0.2–1.1 M against the empty
file's 0.33 M: the shape's own fixed part (the enclosing item, the first nesting levels) is in
it.

| shape | k (pure) | k (net) | p, base | a, base | p vs output bytes, base | p, head | p over 8–64 (base → head) |
|---|---|---|---|---|---|---|---|
| chain | 0.688 | 0.947 | 0.995 | 366,320 | 0.969 | 0.995 | 0.994 |
| chain-closures | 0.814 | 0.955 | 0.991 | 392,924 | 0.978 | 0.988 | 1.007 |
| closures-block | 0.950 | 1.047 | 1.338 | 949,687 | 0.799 | 1.349 |  |
| closures-expr | 2.386 | 2.584 | 3.560 | 845,601 | 1.436 | 3.540 | 3.639 (8 depths, 2–16; p vs output 1.344) |
| generic-calls | 0.908 | 0.988 | 1.264 | 1,066,030 | 0.709 | 1.173 |  |
| match-arms | 0.915 | 0.990 | 0.998 | 357,668 | 0.966 | 0.998 | 0.998 |
| args | 0.598 | 0.932 | 0.975 | 350,979 | 0.941 | 0.977 | 0.984 |
| params | 0.595 | 0.941 | 0.985 | 350,412 | 0.958 | 0.986 | 0.987 |
| struct-lit | 0.733 | 0.945 | 1.001 | 387,126 | 0.951 | 1.002 | 0.972 |
| block-comment | 0.425 | 0.832 | 1.024 | 382,914 | 1.014 | 1.022 | 1.022 |
| line-comments | 0.470 | 0.867 | 1.004 | 373,859 | 0.995 | 1.004 | 0.997 |
| string-lit | 0.467 | 0.849 | 1.326 | 444,916 | 1.331 | 1.318 | 0.999 |
| string-lines | 0.546 | 0.887 | 1.240 | 453,367 | 1.229 | 1.276 | 1.012 |
| items | 0.972 | 1.005 | 0.992 | 200,842 | 0.979 | 0.993 | 0.997 |
| nested-blocks | 0.774 | 0.932 | 1.270 | 702,454 | 0.727 | 1.273 |  |
| where | 0.709 | 0.951 | 0.968 | 344,190 | 0.922 | 0.968 | 0.964 |
| nested-macros | 1.187 | 1.308 | 1.559 | 777,566 | 0.534 | 1.455 | 1.570 (8 depths; p vs output 0.552) |
| macro-args | 0.768 | 0.946 | 1.021 | 425,366 | 0.987 | 1.018 | 0.988 |
| binary | 0.525 | 0.901 | 0.950 | 348,570 | 0.932 | 0.949 | 1.040 |
| array | 0.768 | 0.973 | 0.996 | 355,226 | 0.926 | 0.994 | 1.018 |
| if-else | 0.700 | 0.949 | 1.006 | 376,093 | 0.999 | 0.984 | 1.268 → 1.014 |
| use-list | 1.004 | 1.148 | 1.197 | 417,108 | 1.115 | 1.146 | 1.160 → 1.127 |
| enum | 0.786 | 0.981 | 0.974 | 319,321 | 0.939 | 0.976 | 0.964 |
| nested-types | 0.991 | 1.287 | 1.754 | 543,639 | 0.400 | 1.564 | 1.651 (8 depths; p vs output 0.317) |
| nested-tuples | 0.945 | 1.204 | 1.970 | 655,165 | 0.616 | 1.612 | 1.837 (8 depths; p vs output 0.572) |
| attrs | 0.704 | 0.962 | 1.011 | 368,123 | 0.999 | 1.010 | 1.020 |
| nested-modules | 0.595 | 0.761 | 1.019 | 598,268 | 0.547 | 1.021 |  |

With three parameters on four points the fit is loose: string-lit and string-lines read 1.33
and 1.24 at scales 1–8, where their per-chunk cost is still settling, and 0.999 and 1.012
over 8–64. **The exponent acted on** is `p` of the additive fit over the longest series
measured for the shape (8–64 for count shapes, the eight depths for closures-expr and the
nested shapes), read against output bytes for the depth shapes, whose output is quadratic in
the depth. On it the findings are those of the pure fits: closures-expr is a cliff (`p` 3.6
against the depth, 1.34 against output bytes), if-else was one (1.268 → 1.014), use-list is
the sort's n log n with a heavy comparator (1.160 → 1.127), every other count shape lies in
0.96–1.04, and the depth shapes are sublinear or linear in their output (`p` 0.30–0.81, with
nested-macros' quadratic lexing inside a sublinear total).

### Count shapes at scales 8–64 (measured, base)

A quadratic term with a small coefficient shows first as a rising local slope, so the 19
count shapes were run again at 8, 16, 32, 64 (up to 4,096 array elements, 2,048 arms, use
names or items, 131 KB of string literal).

| shape | Ir s8 | Ir s16 | Ir s32 | Ir s64 | k (net) over 8..64 | local slopes |
|---|---|---|---|---|---|---|
| chain | 2,821,551 | 5,296,063 | 10,516,862 | 20,340,929 | 1.005 | 0.994 1.036 0.974 |
| chain-closures | 5,993,576 | 11,615,994 | 23,103,305 | 45,830,194 | 1.003 | 0.994 1.013 0.998 |
| match-arms | 12,996,559 | 25,623,596 | 50,635,125 | 100,976,018 | 0.996 | 0.998 0.992 1.000 |
| args | 1,940,034 | 3,532,049 | 6,850,167 | 13,092,387 | 0.998 | 0.990 1.025 0.969 |
| params | 1,883,934 | 3,396,485 | 6,658,690 | 12,615,982 | 0.999 | 0.979 1.045 0.957 |
| struct-lit | 3,612,227 | 7,135,238 | 13,594,676 | 27,132,805 | 1.005 | 1.051 0.963 1.015 |
| block-comment | 1,147,117 | 1,933,194 | 3,441,975 | 6,661,213 | 0.980 | 0.970 0.955 1.024 |
| line-comments | 1,297,931 | 2,214,936 | 4,055,288 | 7,712,319 | 0.976 | 0.959 0.982 0.986 |
| string-lit | 1,329,710 | 2,263,273 | 4,126,692 | 7,853,762 | 0.970 | 0.949 0.973 0.986 |
| string-lines | 1,716,931 | 3,036,993 | 5,687,080 | 11,056,035 | 0.983 | 0.963 0.984 1.001 |
| items | 32,702,609 | 64,499,484 | 128,276,962 | 254,935,615 | 0.992 | 0.987 0.996 0.993 |
| where | 3,120,840 | 6,060,532 | 11,478,160 | 22,645,582 | 0.995 | 1.037 0.960 1.001 |
| macro-args | 4,494,635 | 8,791,933 | 17,180,100 | 34,087,247 | 1.005 | 1.022 0.993 1.002 |
| binary | 1,519,899 | 2,650,278 | 4,964,213 | 9,741,938 | 0.994 | 0.962 0.997 1.022 |
| array | 4,032,253 | 7,734,690 | 15,581,048 | 30,787,746 | 1.016 | 0.999 1.042 0.998 |
| **if-else** | 2,983,536 | 5,620,739 | 11,825,681 | 27,072,261 | **1.111** | 0.995 1.119 **1.218** |
| **use-list** | 9,244,708 | 20,006,966 | 45,635,897 | 99,394,711 | **1.162** | 1.142 1.203 1.129 |
| enum | 4,362,621 | 8,596,224 | 16,575,949 | 32,686,772 | 0.998 | 1.035 0.975 0.994 |
| attrs | 2,942,151 | 5,525,902 | 10,801,444 | 21,423,513 | 1.005 | 0.991 1.011 1.010 |

At the head (`8704724` (was `f18dac3`), same scales): **if-else** 2,841,888 / 5,291,730 / 10,338,724 /
20,332,636, k 0.999 (local slopes 0.981, 1.012, 0.999); **use-list** 4,346,049 / 8,896,286 /
19,470,061 / 41,206,002, k 1.120 (1.092, 1.160, 1.095).

### The shapes above 1.1, one by one

**closures-expr: a cliff, not fixed (measured).** `run_0(|a_0| run_1(|a_1| ... finish(a, 0)))`
eight times per file. By depth 2, 4, …, 16: 1,031,088, 1,473,886, 4,061,481, 16,219,259,
32,423,722, 55,296,873, 78,792,085, 115,094,913 `Ir`; from depth 8 to 16 the local exponent is
2.65–2.85, and against output bytes it is still 1.65, so the work per byte of output grows.
The jump at depth 6–8 is where the nest stops fitting on one line.

- Call counts at depth 16 (`callgrind`, measured): `rewrite_closure_fn_decl` 9,256 calls for
  128 closures (72 per closure; 22.5 at depth 8), `RewriteContext::memoize` 12,792,
  `overflow::Context::rewrite` 5,344, `rewrite_closure` 4,640; `write_list`, `malloc`/`free`
  and `memcpy` are the largest exclusive costs, building strings for candidates that are
  discarded.
- Memo misses for one nest, by depth (an instrumented debug build printing every miss with its
  shape; measured): 5, 9, 45, 205, 407, 672, 915, 1,286. The call at nesting level `i` is
  rewritten at up to 101 distinct shapes at depth 16 (1, 2, 4, 7, 11, 16, 22, 29, 35, 42, 50,
  59, 67, 78, 86, 95, 101 by level): every shape is a pair (block indent, offset) with block
  indents 4 apart and offsets 12 apart (one `run_k(|a_k| ` prefix), so a level sees O(i²)
  shapes and the nest O(d³) rewrites, each building strings of the remaining nest.
- Mechanism (read; chloro and rustfmt alike): `rewrite_closure` renders a non-block body
  without a block first (`rewrite_closure_expr`) and discards the result when it is
  multi-line (`veto_multiline`), then renders the block form; the call around it tries the
  overflowed last argument at `one_line_shape` and, failing that, the item at
  `nested_shape`. Each level thus renders the next level at an offset shape (expression body)
  and at a block-indented shape; offsets accumulate until the width runs out, block indents
  until the depth runs out.
- rustfmt itself is exponential here (measured, wall, provisional): 0.042 s at depth 8, 0.243 s
  at 12, 3.398 s at 16, against chloro's 7, 11 and 25 ms (process times of `fmt_stdin`). The
  memo turns rustfmt's 2^d into chloro's polynomial; the remaining exponent comes from distinct
  shapes, which the memo key (node, shape, flags) cannot merge.
- Why no fix here: the discarded renders are needed to learn that the body is multi-line. An
  exact skip needs a proof that no one-line rendering can exist at the shape, i.e. a lower
  bound on a node's one-line width that holds for every rewrite, including the fallbacks that
  return source text wider than the shape (macro calls returned as written, lost comments).
  That is the width model of T7/E26; a "one-line only" mode is not exact (a `None` instead of a
  multi-line candidate changes which tactic the enclosing list takes). See *Follow-up*.

**if-else: a cliff, fixed by `de2c229` (was `d17cb74`) (measured).** The local slope rose to 1.218 at 512
branches; `memcpy` was 22.7% of the run at 512 branches (6,146,654 `Ir`), 5,495,831 of it
called from `ControlFlow::rewrite`: each link of the chain appended the rest of the chain to
its own result (rustfmt's recursion), copying O(n²) bytes. After the fix the fit over 64–512
branches (scales 8–64) is 0.999 (local slopes 0.981, 1.012, 0.999), against 1.111.

**use-list: n log n with an expensive comparator, cut by `339c42a` (was `9c43d8c`) (measured).** 26,029
`UseTree::cmp` calls for 2,048 names (≈ n log2 n), 80.5 M of the run's 99.4 M; each compare of
two different names compared them a second time on `remove_alias()` copies (a `String` clone
each, a deep clone for nested lists) and built four `StrSearcher`s for `trim_start_matches
("r#")`. After the fix 41.2 M at 2,048 names and k 1.120 (8–64), the n log n of the sort's
comparison count with `version_sort` (about 760 `Ir` per call) as the remaining comparator
cost; not a quadratic.

**nested-types, nested-tuples, nested-macros, and the last slopes of closures-block,
generic-calls, nested-blocks: superlinear in depth, sublinear in output (measured).** Their
exponents against output bytes are 0.60–0.80: the output itself grows as the square of the
depth (indentation), and chloro's cost tracks it. nested-tuples and nested-types fell to
k 1.038 and 1.207 at the branch head (the two `lists.rs` levers remove width scans of
multi-line items). One component of nested-macros does grow faster than its input:
`LexedStr::new` runs about three times per macro call over the remaining interior (`MacroTokens::new` for the token
tree, `prefix_parse_window` per argument), 65, 113, 209, 401 calls and 235,810, 549,594,
1,583,587, 5,083,169 `Ir` (17%, 21%, 28%, 31% of the run) at depths 2, 4, 8, 16 — quadratic
lexing; rustc lexes a file once and nested token trees are not lexed again. `daee9b5` (was `25b1bf9`) removes
the first of those lexings (the token tree's interior, taken from the tree): 12,847,590 `Ir`
at depth 16 (−15.5%), k 1.242; the per-argument window lexing remains (*Follow-up*).

## 2. The repeated-work audit on the 49-file sample

### Method (measured)

- Call counts and exclusive/inclusive `Ir` from the base profile (`profile.sh` output,
  248,195,929 `Ir`) by a small parser of the callgrind file (`calls=` lines per callee;
  inclusive = self + outgoing calls, direct recursion excluded; recursive functions
  marked `'2` double count and are not used for shares).
- Logical counts from an instrumented build (a scratch copy of the base source, not committed:
  thread-local counters at 25 functions, the distinct inputs keyed by tree, tree id and node
  index, or by source pointer and length; built without debug assertions so that no
  `debug_assert!` adds calls). Its call counts equal the release profile's wherever both see
  the function (`rustc_span` 70,312, `outer_attributes` 43,807, `contains_skip` 29,074,
  `recover_comment_removed` 23,115).
- The sample's tree (`dump_tree`): 172,021 elements, 80,445 nodes, 91,576 tokens; macro
  fragments add more during formatting.

### Findings (measured; share of 248,195,929)

| fact | calls | distinct inputs | per input | Ir self / incl | share | stored once? | outcome |
|---|---|---|---|---|---|---|---|
| `rustc_span` (node start after leading trivia) | 70,312 | 23,388 nodes | 3.0 | 3,626,493 / 3,635,663 | 1.46% | a leading-trivia length in a side array would answer in O(1), but 69,987 of the 70,312 calls stop at the first child: the cost is the out-of-line call and loop set-up, not the scan | **lever 3** (`ab342c2` (was `a4b51d5`)): first child decided inline |
| `outer_attributes` / `has_leading_attrs` | 43,807 / 87,614 | 18,548 nodes | 2.4 | 2,441,955 / 2,879,845 | 0.98% / 1.16% | as above; 86,960 of 87,614 `has_leading_attrs` calls stop at the first child; 2,340 calls find an attribute | **lever 3** |
| list widths in `definitive_tactic` (`total_item_width`, `is_multiline`) | 21,637 / 10,184 + 1,848 | 9,643 item strings (by address and length: a lower bound) | 2.2 | 2,703,255 + 224,653 (widths), 442,458 + 102,243 (newlines), incl. 1,576,685 of whole-string widths of multi-line items | 1.4% | widths stored on `ListItem` would need every one of 45 assignment sites to keep them; most of the cost is not needed for the answer at all | **lever 4** (`6a0fe4e` (was `774559d`)): stop at the deciding item |
| `write_list`'s last-line width per item | 14,171 `is_multiline` + reverse searches + width scans | read only for items with a post-comment | — | 968,167 (`is_multiline`) + memrchr, reverse char search, memcmp | ≈1.5% (realized) | computed for every item, read by `rewrite_post_comment` only | **lever 5** (`8704724` (was `f18dac3`)): computed where read |
| `recover_comment_removed` (comment check after each expression rewrite) | 23,115 | 15,528 spans | 1.5 | 2,598,960 / 5,433,412 | 2.19% | no: the answer depends on the fresh rewrite; in 21,026 calls (91%) the rewrite equals the source and the cost is the comparison | not a lever |
| `span_before`/`span_after` (`opt_span_before`, `find_uncommented`) | 20,589 | 14,182 (span, needle) | 1.45 | 2,123,641 incl (`find_uncommented`) | 0.86% (repeats ≈ 0.27%) | the repeated share is below 0.5% | not built |
| `contains_skip` | 29,074 | 2,188 attributes in all | — | 710,638 / 1,385,720 | 0.56% incl | mostly empty slices: call overhead, ≤ 0.29% | not built |
| `OverflowableItem::span` (twice per item per list: lo and hi) | 33,349 | — | 2 | 530,660 / 2,240,550 | 0.90% incl; after lever 3 1,355,481 self | half avoidable by one span per item (≈ 0.28%) | not built |
| `format_expr_uncached` (renders of one expression) | 20,359 | 14,076 nodes | 1.45 | — | — | the memo's domain (shape differs per render; 315 hits of 9,766 `memoize` calls) | E24/E26, not here |
| literal fit (`filtered_str_fits` from `rewrite_literal`) | 1,938 | 1,746 literals | 1.11 | 2,208,877 incl | 0.89% | no: the cost sits in the 1,501 renders of 1,444 multi-line raw and other non-`Str` literals (4% repeats); it is one scan per literal | not a lever |
| macro token-tree interiors lexed again (`MacroTokens::new` → `LexedStr::new`) | 702 | 702 token trees, every byte already lexed with the file | 1 (2 with the file's lexing) | 1,834,877 | 0.74% | yes: the tree holds the tokens | **lever 6** (`daee9b5` (was `25b1bf9`)) |
| macro argument windows lexed again (`prefix_parse_window`) | 995 | windows inside the interiors | — | 1,664,995 | 0.67% | the parser needs a `LexedStr` (private fields) for its input and the tree build; not without a token-source abstraction in `TreeBuilder` | not built |
| `parent()` | 116 | 110 | 1.05 | — | ~0 | E17 removed the hot ones | — |
| `RunState::line_of` | 776 | — | — | 418,742 / 488,243 | 0.20% | | — |
| `Indent::to_string*` | inline slice (E16 S15) | | | not in the profile | ~0 | | — |
| `last_line_width` / `first_line_width` | 31,003 / 15,267 | fresh rendered strings | — | 1,973,730 / 909,592 | 0.80% / 0.37% | not facts of a node: each string is a new render (the sink's per-range stats, E20) | E20 |

Finding (b) therefore holds for five computations above 0.5% (levers 3–6 realize 1.30%,
0.83%, 1.49% and 0.69%). Three are recomputation in the brief's sense: `rustc_span` 3.0 times
per node and `outer_attributes` 2.4 times, where the store-once remedy loses to an inline test
of the first child because almost every query is answered there, and the macro token trees,
whose bytes were lexed twice (lever 6 takes them from the tree). The other two are facts
computed and then not needed (widths of items after the answer is known; a last-line width
read only for post-comments).

## 3. Levers (read, measured)

Each commit message states the mechanism, the exactness argument, the test and the counts.

| commit | lever | sample `Ir` before → after | change | per-file sum before → after | change | predicted | decision |
|---|---|---|---|---|---|---|---|
| `de2c229` (was `d17cb74`) | else-if chain into one buffer (cliff) | 248,195,929 → 248,671,306 | +475,377 (+0.19%) | 261,502,370 → 261,500,623 | −1,747 | shape: the quadratic copy gone; sample: no change (few long chains) | kept: if-else k 1.111 → 0.999 (64–512 branches), 27.07 M → 20.33 M at 512 branches; the sample's +0.19% is heap layout (below) |
| `339c42a` (was `9c43d8c`) | use-segment comparison without copies or `StrSearcher` (cliff) | 248,671,306 → 249,257,968 | +586,662 (+0.24%) | 261,500,623 → 260,716,168 | −784,455 (−0.30%) | shape: compares 2–4x cheaper; sample: −0.3 to −0.4% (`UseTree::cmp` 0.43% incl) | kept: use-list 99.4 M → 41.2 M at 2,048 names; the sample's +0.24% is heap layout |
| `ab342c2` (was `a4b51d5`) | `rustc_span`, `has_leading_attrs`, `outer_attributes` decided from the first child inline | 249,257,968 → 246,009,335 | −3,248,633 (−1.30%) | 260,716,168 → 257,475,192 | −3,240,976 | −4 M (2.5 M + 1.5 M) | kept (81% of the prediction) |
| `6a0fe4e` (was `774559d`) | `definitive_tactic` stops at the first multi-line item or at a prefix over the limit | 246,009,335 → 243,970,939 | −2,038,396 (−0.83%) | 257,475,192 → 255,443,984 | −2,031,208 | −1 to −2 M | kept |
| `8704724` (was `f18dac3`) | `write_list` computes an item's last-line width only for its post-comment | 243,970,939 → 240,329,974 | −3,640,965 (−1.49%) | 255,443,984 → 251,692,904 | −3,751,080 | ≈ −1.5 M (`is_multiline` 0.97 M plus the scans) | kept (more than predicted: the reverse searches and indent comparisons went too) |
| `daee9b5` (was `25b1bf9`) | token-tree tokens taken from the tree | 240,329,974 → 236,799,223 | −3,530,751 (−1.47%) | 251,692,904 → 249,968,643 | −1,724,261 (−0.69%) | −1.5 M (the 1.83 M lexing less the walk) | kept: the lexer and its allocations −1.8 M; the sample's further −1.67 M is realloc's `memcpy` at 339 fewer calls (heap layout, this time in favour) |
| **total** | | 248,195,929 → 236,799,223 | **−11,396,706 (−4.59%)** | 261,502,370 → 249,968,643 | **−11,533,727 (−4.41%)** | | |

Errata in commit messages (the figures in this record are the consistent ones):

- `de2c229` (was `d17cb74`) gives the if-else exponent as "over 8..512 branches 1.111 -> 0.971": the first
  figure is the fit over 64–512 branches (scales 8–64), the second over 8–512 (scales 1–64).
  Over the same range, 64–512 branches, the head reads 0.999.
- `339c42a` (was `9c43d8c`) gives the use-list exponent as "over 256..2,048 names 1.162 -> 1.101": the second
  figure is over 32–2,048 names. Over 256–2,048 names the head (`8704724` (was `f18dac3`)) reads 1.120.
- `ab342c2` (was `a4b51d5`) gives its per-file change as 260,732,104 → 257,482,737; the first figure was taken
  with the per-file directory under a path of another length.

The per-file figures above are all from one path length (`/tmp/chloro-bench/1f-bench-<x>`,
binaries at `/tmp/chloro-bench/bin/bench-<x>`); two runs of the same binary differ by up to
7.5 K.

### Heap layout (measured)

The first two levers raised the sample's count while removing instructions from the code they
change. The per-function difference shows where (`de2c229` (was `d17cb74`) against the base): `memcpy`
+551,402 at 139 fewer calls, all of it in `_int_realloc`'s copy (`0x48a26b0` → `memcpy`
3,437,663 → 3,989,262 at 13,571 → 13,449 calls), and the parser's `Vec<Event>` growth
(`RawVec<Event>::grow_one` → `grow_amortized`) +634,373 at the same 3,211 calls: event
vectors of later files that used to grow in place at the top of the heap now move, because
the formatter's allocations for an earlier file left the heap in another state. The same
happened for `339c42a` (was `9c43d8c`) (`memcpy` +1,181,307 at 739 fewer calls) while its own functions fell
by 0.75 M. EXP-016 recorded the same effect (S10, the second S7 round) and named the
parser's event vector as the likely source. With one process per file the coupling between
files disappears and both levers show what their code does: −1,747 and −784,455. The protocol's
`Ir` stays the primary metric and is reported first; the per-file sum is reported beside it
so that a lever's effect can be told from a layout shift. Levers 3–5 move both metrics by the
same amount (within 0.12 M); lever 6 moved the sample by 1.8 M more than the per-file sum, in
its favour. Over the six levers the two metrics agree within 0.14 M (−11.40 M and −11.53 M).

## Wall clock (measured, provisional)

`bench -n 5` on the 1238 fixtures (13.9 MB), best of 5 rounds, the base binary (`73f8a85` (was `8ebd3bb`),
built in this worktree before the first change) and each commit's binary interleaved, three
pairs each, `taskset -c 3` and later `taskset -c 1`. Each binary holds every lever up to its
commit. Core 3 was shared with another experiment's process (`chloro-e21b`, pinned there
during the second set); the halved rounds (3.4–4.2 s) are that contention.

| binary (levers up to) | core | pair | load at start–end | base s, MB/s | lever s, MB/s | base / lever time |
|---|---|---|---|---|---|---|
| `de2c229` (was `d17cb74`) | 3 | 1 | 6.33–5.22 | 2.006, 6.93 | 2.008, 6.92 | 0.999 |
| `de2c229` (was `d17cb74`) | 3 | 2 | 5.22–4.31 | 1.974, 7.04 | 1.878, 7.40 | 1.051 |
| `de2c229` (was `d17cb74`) | 3 | 3 | 4.31–3.52 | 1.943, 7.16 | 1.950, 7.13 | 0.996 |
| `339c42a` (was `9c43d8c`) | 3 | 1 | 3.52–3.16 | 2.022, 6.88 | 1.835, 7.58 | 1.102 |
| `339c42a` (was `9c43d8c`) | 3 | 2 | 3.16–2.83 | 1.920, 7.24 | 1.829, 7.60 | 1.050 |
| `339c42a` (was `9c43d8c`) | 3 | 3 | 2.83–2.60 | 1.916, 7.26 | 1.924, 7.23 | 0.996 |
| `ab342c2` (was `a4b51d5`) | 3 | 1 | 2.60–2.51 | 1.964, 7.08 | 1.843, 7.54 | 1.066 |
| `ab342c2` (was `a4b51d5`) | 3 | 2 | 2.51–2.37 | 1.807, 7.69 | 1.932, 7.19 | 0.935 |
| `ab342c2` (was `a4b51d5`) | 3 | 3 | 2.37–2.34 | 1.827, 7.61 | 2.238, 6.21 | 0.816 |
| `6a0fe4e` (was `774559d`) | 3 | 1 | 2.34–2.38 | 2.100, 6.62 | 2.019, 6.89 | 1.040 |
| `6a0fe4e` (was `774559d`) | 3 | 2 | 2.38–2.62 | 1.994, 6.97 | 1.971, 7.05 | 1.012 |
| `6a0fe4e` (was `774559d`) | 3 | 3 | 2.62–2.92 | 2.022, 6.87 | 1.748, 7.95 | 1.157 |
| `8704724` (was `f18dac3`) | 3 | 1 | 2.92–3.55 | 2.008, 6.92 | 3.719, 3.74 | 0.540 |
| `8704724` (was `f18dac3`) | 3 | 2 | 3.55–4.27 | 3.703, 3.75 | 1.942, 7.16 | 1.907 |
| `8704724` (was `f18dac3`) | 3 | 3 | 4.27–6.53 | 2.102, 6.61 | 1.959, 7.09 | 1.073 |
| `daee9b5` (was `25b1bf9`) | 3 | 1 | 6.53–7.79 | 1.999, 6.95 | 1.939, 7.17 | 1.031 |
| `daee9b5` (was `25b1bf9`) | 3 | 2 | 7.79–8.42 | 2.049, 6.78 | 1.934, 7.19 | 1.059 |
| `daee9b5` (was `25b1bf9`) | 3 | 3 | 8.42–9.16 | 1.986, 7.00 | 1.824, 7.62 | 1.089 |
| `ab342c2` (was `a4b51d5`) | 3 | 1 | 7.77–7.38 | 1.929, 7.21 | 1.983, 7.01 | 0.973 |
| `ab342c2` (was `a4b51d5`) | 3 | 2 | 7.38–6.86 | 2.074, 6.70 | 1.890, 7.36 | 1.097 |
| `ab342c2` (was `a4b51d5`) | 3 | 3 | 6.86–6.92 | 1.995, 6.97 | 1.915, 7.26 | 1.042 |
| `8704724` (was `f18dac3`) | 3 | 1 | 6.92–6.64 | 4.194, 3.31 | 3.412, 4.07 | 1.229 |
| `8704724` (was `f18dac3`) | 3 | 2 | 6.64–6.80 | 4.070, 3.42 | 3.620, 3.84 | 1.124 |
| `8704724` (was `f18dac3`) | 3 | 3 | 6.80–5.60 | 3.691, 3.77 | 1.733, 8.02 | 2.130 |
| `daee9b5` (was `25b1bf9`) | 3 | 1 | 5.60–4.80 | 1.819, 7.64 | 1.778, 7.82 | 1.023 |
| `daee9b5` (was `25b1bf9`) | 3 | 2 | 4.80–3.98 | 1.802, 7.72 | 1.869, 7.44 | 0.964 |
| `daee9b5` (was `25b1bf9`) | 3 | 3 | 3.98–3.52 | 2.070, 6.71 | 1.669, 8.33 | 1.240 |
| `daee9b5` (was `25b1bf9`) | 1 | 1 | 3.13–2.96 | 1.594, 8.72 | 1.563, 8.89 | 1.020 |
| `daee9b5` (was `25b1bf9`) | 1 | 2 | 2.96–2.98 | 1.632, 8.52 | 1.548, 8.98 | 1.054 |
| `daee9b5` (was `25b1bf9`) | 1 | 3 | 2.98–3.50 | 1.802, 7.71 | 1.714, 8.11 | 1.051 |
| `6a0fe4e` (was `774559d`) | 1 | 1 | 3.50–3.52 | 1.517, 9.17 | 1.593, 8.72 | 0.952 |
| `6a0fe4e` (was `774559d`) | 1 | 2 | 3.52–3.68 | 1.616, 8.60 | 1.543, 9.01 | 1.047 |
| `6a0fe4e` (was `774559d`) | 1 | 3 | 3.68–4.42 | 1.651, 8.42 | 1.607, 8.65 | 1.027 |

No single lever's pairs separate: the cumulative `Ir` change up to each commit is −0.0%, −0.3%,
−1.5%, −2.3%, −3.8% and −4.4% (per-file sum), inside the ±4% spread of `bench` on this host
under load 2.3–9.2. For the branch head (`daee9b5` (was `25b1bf9`)) the first set on core 3 gave ranges apart
(base 6.78–7.00 MB/s, head 7.17–7.62), the second set on core 3 and the set on core 1 (load
3.0–3.5; head faster in each pair, 1.02x–1.05x) overlap. The `Ir` figures are the claim; a
quiet-host measurement is owed (protocol § 6).

## Gates (measured)

| gate | corpus | `de2c229` (was `d17cb74`) | `339c42a` (was `9c43d8c`) | `ab342c2` (was `a4b51d5`) | `6a0fe4e` (was `774559d`) | `8704724` (was `f18dac3`) | `daee9b5` (was `25b1bf9`) |
|---|---|---|---|---|---|---|---|
| `cargo fmt --all --check` | | clean | clean | clean | clean | clean | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | clean | clean | clean | clean | clean | clean |
| `cargo test --workspace --all-features` | | 290 passed, 0 failed | 292, 0 | 292, 0 | 294, 0 | 294, 0 | 295, 0 |
| `bench --check`, sequential | 1238 fixtures, `fixtures-e8d87c2.tsv` | 0 changed | 0 | 0 | 0 | 0 | 0 |
| `bench --check --threads 4` | same | 0 changed, 0 differ from `format_source` | 0, 0 | 0, 0 | 0, 0 | 0, 0 | 0, 0 |
| `bench --check`, sequential and `--threads 4` | 20,850 recorded registry files (25,024 present), `registry-e8d87c2.tsv` | 0 changed, 0 differ | 0, 0 | 0, 0 | 0, 0 | 0, 0 | 0, 0 |
| `conform -i` | 1220 fixtures, rustfmt 1.9.0 | 1220/1220, 0 not idempotent | same | same | same | same | same |
| `conform -i -V 1.8.0` | 1220 fixtures, rustfmt 1.8.0 | 1220/1220, 0 not idempotent | same | same | same | same | same |
| cliff shapes against rustfmt | 108 generated files | — | — | — | — | 108/108 identical | 108/108 identical |

The `daee9b5` (was `25b1bf9`) column ran on `6419b33` (was `f81f9ca`), which adds only `benchmarks/` files to it. `clippy` and
the tests ran on an archive of each commit with their own target directory, the
goldens and `conform` on copies of each commit's release binaries; `config_matrix` and the
registry `config_matrix` (gates 6–7) were not run: no output changed on either hash corpus.

## Tests added (read)

| test | file | compares |
|---|---|---|
| `cmp_without_alias_matches_comparing_copies_without_aliases` | imports.rs | `cmp_without_alias` with `remove_alias().cmp(remove_alias())` on all 4,624 ordered pairs of 68 segments, at style editions 2024 and 2021 and rustfmt 1.9.0 and 1.8.0 |
| `trim_raw_prefix_matches_trim_start_matches` | imports.rs | `trim_raw_prefix` with `trim_start_matches("r#")` |
| `first_child_fast_paths_match_the_scans` | nodes.rs | the inline `has_leading_attrs` and `rustc_span` with their scans on every node of every fixture file (over 1,000,000 nodes) and a source with comments, doc comments and attributes in leading positions |
| `definitive_tactic_matches_the_full_sum` | lists.rs | the early-exit `definitive_tactic` with rustfmt's full sum on every list of up to three items over twelve kinds of item, six tactics, both separators, widths 0–40 |
| `interior_tokens_match_a_lexing_of_the_interior` | macro_args.rs | the tree's tokens of every token tree of every fixture file, at editions 2024 and 2018, with a lexing of the interior; sources with comments, doc comments, raw identifiers and interiors starting with `#!`, `---` and `-` |
| debug assertion in `MacroTokens::new` | macro_args.rs | both paths on every token tree that a debug build formats (the conformance tests run every fixture) |

Each was checked against a mutation of the code it covers (the equal-alias condition dropped;
a comment taken for a significant first token; a separator dropped from the prefix sum;
comments dropped from the tree path); every mutation failed its test. `de2c229` (was `d17cb74`) and `8704724` (was `f18dac3`) move code without changing what it
computes and rely on the goldens and `conform`.

## Not built

| idea | why not | bound |
|---|---|---|
| closures-expr: skip the expression-body render when no one-line rendering can fit | needs a one-line width lower bound valid for every rewrite and every fallback (T7/E26) | the shape only; on the sample the vetoed renders are few (decide-once-and-exact-reuse.md at `1b89e4c` (was `31ce20e`): of 206 expression-body attempts at the outer level, 18 fell back to the block form) |
| a "one-line only" mode under `veto_multiline` | not exact: returning `None` for a multi-line candidate changes the enclosing list's tactic | — |
| nested macros: parse each argument window from the tree's tokens instead of a new lexing | the parser's input comes from `LexedStr::to_input` and the fragment tree from `TreeBuilder::steps` over a `LexedStr` (private fields): needs a token-source abstraction in the tree builder | `prefix_parse_window`'s lexing, 1,664,995 `Ir` (0.67%) on the sample; the rest of nested-macros' quadratic term |
| widths cached on `ListItem` | 45 assignment sites of its fields would have to keep the cache; levers 4–5 removed most of the cost without it | ≤ 0.3% after levers 4–5 |
| `contains_skip`, `OverflowableItem::span` once per item, `opt_span_before` repeats | each below 0.5% | 0.29%, 0.28%, 0.27% |
| `version_sort` fast path for a common prefix | exact only with care for digit runs, underscores and the `usize` overflow that ends rustfmt's iteration; 0.15% of the sample | use-list's remaining n log n constant |

## Decision

Kept: the six commits, together −11,396,706 `Ir` on the sample (248,195,929 → 236,799,223,
−4.59%) and −11,533,727 on the per-file sum (−4.41%), with byte-identical output on both hash corpora (sequential and four threads)
and `conform` 1220/1220 at both rustfmt versions. Branch `20261015-cliffs`, for the
integrator's gates 1–5 on the merged tree; the integrator should read the first two levers'
sample counts with *Heap layout*. The generator and its README stay in `benchmarks/cliffs/`.

## Follow-up

- closures-expr (k 2.57): the width lower bound for one-line renderings (T7/E26) would let
  `rewrite_closure_expr` skip bodies that cannot be one line; measure it on this shape first.
- Nested macros: parse macro argument windows from the token tree's own tokens (a token
  source for `TreeBuilder` beside `LexedStr`), which removes the remaining quadratic lexing
  and 0.67% of the sample.
- The parser's `Vec<Event>` (reserve from the token count, or a reused buffer) would remove
  the heap-layout coupling that made the first two levers read +0.19% and +0.24%; until then a
  lever under about 1 M should be judged with the per-file sum beside the sample's count.
- use-list's remaining exponent is the comparison count; a sort key per segment
  (`sort_by_cached_key`) would need `version_sort` as a key, which it is not.

## Merged with the integration head

At the coordinator's request the branch took the integration head before its own merge:
`c20dab9` (was `fdcbcd5`) merges `275690f` (was `9d78713`) (E20's sink, E22's `steps_fused` and `profile.sh --sim`) and
`b745cbc` (was `72df381`) the journal commit `efc206c` (was `2f00988`) above it.

**Resolution (read).** `lists.rs` conflicted in `write_list` and in the test module. E20's
`5ddfd19` (was `c40a441`) moves the item's last-line width into the post-comment branch, as `8704724` (was `f18dac3`) did, and
also moves the nested-import test under `formatting.nested`; `8704724` (was `f18dac3`) adds nothing beyond it,
so E20's form stands and `8704724` (was `f18dac3`)'s is dropped. The `definitive_tactic` early stop
(`6a0fe4e` (was `774559d`)) and its test were re-applied on E20's file unchanged: `lists.rs` at the merged head
is E20's file plus exactly `6a0fe4e` (was `774559d`)'s 111 added and 4 removed lines. `experiments.md` keeps
every row (E20, E22, E19). No other file conflicted; the other five E19 levers merged as
they were.

**Figures (measured).** `benchmarks/profile.sh --core 3` at the script's default sample path
(`/tmp/chloro-bench/sample49`):

| build | binary path | `Ir` |
|---|---|---|
| integration head `275690f` (was `9d78713`) | this worktree's `target/release/examples/bench` | 237,770,134 |
| integration head `275690f` (was `9d78713`) | `/tmp/chloro-bench/bin/bench-I` | 237,770,050; 237,768,455 |
| merged head `b745cbc` (was `72df381`) | this worktree's `target/release/examples/bench` (the script's default) | **232,050,549; 232,050,509** |
| merged head `b745cbc` (was `72df381`) | `/tmp/chloro-bench/bin/bench-M` | 232,048,568; 232,050,352 |

The merged head is −5,719,585 `Ir` (−2.41%) against the integration head at the same paths.
The per-file sum (one process per sample file, `/tmp/chloro-bench/1f-bench-<x>`): 251,045,466
→ 243,308,946, **−7,736,520 (−3.08%)**, which is the combined E19 effect net of heap layout:
E19's −11.53 M on its own base less the −3.75 M of `8704724` (was `f18dac3`), which E20 had already taken
(−7.78 M expected). The sample's count shows 2.0 M less because realloc's `memcpy` rises
+1,973,619 at 1,291 fewer calls (the heap-layout term); by function the E19 levers move as
on their own branch (`rustc_span` −3.63 M, `outer_attributes` −2.44 M, the multi-line width
scans −1.57 M, `total_item_width` −0.55 M, lexing −0.9 M, `StrSearcher` −0.36 M).

**Gates on `b745cbc` (was `72df381`) (measured).**

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace --all-features` | 296 passed, 0 failed (1 ignored) |
| `bench --check` on 1238 fixtures (`fixtures-e8d87c2.tsv`), sequential and `--threads 4` | 0 changed; 0 differ from `format_source` |
| `bench --check` on 20,850 recorded registry files (`registry-e8d87c2.tsv`, 25,024 present), sequential and `--threads 4` | 0 changed; 0 differ |
| `conform -i`, `conform -i -V 1.8.0` | 1220/1220 identical, 0 not idempotent, at both versions |
| `profile.sh --core 3`, default sample path | 232,050,509–232,050,549 (above); at the private path `cliffs49` with the `bench-g` copy 232,048,721 |
| per-file sum, again under the gate script | 243,307,307 (243,308,946 above; run-to-run spread) |

`clippy` and the tests ran on an archive of `b745cbc` (was `72df381`) with their own target directory; the
goldens and `conform` on copies of its release binaries.

**Errata** (from § 3, unchanged): the if-else and use-list exponent ranges in the messages of
`de2c229` (was `d17cb74`) and `339c42a` (was `9c43d8c`), and the per-file baseline in `ab342c2` (was `a4b51d5`)'s message; the figures of this
record are the consistent ones.
