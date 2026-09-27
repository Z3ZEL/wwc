#!/usr/bin/env bash
# Seed dev data: 3 users (+ promote the first to admin), ~50 campsites over France,
# ratings, comments and a few open reports. Safe to re-run: users are reused, campsites are only
# created when the database has none.
#
# Usage: backend/seed/seed.sh            (PB_URL defaults to http://localhost:8090)
# Needs: curl, jq. Reads PB_ADMIN_EMAIL / PB_ADMIN_PASSWORD from .env.
set -euo pipefail

cd "$(dirname "$0")/../.."
[ -f .env ] && set -a && . ./.env && set +a
PB_URL="${PB_URL:-http://localhost:8090}"
PASSWORD="password123"
USERS=(alice bob carol)
N_CAMPSITES="${N_CAMPSITES:-50}"

api() { # api METHOD PATH [TOKEN] [JSON]
  curl -sS -X "$1" "$PB_URL$2" -H "Content-Type: application/json" \
    ${3:+-H "Authorization: $3"} ${4:+-d "$4"}
}

login() { # login EMAIL PASSWORD -> "token id"
  api POST /api/collections/users/auth-with-password "" \
    "{\"identity\":\"$1\",\"password\":\"$2\"}" | jq -r 'if .token then "\(.token) \(.record.id)" else empty end'
}

ADMIN_TOKEN=$(api POST /api/collections/_superusers/auth-with-password "" \
  "{\"identity\":\"$PB_ADMIN_EMAIL\",\"password\":\"$PB_ADMIN_PASSWORD\"}" | jq -r '.token // empty')
[ -n "$ADMIN_TOKEN" ] || { echo "superuser login failed (is PocketBase up at $PB_URL?)"; exit 1; }

declare -A TOKEN ID
for u in "${USERS[@]}"; do
  email="$u@example.com"
  auth=$(login "$email" "$PASSWORD")
  if [ -z "$auth" ]; then
    name="$(tr '[:lower:]' '[:upper:]' <<<"${u:0:1}")${u:1}"
    api POST /api/collections/users/records "" \
      "{\"email\":\"$email\",\"password\":\"$PASSWORD\",\"passwordConfirm\":\"$PASSWORD\",\"name\":\"$name\"}" >/dev/null
    auth=$(login "$email" "$PASSWORD")
  fi
  read -r TOKEN[$u] ID[$u] <<<"$auth"
  # Seed users skip the confirmation email: posting requires a verified email (ADR 0012).
  api PATCH "/api/collections/users/records/${ID[$u]}" "$ADMIN_TOKEN" '{"verified":true}' >/dev/null
  echo "user $email ($PASSWORD) id=${ID[$u]}"
done

# alice is the dev admin
api PATCH "/api/collections/users/records/${ID[alice]}" "$ADMIN_TOKEN" '{"role":"admin"}' >/dev/null

existing=$(api GET "/api/collections/campsites/records?perPage=1&fields=id" "$ADMIN_TOKEN" | jq -r '.totalItems')
if [ "$existing" != "0" ]; then
  echo "$existing campsites already present, skipping campsite seeding."
  exit 0
fi

mapfile -t TAG_IDS < <(api GET "/api/collections/tags/records?perPage=100" | jq -r '.items[].id')
NAMES=(Pine Lake River Meadow Ridge Cliff Forest Valley Brook Hill Dune Glade)
KINDS=("Spot" "Camp" "Clearing" "Bivouac" "Corner" "Retreat")
COMMENTS=("Great place, very quiet." "Lots of mosquitoes in the evening." "Beautiful view at sunrise!" "Hard to find, follow the dirt road." "Perfect for a family weekend.")

for i in $(seq 1 "$N_CAMPSITES"); do
  author=${USERS[$((i % 3))]}
  # Mainland France-ish bounding box
  lat=$(awk -v s="$RANDOM" 'BEGIN{srand(s); printf "%.5f", 43.3 + rand()*6.9}')
  lng=$(awk -v s="$RANDOM" 'BEGIN{srand(s); printf "%.5f", -1.2 + rand()*8.6}')
  tents=$((RANDOM % 10 + 1))
  tags=$(for t in "${TAG_IDS[@]}"; do if (( RANDOM % 2 )); then echo "$t"; fi; done | jq -R . | jq -sc .)
  title="${NAMES[$((RANDOM % ${#NAMES[@]}))]} ${KINDS[$((RANDOM % ${#KINDS[@]}))]} #$i"
  body=$(jq -nc --arg t "$title" --arg a "${ID[$author]}" --argjson lat "$lat" --argjson lng "$lng" \
    --argjson tents "$tents" --argjson tags "$tags" \
    '{title:$t, description:"A seeded campsite for development.", lat:$lat, lng:$lng,
      tent_capacity:$tents, tags:$tags, author:$a}')
  cid=$(api POST /api/collections/campsites/records "${TOKEN[$author]}" "$body" | jq -r '.id')

  # Ratings and a comment from the two other users
  for other in "${USERS[@]}"; do
    [ "$other" = "$author" ] && continue
    if (( RANDOM % 3 )); then
      api POST /api/collections/ratings/records "${TOKEN[$other]}" \
        "{\"campsite\":\"$cid\",\"author\":\"${ID[$other]}\",\"score\":$((RANDOM % 5 + 1))}" >/dev/null
    fi
    if (( RANDOM % 2 )); then
      c=$(jq -nc --arg b "${COMMENTS[$((RANDOM % ${#COMMENTS[@]}))]}" --arg cs "$cid" --arg a "${ID[$other]}" \
        '{campsite:$cs, author:$a, body:$b}')
      api POST /api/collections/comments/records "${TOKEN[$other]}" "$c" >/dev/null
    fi
  done
done

# A few open reports (carol flags a campsite and a comment she didn't write)
cid=$(api GET "/api/collections/campsites/records?perPage=1&filter=author%3D%27${ID[bob]}%27&fields=id" | jq -r '.items[0].id')
api POST /api/collections/reports/records "${TOKEN[carol]}" \
  "{\"reporter\":\"${ID[carol]}\",\"campsite\":\"$cid\",\"reason\":\"wrong_location\",\"details\":\"The pin is on the other side of the river.\"}" >/dev/null
kid=$(api GET "/api/collections/comments/records?perPage=1&filter=author%3D%27${ID[alice]}%27&fields=id" | jq -r '.items[0].id // empty')
if [ -n "$kid" ]; then
  api POST /api/collections/reports/records "${TOKEN[carol]}" \
    "{\"reporter\":\"${ID[carol]}\",\"comment\":\"$kid\",\"reason\":\"spam\"}" >/dev/null
fi

echo "Seeded $N_CAMPSITES campsites and a few reports."
