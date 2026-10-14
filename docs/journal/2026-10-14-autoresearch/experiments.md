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
| E1 | `20261014-parallel-items` | top-level items of one file formatted on several threads, stitched by the sequential visitor's gap handling, give identical output | wall time of the five largest fixtures and of `hir`/`hir-ty` crate-level runs; expected up to the core count on one large file, no `Ir` change | running (sibling session) | | | | 2026-10-14 |
| E2 | `20261014-cli-build` | LTO + 1 codegen unit, PGO, a CLI-only allocator and largest-first dynamic scheduling each move process-level time; measured one at a time then combined | `bench` throughput and hyperfine crate-level/single-file/tiny-file wall time; expected 5–25% from codegen, scheduling gain on skewed crates | running (sibling session) | | | | 2026-10-14 |
| E6 | `20261014-cli-cache` | a CLI cache keyed by content digest, effective config and formatter identity makes `--check`/`--write` on an unchanged tree I/O-bound with no output change | warm vs cold wall time on the 1238 fixtures; expected ≥10x on the hit path | running (sibling session) | | | | 2026-10-14 |
| E3 | `20261014-hot-paths` | the slow paths the profile names (`recover_comment_removed` slow path 2.5%, `span_ends_with_comma` 1.5%, `CharClasses::next` 4.9%, `filter_normal_code` 1.5%, `line_of` table 0.86%, `node_text` copies 1.0%, memo result clone 0.45%, `single_ident_path`/`rewrite_method_call` growth 0.85%, `combine_strs_with_missing_comments` strings) each shrink with identical output; measured one commit at a time | `Ir` on the sample; expected 5–10% in total | running (in-process agent) | | | | 2026-10-14 |
| E4 | `20261014-green-reads` | green-tree reads on the top-25 red-node creation paths (D, S and C classes; `stmts_of` first) and a per-block cache for `Block::from_block_expr` remove 2.4–5% of `Ir` with identical output | `Ir` and red-node count on the sample | running (sibling session) | | | | 2026-10-14 |
| E7 | `20261014-flat-cst` | a flat preorder syntax array with a generated typed accessor layer that keeps `ra_ap_syntax`'s `ast` names can replace rowan under the formatter; the prototype measures build cost, accessor cost and the API surface to port before anything is ported | parse-only build `Ir`, accessor microbench `Ir`, count of formatter call sites per rowan API | running (sibling session, feasibility only) | | | | 2026-10-14 |
