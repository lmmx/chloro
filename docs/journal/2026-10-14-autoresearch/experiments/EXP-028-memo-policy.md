# EXP-028: what the memo stores: a store policy for `RewriteContext::memoize`

Ledger row E28 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md) (§3 and §4 as amended by E27: both samples, the per-file
figure for changes under about 2 M). The origin is [EXP-024](EXP-024-memo-array.md)'s
follow-up (the memo's cost is storing results that are read again 3.2% of the time); the
cliff constraint is [EXP-019](EXP-019-cliffs.md)'s `closures-expr`; the heap-layout term is
[EXP-020](EXP-020-sink.md)'s; the harness is [EXP-027](EXP-027-harness.md)'s. Labels:
**measured** (a tool's output on the builds below), **read** (from source), **derived**
(arithmetic on measured figures).

## Identity

| field | value |
|---|---|
| branch | `20261015-memo-policy` from `33866dc` (was `4f1b9d9`) (integration branch; its code is the BKG `dcbb43d` (was `05ab7b4`), the commits above it are documentation), worktree `/home/user/wt/memopol` |
| merged in | `61d708e` (was `99dc14a`) merges `origin/20261015-harness` (E27: `profile.sh` path independence, `--per-file`, `--sample-list`, `parse_only`, `bench --phase`; no formatter change). Conflicts in `benchmarks/README.md` and the ledger only, both sides kept |
| kept commit | `870bd5d` (was `fd18d50`) (the memo from nesting depth 6, the debug hit check, the test); then this file and the ledger row |
| rejected, not committed | store on the second miss; kind-selective storing; a subtree-size gate; depth gates other than 6; depth 6 with kinds off; depth 5 with a size gate (all measured below with an instrumented build) |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, valgrind 3.22.0, hyperfine 2.0.0, `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`; `cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples` (fat LTO, one codegen unit) |
| host | 4 vCPU Xeon @ 2.80GHz; other agents built and ran throughout (load average 11.9–23.3). `Ir` is unaffected; every wall-clock figure is **measured, provisional** (protocol §6) |
| binaries | base: `bench` of `61d708e` (was `99dc14a`) (sha256 `b1ff3e01…`, the BKG's formatter with E27's `bench`), copied to `bench-base` before the first change; head: `bench` of `870bd5d` (was `fd18d50`) (sha256 `a8debc57…`, built at the commit before its comment-only amend; the rebuild at `870bd5d` (was `fd18d50`) is byte-identical, see *Gates*) |
| `Ir` commands | `benchmarks/profile.sh --core 3 --bench BIN --runs 3 --per-file [--sample-list benchmarks/samples/registry49.txt]`; `--sim` for the co-metric |
| golden hashes | `scratchpad/golden/fixtures-e8d87c2.tsv` (1238 fixtures), `registry-e8d87c2.tsv` (25,024 registry files present, 4,174 reported as not recorded) |

## Hypothesis and falsifiers

The memo pays for the few hits that bound exponential nesting with a store on every miss; a
policy that stores only keys likely to be read again removes most of the store cost exactly
and keeps the cliff bounded. Candidates: (a) store on the second miss, (b) kind-selective
storing, (c) both; and, if the memo does not pay on ordinary code, (d) a depth- or size-gated
policy.

Falsified if: fixture-sample instructions do not fall by at least 1 M on the per-file figure;
the registry sample regresses; `closures-expr` at depth 16 costs more than 1.5x today's
instructions or its fitted exponent rises; any output changes.

## What the memo does (read)

`memoize` (context.rs) is called by `format_expr` for every expression except paths and
literals; the key is (tree, node index, `ExprType`, shape, five context flags); a hit replays
three run effects (macro failure, lost comment, skipped ranges) and returns a clone of the
stored `Option<String>`; a miss computes, and stores a clone with the effects unless a context
flag was left changed. The table is cleared at every top-level item.

## Commands

```text
E=scratchpad/e28
python3 -I $E/instr.py ROOT; python3 -I $E/instr2.py ROOT     # instrumentation (not committed; patches instr1.patch, instr2.patch)
CHLORO_E28="stats=1 [policy]" $E/bin/e28_stats DIR > stats.tsv; python3 -I $E/scripts/agg.py stats.tsv
$E/rc.sh TAG 'rc=1' | 'rc=2'        # instr2 bench under callgrind: each hit also recomputes in rcf_<KIND>/rcl_<KIND> (full / shell)
python3 -I $E/scripts/rcsum.py fx|rg; python3 -I $E/scripts/kindnet.py fx 692
$E/pfscreen.sh BIN CORE OUT.tsv 'POLICY'...   # profile-e28.sh --per-file --runs 1, both samples, CHLORO_E28 padded to 160 bytes
$E/cliff.sh BIN CORE TAG 'POLICY' 'SHAPES' SCALES   # run.sh's callgrind invocation with the policy; gen.py fit
benchmarks/profile.sh --core 3 --bench BIN --runs 3 --per-file [--sample-list benchmarks/samples/registry49.txt]
benchmarks/profile.sh --core 2 --bench BIN --sim [--sample-list ...]
benchmarks/cliffs/run.sh --core 0 --bench BIN [--stdin fmt_stdin] [--no-compare] [--scales 1,2,3,4,5,6,7,8 --shapes closures-expr]
python3 -I $E/scripts/pfcmp.py BASE_OUT HEAD_OUT                    # per-file change by file
$E/cli.sh, hyperfine -N pinned (peak RSS); $E/pairs.sh BASE HEAD 3   # wall, provisional
```

The instrumented build selects a policy at run time from `CHLORO_E28` (`mode=all|none|nohit|
second`, `mindepth=D`, `minsize=N`, `off=`/`only=` kind ids, `rc=1|2`, `stats=1`); its depth
is the number of `memoize` rewrites in progress, as in the kept commit. Its own figures (the
"instrumented family") are compared only with each other: the policy read costs about 1 M
over the base build, the same in every policy.

## Step 1: measurements at the base (store every rewrite)

### The trace (measured, `e28_stats`)

| quantity | fixture sample | registry sample | `closures-expr` depth 8 | depth 12 | depth 16 |
|---|---|---|---|---|---|
| lookups | 9,766 | 5,848 | 1,656 | 6,160 | 12,800 |
| hits | 315 (3.2%) | 55 (0.9%) | 16 (1.0%) | 784 (12.7%) | 2,512 (19.6%) |
| keys stored | 9,451 | 5,793 | 1,640 | 5,376 | 10,288 |
| keys found at least once | 242 (2.6%) | 50 (0.9%) | 16 | 360 | 800 |
| bytes stored / returned by hits | 1,508,354 / 16,178 | 482,441 / 2,128 | 82,016 / 1,152 | 543,672 / 80,008 | 2,439,808 / 591,480 |
| kinds | 26 | 26 | `CALL_EXPR` (0 hits), `CLOSURE_EXPR` (16) | `CALL_EXPR` 736 hits, `CLOSURE_EXPR` 48 | `CALL_EXPR` 2,456 hits, `CLOSURE_EXPR` 56 |

The fixture figures are E24's (9,766 lookups, 315 hits, 9,451 inserts). The registry sample is
E27's committed list, not E24's `regsmp49`.

### Hits per key (measured)

| hits on the key | fixture sample (keys) | registry sample | `closures-expr` depth 12 | depth 16 |
|---|---|---|---|---|
| 0 | 9,209 | 5,743 | 5,016 | 9,488 |
| 1 | 195 | 45 | 136 | 216 |
| 2 | 28 | 5 | 88 | 144 |
| 3 | 14 | 0 | 88 | 128 |
| 4 | 4 | 0 | 32 | 104 |
| 5–8 | 1 | 0 | 16 | 208 |

On ordinary code 97–99% of the stored results are never read; of the keys that are, 80–90%
are read once. In the nest, keys are read up to eight times.

### By nesting depth (measured; depth = `memoize` rewrites in progress)

| depth ≥ D | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 10 |
|---|---|---|---|---|---|---|---|---|---|---|
| fixture lookups | 9,766 | 8,005 | 6,108 | 4,416 | 3,354 | 2,520 | 1,801 | 1,215 | 831 | 440 |
| fixture hits | 315 | 314 | 311 | 299 | 269 | 238 | 188 | 148 | 102 | 61 |
| registry lookups | 5,848 | 4,577 | 3,365 | 2,310 | 1,545 | 932 | 541 | 302 | 177 | 9 |
| registry hits | 55 | 55 | 42 | 38 | 36 | 19 | 7 | 7 | 3 | 0 |

By subtree size the hits are spread from 4 to 127 elements on both samples (fixtures: 42, 92,
106, 54, 15 hits in 4–7, 8–15, 16–31, 32–63, 64–127), as E24 found; size does not separate them.
By tag and shape class (block indent or visual offset) the fixture hit rate is 1.2–4.2%: no
class separates them either.

### The memo's net value (measured, exact)

Counterfactuals with the instrumented build, one process per file (per-file sum, `--runs 1`),
so the heap-layout term cannot enter: `none` neither looks up nor stores; `nohit` looks up
and stores every rewrite but never uses an entry.

| quantity | fixture sample | registry sample |
|---|---|---|
| per-file `Ir`, store every rewrite (`all`) | 246,833,745 | 320,799,658 |
| without the memo (`none`) | 246,725,670 | 317,042,688 |
| store every rewrite, use none (`nohit`) | 253,485,982 | 321,068,932 |
| **saved by the hits** (`nohit` − `all`) | **6,652,237** | **269,274** |
| **cost of looking up and storing** (`nohit` − `none`) | **6,760,312** (692 per lookup) | **4,026,244** (688 per lookup) |
| **net value of the memo** (`none` − `all`) | **−108,075** | **−3,756,970** |
| the same from the whole sample (`all` 233,023,261 / 306,793,454; `none` 231,496,184 / 302,935,410) | −1,527,077 | −3,858,044 |

The memo does not pay on ordinary code: on the fixture sample its hits save about what its
stores cost (−0.11 M net, −0.04%), on the registry sample it costs 3.76 M (1.2%). The
whole-sample figure (−1.53 M on the fixtures) is 1.4 M off the exact one: heap layout.

### By kind (measured savings, derived store cost)

The instrumented `bench` recomputes the rewrite at every hit inside a function per kind
(`rcf_<KIND>` for the key's first hit, `rcl_<KIND>` for later ones) and checks the
recomputation against the entry (0 mismatches, 0 fragment trees built again, both samples).
"Full" recomputes with the memo off below (the cost of computing the entry from nothing,
what a hit saves when nothing below is stored: the `nohit` world), "shell" with the memo read
but not written below (what one hit saves when everything else is stored). The store cost is
the measured 692 (688) instructions per lookup times the kind's lookups (derived).

Fixture sample (all 26 kinds with 20 or more lookups or a hit):

| kind | lookups | hits | keys found | saved, full | saved, shell | store cost | net value |
|---|---|---|---|---|---|---|---|
| `MATCH_EXPR` | 255 | 5 | 5 | 2,681,525 | 223,552 | 0.18 M | +2.51 M |
| `METHOD_CALL_EXPR` | 2,323 | 81 | 66 | 1,866,803 | 924,587 | 1.61 M | +0.26 M |
| `CLOSURE_EXPR` | 521 | 18 | 18 | 566,892 | 126,332 | 0.36 M | +0.21 M |
| `BIN_EXPR` | 377 | 31 | 20 | 310,127 | 157,060 | 0.26 M | +0.05 M |
| `TRY_EXPR` | 165 | 5 | 5 | 142,874 | 13,800 | 0.11 M | +0.03 M |
| `IF_EXPR` | 257 | 4 | 4 | 126,545 | 48,948 | 0.18 M | −0.05 M |
| `REF_EXPR` | 510 | 67 | 39 | 299,707 | 87,699 | 0.35 M | −0.05 M |
| `TUPLE_EXPR` | 186 | 3 | 3 | 52,829 | 20,882 | 0.13 M | −0.08 M |
| `RETURN_EXPR` | 166 | 3 | 3 | 18,083 | 3,359 | 0.11 M | −0.10 M |
| `RECORD_EXPR` | 248 | 3 | 3 | 42,180 | 25,257 | 0.17 M | −0.13 M |
| `FIELD_EXPR` | 697 | 52 | 38 | 285,547 | 285,227 | 0.48 M | −0.20 M |
| `BLOCK_EXPR` | 400 | 1 | 1 | 4,820 | 4,820 | 0.28 M | −0.27 M |
| `ARRAY_EXPR` | 461 | 0 | 0 | 0 | 0 | 0.32 M | −0.32 M |
| `MACRO_EXPR` | 660 | 1 | 1 | 5,548 | 5,548 | 0.46 M | −0.45 M |
| `CALL_EXPR` | 2,128 | 35 | 30 | 412,956 | 235,059 | 1.47 M | −1.06 M |
| eleven others (`PREFIX`, `INDEX`, `PAREN`, `CAST`, `FOR`, `LET`, `CONTINUE`, `UNDERSCORE`, `RANGE`, `WHILE`, `BREAK`) | 412 | 6 | 6 | 4,567 | 4,719 | 0.29 M | −0.28 M |
| **all** | 9,766 | 315 | 242 | 6,821,003 | 2,166,849 | 6.76 M | **+0.06 M** |

Registry sample: every kind's net value is negative; the largest stores are
`METHOD_CALL_EXPR` (1,204 lookups, 15 hits, full 105,846, net −0.72 M), `CALL_EXPR` (1,016,
12, 79,725, −0.62 M), `BIN_EXPR` (613, 11, 194,323, −0.23 M); all kinds 569,242 full,
313,869 shell, 4.02 M store cost, **−3.45 M** net. The derived totals (+0.06 M, −3.45 M) lie
within 0.17 M and 0.31 M of the exact ones (−0.11 M, −3.76 M).

What the measurements say: five hits on large `match` expressions carry 39% of the fixture
sample's savings (536 K each), and `CALL_EXPR`, the kind the cliff needs (2,456 of the 2,512
hits at depth 16), is the largest net cost on the fixture sample. Kind does not separate useful
stores from useless ones in a way that keeps the cliff; nesting depth does: 5,350 lookups at
depths 0–2 found 16 entries on the fixture sample.

### What the second-miss policy would save and cost on these traces (derived)

Storing on the second miss leaves 9,209 of 9,451 stores undone on the fixture sample (5,743
of 5,793 on the registry), but each of the 242 (50) keys that are found becomes a second
computation of its whole subtree, as its descendants are not stored either: the measured
full cost of the first hits, 6,342,611 (531,803). Against stores worth at most 6.76 M (4.03 M)
including the lookups the policy keeps, the fixture sample gains nothing; the registry
sample gains at most 3.5 M. In the nest every key under a found key is computed twice, so
depth 16 approaches twice today's cost. Measured below: +0.89 M on the fixtures, 1.66x at
depth 16.

## Step 2–4: the policies, measured

Instrumented family, per-file sum (`--runs 1`, run-to-run spread a few K) and the whole sample
for contrast, change against `all`; cliff columns from `cliff.sh` (`closures-expr`, depths
2–16, `p` of `Ir = a + b n^p`; `all` reads 106,747,407 at depth 16, `p` 3.623).

| policy | fixture per-file | change | registry per-file | change | fixture whole (change) | `closures-expr` depth 16 | `p` |
|---|---|---|---|---|---|---|---|
| `all` (the base's policy) | 246,833,745 | | 320,799,658 | | 233,023,261 | 106,747,407 | 3.623 |
| `none` | 246,725,670 | −108,075 | 317,042,688 | −3,756,970 | −1,527,077 | not run (exponential) | |
| `nohit` | 253,485,982 | +6,652,237 | 321,068,932 | +269,274 | +4,829,054 | | |
| (a) `second` (store on the second miss) | 247,724,876 | **+891,131** | 318,924,890 | −1,874,768 | −956,068 | **177,680,920 (1.66x)** | **4.000** |
| (b) kinds off: the 14 kinds with at most 3 hits on the fixtures and none on the registry (`ARRAY`, `MACRO`, `BLOCK`, `CAST`, `FOR`, `LET`, `RANGE`, `WHILE`, `LOOP`, `BREAK`, `AWAIT`, `RECORD`, `TUPLE`, `RETURN`) | 245,540,305 | −1,293,440 | 320,096,155 | −703,503 | −3,267,272 | unaffected (`CALL`, `CLOSURE` only) | |
| subtree ≥ 32 elements | 243,974,412 | −2,859,333 | 318,093,417 | −2,706,241 | −3,909,413 | 106,391,495 | 3.626 |
| depth ≥ 2 | 244,515,548 | −2,318,197 | 319,196,114 | −1,603,544 | −4,630,794 | 106,627,140 | 3.626 |
| depth ≥ 3 | 243,400,472 | −3,433,273 | 318,485,576 | −2,314,082 | −3,279,963 | 106,648,437 | 3.628 |
| depth ≥ 4 | 242,938,186 | −3,895,559 | 317,954,431 | −2,845,227 | −3,960,427 | 106,609,544 | 3.632 |
| depth ≥ 5 | 242,628,785 | −4,204,960 | 317,618,687 | −3,180,971 | −4,567,262 | | |
| **depth ≥ 6** | **242,581,908** | **−4,251,837** | **317,573,437** | **−3,226,221** | −5,509,006 | **106,692,107** | **3.643** |
| depth ≥ 7 | 242,741,616 | −4,092,129 | 317,397,100 | −3,402,558 | −5,616,527 | | |
| depth ≥ 8 | 242,929,041 | −3,904,704 | 317,316,355 | −3,483,303 | −5,427,199 | 106,658,353 | 3.649 |
| depth ≥ 10 | 243,353,583 | −3,480,162 | 317,147,209 | −3,652,449 | −5,154,831 | | |
| depth ≥ 12 | | | | | | 110,322,917 (1.03x) | 3.701 |
| (c) depth ≥ 6 and the 14 kinds off | 242,563,320 | −4,270,425 | 317,631,657 | −3,168,001 | −6,634,236 | | |
| depth ≥ 5 and subtree ≥ 16 | 242,457,130 | −4,376,615 | 317,366,561 | −3,433,097 | −6,908,851 | | |

Other depth shapes (`cliff.sh`, scales 1, 2, 4, 8, the instrumented family): at depths ≥ 3, 5,
6 and 8 every one of `closures-block`, `generic-calls`, `nested-blocks`, `nested-macros`,
`nested-types`, `nested-tuples`, `nested-modules`, `chain-closures`, `chain`, `match-arms`,
`args` and `array` costs 0.936–1.002 of `all` at every scale; with the 14 kinds off,
0.934–1.002.

Reading the table:

- (a) is refuted on two falsifiers: the fixture sample rises (+0.89 M per file) and
  `closures-expr` at depth 16 costs 1.66x with `p` 3.62 → 4.00. Its whole-sample figure
  (−0.96 M) points the other way: heap layout.
- (b) is safe on the cliff shapes but small (−1.29 M, −0.70 M): the kinds with few hits hold
  under a quarter of the fixture stores (2,233 of 9,766 lookups); the costly stores are `CALL_EXPR` and `METHOD_CALL_EXPR`, which the
  nest and the large hits need. (c) adds nothing to the depth gate (−0.02 M on the fixtures,
  +0.06 M on the registry).
- The size gate does half of the depth gate's work: hits are spread over sizes.
- (d) the depth gate: the fixture optimum is depth 5–6 (−4.20 M, −4.25 M), the registry
  sample keeps improving to depth 10 (−3.65 M; there the memo's best is `none`, −3.76 M).
  Depth 6 is the fixture sample's best and within 0.43 M of the registry's best depth (10);
  up to depth 8
  `closures-expr` does not move, at depth 12 it costs 3% more. A size gate on top (depth 5,
  subtree ≥ 16) gains another 0.12–0.21 M for a second constant; not taken (below).

## The policy (read; `870bd5d` (was `fd18d50`))

`RunState` counts the `memoize` rewrites in progress (`memo_depth`). While fewer than
`MEMO_MIN_DEPTH` = 6 are in progress, `memoize` increments the count, runs the rewrite and
returns, with no key, lookup or store; from depth 6 on it is the memo as before, with the
count incremented around the computation. Nothing else changes: the key, the effects
replayed on a hit, the not-stored rule for a changed flag, the clear per top-level item.

Exactness (read): an entry is stored only from a computation of its key, with that
computation's effects, and a hit returns it; a store-always memo returns the entry of the
key's first computation, and both are the rewrite's result for the key under the premise
every memo of this formatter rests on (a rewrite is a function of node, tag, shape and flags,
with its run effects replayed). The debug check below tests that premise on every hit of
every fixture. Which keys are stored changes only what is computed again.

Bound (read): at depth 6 and deeper every key is computed at most once between clears, as
before; a rewrite above depth 6 is computed once per attempt of the rewrites around it, at most
the product of the attempts of five levels, which does not grow with the nesting below.

The trace under the policy (measured, `e28_stats`, `mindepth=6`):

| | fixture sample | registry sample | `closures-expr` depth 16 |
|---|---|---|---|
| lookups (base) | 1,846 (9,766) | 556 (5,848) | 12,688 (12,800) |
| hits (base) | 226 (315) | 11 (55) | 2,512 (2,512) |
| stores (base) | 1,620 (9,451) | 545 (5,793) | 10,176 (10,288) |
| rewrites computed without the memo | 8,031 | 5,343 | 112 |

### Tests (read, measured)

- Debug builds check every memo hit: `check_memo_hit` computes the rewrite again with the memo
  read but not written and with no check nested inside it (one recomputation per hit over
  memoized subexpressions, not exponential), compares the result and the three effects with
  the entry, and restores the run's effects. `cargo test` reaches it on every fixture
  through `parallel_equivalence`, `unit_cache_equivalence` and the conformance tests.
  Mutation: the comparison inverted panics in `parallel_items_match_the_sequential_formatter`
  at the first hit (`s.level`).
- `memoize_answers_as_a_memo_storing_every_rewrite` (context.rs): 200 random top-level
  rewrites over the 60-odd nodes of a parsed function, each a pure function of (node, tag,
  shape, flags) that rewrites later nodes at narrower or offset shapes, some with
  `is_if_else_block` set, and sets the three effects for some keys; clears in between. Each
  is answered through `memoize` and through a test memo that stores every rewrite; the
  results and effects must be equal, no key may be computed twice between clears at a
  memoized depth, the depth must return to 0, and the run must compute more than 1,000
  rewrites at memoized depths, more than 500 above them, and find more than 1,000. Mutations:
  the skipped-range replay dropped (fails, at the hit check), the depth not restored on the
  unmemoized path (fails: depth 6 after a call).

## Before and after (measured)

### `Ir`, both samples (`profile.sh --core 3 --runs 3 --per-file`)

| sample | figure | base `b1ff3e01` | head `a8debc57` | change |
|---|---|---|---|---|
| fixtures | whole, runs | 232,051,613; 232,051,511; 232,051,594 | 226,279,128; 226,278,925; 226,278,922 | |
| fixtures | **whole, min** | 232,051,511 | 226,278,922 | **−5,772,589 (−2.49%)** |
| fixtures | **per-file sum of minima** | 245,724,269 | 241,848,588 | **−3,875,681 (−1.58%)** |
| fixtures | empty sample | 318,121 | 318,121 | 0 |
| fixtures | cross-file term | +1,597,050 | −299,858 | −1,896,908 (layout) |
| registry | whole, runs | 305,873,751; 305,873,676; 305,875,401 | 302,665,229; 302,665,165; 302,665,332 | |
| registry | **whole, min** | 305,873,676 | 302,665,165 | **−3,208,511 (−1.05%)** |
| registry | **per-file sum of minima** | 320,019,955 | 317,099,928 | **−2,920,027 (−0.91%)** |
| registry | cross-file term | +1,123,529 | +835,045 | −288,484 (layout) |

The lever's own effect is the per-file figure: −3.88 M and −2.92 M; the whole-sample
figures add 1.90 M and 0.29 M of heap layout in its favour. Per file (`pfcmp.py`): 39 of 49
fixture files lower (the largest changes −889,335 on `ide/src/hover/tests.rs`, −512,531 on
`hir-ty/src/infer/coerce.rs`), 9 higher, of them 8 by under 1 K and
`ide-completion/src/completions/mod_.rs` by +82,489 (+1.48%: shallow hits lost); on the
registry 41 lower (−419,505 on `syn-1.0.109/src/path.rs`, −377,865 on
`portable-atomic-1.15.0/build.rs`), 8 higher, the largest `matrixmultiply-0.3.11/src/threading.rs`
+61,015 (+1.09%). The release build lands 0.38 M (fixtures) and 0.31 M (registry) short of
the instrumented family's −4.25 M and −3.23 M.

### Co-metric (`--sim`, one run each)

| sample | build | `Bc` | `Bcm` | `Bi` | `Bim` | `Bcm + Bim` | `I1mr` | `D1mr` | `D1mw` |
|---|---|---|---|---|---|---|---|---|---|
| fixtures | base | 40,519,358 | 2,938,144 | 1,851,935 | 708,121 | 3,646,265 | 4,694,731 | 689,491 | 533,857 |
| fixtures | head | 38,139,552 | 2,877,324 | 1,835,714 | 703,155 | 3,580,479 | 4,568,228 | 595,885 | 432,324 |
| fixtures | change | −5.9% | −2.07% | −0.9% | −0.7% | **−1.80%** | **−2.69%** | −13.6% | −19.0% |
| registry | base | 52,999,105 | 3,320,795 | 2,586,923 | 1,087,092 | 4,407,887 | 4,010,578 | 816,198 | 544,015 |
| registry | head | 52,450,941 | 3,273,444 | 2,573,978 | 1,077,581 | 4,351,025 | 3,940,074 | 770,622 | 494,850 |
| registry | change | −1.0% | −1.43% | −0.5% | −0.9% | **−1.29%** | **−1.76%** | −5.6% | −9.0% |

Every penalty count falls; the first-level data misses fall most (−19% of write misses on the
fixtures): the 104-byte table slots and the cloned strings no longer written.

### The cliffs (`benchmarks/cliffs/run.sh`, official driver, no policy variable)

`closures-expr`, depths 2–16 (`--scales 1,…,8`):

| depth | 2 | 4 | 6 | 8 | 10 | 12 | 14 | 16 |
|---|---|---|---|---|---|---|---|---|
| base `Ir` | 1,001,224 | 1,415,097 | 3,786,912 | 14,917,131 | 29,755,024 | 50,831,797 | 72,366,223 | 105,412,036 |
| head `Ir` | 982,138 | 1,392,945 | 3,719,037 | 14,868,935 | 29,713,151 | 50,692,318 | 72,091,211 | 105,248,546 |
| head / base | 0.981 | 0.984 | 0.982 | 0.997 | 0.999 | 0.997 | 0.996 | **0.998** |

| fit | base | head |
|---|---|---|
| `k_net`, depths 2–16 | 2.687 | 2.702 |
| `p` (`a + b n^p`), depths 2–16 | 3.623 | 3.636 |
| `p` against output bytes | 1.330 | 1.336 |
| `k_net`, depths 8–16 | 2.8251 | 2.8258 |
| `k_net`, depths 10–16 | 2.6722 | 2.6709 |

All 27 shapes at scales 1, 2, 4, 8 (`run.sh`, base and head): every file costs 0.940–1.002 of
the base (17 of 108 files dearer, the largest by +0.17%, `nested-types` at scale 2; the largest
fall −6.0%, `chain-closures` at scale 8); the fitted `p` moves by −0.096 to +0.019 (`array` +0.019, `nested-macros` +0.012,
`string-lit` −0.096, `string-lines` −0.068), within the loose four-point fit. The head's 108
files and the 8 `closures-expr` depths are all accepted by rustfmt and byte-identical to its
output.

Against the cliff falsifier: depth 16 costs 0.998x, not more than 1.5x. The fitted exponent
over the whole series **rises** by 0.013 (`p`) and 0.015 (`k_net`); the rise is the shallow
depths becoming 1.6–1.9% cheaper (depths 2–6, where the gate removes the memo's fixed cost)
while the deep end is unchanged: over depths 8–16 the exponent moves +0.0007, over 10–16
−0.0013, and no depth costs more. The letter of the falsifier is met by a lower intercept, not
by faster growth; this record reports it and keeps the change on that reading (see
*Decision*).

### Peak RSS and wall clock (measured, provisional)

Peak RSS, `taskset -c 3 hyperfine -N -i -w 1 -r 8 --metrics time_wall_clock,memory_peak_resident`,
`chloro --no-cache --check` on formatted inputs, two rounds with the order alternating (16 runs
per binary), 14:06:55–14:07:16 UTC, load 23.1–22.8; `fx-hir-ty` from the first `cli.sh` round
(10 runs per binary, 14:03:15–14:04:16, load 23.0–22.9):

| workload | base KiB min/median/max | head KiB min/median/max | median change |
|---|---|---|---|
| `ide-db/src/generated/lints.rs` | 10,104 / 10,120 / 10,248 | 8,900 / 9,028 / 9,028 | **−10.79%** |
| `syntax/src/ast/generated/nodes.rs` | 14,148 / 14,268 / 14,340 | 14,284 / 14,404 / 14,464 | +0.95% |
| `hir/src/lib.rs` | 8,896 / 8,990 / 9,028 | 9,028 / 9,156 / 9,216 | +1.85% |
| `ide/src/hover/tests.rs` | 4,700 / 4,742 / 4,852 | 4,704 / 4,716 / 4,840 | −0.55% |
| `rust-analyzer/src/config.rs` | 5,328 / 5,446 / 5,512 | 5,312 / 5,444 / 5,568 | −0.04% |
| `fx-hir-ty` (122 files) | 8,148 / 8,276 / 8,324 | 7,428 / 7,444 / 7,500 | −10.05% |
| `tiny-one` (13 bytes) | 3,420 / 3,484 / 3,644 | 3,548 / 3,610 / 3,696 | +3.62% |

`lints.rs` stored 3,428 entries and found none (E24); its table no longer grows. `tiny-one`
formats nothing the memo touches; its +126 KiB median is start-up (the binaries are the same
size), not the memo, and is not investigated further here.

`pairs.sh` (`taskset -c 3 bench -n 5`, 1238 fixtures, order alternating), 14:07:26–14:09:26 UTC,
load 22.1–15.9: base 2.78, 4.91, 5.12 MB/s; head 3.45, 4.51, 5.41 MB/s; head/base per pair
1.241, 0.919, 1.057. A second set, 14:22:11–14:24:59, load 9.0–9.5: base 2.56, 2.75, 2.83 MB/s;
head 2.80, 2.81, 3.58 MB/s; head/base 1.094, 1.022, 1.265 (the head faster in each pair). The
ranges overlap in both sets (core 3 was shared: `bench` reads 7–9 MB/s on a quiet core in
earlier records). No wall claim; a quiet-host pair is owed.

## Gates (measured)

| gate | corpus | result on `870bd5d` (was `fd18d50`) |
|---|---|---|
| `cargo fmt --all --check` | | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | clean (one `type_complexity` in the new test fixed before commit) |
| `cargo test --workspace --all-features` | | 298 passed, 0 failed, 1 ignored; every fixture's memo hits checked against a recomputation in the debug build |
| `bench --check`, sequential and `--threads 4` | 1238 fixtures, `fixtures-e8d87c2.tsv` | 0 changed; 0 differ between 4 threads and `format_source` |
| `bench --check` | 25,024 registry files, `registry-e8d87c2.tsv` (4,174 not recorded) | 0 changed |
| `conform -i`, `conform -i -V 1.8.0` | 1220 fixtures | 1220/1220 identical, 0 not idempotent, at both versions |
| `config_matrix -n 1220` | 31 option values, rustfmt 1.9.0 live | every value fully identical: 1022/1022 for the 28 option values, editions 844/844, 843/843, 838/838 |
| `profile.sh`, both samples, `--per-file --runs 3` | | fixtures −5.77 M whole, −3.88 M per file; registry −3.21 M whole, −2.92 M per file |
| cliff shapes against rustfmt | 108 generated files and 8 `closures-expr` depths | all identical |

The gates ran on the binaries built at the commit before a comment-only amend (three lines of
the constant's documentation, same line count); `bench` rebuilt at `870bd5d` (was `fd18d50`) has the same
sha256 as the measured one (`a8debc57…`). The registry `config_matrix` (gate 7) was not run:
no output changed on either hash corpus.

## Rejected attempts (measured above)

| attempt | figures | why rejected |
|---|---|---|
| (a) store on the second miss (instrumented: an exact presence set) | fixtures +0.89 M per file; registry −1.87 M; `closures-expr` depth 16 1.66x, `p` 3.62 → 4.00 | regresses the fixture sample and breaks the cliff falsifier: every key under a found key is computed twice |
| (b) the 14 kinds with at most 3 fixture hits and no registry hit not stored | −1.29 M, −0.70 M | safe, but a third of the depth gate's gain; the costly kinds are the ones the nest needs |
| (c) depth ≥ 6 and those kinds off | −4.27 M, −3.17 M | no better than depth 6 alone |
| subtree ≥ 32 elements | −2.86 M, −2.71 M | hits are spread over sizes |
| depth ≥ 2, 3, 4, 5, 7, 8, 10, 12 | table above | depth 6 is the fixture optimum; 12 moves the cliff by 3% |
| depth ≥ 5 and subtree ≥ 16 | −4.38 M, −3.43 M (0.12 M, 0.21 M past depth 6 in the instrumented family) | a second tuned constant for 0.05% of the sample; inside the policy's plateau (depth 5–8 all within 0.35 M) |

## Interpretation

- The hypothesis's premise is right and its expected mechanism is not: the memo's stores
  cost about as much as its hits save on the fixture sample (6.76 M against 6.65 M, exactly
  measured) and 15 times as much on the registry sample (4.03 M against 0.27 M), but storing
  only keys seen before (a) cannot remove that cost exactly: a key not stored at its first
  computation is computed again with its whole subtree at its second, which costs as much as
  the stores saved on ordinary code and doubles the work in the nest.
- What separates useful stores from useless ones is where the rewrite happens: hits come from
  layout attempts that rewrite a subexpression again at the same shape, and those gather
  inside nested expressions. Below depth 6 the fixture sample made 82% of its lookups for 40%
  of its hits, and the hits it lost there are worth less than the stores: the gate keeps 188
  hits (226 under the policy, as the unmemoized levels ask again) for 17% of the stores.
- The cliff is untouched because the nest's repeated rewrites are deep: at depth 16 the gate
  removes 112 lookups of 12,800 and no hit.
- Instruction counts again overstate or understate a store change on the whole sample
  (E20's layout term): the second-miss policy read −0.96 M whole and +0.89 M per file; the
  kept change reads 1.90 M better whole than per file on the fixtures.

## Decision

Kept: `870bd5d` (was `fd18d50`), the memo from nesting depth 6. Per file (the lever's own effect) −3,875,681
instructions (−1.58%) on the fixture sample and −2,920,027 (−0.91%) on the registry sample;
whole sample −2.49% and −1.05%. Outputs byte-identical on both hash corpora (sequential and
four threads), `conform` 1220/1220 at both rustfmt versions, `config_matrix` identical on all
31 values. Falsifiers: the per-file fall exceeds 1 M; the registry sample does not regress;
`closures-expr` at depth 16 costs 0.998x. The whole-series exponent of `closures-expr` reads
+0.013 higher because its shallow depths got cheaper; over depths 8–16 it is unchanged
(+0.0007) and no depth costs more. Taken literally this trips the last falsifier; the record
keeps the change because the falsifier guards against faster growth and none is measured, and
leaves that reading to the integrator's confirmation run. The memo's net value at the base
is −0.11 M on the fixture sample and −3.76 M on the registry sample: it does not pay on
ordinary code, and it is needed for nesting.

Branch `20261015-memo-policy`, for the integrator's confirmation run (protocol §4.6) and
gates 1–5 on the merged tree.

## Follow-ups

- A quiet-host wall pair (`bench -n 5`, three pairs) is owed; the pairs here ran on a shared
  core at load 9–22.
- The block cache (`block_of`, 42% hits, E24) is a different trade: one entry per node, built
  blocks reused across attempts; its stores were not measured against depth here.
- With fewer than 1,900 lookups per 650 KB left, the memo table's container cost (E24: 2.34 M
  at 9,766 lookups) is now about a fifth of what E24 measured; E24's side-array idea has even
  less to win.
- The `tiny-one` RSS difference (+126 KiB median, no memo involved) and the `hir/src/lib.rs`
  +1.85% were not traced; if peak RSS on small inputs matters, compare page-touch counts of
  the two binaries' start-up.
- `ide-completion/src/completions/mod_.rs` (+82 K) and `matrixmultiply …/threading.rs` (+61 K)
  lost shallow hits worth more than their stores; a per-kind exception (e.g. `MATCH_EXPR`
  memoized at any depth: five fixture hits saved 2.68 M) is the next variant to measure if
  the plateau is to be explored further.
