# Cliff hunt: instructions against input size, by shape

Scaling fixtures for EXP-019
(`docs/journal/2026-10-14-autoresearch/experiments/EXP-019-cliffs.md`): for each input
shape, valid Rust files at scale factors 1, 2, 4, 8, the instructions chloro spends on each
(callgrind `Ir` of `examples/bench -n 1` on a directory holding that one file), and the
exponent `k` of a power-law fit `Ir = a · n^k`. A shape whose `k` exceeds 1.1 is a cliff:
some work grows faster than the input.

| file | does |
|---|---|
| `gen.py` | writes the fixtures (`generate`), lists the shapes (`list`), fits the exponents (`fit`); standard library only |
| `run.sh` | generates, checks every file against `rustfmt --edition 2024` (exit status and byte equality of chloro's output), runs callgrind on each, writes `ir.tsv` and prints the fit |

## Regenerate and fit

```sh
cargo build --release -p chloro-core --examples
benchmarks/cliffs/run.sh --core 3 --work /tmp/chloro-bench/cliffs          # 27 shapes x 4 scales
benchmarks/cliffs/run.sh --core 3 --shapes 'nested-macros chain' --scales 1,2,4,8,16
python3 benchmarks/cliffs/gen.py fit /tmp/chloro-bench/cliffs/ir.tsv     # the fit again
python3 benchmarks/cliffs/gen.py generate /tmp/cliffs-fixtures            # files only
```

`run.sh` writes `WORK/fixtures/<shape>/s<k>/<shape>.rs`, `WORK/compare.tsv` (rustfmt's exit
status and whether chloro's output is identical, per file; any line it prints is a
conformance finding), `WORK/ir.tsv` (shape, scale, input bytes, `Ir`) and
`WORK/cg/<shape>.s<k>.out` (the callgrind profiles, for `callgrind_annotate
--inclusive=yes`). The callgrind invocation is `benchmarks/profile.sh`'s (`env -i`,
`setarch -R`, `taskset` with `--core`), so counts are deterministic to a few thousand
instructions.

## The fit

`n` is the scale factor (the size variable is `base · scale`; `gen.py list` prints each
shape's base and what it counts). `run.sh` also counts an empty file; its `Ir` (the fixed
cost of `bench`: process start-up, reading the directory, a zero-byte format) is
subtracted before fitting, since a constant pulls the raw exponent towards 0.

- `k_raw`: least-squares slope of `log2 Ir` on `log2 n`;
- `k_net`: the same on `log2 (Ir − Ir_empty)` — the decisive figure;
- `k_bytes`: slope of `log2 (Ir − Ir_empty)` on `log2` input bytes;
- local slopes: `log2` of the ratio between successive scales, which show a quadratic term
  growing in before the fitted exponent does.

Depth shapes (nesting of closures, calls, blocks, macros, types, tuples, modules) repeat
their nested construct 8 times at every scale, so `n` is the depth alone. Their output grows
faster than their input, by the indentation of each nested line (O(depth²) bytes), and the
fit against output bytes is the one to read for them.

## Results

The first run (base `8ebd3bb` and head `f18dac3`) is in EXP-019: one shape above the 1.1
threshold remains, `closures-expr` (k 2.57: distinct shapes per nesting level, the memo cannot
merge them); `if-else` (a quadratic copy) and `use-list` (copies inside the sort comparator)
were fixed; the depth shapes whose exponent exceeds 1.1 against the scale stay below 1
against output bytes.
