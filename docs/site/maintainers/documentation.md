---
description: "Where chloro's documentation lives, and how this website is built, checked and published to GitHub Pages."
---

# Documentation

<p class="lede">Where chloro's documentation lives, and how this website is built, checked
and published to GitHub Pages.</p>

## Where documentation lives

| where | for | source |
|---|---|---|
| this website, <https://lmmx.github.io/chloro/> | using and maintaining chloro: guides, compatibility, performance, architecture | `docs/site/`, `zensical.toml` |
| [docs.rs/chloro](https://docs.rs/chloro), [docs.rs/chloro-core](https://docs.rs/chloro-core) | the Rust API, item by item | doc comments in the code |
| `README.md` | a first look on GitHub and crates.io, with links here | `README.md`, mirrored to `chloro/README.md` |
| `CHANGELOG.md` | what changed in each release, for users; shown on the [Changelog](../changelog.md) page | `CHANGELOG.md` |
| `docs/journal/` | the record of decisions, experiments and measurements | dated Markdown entries, see the [project history](../history.md) |

Each fact should be documented in one place. The website describes what chloro does and
links to the journal for the evidence, rather than copying experiment records.

## Building the site

The site is built with [Zensical](https://zensical.org), as the documentation of
[nave](https://lmmx.github.io/nave/) is. It needs only [uv](https://docs.astral.sh/uv/):

```sh
just site-serve        # live preview at http://localhost:8000/chloro/
just site-build        # build into site/ (ignored by git)
```

The recipes pin Zensical's version (`ZENSICAL` in `.just/site.just`), and the deployment
workflow pins the same one. To upgrade, change both and check the site.

## Checks

```sh
just site-check        # nav, links and the CLI help snippet
just site-examples     # compile and run every Rust example
```

- `site-check` builds the site, then runs `docs/scripts/check_site.py`: every page is in the
  nav and every nav entry exists; every internal link, image and stylesheet of the built
  site resolves, anchors included, under the `/chloro/` base path the site is served from;
  and `docs/site/_snippets/cli/help.txt` matches the current `chloro --help`. Zensical's own
  build also reports links to missing pages and anchors.
- `site-examples` runs `docs/scripts/check_examples.py`, which turns every fenced `rust` block
  under `docs/site/` into a test of a scratch crate (in `target/docs-examples/`) that depends
  on the workspace's crates, and runs them. A block is the body of a test function, as in a
  doctest, so it can declare items and `assert!` its results.

When the CLI's help text changes, `just site-snippets` regenerates the snippet that the
[CLI reference](../reference/cli.md) includes.

## Writing pages

- Pages are Markdown with the extensions configured in `zensical.toml`: admonitions
  (`!!! note`), tabs (`=== "Title"`), collapsible blocks (`??? info`), definition lists,
  footnotes, and keys (`++"--check"++` renders a flag as a key cap). `--8<-- "path"` includes
  a file, relative to the repository root.
- Add a new page to the `nav` in `zensical.toml`; `site-check` fails on a page that is not
  in it.
- Give each page a front-matter `description` (one sentence, at most 160 characters), and
  list the page with the same description in the `llmstxt` sections of `zensical.toml`.
  The description is the page's meta description for search engines and its entry in
  `llms.txt`; `site-check` fails on a page without one, or one listed with another.
- Link between pages with relative paths to the `.md` files. Link to the journal, the source
  and anything else in the repository with absolute GitHub URLs on `master`, since those
  files are not part of the site.
- State measured figures with their conditions, and link to the record they come from.

## For agents and LLMs

Zensical's built-in `llmstxt` plugin, configured in `zensical.toml`, adds to every build:

- `llms.txt`, an index of the pages in the [llms.txt](https://llmstxt.org/) format, with
  each page's description; the maintainers' pages are under *Optional*;
- `llms-full.txt`, every listed page in one file;
- a Markdown copy of each page beside its HTML, `<page>/index.md`, converted from the built
  page (snippets included, links absolute);
- `<link rel="alternate" type="text/markdown">` and `<link rel="describedby">` (to
  `llms.txt`) in each page's head.

Each page also has a *Copy as Markdown* button (`content.action.copy`). `AGENTS.md` at the
repository root is the entry point for coding agents working on the code, and the README
links all of these.

GitHub Pages serves the site from a subpath, so a `robots.txt` (which must be at the domain
root) is not possible, and it cannot negotiate content or set headers: an agent asking for
`text/markdown` gets HTML, and finds the Markdown through the links above. The sitemap is at
`sitemap.xml`.

## Design

The site follows the design of nave's documentation: Zensical's modern theme, Instrument
Serif for page titles, Encode Sans Condensed for headings, Work Sans for text, commands set
as key caps. chloro's own colours are chlorophyll greens, defined as CSS variables for the
light and dark schemes in `docs/site/assets/stylesheets/extra.css`, with a toggle in the
header. The mark is a seedling whose big leaf has veins that branch like a syntax tree, in
`docs/site/assets/images/`: `chloro-hero.svg` and `chloro-hero-dark.svg` on the home page,
with a ground shadow, sheen and highlights drawn with gradients (no filters);
`chloro-small.svg` and `chloro-small-dark.svg` (flat, without veins) in the header; and
`chloro.svg`, the favicon, which switches to the dark palette when the browser prefers dark. Charts and diagrams are inline SVG drawn with the page's colour variables, so
they follow the scheme, and each chart has its data in a table beside it.

## Deployment

GitHub Pages serves the site from the `gh-pages` branch (Settings → Pages → Build and
deployment → Source: *Deploy from a branch*, `gh-pages`, `/ (root)`).
`.github/workflows/site.yml` writes to that branch:

- **Production.** Every push to `master` builds the site and deploys it to the root of
  `gh-pages`, which is served at <https://lmmx.github.io/chloro/>. The deploy replaces
  everything at the root except the `pr-preview/` directory.
- **Pull request previews.** A pull request that changes the site (`docs/site/`,
  `zensical.toml`, `CHANGELOG.md`, `docs/scripts/` or the workflow) is built with its
  `site_url` set to its preview path and deployed to
  `gh-pages/pr-preview/pr-<number>/`, served at
  `https://lmmx.github.io/chloro/pr-preview/pr-<number>/`. The workflow posts the link
  as a comment on the pull request once GitHub Pages has published it, edits that same
  comment on later pushes, and deletes the preview when the pull request is closed or
  merged.

Every pull request also runs the build and `docs/scripts/check_site.py`, whether or not it
gets a preview. Pull requests from forks get the checks but no preview: their workflows
have a read-only token and cannot push to `gh-pages`. The job that deploys a preview checks
out the default branch, not the pull request, and publishes only the files the build job
produced.

The production and preview deploys write to separate paths of the branch, and each
rebases and retries if the branch moved while it ran, so they can run at the same time.
The `gh-pages` branch keeps every deploy as a commit.

The workflow reads the Pages source at the start of each run. While the source is still
*GitHub Actions*, it also publishes `master` with `actions/deploy-pages` and does not deploy
previews; once the source is the `gh-pages` branch, that job is skipped. The workflow can
also be run by hand from the Actions tab.
