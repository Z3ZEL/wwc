# 0022 — Audience measurement with Umami Cloud, only with consent

Status: proposed (2026-10-11).

## Context
We don't know how the site is used: how many people visit, whether they open campsites, search or use the
filters, how many sign up and add campsites. The consent panel (ADR 0016, ARCHITECTURE §5.12) was built for
this and left disabled until analytics exist.

The app is one egui canvas: no routes, and panels live in the URL hash. A script that tracks page loads and
URL changes would see one pageview per visit and nothing else, so useful numbers have to come from events
the app sends itself.

## Decision
- **Umami Cloud, free Hobby plan** (100k events a month, 6 months of data). It is cookieless, stores nothing
  in the browser, doesn't store IP addresses, has a dashboard, and its tracker is a small script with a JS
  API (`umami.track`). No server to run.
- **Only with consent.** `consent.json` is enabled. `app.rs` syncs `analytics::Analytics` with
  `consent_allows("analytics")` at startup and after every frame's actions. Nothing is loaded from Umami
  before "Accept"; after a withdrawal the app stops sending, without a reload.
- **The script is added at runtime** by `web::analytics::start`, not in `index.html`, with
  `data-auto-track="false"`, `data-do-not-track`, `data-exclude-search` and `data-exclude-hash`. It counts a
  pageview once loaded. With auto-track off it sends nothing on its own, so it can stay loaded after a
  withdrawal.
- **A short list of events, mapped by pure, host-tested functions** (`analytics::for_action` /
  `for_event`): `campsite-open` (from map or search), `search`, `filter`, `locate`, and on success
  `signup`, `campsite-create`, `photos-add`, `rating`, `comment`, `report`. Only actions the visitor
  took in the UI count, not the follow-ups that `state::apply` emits. **No personal data in events:** no
  record ids, search text, emails or positions; at most one fixed label or a count.
- **Configured at build time:** `WWC_UMAMI_WEBSITE_ID` (a UUID) and optionally `WWC_UMAMI_SCRIPT_URL`
  (default `https://cloud.umami.is/script.js`). Without a website id, consent stays off
  (`ConsentConfig::needing`): no panel, no `wwc_consent` key, and the privacy notice as before. Dev builds
  and forks therefore don't ask about analytics they don't have.
- Rejected:
  - Plausible (paid only), GA4 (cookies, heavy, advertising company) and PostHog (more than we need).
  - Self-hosted Umami or Plausible: a second server and database to run for a few counters.
  - Events in PocketBase: no third party, but we would build the storage, aggregation and charts ourselves,
    and the traffic would load the backend.
  - Server or CDN logs only: no consent needed, but they can't see what happens inside the canvas.

## Consequences
- The Privacy Policy describes the measurement (section 5), lists Umami as a processor (vars
  `analytics_provider`, `analytics_retention`) and a US transfer. Its `updated` date changed, so returning
  visitors see the consent panel.
- A new event or purpose updates the policy; a new purpose also bumps `consent.json` `version`.
- The free plan's 100k events a month bound how many events a visit may send. Keep the list short.
- If a Content-Security-Policy is added, it must allow the script host and `api-gateway.umami.dev`.
- Not done: server-side proof of consent (§5.12 Limit), funnels and retention reports, events for errors.
