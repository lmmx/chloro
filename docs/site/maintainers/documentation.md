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
- Link between pages with relative paths to the `.md` files. Link to the journal, the source
  and anything else in the repository with absolute GitHub URLs on `master`, since those
  files are not part of the site.
- State measured figures with their conditions, and link to the record they come from.

## Design

The site follows the design of nave's documentation: Zensical's modern theme, Instrument
Serif for page titles, Encode Sans Condensed for headings, Work Sans for text, commands set
as key caps. chloro's own colours are chlorophyll greens, defined as CSS variables for the
light and dark schemes in `docs/site/assets/stylesheets/extra.css`, with a toggle in the
header. The mark in `docs/site/assets/images/chloro.svg` is a leaf whose veins branch like a
syntax tree. Charts and diagrams are inline SVG drawn with the page's colour variables, so
they follow the scheme, and each chart has its data in a table beside it.

## Deployment

`.github/workflows/site.yml` builds the site on every pull request that touches it, and on
every push to `master` builds it and deploys it to GitHub Pages with
`actions/deploy-pages`. It can also be run by hand from the Actions tab.

The deploy job needs the repository's Pages source to be set to **GitHub Actions**
(Settings → Pages → Build and deployment → Source).
