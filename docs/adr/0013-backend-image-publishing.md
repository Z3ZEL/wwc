# 0013 — Backend image published to GHCR on release

Status: accepted (2026-09-27)

## Context
ADR 0012 made the backend a self-contained image, but it was built wherever it was deployed ("No CI
publishing yet"). Every host needed the source and a Docker build, and nothing tied a running backend to a
release.

## Decision
- `.github/workflows/backend-image.yml` builds `backend/Dockerfile` and pushes it to the GitHub Container
  Registry as `ghcr.io/<owner>/wwc-backend` when a GitHub Release is published, or by hand
  (`workflow_dispatch`). Authentication is the workflow's `GITHUB_TOKEN`; no extra secrets.
- Tags: `X.Y.Z` and `X.Y` from the release tag `vX.Y.Z`, `latest` for non-prerelease releases only,
  `sha-<short>` always, the branch name for manual runs. OCI labels link the package to the repo, and a
  build-provenance attestation is pushed with the image.
- Platforms: `linux/amd64` and `linux/arm64/v8` (multi-arch manifest). arm64 is built under QEMU; that's
  cheap because the Dockerfile only downloads PocketBase for `TARGETARCH`, nothing is compiled.
- Releases only, not every push to master: an image is something we deploy, and migrations run on the
  host at startup (`migrate up`), so each published tag is a deliberate schema version.
- The frontend is not published as an image: its API URL is compiled in (ADR 0012), so it's built per
  environment. A full release triggers its deploy on the static host instead (ADR 0015).

## Consequences
- Hosts pull a versioned image instead of building from source; rolling back = deploying the previous tag
  (only if no migration ran in between; migrations are forward-only).
- The GHCR package is private on first publish: make it public in the package settings, or give the host a
  registry credential (a token with `read:packages`).
- Building locally (`docker build backend`) still works unchanged.
