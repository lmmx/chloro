# 2026-10-17: Documentation for agents and LLMs

#101 makes the site easier for coding agents and LLM tools to find and read, using only what
the pinned Zensical (0.0.71) provides. A research pass first established what Zensical
supports and which mechanisms anything actually consumes; the findings, with their sources,
are below so the choices can be revisited.

## Current State

- **Zensical's `llmstxt` plugin** (native since 0.0.67, a reimplementation of mkdocs-llmstxt)
  is configured in `zensical.toml`. Each build writes `llms.txt` (the pages in nav order with
  descriptions, the maintainers' pages and history under *Optional*), `llms-full.txt`, and a
  Markdown copy of each page at `<page>/index.md`, converted from the rendered HTML (snippets
  expanded, links absolute), so it cannot drift from the pages. Since 0.0.70 each page's head
  carries `<link rel="alternate" type="text/markdown">` and `<link rel="describedby">` to
  `llms.txt` (https://zensical.org/docs/compatibility/mkdocs/plugins/#llmstxt).
- **Descriptions.** Every page has a front-matter `description`, its meta description; the
  plugin cannot read it, so the same text is repeated in the `llmstxt` sections, and
  `check_site.py` fails on a page without one, one missing from the sections, one listed with
  another description, or one without a Markdown copy in the build.
- **Titles.** The CLI reference was titled "chloro" (now "Command-line reference") and the
  home page's browser title "chloro - chloro".
- **Copy as Markdown** (`content.action.copy`) on every page.
- **`AGENTS.md`** at the root (build and test commands, the rules a change must keep), read by
  Codex, Cursor, Copilot's agent, Gemini CLI and others (https://agents.md/), and by Claude
  Code when there is no `CLAUDE.md` (https://code.claude.com/docs/en/memory). The README links
  it with `llms.txt` and `llms-full.txt`.
- **What consumes these.** Coding agents ask for Markdown: Claude Code's fetch sends
  `Accept: text/markdown`, as do Cursor and OpenCode
  (https://www.checklyhq.com/blog/state-of-ai-agent-content-negotation/). GitMCP and Context7
  read `llms.txt`. Search does not: Google says AI features need no special files
  (https://developers.google.com/search/docs/appearance/ai-features), and Ahrefs found 97% of
  `llms.txt` files received no requests
  (https://www.searchenginejournal.com/97-of-llms-txt-files-got-no-requests-ahrefs-data-shows/579478/).
  No agent is documented to follow `rel="alternate"` Markdown links. The llms.txt spec allows a
  subpath such as `/chloro/llms.txt` (https://llmstxt.org/).

## Missing

- **Content negotiation and `Link:` headers.** GitHub Pages ignores `Accept` and cannot set
  headers, so an agent asking for Markdown gets HTML.
- **`robots.txt`**, which only counts at the domain root (`lmmx.github.io`, not this
  repository). A missing one allows everything.
- **Sitemap `lastmod`**, which Zensical does not emit; submitting the sitemap to Search Console
  and Bing Webmaster Tools is a manual step left to the owner.
- Left out on purpose: JSON-LD (no documented consumer; Google's software rich result needs
  price and ratings), Open Graph cards (the `social` plugin renders images and fetches fonts at
  build time), "Open in Claude/ChatGPT" buttons (no official prefill URL found).
