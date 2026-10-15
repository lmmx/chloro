# EXP-024: the memo table as a dense side array over the flat tree

Ledger row E24 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md). The brief is trick T2 of
[../../2026-10-15-unmined-tricks.md](../../2026-10-15-unmined-tricks.md) §5, with the
initialisation-cost caution of
[../../2026-10-15-external-research-brief.md](../../2026-10-15-external-research-brief.md) §1.3
(report section A2) and the heap-layout term of [EXP-020](EXP-020-sink.md). Labels:
**measured** (a tool's output on the builds below), **read** (from source), **derived**
(arithmetic on measured figures).

## Identity

| field | value |
|---|---|
| branch | `20261015-memo-array` from `2f00988` (integration branch `claude/chloro-perf-research-swarm-nrscj8`), worktree `/home/user/wt/memo` |
| base BKG | `9d78713`; `2f00988` adds only documentation after it |
| kept commits | none in code; this file and the ledger row |
| rejected, reverted | the memo as a node-indexed side array in four forms (v1–v4), a two-word `MemoKey` hash (v5), the table sized per item (v6); the block cache's side array not attempted. Patches: `scratchpad/e24/memo-side-array-v4.patch` (v4 with its test), `scratchpad/e24/memo-hash-v5.patch` |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, valgrind 3.22.0, hyperfine 2.0.0; `cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples` (fat LTO, one codegen unit) |
| host | 4 vCPU Xeon @ 2.80GHz; other agents built and ran throughout (load average 2.4–8.9). `Ir` is unaffected; every wall-clock figure is **measured, provisional** (protocol §6) |
| `Ir` command | `benchmarks/profile.sh --core 3 --bench BIN [--sim]` at the default sample path (`/tmp/chloro-bench/sample49`). Comparisons are between copies of the binaries at paths of one length (`scratchpad/e24/bin/bench-<4 chars>`); the base reads 237,772,526 there and 237,770,023 as `target/release/examples/bench` of this worktree (the BKG's 237,768,533 was taken at `/home/user/chloro`) |
| registry sample | `/tmp/chloro-bench/regsmp49`, built below; the same `valgrind --tool=callgrind` command line as `profile.sh` (`scratchpad/e24/irreg.sh`). Base 392,102,315 (391,532,987 as `target/release/examples/bench`) |
| per-file figure | one callgrind process per file (`scratchpad/e24/pf.sh`), each file alone in `/tmp/chloro-bench/pfmemo/{fNN,rNN}`: the parser of a file then runs before any formatter allocation of that process, so the heap-layout term of EXP-020 cannot enter, and the per-file sums compare the formatter exactly |
| golden hashes | `scratchpad/golden/fixtures-e8d87c2.tsv` (1238 fixtures) and `registry-e8d87c2.tsv` (25,024 registry files present, one not recorded) |

### The registry sample (E27 will commit a canonical list)

The 25,024 `.rs` files under `/root/.cargo/registry/src` (one index directory,
`index.crates.io-1949cf8c6b5b557f`), relative paths sorted bytewise, every 510th (lines 510,
1020, …, 24,990; 510 is the largest N giving 49 files): 49 files, 974,418 bytes. List
sha256 `c2dc39fcc76f83a98360b83806eac5bef93a2886a0843f59f29d005b771e1d92` (paths relative to
`/root/.cargo/registry/src`, one per line, with the index directory).

```sh
cd /root/.cargo/registry/src && find . -name '*.rs' -type f | sed 's|^\./||' | LC_ALL=C sort | awk 'NR % 510 == 0' > regsmp49.txt
mkdir -p /tmp/chloro-bench/regsmp49 && xargs -a regsmp49.txt cp --parents -t /tmp/chloro-bench/regsmp49
```

```text
 1 base64-0.22.1/src/read/decoder.rs                    26 ra_ap_parser-0.0.307/test_data/parser/inline/ok/mod_item.rs
 2 borsh-1.5.7/tests/tests.rs                           27 ratatui-0.28.1/src/text/span.rs
 3 chrono-0.4.45/src/naive/datetime/mod.rs              28 regex-automata-0.4.18/src/nfa/thompson/backtrack.rs
 4 cranelift-codegen-0.136.2/src/incremental_cache.rs   29 rustix-0.38.44/src/backend/libc/io/windows_syscalls.rs
 5 csv-1.4.0/examples/cookbook-read-no-headers.rs       30 rustix-1.1.2/src/ioctl/bsd.rs
 6 defmt-1.1.1/src/impls/core_/slice.rs                 31 rustls-0.23.46/src/time_provider.rs
 7 diesel_derives-2.3.10/src/tests/sql_function.rs      32 sea-query-0.31.1/src/backend/mysql/query.rs
 8 facet-reflect-0.30.0/src/scalar.rs                   33 signal-hook-0.3.18/src/low_level/signal_details.rs
 9 futures-util-0.3.34/src/stream/try_stream/try_filter_map.rs  34 syn-2.0.117/src/classify.rs
10 hashlink-0.10.0/src/lib.rs                           35 tera-1.20.1/src/renderer/tests/whitespace.rs
11 image-0.25.10/src/codecs/hdr/encoder.rs              36 time-0.3.55/src/weekday.rs
12 itertools-0.15.0/src/tuple_impl.rs                   37 tokio-1.53.3/tests/_require_full.rs
13 libc-0.2.185/src/new/qurt/sys/sched.rs               38 toml_parser-1.1.5+spec-1.1.0/src/error.rs
14 libc-0.2.186/src/unix/linux_like/mod.rs              39 tracing-subscriber-0.3.23/benches/filter.rs
15 libm-0.2.16/src/math/ceil.rs                         40 wait-timeout-0.2.1/src/bin/exit.rs
16 linux-raw-sys-0.11.0/src/x86_64/netlink.rs           41 web-sys-0.3.106/src/features/gen_FileSystemGetFileOptions.rs
17 linux-raw-sys-0.4.15/src/csky/if_arp.rs              42 web-sys-0.3.106/src/features/gen_Navigator.rs
18 memchr-2.8.3/src/arch/x86_64/mod.rs                  43 web-sys-0.3.106/src/features/gen_SvgfeFuncRElement.rs
19 ndarray-0.16.1/benches/reserve.rs                    44 winapi-0.3.9/src/um/errhandlingapi.rs
20 nom-7.1.3/tests/css.rs                               45 windows-sys-0.48.0/src/Windows/Win32/System/Threading/mod.rs
21 os_pipe-1.2.3/src/windows.rs                         46 windows-sys-0.59.0/src/Windows/Win32/UI/Magnification/mod.rs
22 pin-project-1.1.13/tests/ui/pin_project/packed.rs    47 windows-sys-0.61.2/src/Windows/mod.rs
23 polars-arrow-0.43.1/src/scalar/null.rs               48 winnow-1.0.4/src/lib.rs
24 proc-macro2-1.0.103/src/fallback.rs                  49 zlib-rs-0.6.8/src/deflate/longest_match.rs
25 r-efi-6.0.0/src/protocols/debugport.rs
```

## Hypothesis and falsification

`context.rs` memoizes rewrites in `RefCell<HashMap<MemoKey, MemoEntry, FxBuildHasher>>` and
built blocks in `RefCell<HashMap<(u32, u32), CachedBlock, FxBuildHasher>>`. Every node is a
`u32` index into a flat array, so a side array indexed by node, with the node's few
(shape → entry) pairs in a short list, answers a lookup with one bounds-checked load and no
hashing, probing or key comparison. Hypothesis (the brief): the memo's lookup and insertion
cost (hash, probe, key compare, rehash on growth) is 1–3% of the sample's instructions, and
the side array removes most of it exactly.

Falsified if `Ir` on the fixture sample does not fall by at least the measured lookup cost;
or small files (the initialisation cost) or the registry sample regress; or peak RSS rises
more than 5% on the largest files. Per step: reverted if `Ir` does not fall by at least half
the step's measured site cost.

## Commands

```text
S=.../scratchpad/e24
python3 -I $S/stats_patch.py /home/user/wt/memo; cp $S/memo_stats.rs chloro-core/examples/  # instrumentation, not committed
CARGO_TARGET_DIR=$S/target-stats cargo build --release -p chloro-core --example memo_stats
$S/bin/memo_stats DIR_OR_FILE... > stats-<set>.tsv; python3 -I $S/scripts/agg.py stats-<set>.tsv
python3 -I $S/stats_patch2.py ...                                # the same with lookups and hits by subtree size (memo_stats2)
$S/prof.sh BIN TAG                     # profile.sh plain and --sim on the fixture sample, irreg.sh plain and --sim on regsmp49
python3 -I $S/scripts/cg.py <callgrind.out> <tag>.pkl; python3 -I $S/scripts/diff.py base.pkl <tag>.pkl
python3 -I $S/scripts/allocsplit.py <tag>.pkl                 # allocator arcs: formatter / parser+lexer / cst
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only CARGO_TARGET_DIR=$S/target-dbg cargo build --release -p chloro-core --example bench
taskset -c 3 env -i setarch x86_64 -R valgrind --tool=callgrind --dump-instr=yes --callgrind-out-file=$S/cg-line-<tag>.out \
    $S/bin/bench-dbg-<tag> -n 1 --root /tmp/chloro-bench/memoxx49
python3 -I $S/scripts/frames.py $S/cg-line-<tag>.out $S/bin/bench-dbg-<tag> 'RewriteContext>::memoize' 2   # per source line and inlined frame
$S/pf.sh BIN TAG; python3 -I $S/scripts/pfcmp.py $S/pf-base.tsv $S/pf-TAG.tsv    # per-file figure
$S/cli.sh $S/bin/chloro-base $S/bin/chloro-<tag> <tag>     # hyperfine -N, pinned, small files -w 2 -r 15, large -w 1 -r 5, peak RSS
$S/pairs.sh $S/bin/bench-base $S/bin/bench-<tag> 3          # taskset -c 3 bench -n 5, order alternating
$S/gates.sh <tag>                                            # protocol gates 1-4
```

## Step 1: what the memo and the block cache do (measured)

Instrumentation: a temporary patch (`scratchpad/e24/stats_patch.py`) counts, per
`format_source` run, every lookup and insert of both tables, records each node's distinct keys
and the entries present at each lookup, folds them at every `clear_memo` and at the run's end,
and an example (`memo_stats`) prints one line per file. One `format_source` per file; a run is
a `RunState` (the file's, and one per macro-definition body formatted as a snippet). The memo
is cleared at every top-level item (`visit_item` at block indent 0), so "per node" is per
item, and nodes never recur across items.

### The memo

| quantity | fixture sample (49 files, 650,213 B) | registry sample (49 files, 974,418 B) | five largest fixtures (1,566,972 B) | all 1238 fixtures (13.9 MB) |
|---|---|---|---|---|
| tree elements (nodes and tokens) / nodes | 172,014 / 80,438 | 366,085 / 137,138 | 568,200 / 271,251 | 5,018,278 / 2,398,412 |
| lookups (`memoize` calls) | 9,766 | 5,893 | 20,618 | 308,617 |
| hits | 315 (3.2%) | 34 (0.6%) | 235 (1.1%) | 11,396 (3.7%) |
| inserts / not stored (a flag left changed) / inserts of a key already present | 9,451 / 0 / 0 | 5,859 / 0 / 0 | 20,383 / 0 / 0 | 297,221 / 0 / 0 |
| lookups on macro-argument fragment trees | 628 | 606 | 711 | 13,387 |
| distinct nodes memoized (of them on fragments) | 6,379 (556) | 4,462 (523) | 14,908 (649) | 182,115 (9,130) |
| occupancy: memoized nodes / tree nodes; / elements | 7.93%; 3.71% | 3.25%; 1.22% | 5.50%; 2.62% | 7.59%; 3.63% |
| distinct keys (shapes, tags, flags) per memoized node: max, mean | 29, 1.474 | 11, 1.313 | 21, 1.367 | 47, 1.629 |
| histogram 1 / 2 / 3 / 4 / 5–8 / 9+ | 4,543 / 1,349 / 193 / 191 / 77 / 26 | 3,394 / 914 / 62 / 68 / 17 / 7 | 10,556 / 3,847 / 212 / 193 / 80 / 20 | 114,173 / 48,098 / 7,795 / 7,716 / 3,418 / 915 |
| lookups per memoized node; hits per node | 1.53; 0.05 | 1.32; 0.01 | 1.38; 0.02 | 1.69; 0.06 |
| misses on a node with 0 / 1 / 2 / 3+ entries | 6,379 / 1,836 / 487 / 749 | 4,462 / 1,068 / 154 / 175 | 14,908 / 4,352 / 505 / 618 | 182,115 / 67,942 / 19,844 / 27,320 |
| hits: the entry's position from the node's newest, 0 / 1 / 2+ | 205 / 68 / 42 | 25 / 8 / 1 | 165 / 44 / 26 | 7,700 / 2,526 / 1,170 |
| clears (of them with entries) | 837 (631) | 4,697 (298) | 2,555 (1,869) | 13,033 (8,953) |
| live entries at a clear: max; sum of each file's maximum | 679; 4,026 | 379; 2,922 | 1,522; 3,739 | 5,750; 151,950 |
| table growths (`reserve_rehash`) | 187 | 166 | 51 | 4,650 |
| fragment trees with entries (their elements, largest) | 499 (3,899, 175) | 339 (6,487, 94) | 580 (7,319, 635) | 6,058 (106,162, 637) |

By the memoized node's subtree size (elements, the node included), fixture / registry sample:

| subtree | 2–7 | 8–15 | 16–31 | 32–63 | 64–127 | 128–255 | 256+ |
|---|---|---|---|---|---|---|---|
| lookups | 817 / 338 | 2,585 / 2,026 | 3,271 / 1,958 | 1,610 / 949 | 863 / 326 | 398 / 170 | 222 / 126 |
| hits | 46 / 3 | 92 / 15 | 106 / 13 | 54 / 3 | 15 / 0 | 0 / 0 | 2 / 0 |

The 315 hits on the fixture sample are of nodes holding 7,969 elements together (4.6% of the
sample's elements); on the registry sample 606 elements.

The five largest fixtures, one by one:

| file | bytes | elements | nodes | lookups | hits | inserts | memoized nodes | occupancy (of nodes) | keys per node max, mean | live max | clears | growths | blocks lookups / hits / inserts |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ide-db/src/generated/lints.rs` | 639,689 | 120,391 | 53,506 | 3,428 | 0 | 3,428 | 1,714 | 3.20% | 2, 2.00 | 1,522 | 10 | 10 | 0 / 0 / 0 |
| `syntax/src/ast/generated/nodes.rs` | 322,295 | 245,871 | 123,183 | 6,843 | 0 | 6,843 | 6,099 | 4.95% | 2, 1.12 | 226 | 1,958 | 8 | 2,757 / 154 / 2,603 |
| `hir/src/lib.rs` | 227,843 | 127,053 | 63,140 | 6,529 | 202 | 6,327 | 4,139 | 6.56% | 21, 1.53 | 1,081 | 227 | 10 | 1,436 / 586 / 850 |
| `ide/src/hover/tests.rs` | 210,028 | 25,537 | 11,371 | 1,692 | 6 | 1,686 | 1,513 | 13.31% | 6, 1.11 | 86 | 257 | 6 | 306 / 38 / 268 |
| `rust-analyzer/src/config.rs` | 167,117 | 49,348 | 20,051 | 2,126 | 27 | 2,099 | 1,443 | 7.20% | 12, 1.45 | 824 | 103 | 17 | 371 / 142 / 229 |

### The block cache

| quantity | fixture sample | registry sample | five largest | all fixtures |
|---|---|---|---|---|
| lookups / hits / inserts | 2,570 / 1,085 (42.2%) / 1,485 | 1,621 / 652 (40.2%) / 969 | 4,870 / 920 (18.9%) / 3,950 | 73,546 / 35,611 (48.4%) / 37,935 |
| entries per node (the key is the node) | 1 | 1 | 1 | 1 |
| occupancy: block entries / tree nodes | 1.85% | 0.71% | 1.46% | 1.58% |
| inserts on fragments | 0 | 6 | 2 | 67 |
| live entries at a clear, max | 97 | 43 | 139 | 374 |
| table growths | 100 | 92 | 23 | 2,569 |

### Where the instructions are, at the base (measured)

Fixture sample, base 237,772,526. Exclusive `Ir` per source line of `memoize` and of the
clear (inlined into `visit_item`), from the line-table build (237,773,527 `Ir`, the same code),
attributed to the outermost formatter line and the inlined frames under it (`frames.py`).

| site | `Ir` | what it is |
|---|---|---|
| `memoize`, exclusive (9,766 calls) | 3,057,812 | |
| — lookup, `self.run.memo.borrow().get(&key)` (context.rs:368) | 496,324 | FxHash of the key's eight fields ≈ 195 K (≈ 20 per lookup), SSE2 probe ≈ 150 K, `RefCell` borrow and release ≈ 70 K, key compare |
| — insert, `self.run.memo.borrow_mut().insert(..)` (393) | 977,404 | hash and probe again, the 104-byte element written (198,471), `RefCell` |
| — the stored clone, `result.clone()` (396) | 196,701 + `malloc` 604,225 + `memcpy` 293,500 | the copy of the result the table keeps |
| — `skipped_range[..].to_vec()`, flags (391–392) | 397,326 | |
| — the rest (flag save and restore, the replay of a hit, the closure call) | ≈ 1.0 M | |
| `reserve_rehash` of the memo table (187 calls), inclusive | 558,443 | moves every entry to a table twice the size |
| the clear (visitor.rs:429): exclusive, `memset` of the control bytes | 225,780 + 19,888 | walk of the occupied buckets, the drops |
| the clear's `free` of the entries' strings and vectors (both tables) | 977,480 (9,258 calls) | any container pays it |
| `drop_glue::<RunState>`, exclusive | 57,956 | both tables and the rest of the run |
| `block_of`, exclusive (2,570 calls) | 1,054,679 | lookup (159) 85,694; build and `to_cached` (162–163) 786,269, of which the insert's hash, probe and write ≈ 0.10 M |
| `reserve_rehash` of the block table (100 calls), inclusive | 66,593 | |

`callgrind_annotate --inclusive=yes` gives `memoize` 103,222,328 (43.4%) for its outer calls
and 256,794,653 for the recursive ones (108%: callgrind counts each recursion level again), so
its inclusive cost is the rewriting below it; the table's own functions are the
`reserve_rehash` rows above (`scratchpad/e24/incl-base.txt`, `excl-base.txt`; caller arcs from
`cg.py`).

Derived: the memo table's container cost — lookup, insert, growth, clear, the table's drop
— is ≈ 2.34 M (0.98% of `Ir`), at the bottom of the brief's 1–3%; of it the lookup proper
(hash, probe, compare, without the `RefCell`) is ≈ 0.43 M (0.18%). The memo machinery as a
whole (`memoize` exclusive, the stored clone's allocation and copy, growth, the clear and its
frees, the block table's 1,485 frees included) is ≈ 5.77 M (2.42%). The block table's
container cost is ≈ 0.25 M (0.11%).

What the measurements say for the design:

- Lookups are few: 9,766 on 650 KB (1.53 per memoized node), and 65% of them find the node
  without an entry. A hashed lookup of the 48-byte key costs ≈ 51 `Ir`, of which the hash is
  ≈ 20 and the `RefCell` ≈ 7; a node-indexed lookup still pays the `RefCell`, a bounds check,
  and on a node with entries a walk over them, compared field by field.
- Keys per node are few (mean 1.47, 71% of nodes have one), so a per-node list is short; a
  per-node×shape table is out of the question (shapes are four `usize`s).
- Occupancy is low (3.7% of elements on the fixtures, 1.2% on the registry), so a slot per
  element holding the entry inline (112 bytes) would be 19.3 MB over the fixture sample's
  trees for at most 679 live entries; the per-node structure has to be a 4-byte head per
  element and the entries elsewhere.
- The table is cleared at every top-level item (837 clears on the fixture sample, 4,697 on
  the registry sample), and live entries peak at 679; a dense array cleared per item, or
  per file, pays its length each time, which the generation stamp or a reset from the
  entries' own list avoids.
- Hits are rare (3.2%, 0.6% on the registry) and spread over all subtree sizes, so the memo's
  cost is mostly the storing of results that are never read again: the clone, its
  allocation, and its free at the clear (≈ 1.9 M together), not the hashing.

## Step 3: the memo as a side array (measured; rejected)

Each form keeps `MemoKey` and `MemoEntry` and the eviction exactly: entries for a key live in
the list of the key's node, newest first, so a lookup returns the newest entry equal to the key
(what the map holds after the same inserts); the clear drops every entry at the same points.
All forms: 0 of 1238 fixture outputs changed.

| form | design |
|---|---|
| v1 | per tree id, a head array of the tree's length (`vec![0u32; len]`, allocated at the tree's first entry); the entries in one `Vec<MemoSlot>` (key, link, entry; 112 bytes) |
| v2 | heads only for the *window*, the top-level item being formatted (`clear_memo(item)` sets it; the array keeps its length from item to item and grows while empty), every other node (fragments, nodes outside the item) in a small hashed map of heads; entries in chunks of 64 that never move |
| v3 | v2 with the walk comparing tag, flags and shape only (tree and index are the node's own along its list, `debug_assert`ed) and the slot fetched once; the clear by `drain` |
| v4 | v3 with the clear resetting heads by reference and dropping each chunk in place |

`Ir`, one callgrind run per build. "Parser arcs" is the allocator arcs from `ra_ap_*` (the
heap-layout term of EXP-020); the per-file figure is the exact comparison of the formatter.

| form | fixture sample `Ir` | change | parser arcs change | registry sample `Ir` | change | per-file, fixture | per-file, registry |
|---|---|---|---|---|---|---|---|
| base | 237,772,526 | | | 392,102,315 | | 253,376,517 | 406,809,342 |
| v1 | 239,968,341 | +2,195,815 | +1,615,588 | 392,698,930 | +596,615 | — | — |
| v2 | 239,371,468 | +1,598,942 | +1,936,721 | 391,570,078 | −532,237 | — | — |
| v3 | 239,401,376 | +1,628,850 | +1,936,721 | 391,526,838 | −575,477 | — | — |
| v4 | 239,304,690 | +1,532,164 | +1,936,721 | 391,474,663 | −627,652 | **−407,911 (−0.161%)** | **+39,283 (+0.010%)** |

Where v1's instructions went (`diff.py`, base → v1): the head arrays' zeroing `memset` +667,082
and `calloc` +39,637 (≈ 1 `Ir` per byte: glibc zeroes the 4 bytes per element of every tree
with `rep stosb`, which callgrind counts per byte), the entries' `Vec` growth +411,428 (the
same doubling as the table's, now copying 112-byte slots with `rep movsb`) against the
table's `reserve_rehash` −558,443; `memoize` exclusive −284,849. Net of the parser term v1 is
+0.58 M: dearer than the map.

Where v4's went (base → v4, fixture sample; v4's line-table build, 239,304,665 `Ir`):
`memoize` exclusive 3,057,812 → 2,812,592 (−245,220: the lookup line 496,324 → 431,266, 6.7
`Ir` per lookup; the insert line 977,404 → 780,355, the slot's 112 bytes written as the map's
element was), `reserve_rehash` −558,443 inclusive, the window heads' zeroing `memset`
+186,879, the chunks' `malloc` +67,106 (86 calls), the clear in `visit_item` +59,901, the
small map of other heads +12,036 inclusive: −477,741 together, and the per-file figure,
−407,911, adds ≈ +70 K of allocator bookkeeping (`unlink_chunk`, `_int_malloc`). Per file: 31
of 49 fixture files lower, the largest change −86,406 (of 34.5 M); on the registry sample 29
of 49 higher, up to +117,163 on `linux-raw-sys …/netlink.rs` (203 KB of generated
declarations: the window is sized for large items that memoize little).

Against the falsifiers:

| test | needed | v4 |
|---|---|---|
| the brief's: `Ir` down by at least the lookup cost on the fixture sample | −0.50 M (the lookup line; −0.43 M without the `RefCell`) | +1.53 M in total; −0.41 M per file |
| the step's: `Ir` down by at least half the site cost (≈ 2.34 M) | −1.17 M | −0.41 M per file (17% of the site) |
| registry sample not regressed | ≤ 0 | −0.63 M in total; +0.04 M per file |
| small files | not regressed | the 20 fixture files under 0.6 M `Ir` alone: −1,512 together (−1,552 to +1,367 each); the 6 registry ones +6,625 (+277 to +1,696 each, +0.25%) |
| peak RSS ≤ +5% on the largest files | | +0.0% to +1.85% on the five largest fixtures, +4.90% on `fx-hir-ty` (below) |

The ceiling (derived): with the head zeroing and the chunk allocations free, v4 would be
≈ −0.66 M, 28% of the site cost. The lookup itself saves 6.7 `Ir` of 51 (the hash and probe
are replaced by the window test, a bounds check and the walk; 35% of lookups walk a non-empty
list, 2.4 entries on average when they miss). The only large removable item is the
growth's element moves (0.56 M), which a map avoids as well when it does not have to grow
(v6 below).

### Co-metric (`--sim`), base → v4

| sample | `Bc` | `Bcm` | `Bi` | `Bim` | `Bcm + Bim` | `I1mr` | `D1mr` |
|---|---|---|---|---|---|---|---|
| fixtures | 39,153,186 → 41,305,678 | 2,934,478 → 2,868,148 (−2.26%) | 1,871,808 → 1,871,534 | 685,053 → 698,369 (+1.94%) | 3,619,531 → 3,566,517 (−1.46%) | 4,732,093 → 4,731,932 (0.00%) | 658,029 → 686,413 (+4.3%) |
| registry | 66,746,015 → 66,369,741 | 3,917,361 → 3,851,401 (−1.68%) | 3,322,607 → 3,322,858 | 1,345,382 → 1,377,902 (+2.42%) | 5,262,743 → 5,229,303 (−0.64%) | 4,566,150 → 4,564,550 (−0.04%) | 1,013,318 → 1,002,653 |

`Bc` rises by about as much as the parser's realloc copies (`memcpy` +1.90 M `Ir`), most
likely callgrind's count of `rep movsb` iterations as conditional branches (not checked);
53 K and 33 K fewer mispredicts are ≈ 1 M cycles at most (derived), below any wall
measurement here.

### Wall clock and peak RSS (measured, provisional)

`cli.sh`: `taskset -c 3 hyperfine -N -i --metrics time_wall_clock,memory_peak_resident`,
`chloro --no-cache --check` on formatted inputs (one thread under the pin), base and v4 CLIs,
two rounds with the order alternating; small workloads `-w 2 -r 15`, large `-w 1 -r 5`.
Window 12:04:44–12:05:02 UTC, load 2.50 4.83 5.46.

| workload | base wall min / mean, ms | v4 wall min / mean, ms | v4 / base, mean | base peak RSS, KiB | v4 peak RSS, KiB | RSS change |
|---|---|---|---|---|---|---|
| `tiny-one` (13 B) | 1.79 / 2.12 | 1.85 / 2.23 | 1.056 | 3,764 | 3,764 | 0.00% |
| `tiny-test-data` (287 files) | 14.52 / 18.05 | 14.66 / 16.56 | 0.917 | 4,580 | 4,580 | 0.00% |
| `lints.rs` | 40.15 / 67.75 | 73.28 / 74.91 | 1.106 | 10,176 | 10,364 | +1.85% |
| `nodes.rs` | 63.91 / 85.52 | 58.44 / 83.23 | 0.973 | 14,448 | 14,460 | +0.08% |
| `hir/src/lib.rs` | 42.35 / 48.52 | 41.95 / 44.40 | 0.915 | 9,148 | 9,148 | 0.00% |
| `hover/tests.rs` | 10.57 / 11.17 | 10.93 / 12.46 | 1.115 | 4,840 | 4,872 | +0.66% |
| `config.rs` | 17.54 / 20.46 | 17.93 / 18.60 | 0.909 | 5,448 | 5,508 | +1.10% |
| `fx-hir-ty` (122 files, 2.8 MB) | 334.44 / 429.14 | 330.57 / 420.30 | 0.979 | 8,248 | 8,652 | +4.90% |

`pairs.sh` (`taskset -c 3 bench -n 5`, 1238 fixtures), 12:05:02–12:06:12, load 2.39 4.69 5.41:
base 8.78, 6.98, 8.41 MB/s; v4 7.71, 9.19, 8.94; v4/base per pair 0.878, 1.317, 1.063;
ranges overlap. No wall claim.

## Alternatives inside the map (measured; rejected)

Step 1 points at two costs the side array was meant to remove but that a map can lose without
changing its layout; each was measured as its own form, on the base.

| form | change | fixture `Ir` (change) | registry `Ir` (change) | per-file, fixture | per-file, registry | site cost | decision |
|---|---|---|---|---|---|---|---|
| v5 | `MemoKey`'s `Hash` writes two words (tree and index; the shape's four fields, tag and flags folded by rotations) instead of eight; `Eq` unchanged | 238,134,736 (+362,210; parser arcs +240,806) | 391,405,139 (−697,176) | −35,185 | −44,735 | the hash, ≈ 0.39 M in lookups and inserts | rejected: an FxHash round is three instructions and the folding costs nearly as many; `memoize` exclusive −23,703 |
| v6 | `clear_memo(item)` reserves `item.subtree_len() / 32` entries in the emptied table, so that it grows while empty | 240,470,959 (+2,698,433; parser arcs +2,877,755) | 391,246,545 (−855,770) | −179,062 | +14,016 | the growth, 0.56 M | rejected: 68 of 187 growths avoided (`reserve_rehash` −144,464), below half its site |

v5 changes no allocation, yet the parser arcs move: the clear drops entries in bucket order,
which the hash decides, and the order of the frees shapes the free lists the next file's
parser allocates from. The per-file figure is the only exact comparison for any change to
this table.

## The block cache (not attempted)

Its container cost is ≈ 0.25 M (lookup 85,694 for 2,570 calls, ≈ 0.10 M in the insert,
`reserve_rehash` 66,593). One entry per node (no list walk) is the side array's best case,
but the memo's measured ratio (17% of the site per file; 28% at the ceiling) puts the gain at
0.04–0.07 M, below half its site (0.13 M), and a window array of its own would pay the same
zeroing (0.19 M on the fixture sample) as the memo's. Not attempted.

## Tests (read, measured)

On the v4 patch (not committed): `memo_answers_as_a_hash_map` (context.rs) runs 200,000
random inserts, lookups and clears (four trees, 30 indices, 48 keys per node, windows of
random tree, start and length) against a `HashMap<MemoKey, String>`; debug builds also keep the
hashed map inside `Memo` and assert at every lookup that both answer the same entry, which
`cargo test` reaches over every fixture through `parallel_equivalence` and the conformance
tests. Mutations checked: the walk without the shape comparison, and the clear without the
head reset; each fails the test.

## Gates (measured)

No code is kept: the branch's code is `2f00988`'s (`git diff 2f00988 HEAD -- chloro chloro-core`
is empty). The gates were run on the v4 form to show that its rejection is on performance
only.

Window 12:27–12:39 UTC, load 4.8–8.9 (`scratchpad/e24/gates.sh`, log `gates-v4.log`).

| gate | corpus | v4 (`memo-side-array-v4.patch`) |
|---|---|---|
| `cargo fmt --all --check` | | two expressions reformatted by `cargo fmt`; clean after it (the saved patch is the formatted one) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | | clean |
| `cargo test --workspace --all-features` | | 292 passed, 0 failed, 1 ignored (the base's 291 and `memo_answers_as_a_hash_map`); every lookup checked against the hashed reference in the debug build |
| `bench --check`, sequential and `--threads 4` | 1238 fixtures against `fixtures-e8d87c2.tsv` | 0 changed; 0 differ between 4 threads and `format_source` |
| `bench --check` | 25,024 registry files against `registry-e8d87c2.tsv` (one not recorded) | 0 changed |
| `conform -i`, `conform -i -V 1.8.0` | 1220 fixtures | 1220/1220 identical, 0 not idempotent, at both versions |
| `benchmarks/profile.sh --core 3 --bench target/release/examples/bench` | 49-file sample, default path | 239,303,852 against the base's 237,770,023 at the same path (+1,533,829) |

The forms v1–v3, v5 and v6 were checked against the fixture hashes only (0 of 1238 changed
each).

## Interpretation

- The hypothesis's premise holds at its low end: the map costs ≈ 0.98% of the fixture sample's
  instructions. Its conclusion does not: a node-indexed array removes 17% of that (−0.41 M,
  −0.16%) on the fixture sample and nothing on the registry sample (+0.04 M), exactly measured.
- A hashed lookup of this key is cheap (≈ 51 `Ir`, the hash ≈ 20); the side array's lookup
  still pays the `RefCell`, a bounds check and a walk over the node's other shapes, and saves
  6.7 per lookup over 9,766 lookups. The insert writes the same 100-odd bytes either way.
- Dense storage has an initialisation cost even in its windowed form: zeroing 4 bytes per
  element of the largest item seen (0.19 M on the fixture sample, more on generated registry
  files with large items that memoize little), and a whole-tree array per file costs more
  (0.71 M). The external report's caution (A2) is confirmed in instruction terms, though in
  wall terms `rep stosb` zeroing is cheap.
- The memo's real cost is elsewhere: of its ≈ 5.8 M (2.4%), ≈ 1.9 M is storing a copy of
  results that are read again 3.2% of the time (0.6% on the registry), and freeing it at the
  next item.
- Any change to this table's allocations or to its drop order moves the parser's vector
  growth on the next file by −0.7 to +2.9 M; only the per-file figure judges such a change.

## Decision

Rejected: the memo as a node-indexed side array (v1–v4), the two-word hash (v5), the table
sized per item (v6); the block cache's side array not attempted. No code kept. T2 closed at a
measured gain of 0.41 M (0.16%) on the fixture sample and none on the registry sample, against
a container cost of 2.34 M (0.98%), with a derived ceiling of 0.66 M (0.28%).

## Follow-ups

- The memo's cost is in storing results (clone, allocation, free: ≈ 1.9 M of ≈ 5.8 M on the
  fixture sample) for 315 hits that cover 7,969 elements of rewrite. Whether the memo pays for
  itself on ordinary code, and what a cheaper store would be (the result moved into the table
  and a clone returned only on a hit cannot work, as the caller needs it; a shared `Rc<str>`
  changes every caller), is a question for the cost of the rewrites a hit saves, which the
  instrumentation here does not measure. The memo exists for exponential nesting (E19's
  cliffs); any change must keep those bounded.
- The per-file harness (`scratchpad/e24/pf.sh`, 98 callgrind processes, ≈ 4 minutes on one
  core) resolves formatter changes of a few thousand instructions independently of the parser's
  heap layout; E27 could make it a `profile.sh` option beside the registry sample.
