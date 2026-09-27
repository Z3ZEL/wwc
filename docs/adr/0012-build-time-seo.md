# 0012 — Build-time SEO from `seo.json`

Status: accepted (2026-09-27)

## Context
The whole UI is an egui canvas (ADR 0001 already accepted "limited SEO"). `index.html` only had a
`<title>`. Google's renderer handles wasm poorly, and link-preview scrapers (Facebook, Slack, X, Discord)
never run JS. Tags injected by the wasm at runtime would reach neither.

## Decision
- All SEO values live in one file, `frontend/assets/seo.json`, like `theme.json` for visuals.
- A small host binary, `tools/seo-gen` (workspace member; `serde` + `serde_json` only), runs as a Trunk
  `post_build` hook. It validates the file, fills markers in the staged `index.html` (meta description,
  robots, canonical, Open Graph, Twitter card, JSON-LD `WebSite` + `WebApplication`, `theme-color` from
  `theme.json`), writes `robots.txt` and `sitemap.xml`, and copies the favicon and social image. Invalid
  values fail the build; the placeholder domain only warns.
- The crawlable text is a real `<main id="about">` block that describes the app, which the canvas covers once
  it runs. It is not CSS-hidden text (Google may treat that as cloaking), and no-JS visitors see it.
- The running app only sets `document.title` from the open panel (`frontend/src/seo.rs`).

## Consequences
- The first build compiles `seo-gen` for the host once; after that the hook takes milliseconds.
- `site_url` is a placeholder (`https://example.com`) until production deployment exists.
- Only `/` is indexable: panel routes live in the hash. Indexing individual campsites would need
  server-rendered pages, e.g. a PocketBase hook serving `/campsite/<id>` HTML with its own meta tags and a
  sitemap built from the `campsites` collection. That is future work.
