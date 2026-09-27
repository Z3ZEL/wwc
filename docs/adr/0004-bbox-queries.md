# 0004 — Load campsites by viewport

Status: accepted (2026-09-25)

## Context
Loading every campsite doesn't scale.

## Decision
The map reports its bounding box (debounced 300 ms). The API returns at most 200 markers with only `id,title,lat,lng,author`. Details load when a campsite is opened. A generation counter drops stale responses.

## Consequences
Needs the `(lat, lng)` index. Very dense areas will need clustering later.
