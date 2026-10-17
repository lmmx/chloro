#!/usr/bin/env python3
"""Compile and run every ```rust block of the documentation site and the README as a test.

    python3 docs/scripts/check_examples.py [--keep]

Each block under docs/site/ and in README.md becomes one test in a scratch crate under
target/docs-examples/ that depends on the workspace's `chloro` and `chloro-core`, the way
rustdoc runs a doctest: the block is the body of a test function, so it can declare
items, `use` paths and `assert!` its results. The exit status is cargo's.
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SITE = ROOT / "docs" / "site"
CRATE = ROOT / "target" / "docs-examples"
FENCE = re.compile(r"^(?P<indent>[ \t]*)```rust[ \t]*\n(?P<body>.*?)^(?P=indent)```[ \t]*$", re.M | re.S)


def blocks():
    for page in [ROOT / "README.md", *sorted(SITE.rglob("*.md"))]:
        text = page.read_text()
        for m in FENCE.finditer(text):
            indent = m.group("indent")
            body = "".join(
                line[len(indent):] if line.startswith(indent) else line
                for line in m.group("body").splitlines(keepends=True)
            )
            line = text.count("\n", 0, m.start()) + 1
            yield page.relative_to(ROOT), line, body


def main():
    tests = CRATE / "tests"
    tests.mkdir(parents=True, exist_ok=True)
    for old in tests.glob("*.rs"):
        old.unlink()
    (CRATE / "src").mkdir(exist_ok=True)
    (CRATE / "src" / "lib.rs").write_text("")
    (CRATE / "Cargo.toml").write_text(
        "[package]\nname = \"docs-examples\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n"
        "[workspace]\n\n"
        "[dependencies]\n"
        f"chloro = {{ path = \"{ROOT / 'chloro'}\" }}\n"
        f"chloro-core = {{ path = \"{ROOT / 'chloro-core'}\" }}\n"
    )
    count = 0
    for page, line, body in blocks():
        count += 1
        name = re.sub(r"[^a-z0-9]+", "_", str(page.with_suffix("")).lower()).strip("_")
        indented = "".join("    " + l if l.strip() else l for l in body.splitlines(keepends=True))
        (tests / f"{name}_line_{line}.rs").write_text(
            f"// {page}:{line}\n#[test]\n#[allow(unused)]\nfn example() {{\n{indented}}}\n"
        )
    print(f"{count} Rust examples from README.md and {SITE.relative_to(ROOT)}", file=sys.stderr)
    status = subprocess.run(
        ["cargo", "test", "--quiet", "--manifest-path", str(CRATE / "Cargo.toml"),
         "--target-dir", str(ROOT / "target")],
    ).returncode
    sys.exit(status)


if __name__ == "__main__":
    main()
