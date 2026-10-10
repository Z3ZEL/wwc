#!/bin/sh
# Production build of the frontend: a static site (index.html + JS + wasm) in frontend/dist/.
# Usage: WWC_API_URL=https://api.example.com scripts/build-frontend.sh
#   WWC_API_URL  public PocketBase URL, baked into the wasm (required)
#   PUBLIC_URL   path the site is served under (default /)
#   WWC_BUILD_ID cache key of the files generated at build time (default: the UTC build time)
#   GITHUB_TOKEN optional, read-only: lifts GitHub's rate limit for the release notes (tools/changelog-gen)
# Needs the pinned toolchain (rust-toolchain.toml) and trunk: scripts/install-frontend-tools.sh installs both.
# Or run it in the dev image:
#   docker compose run --rm -e WWC_API_URL=... frontend /app/scripts/build-frontend.sh
set -eu
: "${WWC_API_URL:?set WWC_API_URL to the public PocketBase URL, e.g. https://api.example.com}"
export WWC_API_URL
# Changes with every build, even of the same commit (a release usually rebuilds it), so
# browsers fetch the new releases.json.
WWC_BUILD_ID="${WWC_BUILD_ID:-$(date -u +%Y%m%d%H%M%S)}"
export WWC_BUILD_ID
cd "$(dirname "$0")/.."
. scripts/rust-env.sh
cd frontend
trunk build --release --public-url "${PUBLIC_URL:-/}"
echo "Static site ready in frontend/dist/ (API: $WWC_API_URL)"
