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
    platform. The 0.8.0 release has binaries for `x86_64-unknown-linux-gnu`,
    `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`; on any other platform
    `cargo binstall` falls back to building from source.

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
chloro = "0.8"
```

`chloro` re-exports `format_source`, `format_source_with_config` and `Config` from
`chloro-core`. The parallel and cached entry points, and the syntax tree, are only in
`chloro-core`:

```toml
[dependencies]
chloro-core = "0.8"
```

Both crates have no default features. The [Library guide](../guide/library.md) shows what
each entry point does, and the [Rust API reference](../reference/api.md) lists them with
links to docs.rs.

!!! note "Versions"

    The performance work and the rustfmt 1.8.0 support described on this site are on the
    main branch and not yet in a published release; 0.8.0 is the latest release. The
    [Changelog](../changelog.md) says which version each change is in.

## Run-time requirements

chloro has no run-time dependencies. It does not call `rustfmt`, `rustc` or `cargo`, and it
reads no configuration file: options come from `--config` on the command line or from
`Config` in the library. Apart from the files it formats, the only file it writes is its
[cache](../guide/caching.md), which lives in the user cache directory by default.
