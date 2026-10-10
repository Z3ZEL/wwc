# 0020 — Welcome card with release notes synced from GitHub Releases

Status: proposed (2026-10-10).

## Context
Visitors land on the bare map. Nothing tells a first-time visitor what the site is for or how to use it,
and nothing tells a returning one what changed. Release notes are already written on GitHub: every
release has a tag, a name and a Markdown body, and a full release triggers the frontend deploy (ADR 0015).

The UI is one page with side panels (ADR 0008). The map controls (zoom buttons) sit in the top-left corner,
"Locate me" in the bottom-right (ADR 0019).

## Decision
- **A floating card in the top-left corner of the map** (`ui/welcome_card.rs`), drawn like the privacy
  notice: an `Area` constrained to the map, over a semi-transparent fill (`colors.welcome_bg`). It is the
  second exception to "everything is a side panel", after the photo viewer. It has two tabs:
  - **Welcome**: the `welcome` document (`assets/documents/welcome.md`), drawn with the documents' Markdown
    renderer. Links to other documents open them in the Document panel.
  - **Updates**: the latest releases, newest first: tag, title, date, a "Pre-release" badge, the notes in
    Markdown, and "View on GitHub".
- **The +/− zoom buttons move to the right of the card**, and follow it when it collapses. The card's width
  shrinks so they always fit. Their sizes move to the theme (`map.zoom_button_size`, `zoom_button_gap`).
- **No storage.** The card opens on Welcome on wide screens and starts collapsed on phones, on every visit.
  Collapsing it and the open tab last for the session only. A remembered state or a "new release" dot would
  need a new browser storage key, which §5.3 only allows for strictly necessary data, plus a Privacy Policy
  change that shows the privacy notice to everyone again.
- **Release notes are synced at build time.** `tools/changelog-gen`, a second Trunk `post_build` hook,
  writes `releases.json` into the staging dir:
  - Release builds (`TRUNK_PROFILE=release`: `scripts/build-frontend.sh`, the Render deploy) call
    `GET api.github.com/repos/<repo>/releases`, with `GITHUB_TOKEN` as a bearer token when set.
  - Dev builds use a recorded fixture: `trunk serve` rebuilds on every save and never calls GitHub.
    `WWC_CHANGELOG=fetch|fixture|off` overrides the choice.
  - Drafts are dropped. Prereleases are kept, labelled (both releases so far are prereleases), and the list
    is capped. Settings are in `assets/updates.json`.
  - **A failed call (network, rate limit, bad answer) warns and writes an empty list.** Release notes never
    block a deploy.
  - It uses `ureq`, already in the tree through `ehttp`'s native backend, so no new crate.
- **The app fetches `releases.json?v=<build id>`** the first time the Updates tab is shown (`api::fetch_text`,
  `Event::Changelog`, `AppState::changelog`). `WWC_BUILD_ID` is the build time, set by
  `scripts/build-frontend.sh` and read by `config::build_id`. It is not the commit: a release usually
  rebuilds the same commit, and the file must still be fetched again.
- Rejected:
  - **Calling GitHub from the browser.** It would send every visitor's IP to GitHub, a new third party for
    the Privacy Policy, and hit the 60-calls-an-hour limit per visitor.
  - **A bot commit of the notes after each release.** ADR 0015 needs nothing merged after the tag, because
    Render builds the branch head.
  - **A hand-written changelog file.** Notes would be written twice and drift apart.

## Consequences
- A full release shows up in the app once its deploy finishes. A prerelease doesn't deploy (ADR 0015), so it
  shows up at the next deploy. So does an edit to a release's notes on GitHub.
- The build host needs outbound HTTPS to `api.github.com`; it already downloads the toolchain and Trunk.
  Unauthenticated calls share GitHub's limit of 60 an hour per IP. If the build warns about it, set a
  read-only `GITHUB_TOKEN` on the host.
- The first release build compiles `ureq`'s TLS stack for the host tool (a one-time cost per clean target dir).
- Release notes use the documents' Markdown subset: GitHub's generated notes (headings, lists, links) render.
  Tables, images and HTML don't. Long notes are cut at `max_notes_chars`, with the full text on GitHub.
- `releases.json` is served from the same host as the app: visitors contact GitHub only when they click
  "View on GitHub". The Privacy Policy doesn't change.
- The zoom buttons follow the card one frame later (the map is drawn first); the app repaints at once, so
  it isn't visible.
- Not done: a "new" dot on Updates, remembering the collapsed state, release notes in the crawlable page,
  and filtering notes by audience (they are written for developers today).
