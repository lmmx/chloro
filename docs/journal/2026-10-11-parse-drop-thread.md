# 2026-10-11: The parse-drop thread (D1), G2 landed, snapshots tracked

Follows [2026-10-10-green-attributes-in-production.md](2026-10-10-green-attributes-in-production.md).
That entry left two things open:
- instruction counts rose by up to 2.7% when other processes loaded the machine, for no
  known reason;
- G2 and snapshot tracking were candidates, not decided.

This entry covers:
- **the cause of the load-dependent counts:** found;
- **D1:** a fix that also turns out to be the largest wall-clock gain of phase 2;
- **G2:** landed;
- **the conformance snapshots:** tracked.

Commits, each revertible on its own:
- `fc90e53` (D1): trees built without a `Parse`; chloro is single-threaded;
- `14e45a1`: the 581 `.rustfmt.rs` snapshots tracked; README documents regeneration;
- `f6c5be9` (G2): token presence from the green tree.

The cache and representation decisions stay deferred. The architecture decision stays open.

The host is the "Xeon @ 2.10GHz" of the previous entries. It restarted twice during this
work. Labels as before: **measured**, **read**, **hypothesis**.

## The experiment (measured)

Conditions:
- Runs of one binary (`9202b42`'s `bench`) alternate between quiet (Q) and loaded (L).
  - With CPU load: four `while :; do :; done` shells.
  - With rustfmt load: four shells looping `rustfmt` over a fixture directory, like
    `config_matrix`.
- valgrind 3.22.0, `valgrind --tool=callgrind`, under `setarch -R`. The process may run
  on all 4 vCPUs (affinity mask `f`). The metric is callgrind's total `I refs`.
- The script (`run.sh`) and the profile comparison (`fndiff.py`) are in the appendix.

| workload | load | Q runs | L runs |
|---|---|---|---|
| one 476-byte file | CPU | 1,531,933 · 1,533,390 · 1,531,890 | 1,569,415 · 1,570,201 · 1,531,890 |
| 49-file sample | CPU | 597,854,946 · 595,605,490 · 595,313,447 | 612,338,567 · 612,273,553 |
| 49-file sample | rustfmt | 604,372,976 · 595,313,465 | 610,934,386 · 607,231,428 |

Observations:
- The effect appears on a single small file as well as on the sample, and with either kind
  of load.
- Load raises the counts, but a quiet run is not reliably low: three quiet runs came in at
  597.9M, 598.9M and 604.4M.

### Where the extra instructions are

Comparing the quiet 595.3M run with the loaded 607.2M run, function by function (exclusive
instructions):

| function | Q | L | L − Q |
|---|---|---|---|
| glibc `_int_free` | 35.80 M | 39.23 M | +3.43 M |
| rowan `Arc<GreenNode…>::drop_slow` | 0.16 M | 2.53 M | +2.37 M |
| glibc `malloc_consolidate` | 2.25 M | 4.37 M | +2.12 M |
| glibc `free` | 16.41 M | 17.59 M | +1.18 M |
| glibc `unlink_chunk` | 1.34 M | 2.28 M | +0.94 M |
| glibc `memcpy` | 9.25 M | 10.02 M | +0.77 M |
| `mpmc::Receiver<GreenNode>::recv`, `Mutex::lock_contended`, `Thread::park` | 0 | 0.12 M | +0.12 M |
| **total** | 595.31 M | 607.23 M | **+11.92 M** |

- No formatter function changes.
- The difference is in freeing green trees, plus channel and lock functions that do not
  appear in the quiet profile at all.

### The cause (read, then measured)

`ra_ap_syntax-0.0.307/src/lib.rs:204-222`:

```rust
impl<T> Drop for Parse<T> {
    fn drop(&mut self) {
        let Some(green) = self.green.take() else { return };
        static PARSE_DROP_THREAD: OnceLock<mpsc::Sender<GreenNode>> = OnceLock::new();
        PARSE_DROP_THREAD
            .get_or_init(|| { /* spawn "ParseNodeDropper": receiver.iter().for_each(drop) */ })
            .send(green)
            .unwrap();
    }
}
```

chloro never called `SourceFile::parse`, but it built every tree with
`SyntaxTreeBuilder::finish()`, which returns a `Parse`. It took the root with
`.syntax_node()` and dropped the `Parse` at once.

There were three such sites:
- the file in `formatting::parse`;
- every macro-argument fragment in `MacroTokens::build`;
- every `?Trait` fragment in `MacroTokens`.

So every tree had two owners: chloro's root node, and the copy sent to the
`ParseNodeDropper` thread. Whichever let go last freed the whole tree.

Under valgrind, only one thread runs at a time, and which thread gets to run depends on
the OS scheduler. Separating the counts by thread (`--separate-threads=yes`):

| run | main thread | `ParseNodeDropper` |
|---|---|---|
| Q | 595,313,807 | never ran (no profile written) |
| Q | 597,347,219 | 1,539,212 |
| L | 611,265,545 | 986,163 |
| L | 610,917,986 | 1,055,269 |

- **When the dropper thread never runs,** the main thread only decrements reference counts,
  no tree is freed before exit, and the count is 595.3M.
- **When it runs,** it releases its copies, the main thread becomes the last owner, and the
  main thread frees the trees: about 16M more instructions.
- Concurrent load preempts the main thread, which lets the dropper thread run.
- The "quiet minimum" that every count since phase 2 began has used is therefore the
  formatter's work *without* freeing any tree. The "outliers" were runs in which some trees
  were freed.

`--fair-sched=yes`, which hands the thread lock out in FIFO order, lets the dropper thread
run every time:
- Q: 611,078,660 · 611,089,453 · 611,127,685
- L: 612,185,348 · 611,920,038 · 612,190,486

That narrows the spread from 16M to 1.1M (0.2%), but does not make counts exact.

## D1: trees without a `Parse` (measured; `fc90e53`)

`formatting::TreeBuilder` has the same methods as `SyntaxTreeBuilder`. It wraps rowan's
`GreenNodeBuilder` (a dependency chloro already had; not a fork), and its `finish` returns
`SyntaxNode::new_root(green)`. All three sites use it.

The tree is the same:
- `SyntaxTreeBuilder::finish` only wraps the green node in a `Parse`; its validation block
  is disabled (`if cfg!(debug_assertions) && false`);
- the unit test `tree_builder_builds_the_tree_syntax_tree_builder_builds` compares both
  builders' trees (debug dump and green equality).

No `Parse` is created anywhere in chloro's formatting path any more, so the
`ParseNodeDropper` thread is never started.

Validation of `fc90e53`:

| check | result |
|---|---|
| fmt, clippy, workspace tests | clean; 141 library tests |
| output hashes against `bcef0f9` | 0 of 1238, 0 of 5659 changed |
| `conform` | 1219/1220; `conform -i` 0 not idempotent |
| option matrix | identical to phase 1 on all 31 options |

Determinism. A scratch build (`eadb4a5` plus D1) ran six times, interleaved quiet and
loaded:
- 593,646,940 · 593,648,822 · 593,646,825 · 593,647,170 · 593,648,737 · 593,650,087;
- spread 3.3K (0.0006%), with one thread in every run.
- The committed build measured 593,571,297 and 593,571,240.

The count is higher than G1's 580.7M because it now includes freeing every tree. It is
lower than G1 with the tree freed on the main thread (about 596M under load), because the
channel sends and remote frees are gone.

System calls, one pass over the fixtures (`strace -f -c`):

| build | `futex` calls | of which failed (EAGAIN) |
|---|---|---|
| `eadb4a5` | 27,059 | 6,033 |
| D1 | 0 | 0 |

Wall clock, interleaved runs of the two builds:

| corpus | `eadb4a5` | D1 |
|---|---|---|
| fixtures, best of 5, three pairs | 5.17–5.27 MB/s | 5.56–5.73 MB/s (+6 to +10%) |
| registry, best of 2, two pairs | 6.89–6.92 MB/s | 8.37–8.56 MB/s (+21 to +24%) |

After the host restarted, the same D1 binary ran about 25% slower in absolute terms (4.1–4.3
MB/s on the fixtures), so the comparison was repeated in that window, against the scratch G1
build (G1 on, the same code as `eadb4a5`):

| corpus | G1 | D1 |
|---|---|---|
| fixtures, best of 5, two pairs | 3.57–3.69 MB/s | 4.10–4.26 MB/s (+15%) |
| registry, best of 2, one pair | 4.86 MB/s | 5.97 MB/s (+23%) |

Findings:
- **The load-dependent counts were a property of the program, not of the counter.** Two
  threads raced to free each tree, and valgrind's serialised scheduler made the race visible
  as instruction counts.
- **Removing the race is the largest wall-clock gain of phase 2.** On a real multi-core
  machine the dropper thread runs promptly. The cost is then:
  - a channel send per tree, including per macro fragment;
  - futex wake-ups;
  - frees taking glibc's arena locks from another thread.

  None of this shows in instruction counts. **Hypothesis**, not separately measured: the
  futex syscalls and the cross-thread frees (glibc arena locking, chunks returning through
  another thread's cache) account for the gain. The registry gains more, possibly because
  it parses more macro fragments per byte.
- **Counts before and after D1 measure different things.** Before D1, minimum counts
  exclude freeing trees; from `fc90e53` they include it. Comparisons stay within one side
  of that line:
  - 595.3M (`9202b42`) → 580.7M (G1) is a valid comparison;
  - 580.7M → 593.6M (D1) is not a regression.

New measurement practice:
- one run suffices, on any machine state: repeated runs agree to within 3.3K instructions
  (0.0006%), loaded or not, and across a host restart (593,571,297 and 593,569,512 for
  `fc90e53`);
- the minimum-of-three rule and the "idle only" rule of the previous entry are no longer
  needed for builds from `fc90e53` on;
- `--separate-threads=yes` confirms there is one thread.

## G2 landed (measured; `f6c5be9`)

Reviewed against the generated accessors (read):
- `ra_ap_syntax-0.0.307/src/ast.rs:173-175`: `support::token(parent, kind)` is
  `parent.children_with_tokens().filter_map(into_token).find(|t| t.kind() == kind)`.
- Its `.is_some()` is "some direct child token has this kind". That is what `has_token`
  computes, reading the same kinds from the green children.
- At each of the four sites only presence was used. The `RefExpr` match ignored
  `const_token` in every arm.
- `IdentPat::{ref,mut}_token`, `RefExpr::{raw,mut}_token`,
  `ClosureExpr::{async,gen}_token` and `LetStmt::super_token` are all plain
  `support::token` accessors (`ast/generated/nodes.rs`).

Regression test `has_token_matches_the_generated_token_accessors` compares `has_token`
with the generated accessor on every `IdentPat`, `RefExpr`, `ClosureExpr` and `LetStmt`
of a source that contains:
- `ref`, `mut` and `ref mut` bindings, including `ref n @ 1`;
- `&raw const`, `&raw mut`, `&mut` and `&`;
- `async`, `gen`, `async gen` and `move` closures;
- `super let`.

It asserts that each keyword occurs at least once.

Validation:

| check | result |
|---|---|
| fmt, clippy, workspace tests | clean; 142 library tests |
| output hashes against `bcef0f9` | 0 of 1238, 0 of 5659 changed |
| `conform`, `conform -i` | 1219/1220; 0 not idempotent |
| option matrix | identical to phase 1 on all 31 options |

Instructions on the sample, two runs each:

| build | instructions | red nodes | allocations |
|---|---|---|---|
| `fc90e53` (D1) | 593,569,512 (twice) | 305,788 | 602,027 |
| `f6c5be9` (G2) | 586,768,390 · 586,768,269 (−1.15%) | 284,789 (−20,999) | 581,028 (−20,999) |

- G2 saves 6.80M instructions, about 324 per red node removed. Each removed red node is
  again exactly one allocation.
- Measured before D1 against G1, G2 saved 6.45M (about 307 per node). The difference
  between the two baselines is the malloc state: after D1, the trees are freed during the
  run.
- Wall clock, interleaved with D1 in the slower window: G2 was ahead in all five pairs.
  - fixtures, best of 5: 4.03–4.23 MB/s for D1 against 4.16–4.28 MB/s for G2;
  - registry, best of 2: 5.54–5.57 MB/s for D1 against 5.78–5.89 MB/s for G2.

  The direction is consistent; the size, 1–6%, is not well resolved on this host.
- G2 is smaller than G1 per red node (G1: about 392, measured before D1). The previous
  entry's hypothesis for the gap still stands and is still not measured.

## Snapshots tracked (measured; `14e45a1`)

- `.gitignore` now ignores everything in `snapshots/ra/{parser,syntax}/` except
  `*.rustfmt.rs`. The 581 expected outputs `examples/conform.rs` reads are tracked (3.2 MB).
  The chloro outputs and diffs the batch tests write stay local.
- Before committing, the batch tests regenerated every snapshot with rustfmt 1.9.0
  (`cargo test -p chloro-core --test conformance batch`): 12 passed, and no tracked
  snapshot changed.
- A fresh checkout of `14e45a1` (`git archive`, built there): **1219/1220**, 0 not
  idempotent. The reproducibility gap is closed: a fresh checkout now reports the full
  corpus and its one known difference.
- README documents where the expected outputs live and how to regenerate them.
- README's link to the one differing case pointed at an untracked `.diff`, so it was dead
  on GitHub. It now points at the tracked fixture.
- The `just conf*` recipes read `.diff` files, so for the parser and syntax crates they
  only see what the batch tests have written locally.

## Current State

- `TreeBuilder` builds syntax trees without a `Parse`
  (chloro-core/src/formatter/formatting.rs:35-59), used by `parse`
  (chloro-core/src/formatter/formatting.rs:61-87) and the macro fragment builders
  (chloro-core/src/formatter/macro_args.rs:244, 278)
- Token presence at the four G2 sites uses `has_token` (chloro-core/src/formatter/patterns.rs:212-215,
  chloro-core/src/formatter/expr.rs:272-275, chloro-core/src/formatter/closures.rs:238-241,
  chloro-core/src/formatter/items.rs:136)
- The parser and syntax `.rustfmt.rs` snapshots are tracked (.gitignore:1-6)
- Callgrind counts are deterministic: one thread, freeing every tree

## Missing

- No direct measurement of which part of the D1 wall-clock gain comes from futex syscalls
  and which from cross-thread frees
- No re-ranking of red-node paths on the D1 baseline (the remaining `child::<T>` and
  `support::child` accessors)
- No cache for `Block::from_block_expr` — deferred

## Divergence

- Pre-D1 instruction counts in earlier entries exclude freeing syntax trees; counts from
  `fc90e53` on include it

## Appendix

<details>
<summary><code>run.sh</code>: interleaved quiet/loaded callgrind runs</summary>

```bash
#!/bin/bash
# Interleaved quiet/loaded callgrind runs of one binary. Usage: run.sh BIN ROOT TAG LOAD(cpu|rustfmt) N
S=$(dirname "$0"); BIN=$1; ROOT=$2; TAG=$3; LOAD=$4; N=$5; VG=${VG:-}
start_load() {
  PIDS=()
  for k in 1 2 3 4; do
    if [ "$LOAD" = cpu ]; then bash -c 'while :; do :; done' & else
      bash -c 'while :; do for f in /home/user/chloro/chloro-core/tests/conformance/fixtures/rust-analyzer/hir/src/*.rs; do rustfmt --edition 2024 < "$f" > /dev/null 2>&1; done; done' & fi
    PIDS+=($!)
  done
  sleep 2
}
stop_load() { kill "${PIDS[@]}" 2>/dev/null; pkill -f 'while :; do' 2>/dev/null; wait 2>/dev/null; sleep 2; }
for i in $(seq 1 $N); do
  for cond in Q L; do
    [ $cond = L ] && start_load
    out=$S/$TAG.$cond.$i.out
    ir=$(setarch $(uname -m) -R valgrind --tool=callgrind $VG --callgrind-out-file=$out $BIN -n 1 --root $ROOT 2>&1 | grep refs | awk '{gsub(",","",$NF); print $NF}' | tr '\n' ' ')
    la=$(cut -d' ' -f1 /proc/loadavg)
    [ $cond = L ] && stop_load
    echo "$TAG $LOAD $cond $i $ir load1=$la"
  done
done
```

</details>

<details>
<summary><code>fndiff.py</code>: exclusive instructions per function, two profiles</summary>

```python
# Exclusive instructions per function in two callgrind profiles (via callgrind_annotate), and
# the functions whose counts differ most. Usage: fndiff.py A.out B.out [N]
import subprocess, sys, re, collections
def excl(path):
    out = subprocess.run(['callgrind_annotate', '--threshold=100', '--inclusive=no', path],
                         capture_output=True, text=True).stdout
    d = collections.Counter()
    for line in out.splitlines():
        m = re.match(r'^\s*([\d,]+)\s+\([^)]*\)\s+(.*)$', line)
        if m and 'PROGRAM TOTALS' not in line:
            d[re.sub(r"'\d+", '', m.group(2).strip())] += int(m.group(1).replace(',', ''))
    return d
a, b = excl(sys.argv[1]), excl(sys.argv[2])
n = int(sys.argv[3]) if len(sys.argv) > 3 else 25
ta, tb = sum(a.values()), sum(b.values())
print(f'A {ta:,}  B {tb:,}  B-A {tb-ta:+,}')
for k in sorted(set(a) | set(b), key=lambda k: -abs(b[k] - a[k]))[:n]:
    print(f'{b[k]-a[k]:+14,}  A {a[k]:>13,}  B {b[k]:>13,}  {k[:150]}')
```

</details>
