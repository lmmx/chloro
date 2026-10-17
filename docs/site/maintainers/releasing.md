# Releasing

<p class="lede">chloro is released with release-plz from the master branch: both crates move
together, each gets a tag and a GitHub release, and publishing the <code>chloro</code>
release builds the binaries.</p>

## The flow

```sh
just ship            # release-plz picks the version from the commits
just ship 0.9.0      # or set it
```

`just ship` refuses to run unless master is checked out and up to date with `origin/master`.
It then:

1. dry-runs the publish (`release-plz release --dry-run`);
2. bumps the versions of `chloro` and `chloro-core` and updates each crate's
   `CHANGELOG.md` from the Conventional Commit messages (`release-plz update`, grouped by
   `release-plz.toml`);
3. runs `just prepush` (check, clippy, rustdoc) and commits the bump;
4. pushes, and runs `release-plz release`, which publishes both crates to crates.io and
   creates the tags (`chloro-v<version>`, `chloro-core-v<version>`) and GitHub releases.

`just ship-manual` is a fallback for when release-plz is not working.

## Before a release

The repository has no CI workflow that runs the tests on pull requests, so the release
commit must be checked by hand:

- [ ] `cargo test --workspace --all-features` and `just clippy` pass.
- [ ] The conformance checks pass: `conform -i` at 1.9.0 and 1.8.0 (1220 of 1220, none
      non-idempotent) and `config_matrix -n 1220` ([Development](development.md#conformance)).
- [ ] If `ra_ap_parser` changed since the last release, the
      [parser harnesses](parser-upgrades.md) were run.
- [ ] The root `CHANGELOG.md`: move the *Unreleased* entries under the new version and date.
      The per-crate changelogs are written by release-plz; the root one is written by hand
      for users.
- [ ] The README badges (dependency count, binary size) are current: `just emblazon`.
- [ ] `README.md` and `chloro/README.md` are identical (`just docs-match`); the second is
      what crates.io shows.
- [ ] The website's [Installation](../getting-started/installation.md) note about unreleased
      features, and the release named there, are updated.

## Binaries

Publishing the GitHub release for a `chloro-v*` tag runs `.github/workflows/binaries.yml`,
which builds the CLI for each target and attaches the archives to the release. Their names
match the `[package.metadata.binstall]` section of `chloro/Cargo.toml`, which is how
`cargo binstall chloro` finds them.

| target | runner | build |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `ubuntu-22.04` | profile-guided |
| `x86_64-apple-darwin` | `macos-13` | profile-guided |
| `aarch64-apple-darwin` | `macos-13` | plain (cross-compiled) |
| `x86_64-pc-windows-msvc` | `windows-2022` | profile-guided, static CRT |
| `aarch64-pc-windows-msvc` | `windows-2022` | plain (cross-compiled), static CRT |

Each target's job may fail without failing the others. The 0.8.0 release has archives only
for the Linux and Windows targets; check whether the `macos-13` runner is still available
before relying on macOS binaries.

### Profile-guided builds

`benchmarks/pgo.sh` builds chloro with profile-guided optimisation: it compiles an
instrumented binary (`-Cprofile-generate`), runs it over the conformance corpus with
`--check` and through `bench`, merges the profile with `llvm-profdata` (from the
`llvm-tools-preview` component), and builds again with `-Cprofile-use`.

```sh
benchmarks/pgo.sh                                  # local profile-guided build
benchmarks/pgo.sh --phase train --target x86_64-unknown-linux-gnu
benchmarks/pgo.sh --phase use --target x86_64-unknown-linux-gnu
```

The release workflow runs the training phase on each target whose runner executes it
natively and sets `RUSTFLAGS` so the upload step's build uses the profile. A cross-compiled
target, or one whose training fails, gets the plain build with a warning in the log. On the
measurements, the profile-guided build executed 16% fewer instructions than the plain one
and ran in 0.80–0.95 of its wall time, with identical output.

!!! warning "Not yet exercised by a release"

    The profile-guided step was added after 0.8.0 and has been validated by running its
    commands locally on `x86_64-unknown-linux-gnu`, not by a release run. On the first
    release that uses it, read each target's log for `PGO skipped`, `PGO training failed`,
    and `no profile data available` warnings from the compiler.

## Versions

Both crates share a version (0.8.0 at the latest release). `chloro` depends on
`chloro-core` at the same version through the workspace's `[workspace.dependencies]`.
The minimum supported Rust version is the workspace's `rust-version`, 1.88.
