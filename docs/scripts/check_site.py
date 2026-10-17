#!/usr/bin/env python3
"""Check the documentation site's sources and its built output.

    python3 docs/scripts/check_site.py [--site DIR] [--chloro BIN]

- Every page under docs/site/ is in the nav of zensical.toml, and every nav entry exists.
- Every internal link, image and stylesheet in the built site (default: site/) resolves to
  a file of the build, with its #anchor present, as served under the base path of
  site_url (/chloro/ on GitHub Pages).
- With --chloro, docs/site/_snippets/cli/help.txt equals that binary's `--help` output.

Exits with 1 and lists every problem found.
"""

import argparse
import subprocess
import sys
import tomllib
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urljoin, urlparse

ROOT = Path(__file__).resolve().parents[2]


def nav_pages(nav):
    for entry in nav:
        if isinstance(entry, str):
            yield entry
        elif isinstance(entry, dict):
            for value in entry.values():
                yield from nav_pages([value] if isinstance(value, str) else value)


class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links, self.ids = [], set()

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.ids.add(attrs["id"])
        if tag == "a" and attrs.get("name"):
            self.ids.add(attrs["name"])
        for key in ("href", "src"):
            value = attrs.get(key)
            if value and not (tag == "link" and attrs.get("rel") in ("preconnect", "dns-prefetch")):
                self.links.append(value)


def check_nav(config, problems):
    docs = ROOT / config["project"]["docs_dir"]
    pages = {p.relative_to(docs).as_posix() for p in docs.rglob("*.md")}
    listed = set(nav_pages(config["project"]["nav"]))
    for page in sorted(pages - listed):
        problems.append(f"not in the nav: {page}")
    for page in sorted(listed - pages):
        problems.append(f"in the nav but missing: {page}")


def check_links(config, site, problems):
    base = urlparse(config["project"]["site_url"]).path  # "/chloro/"
    parsed = {}
    for html in site.rglob("*.html"):
        p = Links()
        p.feed(html.read_text(errors="replace"))
        parsed[html] = p

    def target(url_path):
        if not url_path.startswith(base):
            return None
        rel = unquote(url_path[len(base):])
        path = site / rel
        if url_path.endswith("/") or path.is_dir():
            path = path / "index.html"
        return path

    checked = 0
    for html, p in sorted(parsed.items()):
        page_url = base + html.relative_to(site).as_posix()
        if page_url.endswith("index.html"):
            page_url = page_url[: -len("index.html")]
        for link in p.links:
            url = urlparse(urljoin("https://host" + page_url, link))
            if urlparse(link).scheme and urlparse(link).netloc and not link.startswith(
                config["project"]["site_url"]
            ):
                continue  # external
            if link.startswith(("mailto:", "javascript:", "data:")):
                continue
            path = target(url.path)
            checked += 1
            where = html.relative_to(site).as_posix()
            if path is None:
                problems.append(f"{where}: {link} leaves the base path {base}")
            elif not path.exists():
                problems.append(f"{where}: {link} -> missing {path.relative_to(site)}")
            elif url.fragment and path.suffix == ".html":
                ids = parsed[path].ids if path in parsed else set()
                if url.fragment not in ids:
                    problems.append(f"{where}: {link} -> no #{url.fragment}")
    return checked


def check_snippet(chloro, problems):
    snippet = ROOT / "docs/site/_snippets/cli/help.txt"
    help_text = subprocess.run([chloro, "--help"], capture_output=True, text=True).stdout
    if snippet.read_text() != help_text:
        problems.append(f"{snippet.relative_to(ROOT)} differs from `{chloro} --help` (just site-snippets)")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--site", default=str(ROOT / "site"))
    parser.add_argument("--chloro")
    args = parser.parse_args()
    config = tomllib.loads((ROOT / "zensical.toml").read_text())
    problems = []
    check_nav(config, problems)
    site = Path(args.site)
    if (site / "index.html").exists():
        checked = check_links(config, site, problems)
        print(f"{checked} internal links checked", file=sys.stderr)
    else:
        problems.append(f"no built site at {site} (just site-build)")
    if args.chloro:
        check_snippet(args.chloro, problems)
    for problem in problems:
        print(problem)
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
