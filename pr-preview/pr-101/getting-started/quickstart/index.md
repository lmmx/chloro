# Quickstart

How to check and format a crate with chloro, and how to run the check in CI. Each command gives the same result as `rustfmt` at edition 2024 with default options.

## Check

From the root of a crate:

```console
$ chloro --check src/ tests/ examples/

Error: 3 file(s) need formatting
```

`--check` formats every `.rs` file under the paths you name, writes nothing, and exits with status 1 if any file's formatted text differs from what is on disk. Every path must exist: a missing one is an error before anything is formatted. Add `--verbose` to see each file as it is processed and a summary at the end.

> [!WARNING]
>
> **Name your source directories**
>
> chloro walks every directory under the paths it is given, hidden directories and `target/` included, and does not read `.gitignore`. With no path at all it starts at the current directory. In a Cargo project, pass `src/` and your other source directories rather than `.`, or `target/` will be formatted too ([\#90](<https://github.com/lmmx/chloro/issues/90>)).

## Format

```sh
chloro --write src/ tests/ examples/
```

`--write` rewrites each file whose formatted text differs, and leaves the others untouched. Without `--check` or `--write`, chloro prints each formatted file to standard output, which is useful for a single file:

```sh
chloro src/lib.rs | diff src/lib.rs -
```

## Choose options

chloro takes rustfmt's stable options by rustfmt's names, as comma-separated `key=value` pairs:

```sh
chloro --config max_width=80,tab_spaces=2 --write src/
chloro --config edition=2021 --check src/
chloro --config rustfmt_version=1.8.0 --check src/
```

An unknown option, a malformed value or an unstable rustfmt option set away from its default stops the run before any file is read:

```console
$ chloro --config wrap_comments=true src/
Error: invalid --config: `wrap_comments` is an unstable rustfmt option; only its default `false` is supported
```

chloro does not read `rustfmt.toml`. If your project has one, translate its stable options into `--config`; [Configuration](<https://lmmx.github.io/chloro/pr-preview/pr-101/guide/configuration/index.md>) lists every option chloro accepts.

## Use it in CI

A GitHub Actions job that fails when a file is not formatted:

```yaml
jobs:
  format:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: taiki-e/install-action@v2
        with:
          tool: cargo-binstall
      - run: cargo binstall --no-confirm chloro
      - run: chloro --check --no-cache src/ tests/
```

`--no-cache` is optional. A CI runner starts with an empty cache, so the cache has nothing to answer from; in the measurements a cold cache cost 364 ms against 356 ms without one on a 1238-file tree (see [Caching](<https://lmmx.github.io/chloro/pr-preview/pr-101/guide/caching/index.md>)).

## Call it from Rust

```rust
let mut config = chloro::Config::default();
config.set("max_width", "80").unwrap();
let formatted = chloro::format_source_with_config("fn main(){let x=1;}", &config);
assert_eq!(formatted, "fn main() {\n    let x = 1;\n}\n");
```

The [Library guide](<https://lmmx.github.io/chloro/pr-preview/pr-101/guide/library/index.md>) covers the rest of the API, including formatting one large file on several threads.
