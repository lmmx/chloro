# EXP-027: harness and diagnostics: a path-independent count, per-file mode, the registry sample, the parse-only phase, shuffled relinks

Ledger row E27 in [../experiments.md](../experiments.md); procedure and gates in
[../protocol.md](../protocol.md), whose §3 and §4 this experiment amends. The brief is §4 of
[../../2026-10-15-external-research-brief.md](../../2026-10-15-external-research-brief.md)
(items 1, 4 and 6 of its §1: a second sample as a validation set, the fixed term, a parse-only
phase), T10 and §7 of [../../2026-10-15-unmined-tricks.md](../../2026-10-15-unmined-tricks.md)
(diagnostics that change what gets attempted), the heap-layout term of
[EXP-020](EXP-020-sink.md), the shuffled-relink follow-up of [EXP-022](EXP-022-layout.md) and
the per-file co-metric of EXP-019 (branch `20261015-cliffs`, its record and the scratch script
`tools/perfile.sh` it names). It is a harness experiment: no formatting code changes. Labels:
**measured** (a tool's output on the builds below), **read** (from source or documentation),
**derived** (arithmetic on measured figures).

## Identity

| field | value |
|---|---|
| branch | `20261015-harness` from `2f00988` (integration branch; its code is the BKG `9d78713`, the commit above it is documentation), worktree `/home/user/wt/harness` |
| commits | `1103485` (path-independent count), `3487091` (`--per-file`), `e615b5e` (the registry sample, `--sample-list`, protocol §3 and §4), `f0ce72e` (`chloro_core::parse_only`, `bench --phase parse`, `profile.sh --phase parse`), `96ce5de` (`--shuffle N`), then this file and the ledger row |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`; linker `rust-lld` through `cc -fuse-ld=lld` (rustc's default on this target); valgrind 3.22.0; `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)`; `cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples` (fat LTO, one codegen unit) |
| host | 4 vCPU Xeon @ 2.80GHz; other experiments built and ran throughout (load average 4.8–8.1). `Ir` is unaffected by load; the two wall-clock figures are **provisional** (protocol §6) |
| binaries | `bench` of `2f00988` (sha256 `59c7d602…`, the BKG's code; "base" below) and of `f0ce72e` (sha256 `8145d035…`, with `parse_only`; "this branch's binary") |
| `Ir` commands | `benchmarks/profile.sh --core 3 [--runs 3] [--per-file] [--sample-list benchmarks/samples/registry49.txt] [--phase parse] [--sim --shuffle 4]` |
| golden hashes | `scratchpad/golden/fixtures-e8d87c2.tsv` (1238 fixtures) and `registry-e8d87c2.tsv` (25,024 registry files present, 20,850 recorded, 4,174 reported as not recorded) |

## What each deliverable had to show

1. The same binary reads the same `Ir` from differently named sample directories, to within
   the run-to-run spread, and the count at the old default stays what it was (237,768,533
   for `9d78713`) or a new baseline is stated with its reason.
2. A per-file figure (one process per file, a fresh heap each) printed beside the whole
   sample's, with a stated way to read the two together.
3. A committed registry sample chosen by a stated deterministic rule, a flag that builds a
   sample from such a list, the head's figures on it, and the protocol rules.
4. A parse-only phase with an additive public function, its figures on both samples (whole
   and per-file) and the parser's share they imply, with no formatting path changed.
5. Time-boxed (60 minutes): the co-metric's mean and spread over N shuffled relinks.

## 1. The count, independent of where the sample lives (`1103485`)

### What moved it (measured)

Before this change the script ran `bench -n 1 --root <sample>` with the binary where it was
built and the sample at `--sample` (default `/tmp/chloro-bench/sample49`). The process holds
those strings, and only their **lengths** reach the count:

| string | measured effect (`9d78713`, one binary) | mechanism |
|---|---|---|
| the root path (and so every file path) | sample at a 6-character name 239,510,992, 8 (the default's) 237,768,585, 12 237,901,113; a relative root of 25 bytes 240,366,953, 26 bytes 237,770,406, 27 bytes 239,759,513 | each path is a heap allocation; the sizes (malloc's size classes) set the heap layout the parser's vector growth meets later (EXP-020's term), worth ±2.6 M |
| `argv[0]` | 46, 47 and 51 bytes: 237,770,241, 237,770,406, 237,770,154 (same class); 7 bytes with another root: 237.96 M | `std::env::args()` copies every argument to the heap; 41–56 bytes share malloc's 64-byte chunk |
| the binary's own path | the same binary at a 51-byte and a 115-byte path: 237,768,805 against 237,771,376 (+2,571), all of it in `pthread_getattr_np` (+2,683: `getline`, `sscanf`, `strtoul` over `/proc/self/maps`) | Rust's runtime asks glibc for the main thread's stack at start-up; glibc parses `/proc/self/maps`, whose lines carry the mapped binary's path: about 40 instructions per byte |
| an absolute against a relative root of the same 26 bytes | 237,768,668–743 against 237,769,748–909 | relative paths cost about 1.2K more to compare (`files.sort()`) and walk |
| the working directory, the text of the paths at equal lengths, `--out` | no effect measured (working directory at 6, 8 and 12 characters: 237,963,355–663 with the same strings; a new random stage name on every run of the table below) | the process does not read the working directory; only lengths set allocation sizes |

### The change

`profile.sh` copies the binary and the sample under `mktemp -d /tmp/chloro-prof.XXXXXXX` (24
bytes, removed at exit) and runs `<stage>/bench_…` (47 bytes) with `--root <stage>/s` (26
bytes), from the stage. The lengths are those of the strings the script used before at its
default (`/home/user/chloro/target/release/examples/bench`, the integration checkout, and
`/tmp/chloro-bench/sample49`), so the count at the old default is unchanged: 237,768,412,
237,768,605 and 237,768,430 (three runs of `9d78713`) against the ledger's 237,768,533.
`--sample` is accepted and ignored with a note, so existing commands keep working. The script
checks that `bench` reports every file of the sample.

### Verification (measured)

Three sample names (6, 8 and 12 characters) and, for the new script, binaries at 28, 51 and
115 bytes; three runs per cell, `--core 3`, the base binary:

| script | where the sample was | binary | `Ir`, three runs |
|---|---|---|---|
| before | `/tmp/chloro-bench/e27a01` | `target/release/examples/bench` | 239,510,992, 239,512,663, 239,511,218 |
| before | `/tmp/chloro-bench/e27a0001` | same | 237,768,585, 237,768,477, 237,768,564 |
| before | `/tmp/chloro-bench/e27a00000001` | same | 237,901,113, 237,899,350, 237,899,366 |
| after | `e27a01` (`--out`) | 28 bytes | 237,768,550, 237,768,641, 237,768,583 |
| after | same | 51 bytes | 237,768,641, 237,768,639, 237,768,647 |
| after | same | 115 bytes | 237,768,602, 237,768,687, 237,768,381 |
| after | `e27a0001` | 28 bytes | 237,768,425, 237,770,094, 237,768,444 |
| after | same | 51 bytes | 237,770,188, 237,768,315, 237,768,375 |
| after | same | 115 bytes | 237,768,504, 237,768,538, 237,768,516 |
| after | `e27a00000001` | 28 bytes | 237,768,385, 237,768,452, 237,768,417 |
| after | same | 51 bytes | 237,770,058, 237,768,452, 237,770,211 |
| after | same | 115 bytes | 237,768,475, 237,770,207, 237,768,523 |

The minima of the nine cells lie within 326 instructions (237,768,315–237,768,641). The
registry sample built from its list reads the same from the registry and from a copy of its
files elsewhere (`--sample-root`): 322,851,835 and 322,851,931 against 322,851,628 and
322,851,580.

### The run-to-run spread (measured)

EXP-022 put the spread at about 220. Over 53 runs of the base binary in this experiment, 43
lie in 237,768,233–237,768,687 and 10 near 237,770,1xx (+1.8K). A per-function difference of a
high run against a low one: `core::fmt::float::float_to_decimal_common_exact` +1,239,
`Big32x40::mul_pow2` +314, the per-file sort by time +118; total +1,950, all in `bench`'s own
report: it sorts the files by their measured times and prints the timings with
`{:.3}`/`{:.2}`, whose cost depends on the values, and for some values the printing takes
`core`'s exact big-number path (the high mode). No formatter function differed between the two
runs. The README says to take the minimum of
`--runs 3` when a difference under about 2K matters; `bench`'s report was not changed, since
that would move the count every record compares against (follow-up below).

## 2. Per-file mode (`3487091`)

EXP-019 formatted each sample file in its own `bench -n 1` process under the same callgrind
invocation and summed (`tools/perfile.sh` in the scratch directory: each file copied flat to
`/tmp/chloro-bench/1f-<binary name>/NN/<basename>`, the binary at a path of its build's name).
Its method is sound and adopted; two parts are changed: each file is staged alone at the same
26-byte root keeping its relative path, so its process holds the strings it holds in the whole
run (E19's paths followed the binary's name, which its errata name as a source of a wrong
figure), and an empty sample is counted, the fixed cost of a process (start-up, arguments, an
empty walk: 315,812–316,002), so both totals can be read net of it.

`profile.sh --per-file` prints each file's `Ir` (its minimum over `--runs`), the sum, the empty
sample, both net totals and the **cross-file term** `(W − E) − (S − n·E)`. The two net totals
count the same formatting work; they differ by what one file's run leaves the next in a shared
process: the heap layout the parser's vectors grow into (EXP-020), the allocator's warm free
lists, and the walk of directories shared by several files. A lever that changes the code's own
work moves `W` and `S` alike; a difference between the two changes is layout (EXP-019's first
two levers: +0.19% and +0.24% on `W`, −0.0007% and −0.30% on `S`).

The per-run sums differ by up to 8.6K (each of 49 processes can land in the report's high mode),
so the per-file figure is the sum of each file's minimum over three runs, which lies 15–28K
under any single run's sum.

| `9d78713`, base binary, three runs | fixture sample | registry sample |
|---|---|---|
| whole `W` (min; runs) | 237,768,390 (390, 459, 439) | 322,851,667 (667, 806, 792) |
| per-run sums | 253,357,534; 253,354,200; 253,359,842 | 336,138,490; 336,136,974; 336,145,538 |
| per-file `S` (sum of minima) | 253,338,727 | 336,117,569 |
| empty sample `E` | 316,002 | 315,812 |
| whole net `W − E` | 237,452,388 | 322,535,855 |
| per-file net `S − 49E` | 237,854,629 | 320,642,781 |
| cross-file term | −402,241 | +1,893,074 |

The term has either sign: on the fixture sample the shared process is 0.17% cheaper than fresh
heaps, on the registry sample 0.59% dearer. A per-file run takes 50 processes (1 min 51 s on a
loaded host against about 6 s for the whole sample).

## 3. The registry validation sample (`e615b5e`)

**The rule** (`benchmarks/samples/registry49.sh`, stated in the list's header): the `.rs` files
(`find -type f`) of the 577 crates of the registry corpus
([corpus/registry-crates.txt](../corpus/registry-crates.txt)) under
`index.crates.io-1949cf8c6b5b557f`, paths relative to the registry's `src` sorted bytewise:
20,593 files (the protocol's table says 20,570 for the same crates; how that figure was
counted is not recorded). N = 20,593 / 49 = 420; candidate k is line 420k (k = 1..49); a candidate that fails a
check is replaced by the next line that passes. A file passes when `rustfmt --edition 2024`
(stdin, empty configuration) exits 0, rustfmt's output is a fixed point of rustfmt, and chloro's
output (`examples/fmt_stdin`) is byte-identical to rustfmt's. Restricting the universe to the
corpus's crate list keeps the rule reproducible: the registry directory of this container holds
742 crates, and agents add more as they build.

**Rejects:** one, line 10,920, `rand-0.8.8/src/rngs/thread.rs` (rustfmt exits 1: `r.gen::<i32>()`,
`gen` is reserved in 2024); line 10,921 replaces it. No candidate failed the fixed-point or the
identity check.

**The list:** [`benchmarks/samples/registry49.txt`](../../../../benchmarks/samples/registry49.txt),
49 files, 731,733 bytes (the fixture sample: 650,213), from 44 crate versions (41 crates): generated bindings
(`windows-sys`, `web-sys`, `linux-raw-sys`, `winapi`), tests and examples, build scripts and
library modules. One file, `windows-sys-0.48.0/src/Windows/Win32/System/Rpc/mod.rs` (280,688
bytes), carries 119.4 M of the per-file sum (35.5%); the rule keeps it. Figures in the table
above. `profile.sh --sample-list FILE` builds a sample from any such list (`# root:
cargo-registry` or `# root: fixtures`, or `--sample-root DIR`).

**Protocol** (amended in this commit): §3 lists the registry sample and the per-file figure as
metrics and adds that every record from E23 on reports both samples and that a kept lever
regresses neither (a rise beyond the run-to-run spread on either sample rejects it); §4's merge
step becomes "confirmation run, then merge": the integrator re-measures from a clean build on
both samples, and a change under about 2 M instructions is reported with the per-file figure,
the per-file figure being the lever's own effect where the two disagree.

## 4. The parse-only phase (`f0ce72e`)

**API (read):** `chloro_core::parse_only(source: &str, config: &Config) -> Option<usize>` runs
what `format_source_with_config` runs before it formats anything (`Settings::new`, the
`disable_all_formatting` check, `normalize_src`, the length check, lexing, parsing, the flat
tree, `rustc_syntax`'s second parse where rust-analyzer rejects, `rejected_by_rustc`) by calling
the same crate-private `parse` through a new `parse_text` beside `format_text`, and returns the
tree's element count or `None` where formatting returns the source unformatted. It is a new
function; no formatting path calls it. A unit test checks it against `cst::Tree::parse`, a
`\r\n` and byte-order-mark source, a syntax error, `disable_all_formatting` and a
`#![rustfmt::skip]` file. `bench --phase parse` times it instead of `format_source` with the
same report (and refuses `--record`, `--check`, `--threads`); `profile.sh --phase parse` passes
it on, whole and per file.

**Figures (measured, this branch's binary, three runs, `--core 3`):**

| sample | phase | whole `Ir` (min; runs) | per-file sum | empty sample | whole net | per-file net | cross-file term |
|---|---|---|---|---|---|---|---|
| fixtures | format | 237,765,227 (766,682; 767,204; 765,227) | 253,335,086 | 315,845 | 237,449,382 | 237,858,681 | −409,299 |
| fixtures | parse | 86,529,639 (703; 724; 639) | 102,938,260 | 317,136 | 86,212,503 | 87,398,596 | −1,186,093 |
| registry | format | 322,841,609 (843,756; 841,609; 841,641) | 336,111,149 | 315,845 | 322,525,764 | 320,634,744 | +1,891,020 |
| registry | parse | 139,886,738 (747; 738; 802) | 155,798,944 | 317,136 | 139,569,602 | 140,259,280 | −689,678 |

**The parser's share (derived):** parse net over format net, 36.3% of the fixture sample
(36.7% per file) and 43.3% of the registry sample (43.7% per file; 43.1% without its largest
file). Per file it ranges from 30.7% (`hir-ty/src/infer/coerce.rs`) to 99.6% (a parser test file
with an error, which is not formatted) on the fixture sample, and from 18.6%
(`serde_json-1.0.154/tests/stream.rs`) to 77.0% (`libc-0.2.190/src/new/mod.rs`) on the registry
sample. The parse phase has a cross-file term of its own (−1.19 M and −0.69 M: the shared
process parses more cheaply than fresh heaps), consistent with the parser's vectors growing
into a warm heap; it was not attributed per function. T9's trigger for reconsidering a parser fork is
"the parser above 40% of the sample": the fixture sample reads 36%, the registry sample already
43% (generated bindings and declaration-heavy files parse much and format little).

**Wall (measured, provisional):** `taskset -c 3 bench -n 5`, three interleaved (format, parse)
pairs at load average 5.8–7.4: on the 1238 fixtures parse 28.84, 25.28, 27.12 MB/s against
format 8.78, 7.63, 7.10; on the registry sample's 49 files parse 26.40, 24.19, 28.57 against
12.64, 10.56, 9.53. Parsing takes 26–30% of the fixtures' formatting time in these pairs,
against 36% of the instructions on the fixture sample, which suggests the parser retires more
instructions per cycle than the formatter (provisional: a loaded host, and the 1238 fixtures
rather than the 49-file sample).

**The format phase moved by −3.2K and −10K (measured).** This branch's binary reads 237,765,227
(minimum of three) on the fixture sample against the base binary's 237,768,390, and 322,841,609
against 322,851,667 on the registry sample, with no formatter function's count changed. The
per-function difference: `__memcmp_avx2_movbe` −4,518 (fixtures) and −11,188 (registry), all of
it in calls from `<&str as Pattern>::is_contained_in` (`str::contains`), at the same 42,554
calls from the same callers on the fixture sample; `bench::main` −594; `Timespec::now` +1,666.
The new string literals moved `.rodata`, so the needles `str::contains` compares against sit at
other alignments, and the AVX2 `memcmp`'s cost depends on its operands' alignment; the other
two are `bench`'s own timing code. **New baseline for `9d78713`'s formatter with this branch's
`bench`: 237,765,019–237,767,204 on the fixture sample (eight runs, five of them in
237,765,019–237,765,296), 322,841,566–322,843,756 on the registry sample (four runs); parse phase
86,529,504–86,529,724 and 139,886,738–139,886,802**; the integrator re-records the BKG figure at the merge (protocol §4.6).

## 5. Shuffled relinks (`96ce5de`, within the 60-minute box)

**Mechanism (read, measured).** `cargo rustc --release -p chloro-core --example bench
--target-dir target/shuffle -- -C save-temps --print link-args` keeps the fat-LTO object
(`bench-<hash>.bench.<hash>-cgu.0.rcgu.o`, 6.5 MB) and the temporary directory the link line
names, and prints the line (`LC_ALL="C" PATH="…" VSLANG="1033" "cc" "-m64" <object>
<compiler_builtins rlib> … "-fuse-ld=lld" … "-o" <binary> …`). The save-temps build is
byte-identical to the ordinary release build, and so is the replayed line without changes;
with `-Wl,--shuffle-sections=.text*=<seed>` appended, lld places the functions (one section
each) in a random order. A fresh example is not linked again and prints nothing, so the
script removes the example's outputs before building: 31 s once the dependencies are built,
140 MB in `target/shuffle`. The script replays the line with Python's `shlex` (no shell
expansion), checks the unshuffled relink against cargo's binary, and runs each order once
under the simulator, staged at the same paths as every other run.

**Figures (measured, `f0ce72e`, `--core 3 --sim --shuffle 4`):**

| sample | order | `Ir` | `Bcm` | `Bim` | `I1mr` | `D1mr` | `D1mw` |
|---|---|---|---|---|---|---|---|
| fixtures | cargo | 237,765,263 | 2,984,285 | 685,184 | 4,710,936 | 655,491 | 505,829 |
| fixtures | seed 1 | 237,768,718 | 2,947,449 | 686,524 | 4,663,243 | 657,752 | 507,589 |
| fixtures | seed 2 | 237,768,661 | 2,998,946 | 681,392 | 4,784,528 | 658,223 | 507,302 |
| fixtures | seed 3 | 237,768,779 | 2,965,013 | 706,543 | 4,752,837 | 659,138 | 507,713 |
| fixtures | seed 4 | 237,768,640 | 2,952,597 | 712,936 | 4,762,416 | 657,898 | 507,600 |
| fixtures | mean of 4 (s.d.) | 237,768,700 (62) | 2,966,001 (23,167) | 696,849 (15,257) | 4,740,756 (53,352) | 658,253 (622) | 507,551 (175) |
| registry | cargo | 322,843,783 | 3,438,200 | 1,155,697 | 3,950,802 | 803,527 | 545,806 |
| registry | mean of 4 (s.d.) | 322,840,538 (51) | 3,432,636 (46,120) | 1,162,050 (14,925) | 4,062,006 (56,857) | 805,860 (348) | 546,359 (142) |

The cargo-order relink reads what the main `--sim` run of the same binary reads (`Bcm`
2,984,285 against 2,984,287, `I1mr` 4,710,936 against 4,710,936). Seeds 1 and 2 read the same
counts in a functional run on core 2 and in this one: an order is a function of the seed and
the object. The spread is EXP-022's (`I1mr` s.d. 51K, `Bcm` 11–22K on `8ebd3bb`); on the
registry sample `Bcm` varies twice as much (46K). Cargo's order is better than random for
`I1mr` on both samples (−0.6% and −2.7% against the means), as EXP-022 found on the fixture
sample (−2.6%). A full run took 2 min 4 s (fixtures) and 2 min 22 s (registry).

## What was not built

- Per-file mode under the simulator (`--per-file --sim`): per-file runs count `Ir` only.
- A deterministic report in `bench`: the high mode of the spread (§1) would go if `bench` printed
  its timings without float formatting (integer microseconds) and did not sort by time unless
  asked, at the cost of moving every recorded count by a few K once.
- The cliff driver's parse exponents (the brief's "with the cliff generator its own exponents"):
  `benchmarks/cliffs/` is on E19's branch, not in this base; `run.sh` can pass `--phase parse`
  to `bench` once both are merged.
- The items §7 of the unmined-tricks entry first listed under E27 (the PGO training mix, a
  `hotpath-rs` feature): the external brief's §4 replaced them with this harness work.
- A committed list for the fixture sample: the default construction computes it, and a list
  written from it (`# root: fixtures`) reads the same count.

## Gates (measured)

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean on both Rust trees: `2f00988`'s (commits 1–3) and `f0ce72e`'s (commits 4–6) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean on both |
| `cargo test --workspace --all-features` | 291 passed, 0 failed (`2f00988`'s tree); 292 passed, 0 failed (`f0ce72e`'s; one test added) |
| `bench --check fixtures-e8d87c2.tsv`, sequential and `--threads 4` | 0 of 1238 changed; 0 differ between 4 threads and `format_source` (base build and `f0ce72e`) |
| `bench --check registry-e8d87c2.tsv` | 0 of 25,024 changed, 4,174 not recorded (base build and `f0ce72e`) |
| `conform -i`, `conform -i -V 1.8.0` | 1220/1220 identical, 0 not idempotent, at both versions (base build and `f0ce72e`) |
| `benchmarks/profile.sh --core 3` | 237,768,412–237,768,605 at commits 1–3 (the base binary; the ledger's 237,768,533); 237,765,019–237,767,204 with `f0ce72e`'s binary (§4), the committed script at `96ce5de` included |

Commits 1, 2, 3, 5 and 6 change only `benchmarks/` and the journal: the Rust of commits 1–3 is
`2f00988`'s (`git diff 2f00988 e615b5e -- chloro-core chloro` is empty; its cargo gates were run
with those sources restored in the worktree) and that of commits 5 and 6 is `f0ce72e`'s.

## Follow-ups

- **The parser's share on the registry sample is above T9's 40% trigger** (43.3%); the
  trigger was written against the fixture sample (36.3%). Which sample the trigger reads is the
  integrator's decision; the parse phase now gives both.
- The integrator re-records the BKG's `Ir` at the merge of this branch: the format phase reads
  3.2K lower with this `bench` for the reason in §4, and from E23 on the registry sample's
  figure (322,841,566, per-file 336,111,149) joins it in the ledger.
- `bench`'s report as the source of the high mode (§1): a separate change with a stated
  one-time shift would make single runs exact.
- The registry sample's largest file is 35.5% of its count; a size-capped list could be added
  beside it (a new file under `benchmarks/samples/`, per the protocol) if a less skewed
  validation sample is wanted.
- `profile.sh` needs a writable `/tmp` for its fixed-length stage.
