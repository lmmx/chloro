# Experiment ledger

Append-only. One row per experiment; the procedure and gates are in
[protocol.md](protocol.md). Rows with more than a page of evidence link to a file under
`experiments/`.

## Best-known-good

| field | value |
|---|---|
| commit | `884f9c9` (master at the start of the programme) |
| `conform -i` | 1219/1220 identical, 0 not idempotent |
| `bench -n 5`, 1238 fixtures, 13.9 MB | 3.041 s, 4.57 MB/s (host: Xeon @ 2.80GHz, 4 vCPU, 2026-10-14) |
| golden hashes | `scratchpad/golden/fixtures-884f9c9.tsv` (outside the repository; re-record with `bench --record` at this commit) |
| `Ir` on the 49-file sample | 587,211,777 ([profile-baseline.md](profile-baseline.md)) |

## Ledger

| id | branch | hypothesis | metric, expected | result | gates | decision | commit | date |
|---|---|---|---|---|---|---|---|---|
| E1 | `20261014-parallel-items` | formatting a file's top-level items on several threads, with the gaps, reorder groups and skips kept sequential, cuts a large file's wall time by close to the core count with identical output ([EXP-001](experiments/EXP-001-parallel-items.md)) | process wall time on the five largest fixtures, ≥ 1.5x on 4 cores; identical output | 1.50x–1.88x (minima of 15 runs) on the five files; crates 1.24x–1.82x, of which 1.24x–1.49x from a largest-first file queue alone; single core within 1% on files, 1–6% on crates; CPU +4–21% per split file; 0 of 1238 fixtures and 0 of 21,466 registry files differ at 1–4 threads; `conform -i` 1219/1220, 0 not idempotent (host: Xeon @ 2.10GHz, not the BKG host) | 1–3 passed; 4 (`Ir`) not applicable (threads); 5 as `bench -n 5` per file; 6–7 not run (no output change) | kept; hypothesis revised: the gain is bounded by the parse and the longest item, about 2x on this corpus; reproduced by the orchestrator: 0 of 1238 fixture outputs and 0 of 20,847 registry outputs differ between 4 threads and `format_source`, 0 changed against the base golden, fmt/clippy/all tests clean on the merged tree; merged into the integration branch (wall-clock reproduction deferred to the quiet-host run) | `d60c1fa`, `f847a87` | 2026-10-14 |
| E2 | `20261014-cli-build` | LTO + 1 codegen unit, PGO, a CLI-only allocator and largest-first dynamic scheduling each move process-level time; measured one at a time then combined | `bench` throughput and hyperfine crate-level/single-file/tiny-file wall time; expected 5–25% from codegen, scheduling gain on skewed crates | running (sibling session) | | | | 2026-10-14 |
| E6 | `20261014-cli-cache` | a persistent cache of content digests known to be fixed points makes `--check`/`--write` on an unchanged tree I/O-bound | CLI only: hash file, look up `(binary, config, content)` key, skip the formatter on a hit | process wall time on a formatted tree, ≥10x | formatted fixtures (1238 files, 14.2 MB): warm 32.7 ± 2.2 ms vs 819.4 ± 65.3 ms no-cache (25.1x, 4 threads), 28.9 ms vs 2904.6 ms on one core (100.7x); raw fixtures unchanged within noise; +188 KB binary; [EXP-006](experiments/EXP-006-cli-cache.md) | kept; reproduced by the orchestrator under load 8.8: warm 46.6 ± 9.3 ms, no-cache 1517.8 ± 158.3 ms (32.5x), cold 2.08 s; fmt, clippy, all tests, release build 2,704,336 B; merged into the integration branch | kept | `6e893f4` | 2026-10-14 |
| E3 | `20261014-hot-paths` | the slow paths the profile names (`recover_comment_removed` slow path 2.5%, `span_ends_with_comma` 1.5%, `CharClasses::next` 4.9%, `filter_normal_code` 1.5%, `line_of` table 0.86%, `node_text` copies 1.0%, memo result clone 0.45%, `single_ident_path`/`rewrite_method_call` growth 0.85%, `combine_strs_with_missing_comments` strings) each shrink with identical output; measured one commit at a time | `Ir` on the sample; expected 5–10% in total | running (in-process agent) | | | | 2026-10-14 |
| E4 | `20261014-green-reads` | green-tree reads on the top-25 red-node creation paths (D, S and C classes; `stmts_of` first) and a per-block cache for `Block::from_block_expr` remove 2.4–5% of `Ir` with identical output | `Ir` and red-node count on the sample | running (sibling session) | | | | 2026-10-14 |
| E7 | `20261014-flat-cst` | a flat preorder syntax array with a generated typed accessor layer that keeps `ra_ap_syntax`'s `ast` names can replace rowan under the formatter; the prototype measures build cost, accessor cost and the API surface to port before anything is ported | parse-only build `Ir`, accessor microbench `Ir`, count of formatter call sites per rowan API | running (sibling session, feasibility only) | | | | 2026-10-14 |
