# EXP-013: profile-guided release binaries in the release workflow

Part of [../../2026-10-14-autoresearch.md](../../2026-10-14-autoresearch.md), run under
[../protocol.md](../protocol.md). E2 kept `benchmarks/pgo.sh` as a script and proposed a
workflow step ([EXP-002](EXP-002-cli-build.md), "Proposed `binaries.yml` change"); M3
measured the profile-guided build at 0.78–0.92 of the plain build's wall time per crate on
one core and −21.7% instructions ([../measurements/m3-quiet-host.md](../measurements/m3-quiet-host.md)).
This experiment puts the step into `.github/workflows/binaries.yml` and validates it by
running its commands locally. Labels: **measured** (run here), **read** (from source or
documentation), **not observed** (only a release run can show it).

## Identity

| field | value |
|---|---|
| branch | `20261015-pgo-ci` from `a168b72` (was `400ed53`) (integration branch `claude/chloro-perf-research-swarm-nrscj8`), worktree `/home/user/wt/pgo-ci` |
| base BKG | `397159b` (was `92fdef0`) (`a168b72` (was `400ed53`) adds only documentation after it) |
| commits | `8e5de88` (was `c2a2739`) (workflow step, `pgo.sh` phases and `--target`), the following commit (this file, ledger row, the two READMEs) |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, LLVM 22.1.6, host `x86_64-unknown-linux-gnu`; `llvm-tools-preview` installed by the step's own `rustup component add` (6.4 s here) |
| host | 4 vCPU "Intel(R) Xeon(R) Processor @ 2.80GHz", Linux 6.18.44, shared with other builds: load 2.3–4.3 during the builds, 3.2–3.4 during the timings. Wall times are **measured, provisional** (protocol § 6); only within-window ratios of interleaved runs are used |
| tools | hyperfine 2.0.0, valgrind 3.22.0; `actionlint` is not installed on this host |
| actions read | `taiki-e/upload-rust-binary-action@v1` at `f0d45ae` (2026-04-17), `taiki-e/setup-cross-toolchain-action@v1` at `12b7ad4` (2026-07-19), shallow clones read, not run |

## Hypothesis and falsification

The published binaries can be built profile-guided inside the existing workflow for the
matrix targets whose runner executes the instrumented binary natively, with the same output
as the plain build and M3's gain; cross-compiled targets keep the plain build.

Falsified if the locally run step does not yield a profile the action's build uses (LLVM's
missing-function warnings for the CLI's own functions), if any gate changes (a fixture or
registry output hash, `conform -i` below 1220/1220 or any file not idempotent), or if the
instruction count and the wall-time ratio do not reproduce M3's (about 244.7 M against
312.7 M; 0.78–0.92 per crate on one core).

## What the workflow does (read)

- **Matrix.** Five active targets. `x86_64-unknown-linux-gnu` on `ubuntu-22.04` (x86-64),
  `x86_64-apple-darwin` on `macos-13` (an Intel image), `x86_64-pc-windows-msvc` on
  `windows-2022` (x86-64): native. `aarch64-apple-darwin` on `macos-13` and
  `aarch64-pc-windows-msvc` on `windows-2022`: cross-compiled; an x86-64 machine cannot run
  the instrumented arm64 binary, so they cannot be trained there. The musl, Linux aarch64
  and FreeBSD rows are commented out.
- **Environment.** `RUSTFLAGS: -D warnings` and `FEATURES: cli` at workflow level; a step
  appends `-C target-feature=+crt-static` to `RUSTFLAGS` through `GITHUB_ENV` on the two
  `windows-msvc` targets.
- **`setup-cross-toolchain-action`** runs on Ubuntu only. For the host target it installs
  nothing that matters here, but always writes `CARGO_BUILD_TARGET=<target>` to
  `GITHUB_ENV` (its `main.sh`, last line), so every later `cargo build` on Ubuntu builds
  with `--target`.
- **`upload-rust-binary-action`** picks `cargo` as its build tool when the target is the
  host or both are Apple or both Windows, `rustup target add`s the target, sets
  `CARGO_PROFILE_RELEASE_STRIP=debuginfo` (not on MSVC) when the manifest has no `strip`,
  and runs `cargo build --release --bin chloro --features cli --target <target>` in the
  workspace with the job's environment, so `RUSTFLAGS` from `GITHUB_ENV` reaches it. It
  passes `--target` explicitly whatever the runner.

## The change

`benchmarks/pgo.sh` gains two options and keeps its default behaviour:

- `--phase all|train|use`: `train` stops after `llvm-profdata merge` and leaves
  `<root>/pgo-gen/profiles/merged.profdata`; `use` runs only the two `-Cprofile-use` builds;
  `all` (the default) is the old script.
- `--target T` (default `$CARGO_BUILD_TARGET`): every `cargo build` gets `--target T` and the
  binaries are looked up under `<dir>/T/release/`. Training refuses a `T` other than the host
  (exit 3).
- Under Git Bash (Windows runners) the paths inside `-Cprofile-generate`/`-Cprofile-use`
  go through `cygpath -m`, since rustc on Windows does not read `/d/a/...`; executables get
  `.exe`. Empty-array expansions are written so that bash 3.2 accepts them under `set -u`.

`binaries.yml` gains one step, "Train PGO profile", after the `+crt-static` step and before
the upload action, under the same `if:` as the upload (a release whose tag starts with
`chloro`, or `workflow_dispatch`):

```bash
host=$(rustc -vV | sed -n 's/^host: //p')
if [[ "${host}" != "${TARGET}" ]]; then
  echo "PGO skipped: ${TARGET} is cross-compiled on ${host}; plain build"
  exit 0
fi
profile="${PWD}/target/pgo-gen/profiles/merged.profdata"
if rustup component add llvm-tools-preview \
  && benchmarks/pgo.sh --phase train --target "${TARGET}" target "${FEATURES}" \
  && [[ -s "${profile}" ]]; then
  if type -P cygpath >/dev/null; then profile=$(cygpath -m "${profile}"); fi
  echo "RUSTFLAGS=${RUSTFLAGS} -Cprofile-use=${profile} -Cllvm-args=-pgo-warn-missing-function" >> "${GITHUB_ENV}"
else
  echo "::warning::PGO training failed for ${TARGET}; the release binary is the plain build"
fi
```

Why this shape:

- **The action keeps building.** It already honours `RUSTFLAGS`, names the archive and
  uploads; replacing its build would mean re-implementing the archive and checksum logic.
  Only the train phase runs in the step, so the release binary is built once, by the
  action, and the script's own optimised build (about a minute) is not repeated.
- **Native targets by the runner's own answer.** The step compares `rustc -vV`'s host with
  the matrix target instead of listing targets, so a runner image that changes architecture
  makes the step skip rather than fail.
- **`--target` in training.** The action builds with `--target`; a profile is keyed by
  symbol names, and those differ with and without `--target` (measured below: 1017
  functions of `chloro` and `chloro_core` left without data). On Ubuntu
  `CARGO_BUILD_TARGET` would add it anyway; on macOS and Windows only the explicit option
  does.
- **Fallback.** Every command that can fail sits in the `if` condition, which `bash -e`
  does not abort on; on failure `RUSTFLAGS` is not touched and the job's annotation names
  the target. `pgo.sh` deletes the profile directory before training, so a stale profile
  cannot pass the `-s` test.
- **`+crt-static` kept.** The step reads `RUSTFLAGS` after the Windows step has appended to
  it, and `pgo.sh` appends that `RUSTFLAGS` to both its flag sets, so the instrumented and
  the final Windows builds both link the static CRT.
- **`-Cllvm-args=-pgo-warn-missing-function` turns the warning on** (LLVM's default is
  off): it makes a profile that does not match the code visible in the action's log. It
  does not fail a `-D warnings` build (measured: a two-function program built with another
  program's profile and `-D warnings` prints five such warnings and exits 0).
- **Nothing new is pinned.** `llvm-tools-preview` comes with whatever `stable` the existing
  `dtolnay/rust-toolchain@stable` step installed; the profile is regenerated on every run.

## Local validation (measured)

The step's script was extracted from the YAML (`yaml.safe_load(...)['steps'][5]['run']`) and
run from a clean worktree (no `target/`) with the job's environment: `RUSTFLAGS='-D
warnings' FEATURES=cli CARGO_INCREMENTAL=0 CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu
TARGET=x86_64-unknown-linux-gnu GITHUB_ENV=<file>`, under `bash --noprofile --norc -eo
pipefail` as Actions runs it. Then the action's build was run by hand with the `RUSTFLAGS`
line the step wrote:

```text
$ bash --noprofile --norc -eo pipefail train-step.sh
info: component llvm-tools is up to date
== instrumented build
    Finished `release` profile [optimized] target(s) in 52.43s
    Finished `release` profile [optimized] target(s) in 39.94s
== training
LLVM Profile Warning: Unable to track new values: Running out of static counters. [...] (10 times, as in E2)
== merging 2 profiles
profile /home/user/wt/pgo-ci/target/pgo-gen/profiles/merged.profdata (1003696 bytes)
STEP exit 0 wall 97.0 s
$ cat $GITHUB_ENV
RUSTFLAGS=-D warnings -Cprofile-use=/home/user/wt/pgo-ci/target/pgo-gen/profiles/merged.profdata -Cllvm-args=-pgo-warn-missing-function

$ CARGO_PROFILE_RELEASE_STRIP=debuginfo RUSTFLAGS=<that line> \
    cargo build --release --bin chloro --features cli --target x86_64-unknown-linux-gnu
    Finished `release` profile [optimized] target(s) in 1m 09s          (0 missing-function warnings)
$ ... cargo build --release --target x86_64-unknown-linux-gnu -p chloro-core --example bench --example conform
    Finished `release` profile [optimized] target(s) in 47.71s          (bench: 0; conform: 71, all in conform's own code, which training never runs)
```

The plain build for comparison is the same two commands with `RUSTFLAGS='-D warnings'` and
`--target-dir target/plain` (42.8 s and 38.5 s).

| binary | bytes | sha256 (prefix) |
|---|---|---|
| PGO `chloro` (`target/x86_64-unknown-linux-gnu/release/chloro`) | 2,689,904 | `d728d8fece462390` |
| PGO `bench` | 2,453,744 | `55a852be7ae78fdb` |
| PGO `conform` | 2,429,288 | `7a1ca4ec37ce213b` |
| plain `chloro` (`target/plain/x86_64-unknown-linux-gnu/release/chloro`) | 2,043,352 | `14be3b1922e16c49` |
| plain `bench` | 1,805,104 | `61baa355436d318a` |

Both branches of the guard, with the same extracted script:

| case | output | exit | `GITHUB_ENV` lines |
|---|---|---|---|
| `TARGET=aarch64-pc-windows-msvc` on this x86-64 host | `PGO skipped: aarch64-pc-windows-msvc is cross-compiled on x86_64-unknown-linux-gnu; plain build` | 0 | 0 |
| training forced to fail (`FEATURES=no-such-feature`) | cargo's error, then `::warning::PGO training failed for x86_64-unknown-linux-gnu; the release binary is the plain build` | 0 | 0 |

**Profile matching with and without `--target`.** `benchmarks/pgo.sh` with no options (its
local mode, no `CARGO_BUILD_TARGET`) ran in 215 s and wrote 13 raw profiles, merged to
4,066,112 bytes: without `--target`, `RUSTFLAGS` also instruments build scripts and proc
macros, whose runs during the build write the 11 extra files. Its own `-Cprofile-use` build
gave 0 missing-function warnings and 245,255,441 `Ir` for its `bench`. The action-shaped
build (`--target x86_64-unknown-linux-gnu`) with that profile gave **1017** warnings (255
in `chloro`, 762 in `chloro_core`): the crate hashes in the mangled names differ, so that
build would have been close to unprofiled. With the step's `--target` training: 0.

## Gates (measured, on the PGO build)

| gate | command | result |
|---|---|---|
| fixture hashes | `bench -n 1 --check scratchpad/golden/fixtures-e8d87c2.tsv --root /home/user/chloro/chloro-core/tests/conformance/fixtures` | 0 of 1238 outputs changed |
| registry hashes | `bench -n 1 --check scratchpad/golden/registry-e8d87c2.tsv --root /root/.cargo/registry/src` | 0 of 24582 outputs changed (3732 "not recorded": files added to the registry since the record; the 20,850 recorded all match) |
| conformance 1.9.0 | `conform -i` | 1220/1220 identical, 0 not idempotent, diff +0 −0 |
| conformance 1.8.0 | `conform -i -V 1.8.0` | 1220/1220 identical, 0 not idempotent, diff +0 −0 |
| CLI output | PGO and plain `chloro --no-cache --threads 1 <fixtures>` (print mode), `--check --no-cache -v` sorted, `--check --no-cache` on the two timed trees | identical stdout (14,190,145 bytes, same sha256); identical sorted reports; exit 0 on both trees for both |
| CLI tests (plain, debug) | `cargo test -p chloro --all-features` | 20 unit, 11 `cli_cache`, 5 `cli_files`, 1 doc: all pass |
| instructions | `benchmarks/profile.sh --bench <bench> --core 3` | PGO 245,639,781; plain 312,842,996; ratio 0.7852 (−21.5%) |

M3: 244,712,537 against 312,734,032 (0.7825). The plain count here is 0.035% above the BKG
figure and the PGO count 0.38% above M3's; the plain `bench` here is built with `--target`
and `CARGO_PROFILE_RELEASE_STRIP=debuginfo`, and the profile now trains the E12 CLI.

## Wall time (measured, provisional: load 3.2–3.4)

`taskset -c 3 hyperfine -N -i -w 2 -r 15`, PGO and plain `chloro --check --no-cache <tree>`
in one invocation per round, round 2 with the order reversed, on M3's formatted copies
`/tmp/chloro-bench/formatted/{fx-hir-ty,reg-winnow}/tree` (122 and 61 files). Cells: min /
mean ± σ / max ms. 2026-10-15 02:24:02–02:25:04 UTC; core 3 `/proc/stat` over the window: 46 steal, 6116 busy, 0 idle ticks.

| workload | round | PGO | plain | PGO/plain, means | mins | ranges |
|---|---|---|---|---|---|---|
| fx-hir-ty | 1 | 608.3 / 647.7 ± 31.0 / 724.5 | 698.2 / 750.9 ± 47.6 / 846.9 | 0.862 | 0.871 | overlap |
| fx-hir-ty | 2 | 576.7 / 652.9 ± 30.2 / 693.6 | 643.1 / 721.3 ± 36.8 / 782.3 | 0.905 | 0.897 | overlap |
| reg-winnow | 1 | 175.4 / 190.2 ± 13.6 / 221.1 | 207.9 / 230.0 ± 22.5 / 283.9 | 0.827 | 0.844 | overlap |
| reg-winnow | 2 | 181.0 / 189.6 ± 9.1 / 216.2 | 201.4 / 227.5 ± 15.8 / 253.9 | 0.833 | 0.899 | overlap |

Merged (n = 30): fx-hir-ty 0.883 of the plain mean, reg-winnow 0.830; every round puts the
PGO mean and minimum below the plain one, inside M3's 0.78–0.92, with min–max ranges that
overlap under this load, so by protocol § 3 the wall-time figure here corroborates M3 and
is not a new speedup claim; the instruction count is.

## Cost in the release job

| item | measured here |
|---|---|
| `rustup component add llvm-tools-preview` | 6.4 s (download included) |
| the step, component already present | 97.0 s: instrumented `chloro` 52.4 s, instrumented `bench` 39.9 s, training and merge about 5 s |
| the action's build, PGO against plain | 69 s against 42.8 s (different moments of a loaded host) |
| profile | 1,003,696 bytes, 2 raw profiles; under `target/pgo-gen`, not uploaded |
| disk | `target/` was 313 MB after the step and the action-shaped builds (instrumented and profile-guided trees together); 1.3 GB at the end of this record with every build in it |

So each native job grows by roughly two minutes of build here; GitHub's runners are not
this host, and the job's 60-minute limit is far away.

## What a release run has to show (not observed)

- That the step runs on the three native targets and skips the two `aarch64` ones; in the
  upload step's log, 0 lines `no profile data available for function` from `chloro` or
  `chloro_core`; no PGO-failure annotation.
- **macOS x86-64**: the step has not been run on macOS (BSD userland, Homebrew bash).
  Read, not verified here: GitHub announced the retirement of the `macos-13` image in 2025;
  if the image is no longer served, both Apple jobs fail before any step, with or without
  this change (the job has `continue-on-error: true`, so the other targets still upload).
  Moving them to another image is outside this experiment; on an arm64 image the
  `x86_64-apple-darwin` job would skip training by the host check.
- **Windows MSVC**: `cygpath -m` paths, `-Cprofile-generate` with `+crt-static`, and the
  profiler runtime on `x86_64-pc-windows-msvc` have not been exercised; a failure in
  training falls back to the plain build with an annotation, but a failure in the action's
  `-Cprofile-use` build would fail that target's upload.
- The speed of the published binaries: no macOS or Windows figure exists, and the
  `x86_64-unknown-linux-gnu` asset is built on `ubuntu-22.04` with that runner's linker, not
  on this host.

## Other observation

`chloro` print mode on a directory with several threads writes the files' formatted texts
in a different order from run to run (three runs of the same binary: three stdout hashes,
equal after sorting lines, same byte count); with `--threads 1` the output is stable. Not
related to PGO; the CLI equality above therefore uses `--threads 1`.

## Decision

Kept on its branch, for the integrator: the step produces a profile that the action's own
build uses with no missing-function warning, every gate is unchanged on the profile-guided
build, `Ir` is 0.785 of the plain build (M3: 0.783), and the wall-time ratios on two crates
sit inside M3's range. The hypothesis holds on `x86_64-unknown-linux-gnu`; for macOS and
Windows it rests on the release run listed above. Merging changes no code path of
`chloro-core` or the CLI; it changes how three release assets are built.
