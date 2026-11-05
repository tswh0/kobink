#!/usr/bin/env bash
set -Eeuo pipefail

DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
BASE=${PUBLIC_URL:-http://127.0.0.1:3000}
KEY=${API_KEY:-smoketestkey123}
LIB=${LIBRARY_PATH:-/tmp/kobink-smoke/library}
COVERS=${COVER_CACHE_DIR:-/tmp/kobink-smoke/covers}
BIN=${BIN:-./target/release/kobink}

WORK=$(mktemp -d)
LOG=$WORK/server.log
HDRS=$WORK/headers.txt
checks=0

rm -rf "$LIB" "$COVERS"
mkdir -p "$LIB" "$COVERS"

export API_KEY=$KEY
export PUBLIC_URL=$BASE
export LIBRARY_PATH=$LIB
export COVER_CACHE_DIR=$COVERS
export RUST_LOG=${RUST_LOG:-info}

section() { printf '\n%s\n' "$1"; }
say() { printf '  ok   %-24s %s\n' "$1" "$2"; }

dump() {
    trap - ERR
    set +e
    {
        echo "--- last 40 log lines ---"
        tail -40 "$LOG" 2>/dev/null
    } >&2
}

fail() {
    echo "::error title=smoke::$1" >&2
    dump
    exit 1
}

trap 'code=$?; echo "::error title=smoke::failure at line $LINENO (exit $code): $BASH_COMMAND" >&2; dump; exit $code' ERR

check() {
    local name=$1 got=$2 want=$3
    [ "$got" = "$want" ] || fail "$name: wanted '$want', got '$got'"
    checks=$((checks + 1))
    say "$name" "$got"
}

json() {
    local name=$1 payload=$2 filter=$3 want=${4:-true} got
    got=$(printf '%s' "$payload" | jq -r "$filter") || fail "$name: jq rejected the filter '$filter'"
    check "$name" "$got" "$want"
}

grab() {
    local name=$1 payload=$2 filter=$3 got
    got=$(printf '%s' "$payload" | jq -r "$filter") || fail "$name: jq rejected the filter '$filter'"
    if [ "$got" = "null" ] || [ -z "$got" ]; then
        fail "$name: missing in the payload"
    fi
    printf -v "$name" '%s' "$got"
}

status() { curl -s -o /dev/null -w '%{http_code}' "$@"; }
api() { printf '%s/%s%s' "$BASE" "$KEY" "$1"; }

header() {
    local name=$1 url=$2
    shift 2
    curl -s -o /dev/null -D - "$@" "$url" | tr -d '\r' |
        awk -v key="$name" 'tolower($1) == (key ":") { print $2 }'
}

dims() {
    python3 - "$1" <<'PY'
import struct
import sys
import urllib.request

data = urllib.request.urlopen(sys.argv[1]).read()
i = 2
while i < len(data) - 9:
    if data[i] != 0xFF:
        i += 1
        continue
    marker = data[i + 1]
    if marker in (0xC0, 0xC1, 0xC2, 0xC3):
        height, width = struct.unpack(">HH", data[i + 5:i + 9])
        print(f"{width}x{height}")
        break
    if marker in (0xD8, 0xD9) or 0xD0 <= marker <= 0xD7:
        i += 2
        continue
    i += 2 + struct.unpack(">H", data[i + 2:i + 4])[0]
PY
}

read_dims() {
    local name=$1 url=$2 got
    got=$(dims "$url") || fail "$name: cannot decode $url"
    printf -v "$name" '%s' "$got"
}

section "fixtures"
python3 "$DIR/fixtures.py" "$LIB"

section "startup"
"$BIN" >"$LOG" 2>&1 &
server=$!
trap 'kill "$server" 2>/dev/null' EXIT

for _ in $(seq 1 30); do
    [ "$(status "$BASE/")" != "000" ] && break
    kill -0 "$server" 2>/dev/null || fail "server exited before answering"
    sleep 1
done
[ "$(status "$BASE/")" != "000" ] || fail "server never answered on $BASE"
say "server" "up"

section "auth"
check "root" "$(status "$BASE/")" 200
check "without key" "$(status "$BASE/kobo/v1/user/profile")" 404
check "wrong key" "$(status "$BASE/not-the-key/kobo/v1/user/profile")" 404
check "right key" "$(status "$(api /kobo/v1/user/profile)")" 200
check "doubled v1" "$(status "$(api /kobo/v1/v1/initialization)")" 404
check "download without key" "$(status "$BASE/download/x/EPUB/y.epub")" 404

section "sync"
sync=$(curl -s -D "$HDRS" "$(api /kobo/v1/library/sync)")

book() { printf '%s' "$sync" | jq --arg title "$1" '.[] | select(.NewEntitlement.BookMetadata.Title == $title)'; }

json "books" "$sync" 'length' 5
json "distinct ids" "$sync" '[.[].NewEntitlement.BookEntitlement.Id] | unique | length' 5
json "complete" "$sync" \
    '[.[] | select((.NewEntitlement.BookEntitlement.Id | type) != "string" or (.NewEntitlement.BookMetadata.Title | type) != "string" or (.NewEntitlement.BookMetadata.DownloadUrls | length) == 0)] | length' 0

blame=$(book "BLAME! Master Edition v01")
json "timestamp" "$blame" \
    '.NewEntitlement.BookEntitlement.Created | test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$")'
json "series name" "$blame" '.NewEntitlement.BookMetadata.Series.Name' "BLAME! Master Edition"
json "series number" "$blame" '.NewEntitlement.BookMetadata.Series.Number == 1'
json "series number float" "$blame" '.NewEntitlement.BookMetadata.Series.NumberFloat == 1'
json "series id" "$blame" \
    '.NewEntitlement.BookMetadata.Series.Id | test("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")'
json "kepub format" "$blame" '.NewEntitlement.BookMetadata.DownloadUrls | map(.Format) | join(",")' KEPUB

section "series"
fixed=$(book "Oshi no Ko v01")
json "fixed layout" "$fixed" '.NewEntitlement.BookMetadata.DownloadUrls | map(.Format) | join(",")' EPUB3FL
json "series from title" "$fixed" '.NewEntitlement.BookMetadata.Series.Name' "Oshi no Ko"

babel=$(book "Babel")
json "no series" "$babel" '.NewEntitlement.BookMetadata | has("Series") == false'
json "epub formats" "$babel" '.NewEntitlement.BookMetadata.DownloadUrls | map(.Format) | join(",")' "EPUB3,EPUB"

section "transport"
check "gzip" "$(header content-encoding "$(api /kobo/v1/library/sync)" -H 'Accept-Encoding: gzip')" gzip

token=$(tr -d '\r' < "$HDRS" | awk 'tolower($1) == "x-kobo-synctoken:" { print $2 }')
[ -n "$token" ] || fail "sync: no x-kobo-synctoken header"
say "synctoken" "$token"
json "incremental sync" "$(curl -s -H "x-kobo-synctoken: $token" "$(api /kobo/v1/library/sync)")" 'length' 0
json "unknown token" "$(curl -s -H 'x-kobo-synctoken: nonsense' "$(api /kobo/v1/library/sync)")" 'length' 5

section "covers"
grab book_id "$blame" '.NewEntitlement.BookEntitlement.Id'
grab url "$blame" '.NewEntitlement.BookMetadata.DownloadUrls[0].Url'

check "metadata" "$(status "$(api /kobo/v1/library/$book_id/metadata)")" 200
check "state" "$(status "$(api /kobo/v1/library/$book_id/state)")" 200
check "delete" "$(status -X DELETE "$(api /kobo/v1/library/$book_id)")" 200
check "cover" "$(status "$(api "/$book_id/149/223/false/image.jpg")")" 200
check "cover type" "$(header content-type "$(api "/$book_id/149/223/false/image.jpg")")" image/jpeg

read_dims thumb "$(api "/$book_id/149/223/70/False/image.jpg")"
case $thumb in
    148x223 | 149x223) say "cover thumb" "$thumb" ;;
    *) fail "cover thumb: unexpected $thumb" ;;
esac

read_dims large "$(api "/$book_id/1264/1680/80/False/image.jpg")"
check "cover large" "$large" 800x1200
check "covers cached" "$(find "$COVERS" -type f | wc -l | tr -d ' ')" 3

section "downloads"
check "download" "$(status "$url")" 200
check "download type" "$(header content-type "$url")" application/x-kobo-epub+zip

grab epub_url "$babel" '.NewEntitlement.BookMetadata.DownloadUrls[0].Url'
check "epub download" "$(status "$epub_url")" 200
check "epub type" "$(header content-type "$epub_url")" application/epub+zip

section "initialization"
init=$(curl -s "$(api /kobo/v1/initialization)")
check "image host" "$(printf '%s' "$init" | jq -r '.Resources.image_host')" "$BASE"
template=$(printf '%s' "$init" | jq -r '.Resources.image_url_template')
[[ $template == *"$KEY"* ]] || fail "image template does not carry the key: $template"
say "image template" "$template"

section "background"
for _ in $(seq 1 20); do
    [ "$(grep -c 'library refreshed' "$LOG" || true)" -ge 1 ] && break
    sleep 1
done
refreshes=$(grep -c 'library refreshed' "$LOG" || true)
[ "$refreshes" -ge 1 ] || fail "no background refresh was logged"
say "refresh logged" "$refreshes"
check "key logged" "$(grep -c "api key: *$KEY" "$LOG" || true)" 1

printf '\n%d checks passed\n' "$checks"
