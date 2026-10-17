# AGENTS.md

chloro is a Rust code formatter that reproduces rustfmt's output byte for byte: a
module-by-module port of rustfmt 1.9.0's formatting code onto rust-analyzer's parser. The
workspace has two crates: `chloro-core` (the formatter) and `chloro` (the command line,
feature `cli`, and a small library facade).

The maintainers' guide is the full reference:
<https://lmmx.github.io/chloro/maintainers/development/> (source:
`docs/site/maintainers/development.md`). The user documentation is indexed for agents at
<https://lmmx.github.io/chloro/llms.txt>.

## Build and test

```sh
cargo build --workspace --all-features
cargo test --workspace --all-features
just clippy                      # -D warnings, all targets and features
cargo fmt --check
```

## Rules a change must keep

- Output is the contract. A change that is not a conformance fix must leave every formatted
  byte unchanged; check with the output hashes of `bench --record` / `bench --check`, then
  `conform -i` at both rustfmt releases and `config_matrix -n 1220` (commands in the
  maintainers' guide, "Checks for a change").
- Formatter modules keep rustfmt's module, function and variable names and control flow.
- The public API changes only by addition.
- No new `unsafe` and no new dependency without a measured reason.
- Commit messages follow Conventional Commits; the release changelogs are built from them.
- `README.md` and `chloro/README.md` stay identical (`just docs-match`).

## Documentation site

`docs/site/` with `zensical.toml`; `just site-build`, then `just site-check` (nav, links,
the CLI help snippet, and the llms.txt descriptions against each page's front matter). Each
page needs a front-matter `description`, repeated in the `llmstxt` sections of
`zensical.toml`.
