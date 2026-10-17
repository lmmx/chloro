---
description: "The repository's layout, how to build and test chloro, the conformance checks, and the checks a change must pass."
---

# Development

<p class="lede">How the repository is laid out, how to build and test it, and the checks a
change must pass. A change that alters any formatted output is a conformance change and must
say so; any other change must leave every output byte unchanged.</p>

## Layout

| path | what is there |
|---|---|
| `chloro-core/src/cst/` | the lexer front end (`lex.rs`), the flat syntax tree (`mod.rs`), and the typed layer (`ast.rs`, `generated.rs` from `gen_ast.py` and `rust.ungram`) |
| `chloro-core/src/formatter/` | the ported rustfmt modules, the adaptation layer, `parallel.rs`, `unit_cache.rs`, `width.rs` |
| `chloro-core/tests/` | unit and integration tests, and the conformance suite under `tests/conformance/` |
| `chloro-core/examples/` | the measurement and comparison tools (`conform`, `config_matrix`, `bench`, `lex_equality`, `unit_cache`, …) |
| `chloro/src/cli/` | the command line: arguments, file discovery, the worker pool, the caches, the trace |
| `benchmarks/` | the benchmark harness against rustfmt, the instruction-count profiler, the PGO build script |
| `docs/site/` | this website's sources; `zensical.toml` at the root configures it |
| `docs/journal/` | the development journal: design records, experiments and measurements, in the format of `docs/JOURNAL.md` |

## Building and testing

```sh
cargo build --workspace --all-features
cargo test --workspace --all-features
```

The `Justfile` wraps the usual commands: `just test` (with `cargo nextest`), `just clippy`
(`-D warnings`, all targets and features), `just check`, `just docs` (rustdoc). Formatting and
lint hooks run through [prek](https://github.com/j178/prek) or pre-commit: `just
install-hooks`, then `just precommit`. Commit messages follow
[Conventional Commits](https://www.conventionalcommits.org/), enforced by a `commit-msg`
hook, because the release tooling builds the changelog from them.

`README.md` and `chloro/README.md` are kept identical (`just docs-match` checks;
`just docs-mirror` copies the newer over the older): the second is the README crates.io
shows for `chloro`.

## Conformance

The conformance suite lives in `chloro-core/tests/conformance/`:

- `fixtures/rust-analyzer/` holds 1238 `.rs` files from rust-analyzer, the corpus.
- `snapshots/ra/<crate>/<case>.rustfmt.rs` holds rustfmt 1.9.0's output for each, or the
  input itself where rustfmt rejects the file. `snapshots/ra-1.8.0/` holds rustfmt 1.8.0's
  output where it differs from 1.9.0's.
- `rustfmt-versions/` holds a reproducer for each class of difference between rustfmt 1.8.0
  and 1.9.0.

`conform` compares chloro with the stored snapshots, without running rustfmt:

```sh
cargo run --release -p chloro-core --example conform -- -i            # 1.9.0
cargo run --release -p chloro-core --example conform -- -i -V 1.8.0   # 1.8.0
```

It reports the number of identical files, diff totals, files that are not idempotent
(`-i`), and throughput. The batch test regenerates the snapshots with the `rustfmt` on
`PATH`, and writes chloro's output and a diff beside each:

```sh
cargo test -p chloro-core --test conformance batch
```

For the `parser` and `syntax` crates (581 small cases) only the `.rustfmt.rs` files are
tracked; the outputs and diffs are written locally and ignored by git. `just conf` summarises
the current diffs, and `just conf-plain` prints the summary without colour.

`config_matrix` compares option values and other corpora against a live rustfmt:

```sh
cargo run --release -p chloro-core --example config_matrix -- -n 1220
cargo run --release -p chloro-core --example config_matrix -- --root ~/.cargo/registry/src -n 30000 default
```

The registry corpus is the 577 crates named in
`docs/journal/2026-10-14-autoresearch/corpus/registry-manifest.toml`, fetched into the cargo
registry.

## Checks for a change

CI (`.github/workflows/ci.yml`) runs on every pull request: `cargo fmt --check`, clippy with
warnings denied, the tests on Linux, macOS (arm64) and Windows, a build of the
`aarch64-pc-windows-msvc` release target, `cargo check` on Rust 1.88 (the workspace's
`rust-version`), and `conform -i` at both rustfmt releases. The arm64 runner matters: the byte
search in `formatter/comment.rs` has an SSE2 path on x86-64 and a portable one elsewhere.

A change that is meant to leave the output alone (a performance change, a refactor) must
show that it does. In order, cheapest first:

1. `cargo fmt --check`, `just clippy`, `cargo test --workspace --all-features`.
2. **Output hashes.** Record the hash of every corpus file's output before the change, and
   check them after:

    ```sh
    cargo run --release -p chloro-core --example bench -- --record before.tsv   # on the base
    cargo run --release -p chloro-core --example bench -- --check before.tsv    # on the change
    cargo run --release -p chloro-core --example bench -- --threads 4 --check before.tsv
    ```

    Zero files may change, sequentially or with four threads. Add
    `--root ~/.cargo/registry/src` to do the same over the registry corpus.

3. `conform -i` at both rustfmt releases: no fewer identical files, none non-idempotent.
4. `config_matrix -n 1220`: no option value loses a file.
5. For a performance claim, the instruction count on both samples
   ([Benchmarking](benchmarking.md)).

A conformance fix changes outputs on purpose: the files it changes should be exactly the
ones it targets, checked file by file, and the case recorded in the conformance ledger.

## Code conventions

- **rustfmt's structure.** Formatter modules keep rustfmt's module, function and variable
  names and its control flow, so that a difference can be traced to the rustfmt code it
  reproduces. Optimisations are written so that this correspondence stays readable.
- **Public API.** Changes are additive: `format_source` and `format_source_with_config` keep
  their signatures, and new capabilities are new functions.
- **`unsafe` code.** `chloro` forbids it. `chloro-core` has one `unsafe` block, with a written
  safety argument and a randomised test. Any new one needs the same, and a safe alternative
  measured first.
- **Dependencies.** A new dependency needs a measured reason. `chloro-core` has 18 crates in
  its runtime tree.

## The journal

`docs/journal/` records design decisions, experiments and measurements as dated entries, in
the format `docs/JOURNAL.md` describes. The journal records why things are the way they are;
this website describes how chloro works now. The [project history](../history.md) is a
guide to the journal.
