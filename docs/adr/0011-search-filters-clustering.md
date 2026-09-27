# 0011 — Search by name, global map filters, marker clustering

Status: accepted (2026-09-26)

## Context
With more campsites the map gets crowded, and there was no way to find a campsite by name or to narrow the
map down to campsites with given tags or a tent capacity range. The top bar had room for a search field,
and ARCHITECTURE §5.5 already planned a filter panel and client-side clustering.

## Decision
- **Search** is a text field in the top bar. Enter runs `title ~ '<query>'` over **all** campsites (not the
  viewport), sorted by title, 50 results at most, and opens a new `Search` side panel that lists them. A
  result click centers the map on the campsite (`AppState.map_focus`, taken by the map) and opens it; the
  Campsite panel links back to the results. Responses carry a generation counter, like markers.
- **Search text in a filter:** the one exception to "filters only from numbers and ids" (§5.4).
  `api::clean_search` drops `'`, `"`, `\`, backtick and control characters, collapses whitespace and caps
  the length at 100, so the text can't leave its quoted string. `%`/`_` stay: at worst they act as LIKE
  wildcards. No full-text index: a substring match over one text column is enough at this size.
- **Filters** (`api::CampsiteFilter` in `AppState.filters`) are global: they apply to the markers and to
  search, whether or not a search is running, and stay on when the `Filters` panel is closed. Tags are
  AND-combined; tent capacity is an inclusive range where 10 = "10+" (no upper bound). Every change is an
  `Action` that reloads markers and the current search.
- **Tag filter syntax:** `tags ~ 'ID'`, one clause per tag. With the pinned PocketBase (0.40.4),
  `tags ?= 'ID'` matched nothing and two `tags.id ?= …` clauses joined with `&&` never matched together.
  `~` does a substring match on the stored JSON array; ids are fixed-length alphanumeric, so one id can't
  match inside another.
- **Clustering** is client-side, in screen space (`map/cluster.rs`, pure and unit-tested): greedy, every
  member within `map.cluster_distance` of the cluster's first marker. It only depends on relative screen
  positions, so panning never reshuffles clusters. A cluster draws as a bigger circle with a count; a
  click zooms in by `cluster_zoom_step` around it. From `cluster_max_zoom` on, every campsite is drawn
  alone. The selected campsite is never put in a cluster. `map/` is split so the pure part builds on the host.

## Consequences
- No schema or rule change.
- Clustering only groups the markers the bbox query returned (at most `MARKERS_PER_REQUEST` = 200). Very
  dense zoomed-out views can still miss campsites; server-side clustering stays on the roadmap.
- The top bar count stays the total, unfiltered.
- The search and filter state is in memory only. `#/search` and `#/filters` join the planned hash routes.
