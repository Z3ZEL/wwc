# 0016 — Legal pages as in-app Markdown documents

Status: accepted (2026-09-29)

## Context
Before launch, the site needs what the law expects from a website run by an individual in the EU that hosts
user content: a legal notice (who publishes it, who hosts it, e-Commerce Directive art. 5), Terms of Use
(including how moderation works and how to report illegal content, DSA art. 14 and 16), a Privacy Policy
(GDPR art. 13), a contact point (DSA art. 11 and 12), and the map data attribution. Users must be able to
reach these texts at any time, and accept the Terms when they sign up.

The UI is an egui canvas with one page and side panels (ADR 0008). These texts are long, change on their own
schedule, and more such texts will come (guides, FAQ). The change had to stay in the frontend.

## Decision
- **Markdown files in `frontend/assets/documents/`**, listed in a manifest (`documents.json`: id, title,
  file, `updated` date, footer flag, and `vars`). Adding a document is a file plus a manifest entry, no code.
- **Rendered in the app**: parsed once with `pulldown-cmark` (parser only) into a small block tree, drawn with
  theme styles by `widgets::markdown` in a Document side panel. Relative links between documents open them in
  the panel; web links open in a new tab.
- **Fetched on demand, not compiled in.** Trunk copies the folder next to the app; the manifest is compiled
  in. The wasm doesn't grow with the texts, and `?v=<updated>` in the URL keeps caches from serving an old
  version. Rejected: `include_str!` of every file (simplest and offline, but every future document would
  weigh on the startup download).
- **`{{vars}}` for facts used in several places** (the operator's identity, contact, providers, minimum age,
  retention periods), filled in after parsing so values stay plain text. Placeholders contain `TODO`, and
  `seo-gen` warns about them on every build, like the placeholder domain.
- **Map footer** with the legal links next to the OpenStreetMap attribution (moved there from the map view).
- **Sign-up checkbox** for the minimum age and the Terms (the Privacy Policy is linked as information: the
  legal basis of the account is the contract, not consent). Checked in the browser only.
- **An informative privacy notice, not a consent banner.** The app stores only the session and the notice's
  own state (strictly necessary, ePrivacy art. 5(3)) and has no trackers, so there is no consent to collect.
  The notice shows again when the Privacy Policy's `updated` date changes.
- **A consent panel, built but disabled** (`assets/consent.json`, `"enabled": false`), for when basic analytics
  are added: purposes off by default, "Accept all" and "Reject all" side by side with the same style, a
  per-purpose "Customize" view, the choice kept in the browser with a version and a 6-month lifetime, and a
  "Privacy choices" footer link to change it. Code asks `state.consent_allows(purpose)`. Once enabled, it
  replaces the notice. Rejected: shipping it enabled with nothing to consent to (a banner with no purpose
  trains people to click it away and misstates what the site does).
- English only, like the rest of the UI.

## Consequences
- The operator must fill in the `TODO` vars (name, address, contact, country, data protection authority,
  providers) before launch, and bump `updated` with every change to a document.
- The texts describe the code (storage keys, log and backup retention, what the API makes public). They must
  change together; a test ties the log retention to `backend/pb_settings.json`.
- Acceptance of the Terms is not recorded on the server, and data export, account deletion and illegal-content
  notices from visitors without an account go through email. These are the legal follow-ups in ARCHITECTURE §13.
- Documents need JS and have no URL of their own until hash routing exists; the raw `.md` files are served
  but show unfilled `{{vars}}`.
- The markdown renderer supports a subset (no tables, no images); egui's default font has no bold, so strong
  text uses the primary color.
- One more dependency (`pulldown-cmark`, MIT, no default features).
- Enabling consent is a checklist, not a switch (ARCHITECTURE §5.12): the Privacy Policy must describe the
  analytics in the same change, and the proof of consent stays in the visitor's browser.
