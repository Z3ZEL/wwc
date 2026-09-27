#!/usr/bin/env bash
# API rule tests: checks the security contract from docs/ARCHITECTURE.md §4.3
# against a running PocketBase. Creates its own throwaway users and records.
#
# Usage: backend/tests/rules.sh          (PB_URL defaults to http://localhost:8090)
# Needs: curl, jq. Reads PB_ADMIN_EMAIL / PB_ADMIN_PASSWORD from .env.
set -uo pipefail

cd "$(dirname "$0")/../.."
[ -f .env ] && set -a && . ./.env && set +a
PB_URL="${PB_URL:-http://localhost:8090}"
RUN="t$(date +%s)$RANDOM"
PASS=0; FAIL=0
BODYF=$(mktemp); trap 'rm -f "$BODYF"' EXIT

# req METHOD PATH TOKEN JSON -> prints HTTP status; response body goes to $BODYF
# (a file, because req usually runs inside $(...) subshells)
req() {
  curl -sS -o "$BODYF" -w '%{http_code}' -X "$1" "$PB_URL$2" \
    -H "Content-Type: application/json" ${3:+-H "Authorization: $3"} ${4:+-d "$4"}
}

# req_multipart METHOD PATH TOKEN [curl -F args...] -> like req, but multipart/form-data (file uploads)
req_multipart() {
  local method="$1" path="$2" token="$3"; shift 3
  local args=(); for f in "$@"; do args+=(-F "$f"); done
  curl -sS -o "$BODYF" -w '%{http_code}' -X "$method" "$PB_URL$path" ${token:+-H "Authorization: $token"} "${args[@]}"
}

expect() { # expect DESCRIPTION EXPECTED_STATUS ACTUAL_STATUS
  if [ "$2" = "$3" ]; then PASS=$((PASS+1)); echo "  ok   $1"
  else FAIL=$((FAIL+1)); echo "  FAIL $1 (expected $2, got $3) $(head -c 300 "$BODYF")"; fi
}

req POST /api/collections/_superusers/auth-with-password "" \
  "{\"identity\":\"$PB_ADMIN_EMAIL\",\"password\":\"$PB_ADMIN_PASSWORD\"}" >/dev/null
SU=$(jq -r .token "$BODYF")

new_user() { # new_user NAME [unverified] -> "token id". Verified (confirmed email) unless asked otherwise.
  local email="$1-$RUN@example.com" id
  req POST /api/collections/users/records "" \
    "{\"email\":\"$email\",\"password\":\"password123\",\"passwordConfirm\":\"password123\",\"name\":\"$1\"}" >/dev/null
  id=$(jq -r .id "$BODYF")
  [ "${2:-}" = unverified ] || req PATCH "/api/collections/users/records/$id" "$SU" '{"verified":true}' >/dev/null
  req POST /api/collections/users/auth-with-password "" \
    "{\"identity\":\"$email\",\"password\":\"password123\"}" >/dev/null
  jq -r '"\(.token) \(.record.id)"' "$BODYF"
}

read -r A A_ID <<<"$(new_user owner)"
read -r B B_ID <<<"$(new_user other)"
read -r U U_ID <<<"$(new_user unverified unverified)"
read -r M M_ID <<<"$(new_user moderator)"
req PATCH "/api/collections/users/records/$M_ID" "$SU" '{"role":"admin"}' >/dev/null
req POST /api/collections/users/auth-refresh "$M" >/dev/null   # token now carries the admin role
M=$(jq -r .token "$BODYF")

SITE="{\"title\":\"Rule test site\",\"lat\":45,\"lng\":3,\"tent_capacity\":3,\"author\":\"$A_ID\"}"

echo "users"
expect "signup cannot set role"            400 "$(req POST /api/collections/users/records "" "{\"email\":\"x-$RUN@example.com\",\"password\":\"password123\",\"passwordConfirm\":\"password123\",\"role\":\"admin\"}")"
expect "user cannot promote themselves"    404 "$(req PATCH "/api/collections/users/records/$A_ID" "$A" '{"role":"admin"}')"
expect "user can edit own name"            200 "$(req PATCH "/api/collections/users/records/$A_ID" "$A" '{"name":"Owner"}')"
expect "user cannot verify themselves"     400 "$(req PATCH "/api/collections/users/records/$U_ID" "$U" '{"verified":true}')"
expect "user cannot edit another user"     404 "$(req PATCH "/api/collections/users/records/$B_ID" "$A" '{"name":"x"}')"
expect "anonymous can view a profile"      200 "$(req GET "/api/collections/users/records/$A_ID" "")"
[ "$(jq -r '.email // ""' "$BODYF")" = "" ]; expect "profile does not expose email" 0 "$?"

echo "campsites"
expect "anonymous cannot create"           400 "$(req POST /api/collections/campsites/records "" "$SITE")"
expect "cannot create as someone else"     400 "$(req POST /api/collections/campsites/records "$B" "$SITE")"
expect "cannot create a hidden campsite"   400 "$(req POST /api/collections/campsites/records "$A" "${SITE%\}},\"hidden\":true}")"
expect "unverified user cannot create"     400 "$(req POST /api/collections/campsites/records "$U" "${SITE/$A_ID/$U_ID}")"
expect "owner can create"                  200 "$(req POST /api/collections/campsites/records "$A" "$SITE")"
CID=$(jq -r .id "$BODYF")
expect "anonymous can list"                200 "$(req GET "/api/collections/campsites/records?perPage=1" "")"
expect "anonymous can view"                200 "$(req GET "/api/collections/campsites/records/$CID" "")"
expect "other user cannot update"          404 "$(req PATCH "/api/collections/campsites/records/$CID" "$B" '{"title":"Hacked"}')"
expect "owner can update"                  200 "$(req PATCH "/api/collections/campsites/records/$CID" "$A" '{"title":"Renamed site"}')"
expect "owner cannot change author"        404 "$(req PATCH "/api/collections/campsites/records/$CID" "$A" "{\"author\":\"$B_ID\"}")"
expect "owner cannot set hidden"           404 "$(req PATCH "/api/collections/campsites/records/$CID" "$A" '{"hidden":true}')"
expect "invalid tent capacity rejected"    400 "$(req PATCH "/api/collections/campsites/records/$CID" "$A" '{"tent_capacity":11}')"
expect "admin cannot edit content"         403 "$(req PATCH "/api/collections/campsites/records/$CID" "$M" '{"title":"Admin edit"}')"

echo "ratings"
expect "cannot rate own campsite"          400 "$(req POST /api/collections/ratings/records "$A" "{\"campsite\":\"$CID\",\"author\":\"$A_ID\",\"score\":5}")"
expect "anonymous cannot rate"             400 "$(req POST /api/collections/ratings/records "" "{\"campsite\":\"$CID\",\"author\":\"$B_ID\",\"score\":5}")"
expect "unverified user cannot rate"       400 "$(req POST /api/collections/ratings/records "$U" "{\"campsite\":\"$CID\",\"author\":\"$U_ID\",\"score\":4}")"
expect "other user can rate"               200 "$(req POST /api/collections/ratings/records "$B" "{\"campsite\":\"$CID\",\"author\":\"$B_ID\",\"score\":4}")"
RID=$(jq -r .id "$BODYF")
expect "second rating rejected"            400 "$(req POST /api/collections/ratings/records "$B" "{\"campsite\":\"$CID\",\"author\":\"$B_ID\",\"score\":2}")"
expect "score out of range rejected"       400 "$(req PATCH "/api/collections/ratings/records/$RID" "$B" '{"score":6}')"
expect "rater can update score"            200 "$(req PATCH "/api/collections/ratings/records/$RID" "$B" '{"score":2}')"
expect "owner cannot change others rating" 404 "$(req PATCH "/api/collections/ratings/records/$RID" "$A" '{"score":5}')"
expect "stats view is public"              200 "$(req GET "/api/collections/campsite_stats/records/$CID" "")"
[ "$(jq -r .rating_count "$BODYF")" = "1" ]; expect "stats count the rating" 0 "$?"

echo "comments"
expect "cannot comment on own campsite"    400 "$(req POST /api/collections/comments/records "$A" "{\"campsite\":\"$CID\",\"author\":\"$A_ID\",\"body\":\"mine\"}")"
expect "unverified user cannot comment"    400 "$(req POST /api/collections/comments/records "$U" "{\"campsite\":\"$CID\",\"author\":\"$U_ID\",\"body\":\"Hi\"}")"
expect "other user can comment"            200 "$(req POST /api/collections/comments/records "$B" "{\"campsite\":\"$CID\",\"author\":\"$B_ID\",\"body\":\"Nice\"}")"
KID=$(jq -r .id "$BODYF")
expect "author cannot hide own comment"    404 "$(req PATCH "/api/collections/comments/records/$KID" "$B" '{"hidden":true}')"
expect "campsite owner cannot delete it"   404 "$(req DELETE "/api/collections/comments/records/$KID" "$A")"
expect "admin can hide a comment"          200 "$(req PATCH "/api/collections/comments/records/$KID" "$M" '{"hidden":true}')"
expect "hidden comment invisible to anon"  404 "$(req GET "/api/collections/comments/records/$KID" "")"
expect "hidden comment visible to author"  200 "$(req GET "/api/collections/comments/records/$KID" "$B")"
expect "author can delete own comment"     204 "$(req DELETE "/api/collections/comments/records/$KID" "$B")"

echo "reports"
R="/api/collections/reports/records"
req POST /api/collections/comments/records "$M" "{\"campsite\":\"$CID\",\"author\":\"$M_ID\",\"body\":\"First\"}" >/dev/null
K1=$(jq -r .id "$BODYF")
req POST /api/collections/comments/records "$M" "{\"campsite\":\"$CID\",\"author\":\"$M_ID\",\"body\":\"Second\"}" >/dev/null
K2=$(jq -r .id "$BODYF")
expect "anonymous cannot report"           400 "$(req POST "$R" "" "{\"reporter\":\"$B_ID\",\"campsite\":\"$CID\",\"reason\":\"spam\"}")"
expect "unverified user cannot report"     400 "$(req POST "$R" "$U" "{\"reporter\":\"$U_ID\",\"campsite\":\"$CID\",\"reason\":\"spam\"}")"
expect "cannot report as someone else"     400 "$(req POST "$R" "$B" "{\"reporter\":\"$A_ID\",\"campsite\":\"$CID\",\"reason\":\"spam\"}")"
expect "cannot set status on create"       400 "$(req POST "$R" "$B" "{\"reporter\":\"$B_ID\",\"campsite\":\"$CID\",\"reason\":\"spam\",\"status\":\"resolved\"}")"
expect "user can report a campsite"        200 "$(req POST "$R" "$B" "{\"reporter\":\"$B_ID\",\"campsite\":\"$CID\",\"reason\":\"wrong_location\",\"details\":\"Wrong valley\"}")"
REP=$(jq -r .id "$BODYF")
[ "$(jq -r .status "$BODYF")" = "open" ]; expect "new report is open" 0 "$?"
expect "cannot report twice"               400 "$(req POST "$R" "$B" "{\"reporter\":\"$B_ID\",\"campsite\":\"$CID\",\"reason\":\"spam\"}")"
expect "user can report a comment"         200 "$(req POST "$R" "$B" "{\"reporter\":\"$B_ID\",\"comment\":\"$K1\",\"reason\":\"inappropriate\"}")"
expect "user can report another comment"   200 "$(req POST "$R" "$B" "{\"reporter\":\"$B_ID\",\"comment\":\"$K2\",\"reason\":\"spam\"}")"
expect "cannot report own campsite"        400 "$(req POST "$R" "$A" "{\"reporter\":\"$A_ID\",\"campsite\":\"$CID\",\"reason\":\"spam\"}")"
expect "cannot report own comment"         400 "$(req POST "$R" "$M" "{\"reporter\":\"$M_ID\",\"comment\":\"$K1\",\"reason\":\"spam\"}")"
expect "cannot report two targets"         400 "$(req POST "$R" "$A" "{\"reporter\":\"$A_ID\",\"campsite\":\"$CID\",\"comment\":\"$K1\",\"reason\":\"spam\"}")"
expect "cannot report without a target"    400 "$(req POST "$R" "$A" "{\"reporter\":\"$A_ID\",\"reason\":\"spam\"}")"
expect "no wrong location on a comment"    400 "$(req POST "$R" "$A" "{\"reporter\":\"$A_ID\",\"comment\":\"$K1\",\"reason\":\"wrong_location\"}")"
expect "user can list reports"             200 "$(req GET "$R" "$A")"
[ "$(jq -r .totalItems "$BODYF")" = "0" ]; expect "user cannot see others reports" 0 "$?"
expect "reporter can view own report"      200 "$(req GET "$R/$REP" "$B")"
expect "anonymous cannot view a report"    404 "$(req GET "$R/$REP" "")"
expect "reporter cannot change status"     404 "$(req PATCH "$R/$REP" "$B" '{"status":"dismissed"}')"
expect "admin can change status"           200 "$(req PATCH "$R/$REP" "$M" '{"status":"resolved"}')"
expect "admin can list reports"            200 "$(req GET "$R?filter=reporter%3D%27$B_ID%27" "$M")"
[ "$(jq -r .totalItems "$BODYF")" = "3" ]; expect "admin sees all reports" 0 "$?"

echo "photos"
PNG="backend/tests/fixtures/pixel.png;type=image/png"
TXT=$(mktemp); echo "not an image" >"$TXT"
BIG=$(mktemp); head -c $((11 * 1024 * 1024)) /dev/zero >"$BIG"
trap 'rm -f "$BODYF" "$TXT" "$BIG"' EXIT
P="/api/collections/campsites/records"
expect "owner can create with a photo"     200 "$(req_multipart POST "$P" "$A" "@jsonPayload=$SITE" "photos+=@$PNG")"
PID=$(jq -r .id "$BODYF"); P="$P/$PID"
[ "$(jq '.photos | length' "$BODYF")" = "1" ]; expect "created campsite has 1 photo" 0 "$?"
expect "cannot create with photo as other" 400 "$(req_multipart POST /api/collections/campsites/records "$B" "@jsonPayload=$SITE" "photos+=@$PNG")"
expect "owner can add two more photos"     200 "$(req_multipart PATCH "$P" "$A" "photos+=@$PNG" "photos+=@$PNG")"
[ "$(jq '.photos | length' "$BODYF")" = "3" ]; expect "campsite has 3 photos" 0 "$?"
FIRST=$(jq -r '.photos[0]' "$BODYF")
expect "a 4th photo is rejected"           400 "$(req_multipart PATCH "$P" "$A" "photos+=@$PNG")"
expect "other user cannot add a photo"     404 "$(req_multipart PATCH "$P" "$B" "photos+=@$PNG")"
expect "anonymous cannot add a photo"      404 "$(req_multipart PATCH "$P" "" "photos+=@$PNG")"
expect "admin cannot add a photo"          403 "$(req_multipart PATCH "$P" "$M" "photos+=@$PNG")"
expect "admin cannot remove a photo"       403 "$(req PATCH "$P" "$M" "{\"photos-\":[\"$FIRST\"]}")"
expect "other user cannot remove a photo"  404 "$(req PATCH "$P" "$B" "{\"photos-\":[\"$FIRST\"]}")"
expect "owner can remove a photo"          200 "$(req PATCH "$P" "$A" "{\"photos-\":[\"$FIRST\"]}")"
[ "$(jq '.photos | length' "$BODYF")" = "2" ]; expect "campsite has 2 photos" 0 "$?"
expect "owner can replace in one request"  200 "$(req_multipart PATCH "$P" "$A" "@jsonPayload={\"title\":\"With photos\",\"photos-\":[\"$(jq -r '.photos[0]' "$BODYF")\"]}" "photos+=@$PNG")"
[ "$(jq -r '"\(.title) \(.photos | length)"' "$BODYF")" = "With photos 2" ]; expect "jsonPayload and files both applied" 0 "$?"
expect "non-image upload rejected"         400 "$(req_multipart PATCH "$P" "$A" "photos+=@$TXT;type=text/plain")"
expect "oversized upload rejected"         400 "$(req_multipart PATCH "$P" "$A" "photos+=@$BIG;type=image/png")"
req GET "$P" "" >/dev/null; PHOTO=$(jq -r '.photos[0]' "$BODYF"); COLL=$(jq -r .collectionId "$BODYF")
expect "anonymous can fetch a thumbnail"   200 "$(curl -sS -o /dev/null -w '%{http_code}' "$PB_URL/api/files/$COLL/$PID/$PHOTO?thumb=320x240")"
expect "owner can delete campsite+photos"  204 "$(req DELETE "$P" "$A")"

echo "moderation"
expect "admin can hide a campsite"         200 "$(req PATCH "/api/collections/campsites/records/$CID" "$M" '{"hidden":true}')"
expect "hidden campsite invisible to anon" 404 "$(req GET "/api/collections/campsites/records/$CID" "")"
expect "hidden campsite visible to owner"  200 "$(req GET "/api/collections/campsites/records/$CID" "$A")"

echo "cleanup"
expect "other user cannot delete"          404 "$(req DELETE "/api/collections/campsites/records/$CID" "$B")"
expect "owner can delete"                  204 "$(req DELETE "/api/collections/campsites/records/$CID" "$A")"
for id in "$A_ID" "$B_ID" "$M_ID"; do req DELETE "/api/collections/users/records/$id" "$SU" >/dev/null; done

echo
echo "$PASS passed, $FAIL failed"
[ "$FAIL" = 0 ]
