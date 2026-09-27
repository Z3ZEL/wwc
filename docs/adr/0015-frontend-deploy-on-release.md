# 0015 — Frontend deployed by the release workflow

Status: accepted (2026-09-27)

## Context
The frontend is a static build hosted on Render (ADR 0012). The backend image is published by the release
workflow (ADR 0013), but the frontend was deployed separately, so a release could ship a backend without
the matching frontend, or the other way round.

## Decision
- The release workflow (`.github/workflows/backend-image.yml`, "Release") gets a `deploy-frontend` job that
  POSTs to Render's deploy hook. The hook URL (it embeds a key) lives in the `DEPLOY_HOOK` repository
  secret, never in the repo.
- It runs after the backend image is pushed (`needs: publish`), so the frontend never goes live ahead of
  the backend it talks to.
- Full releases only. Prereleases and manual runs publish the image but don't deploy the frontend.
- The hook is called as-is: Render builds the HEAD of the branch it tracks, not the release tag.

## Consequences
- Release from the branch Render tracks, with nothing merged after the tag, or Render deploys a different
  commit than the release.
- A missing `DEPLOY_HOOK` secret fails the job (the image is already pushed by then).
- Rotating the hook = regenerate it in Render and update the secret.
