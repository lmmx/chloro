---
description: "Install chloro's command-line tool from a pre-built binary or from source, or add the chloro and chloro-core crates as a library."
---

# Installation

<p class="lede">chloro is published as two crates: <code>chloro</code>, which holds the
command-line tool and a small library facade, and <code>chloro-core</code>, the formatter
itself.</p>

## The command-line tool

=== "Binary"

    ```sh
    cargo binstall chloro
    ```

    [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) downloads the binary
    attached to the [GitHub release](https://github.com/lmmx/chloro/releases) for your
    platform. Releases have binaries for `x86_64-unknown-linux-gnu`,
    `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`, and for macOS where its runner
    built them (the [release page](https://github.com/lmmx/chloro/releases) lists them); on
    any other platform `cargo binstall` falls back to building from source.

=== "Source"

    ```sh
    cargo install chloro --features cli
    ```

    The binary is behind the `cli` feature, so that a library dependency on `chloro` does
    not pull in the command line's dependencies. Building needs Rust 1.88 or later (the
    workspace's `rust-version`).

=== "Checkout"

    ```sh
    git clone https://github.com/lmmx/chloro
    cd chloro
    cargo install --path chloro --features cli
    ```

Check that it runs:

```sh
chloro --help
```

The release profile builds with fat link-time optimisation and one codegen unit, which
makes a from-source build slower than a default release build. Release
binaries for the targets that GitHub's runners execute natively can also be built with
profile-guided optimisation; see [Releasing](../maintainers/releasing.md#profile-guided-builds).

## The library

Add `chloro` to your `Cargo.toml` for the formatting functions and `Config`:

```toml
[dependencies]
chloro = "0.9"
```

`chloro` re-exports `format_source`, `format_source_with_config` and `Config` from
`chloro-core`. The parallel and cached entry points, and the syntax tree, are only in
`chloro-core`:

```toml
[dependencies]
chloro-core = "0.9"
```

Both crates have no default features. The [Library guide](../guide/library.md) shows what
each entry point does, and the [Rust API reference](../reference/api.md) lists them with
links to docs.rs.

!!! note "Versions"

    This site describes chloro 0.9.0. The [Changelog](../changelog.md) says which version
    each change is in, and `chloro --version` which version you have.

## Run-time requirements

chloro has no run-time dependencies. It does not call `rustfmt`, `rustc` or `cargo`. The
command line reads the `rustfmt.toml` and `Cargo.toml` files that rustfmt and `cargo fmt`
read for a file ([Configuration files](../guide/configuration.md#configuration-files)); the
library reads no files. Apart from the files it formats, the only file it writes is its
[cache](../guide/caching.md), which lives in the user cache directory by default.
