# Both tools get the same files: the ones `cargo fmt --all` formats, each once. Nothing is
# written (both run in check mode), so use an already-formatted tree: rustfmt's --check
# also builds a diff of every file that would change. cargo fmt reads the project's
# rustfmt.toml and rust-toolchain.toml; chloro reads neither. Needs hyperfine.
#
# Time chloro (all cores, then one thread) against `cargo fmt` on the Cargo project in DIR
vs-rustfmt dir="." runs="5":
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --release -p chloro --features cli --manifest-path "{{ justfile_directory() }}/Cargo.toml"
    chloro="{{ justfile_directory() }}/target/release/chloro"
    cd "{{ dir }}"
    files=$(mktemp)
    trap 'rm -f "$files"' EXIT
    # rustfmt -v names each file it formats; cargo fmt hands it every target's crate root
    { cargo fmt --all -- --check -v 2>/dev/null || true; } | sed -n 's/^Formatting //p' | sort -u > "$files"
    count=$(wc -l < "$files" | tr -d ' ')
    if [ "$count" -eq 0 ]; then
        echo "cargo fmt formats no files in {{ dir }}" >&2
        exit 1
    fi
    cores=$(nproc 2>/dev/null || sysctl -n hw.ncpu)
    echo "$count files, $cores cores, $(rustfmt --version)"
    hyperfine --shell=default --warmup 1 --runs {{ runs }} --ignore-failure \
        -n "chloro" "tr '\n' '\0' < $files | xargs -0 $chloro --check --no-cache" \
        -n "chloro --threads 1" "tr '\n' '\0' < $files | xargs -0 $chloro --check --no-cache --threads 1" \
        -n "cargo fmt" "cargo fmt --all -- --check"

# ruff is a large Rust workspace chloro was not developed on.
#
# `vs-rustfmt` on a local ruff checkout in DIR, cloned there only if it does not exist yet
vs-ruff dir=(home_directory() / "tmp/ruff") runs="5":
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -d "{{ dir }}" ]; then
        git clone --depth 1 https://github.com/astral-sh/ruff "{{ dir }}"
    fi
    just --justfile "{{ justfile() }}" vs-rustfmt "{{ dir }}" {{ runs }}
