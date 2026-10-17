# Releasing

chloro is released with release-plz from the master branch. Both crates are released together at the same version, each gets a tag and a GitHub release, and publishing the `chloro` release builds the binaries.

## Release process

```sh
just ship            # release-plz picks the version from the commits
just ship 0.9.0      # or set it
```

`just ship` refuses to run unless master is checked out, clean and up to date with `origin/master`. It then:

1. dry-runs the publish (`release-plz release --dry-run`);
2. runs `just prepush` (check, clippy, rustdoc) and `just precommit` (formatting, lints, the README badges) on the tree as it is, and stops if either fails or changes a file, before anything is modified;
3. bumps the versions of `chloro` and `chloro-core` (`release-plz set-version`, or `release-plz update` to pick the version from the commits), checks that `CHANGELOG.md` has a section for the new version, and commits the bump; a failure in this step resets the tree to where it started;
4. pushes, and runs `release-plz release`, which publishes both crates to crates.io and creates the tags (`chloro-v<version>`, `chloro-core-v<version>`) and GitHub releases;
5. sets both releases' notes from the version's section of `CHANGELOG.md` (`just release-notes-publish <version>`; `just release-notes <version>` prints them).

The root `CHANGELOG.md` is the only changelog: release-plz does not update the per-crate ones (`changelog_update = false`), which end at 0.7.5. Its `set-version` used to rename their newest heading instead of adding an entry, which gave 0.8.0 and 0.9.0 the 0.7.5 notes.

`just ship-manual` is a fallback for when release-plz is not working.

## Before a release

CI (`.github/workflows/ci.yml`) runs on every pull request and push to master: formatting, clippy, the tests on Linux, macOS (arm64) and Windows, a build of the Windows arm64 target, `cargo check` on the minimum Rust version, and `conform -i` at the three rustfmt releases.

- \[ \] CI is green on the commit to be released.
- \[ \] `config_matrix -n 1220` (against a live rustfmt, so not in CI) is identical on every value ([Development](<https://lmmx.github.io/chloro/pr-preview/pr-113/maintainers/development/#conformance>)).
- \[ \] If `ra_ap_parser` changed since the last release, the [parser harnesses](<https://lmmx.github.io/chloro/pr-preview/pr-113/maintainers/parser-upgrades/index.md>) were run.
- \[ \] The root `CHANGELOG.md`: move the *Unreleased* entries under the new version and date. `just ship` stops if the section is missing, and the release notes are taken from it.
- \[ \] The README badges (dependency count, binary size) are current: `just emblazon`.
- \[ \] `README.md` and `chloro/README.md` are identical (`just docs-match`); the second is what crates.io shows.
- \[ \] The website's [Installation](<https://lmmx.github.io/chloro/pr-preview/pr-113/getting-started/installation/index.md>) note about unreleased features, and the release named there, are updated.

## Binaries

Publishing the GitHub release for a `chloro-v*` tag runs `.github/workflows/binaries.yml`, which builds the CLI for each target and attaches the archives to the release (`chloro-core-v*` releases have none). Their names match the `[package.metadata.binstall]` section of `chloro/Cargo.toml`, which is how `cargo binstall chloro` finds them.

| target | runner | build |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | `ubuntu-22.04` | profile-guided |
| `x86_64-apple-darwin` | `macos-latest` (arm64) | plain (cross-compiled) |
| `aarch64-apple-darwin` | `macos-latest` (arm64) | profile-guided |
| `x86_64-pc-windows-msvc` | `windows-2022` | profile-guided, static CRT |
| `aarch64-pc-windows-msvc` | `windows-2022` | plain (cross-compiled), static CRT |

A target that fails does not stop the others, and fails the run. The 0.8.0 and 0.9.0 releases have no macOS archives (their jobs waited for the retired `macos-13` runner until cancelled), and 0.9.0 none for `aarch64-pc-windows-msvc` (an `unused_mut` warning on non-x86 targets, made an error by `-D warnings`, which release builds no longer set).

The workflow can be run by hand for an existing tag, from the Actions tab or with `gh workflow run binaries.yml -f tag=chloro-v0.9.0`. By default it is a dry run, which builds and archives every target and uploads nothing; with `-f dry-run=false` it uploads the archives to that tag's release, replacing any of the same name, so name the missing targets with `-f targets="x86_64-apple-darwin aarch64-apple-darwin"`. The build checks out the tag, so `chloro --version` prints the release's plain version.

### Profile-guided builds

`benchmarks/pgo.sh` builds chloro with profile-guided optimisation: it compiles an instrumented binary (`-Cprofile-generate`), runs it over the conformance corpus with `--check` and through `bench`, merges the profile with `llvm-profdata` (from the `llvm-tools-preview` component), and builds again with `-Cprofile-use`.

```sh
benchmarks/pgo.sh                                  # local profile-guided build
benchmarks/pgo.sh --phase train --target x86_64-unknown-linux-gnu
benchmarks/pgo.sh --phase use --target x86_64-unknown-linux-gnu
```

The release workflow runs the training phase on each target whose runner executes it natively and sets `RUSTFLAGS` so the upload step's build uses the profile. A cross-compiled target, or one whose training fails, gets the plain build with a warning in the log. On the measurements, the profile-guided build executed 16% fewer instructions than the plain one and ran in 0.80–0.95 of its wall time, with identical output.

0.9.0 was the first release built this way: the `x86_64-unknown-linux-gnu` job trained a 1.09 MB profile and built with it, with no `no profile data available` warnings. On later releases, read each target's log for `PGO skipped`, `PGO training failed` and those warnings.

## Versions

Both crates share a version (0.9.0 at the latest release). `chloro` depends on `chloro-core` at the same version through the workspace's `[workspace.dependencies]`. The minimum supported Rust version is the workspace's `rust-version`, 1.88.
