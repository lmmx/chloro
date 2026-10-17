# 2026-10-15: rustd feasibility trial on chloro-core (bounded)

Part of [../2026-10-15-extensions-theory.md](../2026-10-15-extensions-theory.md). Labels:
measured, read, derived, hypothesis. No chloro code was changed; nothing of rustd was changed.

## Question

Does `rustd` (DioxusLabs/rustd), a build daemon that compiles functions from rust-analyzer's
MIR with Cranelift and links them ahead of rustc's rlibs, shorten chloro's edit-to-test loop
on chloro-core's own tests with rustc-identical behaviour, at an acceptable memory cost?
Decision rule (set before the run): recommend adoption only if the trial shows a
meaningful improvement of the actual edit-to-test loop with sufficient correctness coverage
and acceptable resource cost; otherwise record and move on. Stop conditions: the build fails,
essential functions are skipped, or the next step needs changes to rustd.

## What rustd is (read, revision `b9c283a`, 2026-10-15 01:14 UTC)

- A fork of rust-analyzer (default branch `rustd-spike`, 41,959 commits) plus one crate,
  `crates/rustd` (1,214 lines of `main.rs`), and a codegen module in `crates/hir-ty`
  (`RUSTD.md` §7). The daemon loads the workspace with `load-cargo`, lowers functions with
  `hir-ty`'s MIR, compiles them with Cranelift (`cranelift-object`), writes an object that is
  linked in front of rustc's rlibs with `--allow-multiple-definition`; `--watch` re-analyses
  on file change and relinks (`RUSTD.md` §1, §6; `harness/bench.py`).
- Scope stated by its plan: debug-quality output only; release builds and optimised code are
  not goals; diagnostics come from `cargo check` beside it; a function the backend has no
  support for is skipped and left to rustc, and every skip is counted as a bug (`RUSTD.md`
  §1–2). Open workstreams: calls and ABI (odd-sized aggregates, fn pointers, closures),
  trait objects (vtables), drop glue and unwinding ("RA's MIR has no cleanup edges at all"),
  intrinsics, statics, async (`RUSTD.md` §5, §8).
- Published results (`harness/SCOREBOARD.md`, `harness/BENCH.md`, generated): rung 0–1 only
  (corpus, toy, semver, itoa, hex, httparse, heck), 100% of functions compiled, behaviour
  matches rustc; edit loop on those tiny crates 30–101 ms against cargo's 86–405 ms, daemon
  RSS 526–887 MB, cold load 3.0–5.4 s. None of this is chloro.
- Toolchain pin: `harness/toolchain` = `nightly-2026-05-10` (rustc 1.97.0-nightly), required
  because rustd's layouts come from RA's vendored `ra-ap-rustc_abi` and everything linked
  must come from that rustc (`RUSTD.md` §6: a 1.99/0.166 mismatch picked different niche
  values and broke every `?` on a `Result` in semver).

## Build (measured)

| attempt | what | result |
|---|---|---|
| 1 | blob-less clone (`--filter=blob:none`) then `git checkout` | the lazy checkout fetched blobs one batch at a time through the proxy for 5 min without finishing; stopped |
| 2 | clone into the existing directory | refused (directory not empty); build then failed on the half-checked-out tree (`xtask/Cargo.toml` missing) |
| 3 | `git clone --depth 1 --single-branch -b rustd-spike` | 1 s, 5.2 MB pack; `cargo build --release -p rustd` with the pinned nightly: refused, `requires rustc 1.98` (workspace `rust-version = "1.98"`) |
| 4 | the same with `--ignore-rust-version` | `error[E0658]: use of unstable library feature substr_range` in `hir-def`: the code needs rustc 1.98 features the 1.97 nightly lacks; at `b9c283a` the harness pin and the code disagree |
| 5 | rustd itself built with stable 1.99.0 (`rustc 1.99.0 (b940084d7 2026-09-28)`); the harness keeps the nightly for the crate it tests and the rlibs it links | `Finished release in 3m 57s`, `target/release/rustd` 34,273,040 bytes, `target/` 1.3 GB |

The pin governs the toolchain whose rlibs are linked (the layout agreement), not the
compiler that builds rustd, so attempt 5 keeps the harness's semantics (read: `difftest.py`
sets `RUSTUP_TOOLCHAIN` from `harness/toolchain` for the cargo steps and `sysroot_rlibs()`;
`RUSTD_BIN` only names the daemon binary).

## chloro's own build loop, baseline (measured, 4 vCPU, other agents running, load 1–3)

After `touch chloro-core/src/formatter/expr.rs`, bash `time`, stable 1.97.0, incremental on:

| step | wall |
|---|---|
| `cargo build -p chloro-core` (debug) | 9.74 s |
| `cargo test -p chloro-core --lib --no-run` | 9.29 s |
| `cargo test --workspace --all-features --no-run` | 17.43 s |
| `cargo build --release -p chloro-core --example bench` (fat LTO, 1 cgu) | 34.4 s |
| `cargo build --release -p chloro --features cli` | 34.9 s |
| no-op `cargo build -p chloro-core` | 1.31 s |
| running the 183 chloro-core lib tests (debug) | 21.75 s |

chloro-core is 36,616 lines under `src/` (`wc -l`); features rustd's plan lists as open
appear in it: `Rc<` 12 sites, `RefCell` 11, `dyn ` 23, `impl Fn` 6, `move |` 8 closures,
`std::thread`/`thread::scope` 2+2, `panic!` 6, `unreachable!` 5, `resume_unwind` 1 (grep).
The release gates (fat LTO builds, 34–35 s each) are outside rustd's scope by its own plan.

## Trial (measured)

(filled from `trial-basic.log`, `rustd-invocations.log`, `basic.json`)

### `basic` test binary (4 tests; `cratetest.py --test basic`, 01:49 UTC)

| crate | functions rustd compiled | skipped | generic instances compiled | codegen | match |
|---|---|---|---|---|---|
| `chloro_core` | 3004 of 3004 root functions (264 more have no rustc symbol: inlined or generic, compiled as instances) | 0 | 15,212 | 11.4 s | yes |
| `basic` (the test crate) | 4 of 8 | 4: the `#[test]` functions themselves, `type_name_of_val` (used by `insta::assert_snapshot!`) is an unsupported intrinsic, so they stay with rustc | 50 | 0.2 s | yes |

rustd invocations (wrapper `rustd-wrap.sh`, VmHWM sampled every 0.2 s): `chloro_core` 33.5 s wall,
peak RSS 1,405,920 kB (1.34 GiB); `basic` 7.3 s, 956,644 kB. The 33.5 s is workspace load plus
codegen of 3004 functions and 15,212 instances; `--watch` mode pays it once per daemon start.
Result `MATCH: 1 test binaries, 4 tests` (stdout, stderr and exit status equal to rustc's build,
`--test-threads=1`). JSON: scratch `rustd-trial/basic.json`.

### The other test binaries (01:51–02:18 UTC)

| test binary | tests | `chloro_core` functions compiled by rustd | skipped | instances | codegen | rustd wall / peak RSS for `chloro_core` | match |
|---|---|---|---|---|---|---|---|
| `parallel_equivalence` (scoped threads, closures, 4-thread formatting of 113 fixtures) | 1 | 3004 of 3004 | 0 | 15,223 | 9.9 s | 29.0 s / 1.34 GiB | yes |
| `rustfmt_version` | 10 | 3004 of 3004 | 0 | 15,231 | 10.2 s | 30.1 s / 1.34 GiB | yes |
| the crate's own `#[cfg(test)]` tests (`--test unit`) | 183 | 3211 of 3286 | 75, all `type_name` (the `#[test]` functions that call `insta::assert_snapshot!`, which uses `type_name_of_val`; they stay with rustc) | 19,463 | 14.8 s | 39.4–41.9 s / 1.49 GiB | yes, on the rerun |

- The first `unit` run reported one differing test, `cst::ast::tests::generated_layer_is_up_to_date`:
  it failed under both binaries (exit 101 each) because `gen_ast.py` runs `rustfmt` through the
  rustup proxy and the pinned nightly had no `rustfmt` component; the harness compared the two
  panics and found rustd's panic location string `src/cst/ast.rs:729:9` against rustc's
  `chloro-core/src/cst/ast.rs:729:9`, plus one backtrace frame without a location. With
  `rustfmt` installed for the nightly, all 183 tests match (`MATCH: 1 test binaries, 183 tests`).
- So the one observable difference found: `Location::file()` in a panic message is the path
  relative to the crate directory under rustd and relative to the workspace root under rustc
  (hypothesis on the cause: rustd's `--remap`/working-directory handling; not traced). A test
  that asserts on a panic message's path would differ.
- No miscompile was found in 3,004 root functions and up to 19,463 instances of chloro-core,
  across 198 tests (4 + 1 + 10 + 183), including threads, closures, `Rc`/`RefCell`, trait
  objects and panics (the `unit` run exercises `#[should_panic]`-free panics inside tests that
  catch none; `resume_unwind` in `parallel.rs` is reached only on a worker panic, not exercised).

### Edit-to-binary latency (`harness/bench.py chloro --edits 5`, 02:19–02:21 UTC)

The bench entry (added to the rustd checkout's `harness/benches.json`, not to chloro): workspace
`/home/user/wt/rustd-trial/chloro-core`, target `--test basic`, crate `chloro_core`, the edit flips
`String::with_capacity(128)` to `129` in `src/formatter/types.rs` (behaviour-neutral, recompiles
the crate). rustd runs in `--watch` mode with the test binary's link line; cargo is timed on the
same edits under the harness's environment (`CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C link-dead-code`,
the pinned nightly). Medians over 5 edits, host load 3–4 (other builds running; the two columns
are from the same window):

| | rustd daemon | cargo, harness setting (`CARGO_INCREMENTAL=0`) | cargo, incremental on (`CARGO_INCREMENTAL=1`, same toolchain and flags, 3 edits, measured separately) |
|---|---|---|---|
| edit → runnable test binary | 1,400 ms (apply 5.5 ms; re-analysis and codegen 1,000 ms; link 287 ms; first edit 1,700 ms) | 6,779 ms | 2,482 / 2,578 / 2,917 ms |
| cold start | load 0.8 s + codegen 14.1 s = 14.9 s | — | — |
| memory | daemon RSS 1,287 MB after the edits | — | — |
| functions | 3004 of 3268 (the rest are `type_name` skips and test functions) | | |

rustd's published tiny-crate figures (30–101 ms edit-to-binary, 86–405 ms cargo) do not carry
over: on chloro-core the daemon's re-analysis and codegen take 1.0 s per edit and the link 0.29 s.
Against the loop a developer actually runs (incremental cargo), the gain is 1.8–2.1x on the
build step (2.5–2.9 s → 1.4 s), not the 4.8x the harness's non-incremental column shows. The
test run itself is untouched: the 183 lib tests take 21.8 s in debug (baseline table above).

## Verdict (no-go for adoption now; keep on the watch list)

Evidence for rustd: on chloro-core at revision `b9c283a` it compiled 100% of the root functions
reachable from four test binaries (3,004–3,211 functions, up to 19,463 generic instances) with
rustc-identical behaviour on all 198 tests, including scoped threads, closures, `Rc`/`RefCell`
and trait objects — far beyond its own scoreboard's rung 1. The edit-to-binary step is 1.4 s
against 2.5–2.9 s for incremental cargo.

Evidence against adopting it in this programme's loop:

- The programme's gates are release builds (fat LTO, one codegen unit, 34–35 s each; the
  profile-guided build on top) plus the `conform`, golden and `profile.sh` runs on them. rustd
  produces debug-quality code only, by design (`RUSTD.md` §1). None of the gate time moves.
- The inner test loop it does address saves about 1.1–1.5 s per edit on the build step, while
  the debug test run costs 22 s; the daemon holds 1.3 GB and takes 15 s to start.
- The revision is a day old and its toolchain pin disagrees with its own code (`nightly-2026-05-10`
  against `rust-version = "1.98"` and a 1.98 library feature); rustd itself had to be built with
  stable 1.99.0. The harness must keep the nightly for the crate under test, which means a second
  toolchain for every developer and `rustfmt` for it (the generator test needs it).
- One observable difference was found (panic location paths relative to the crate directory),
  and a function rustd skips falls back silently to rustc: a correctness regression in rustd would
  show up as a test behaviour difference, which only the differential harness detects. Running
  tests under rustd without the differential check is therefore not a substitute for cargo.
- Diagnostics are not rustd's (`cargo check` beside it); unwinding has no cleanup edges in RA's
  MIR (`RUSTD.md` §8), so a `#[should_panic]` or `catch_unwind` test is outside what was exercised.

What would change the verdict: a pinned, consistent toolchain (stable), rung 2–3 on rustd's own
ladder, and a per-edit re-analysis well under a second on a 36 kLoC crate; at that point the
inner loop `edit → cargo test --lib` could drop from about 12 s (2.5 s build + 9.3 s for a
filtered test run, or 24 s for the whole lib suite) by the build step only.

## Reproduce

```sh
# rustd at b9c283a (rustd-spike), built with stable 1.99.0; the harness pins nightly-2026-05-10
git clone --depth 1 --single-branch -b rustd-spike https://github.com/DioxusLabs/rustd /home/user/dioxuslabs/rustd
rustup toolchain install 1.99.0 --profile minimal
rustup toolchain install nightly-2026-05-10 --profile minimal -c rust-src -c rustfmt
cd /home/user/dioxuslabs/rustd && RUSTUP_TOOLCHAIN=1.99.0 cargo build --release -p rustd
# chloro at 383fa28 in a worktree; the harness builds it with the nightly
python3 harness/cratetest.py /home/user/wt/rustd-trial/chloro-core --test basic --json basic.json
python3 harness/cratetest.py /home/user/wt/rustd-trial/chloro-core --test unit --test parallel_equivalence --test rustfmt_version
python3 harness/bench.py chloro --edits 5     # after adding the `chloro` entry to harness/benches.json as above
```

Logs and JSON in this container's scratch directory `rustd-trial/` (`trial-basic.log`,
`trial-unit.log`, `trial-unit2.log`, `bench-chloro.log`, `rustd-invocations.log`, `basic.json`,
`unit.json`, `unit2.json`, `baseline.txt`).
