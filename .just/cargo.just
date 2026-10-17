absolve:
    #!/usr/bin/env -S bash -euo pipefail
    if ! cargo tree -e no-dev -i syn 2>/dev/null | grep -q .; then
        echo -e "\033[38;2;255;255;255;48;2;0;0;0m free of \033[38;2;255;255;255;48;2;255;105;180m syn
    \033[38;2;255;255;255;48;2;0;0;0m\033[0m"
    else
        echo -e "\033[1;31m❌ 'syn' found in dependency tree. Here's what's using 'syn':\033[0m"
        cargo tree -i syn -e features
        exit 1
    fi

build:
    cargo build --workspace --all-features

check:
    cargo check --workspace --all-features

clippy:
    cargo clippy --workspace --all-targets --all-features --target-dir target/clippy-all-features -- -D warnings

clippy-fix:
    cargo clippy --fix --workspace --all-targets --all-features --target-dir target/clippy-all-features -- -D warnings

install_cmd := "cargo install --path chloro -F cli --target-dir target/install"

# Its own target directory keeps the recorded ID from invalidating other release builds.
#
# Install the CLI from this checkout, with `just build-id` in its `chloro --version`
reinstall:
    CHLORO_BUILD_ID="$(just --justfile '{{ justfile() }}' build-id)" {{ install_cmd }}

# The files that go into the CLI binary (git pathspecs)
build_inputs := "chloro/build.rs chloro/src chloro/Cargo.toml chloro-core/src chloro-core/Cargo.toml Cargo.toml Cargo.lock .cargo"

# A hash of the install command, the toolchain, and the path and content of every file that
# goes into the CLI binary, as they are on disk (committed or not).
#
# The build ID of this checkout
build-id:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ justfile_directory() }}"
    files=$(git ls-files --cached --others --exclude-standard -- {{ build_inputs }} \
        | LC_ALL=C sort -u | while IFS= read -r f; do if [ -f "$f" ]; then printf '%s\n' "$f"; fi; done)
    {
        echo "{{ install_cmd }}"
        rustc -vV
        # the commit and its tags, which `chloro --version` reports
        git rev-parse HEAD
        git tag --points-at HEAD
        paste -d ' ' <(printf '%s\n' "$files" | git hash-object --stdin-paths) <(printf '%s\n' "$files")
    } | git hash-object --stdin

# Reinstall chloro unless the `chloro` on PATH reports the current build ID
[private]
chloro-fresh:
    #!/usr/bin/env bash
    set -euo pipefail
    want=$(just --justfile "{{ justfile() }}" build-id)
    installed_id() { chloro --version 2>/dev/null | sed -n 's/^chloro .*build \([0-9a-f]*\))$/\1/p' || true; }
    have=$(installed_id)
    if [ "$have" != "$want" ]; then
        echo "chloro on PATH is not built from this checkout; reinstalling" >&2
        just --justfile "{{ justfile() }}" reinstall
        have=$(installed_id)
        if [ "$have" != "$want" ]; then
            echo "error: the chloro on PATH ($(command -v chloro || echo none)) is not the one just installed; put cargo's bin directory first on PATH" >&2
            exit 1
        fi
    fi

test *args:
    cargo nextest run {{args}} --no-fail-fast --all-features

test-ci *args:
    #!/usr/bin/env -S echo-comment --color bright-green
    # 🏃 Running Rust tests...
    cargo test {{args}} --all-features

    # 📚 Running documentation tests...
    cargo test --doc {{args}} --all-features
