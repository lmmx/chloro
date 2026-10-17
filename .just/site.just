# The documentation website (docs/site/, zensical.toml), deployed by
# .github/workflows/site.yml. ZENSICAL pins the generator for reproducible builds.

ZENSICAL := "zensical==0.0.71"

# Build the site into site/
site-build:
    uv run --with {{ZENSICAL}} --no-project zensical build --clean

# Serve the site with live reload at http://localhost:8000/chloro/
site-serve:
    uv run --with {{ZENSICAL}} --no-project zensical serve

# Regenerate the CLI help snippet the reference page includes
site-snippets:
    cargo build --release -p chloro --features cli
    mkdir -p docs/site/_snippets/cli
    target/release/chloro --help > docs/site/_snippets/cli/help.txt

# Check the nav, every internal link of the built site, and the CLI help snippet
site-check: site-build
    cargo build --release -p chloro --features cli
    python3 docs/scripts/check_site.py --chloro target/release/chloro

# Compile and run every Rust example on the site
site-examples:
    python3 docs/scripts/check_examples.py
