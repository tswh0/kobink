#!/usr/bin/env bash
set -euo pipefail

BASE=${PUBLIC_URL:-http://127.0.0.1:3000}
KEY=${API_KEY:-smoketestkey123}
LIB=${LIBRARY_PATH:-/tmp/library}
BIN=./target/release/kobink
LOG=server.log

expect() {
    local name=$1 actual=$2 wanted=$3
    printf '%-22s %s\n' "$name" "$actual"
    if [ "$actual" != "$wanted" ]; then
        echo "::error title=smoke::$name: expected '$wanted', got '$actual'"
        tail -40 "$LOG" 2>/dev/null || true
        exit 1
    fi
}

code() { curl -s -o /dev/null -w '%{http_code}' "$@"; }

jpeg_dims() {
    python3 - "$1" <<'PY'
import struct, sys, urllib.request

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

on_error() {
    echo "::error title=smoke::line $1: $2"
    grep -A3 -iE "panicked at" "$LOG" 2>/dev/null | head -8 | while IFS= read -r line; do
        echo "::error title=server::$line"
    done
}

trap 'on_error "$LINENO" "$BASH_COMMAND"' ERR

mkdir -p "$LIB/BLAME!"

python3 - "$LIB" <<'PY'
import os, struct, sys, zipfile, zlib


def solid_png(width, height, rgb):
    raw = (b"\x00" + bytes(rgb) * width) * height

    def chunk(tag, data):
        body = tag + data
        return (
            struct.pack(">I", len(data))
            + body
            + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
        )

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


PNG = solid_png(800, 1200, (40, 40, 40))
root = sys.argv[1]


def make_epub(path, title, author, layout="reflowable", series=None, number=None):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    series_meta = ""
    if series is not None:
        series_meta = (
            f'<meta name="calibre:series" content="{series}"/>\n'
            f'    <meta name="calibre:series_index" content="{number}"/>'
        )
    opf = f"""<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="id">urn:uuid:12345678-1234-1234-1234-123456789abc</dc:identifier>
    <dc:title>{title}</dc:title>
    <dc:creator>{author}</dc:creator>
    <dc:language>en</dc:language>
    <meta property="dcterms:modified">2026-01-01T00:00:00Z</meta>
    <meta property="rendition:layout">{layout}</meta>
    {series_meta}
    <meta name="cover" content="cover-img"/>
  </metadata>
  <manifest>
    <item id="c1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="cover-img" href="cover.png" media-type="image/png" properties="cover-image"/>
  </manifest>
  <spine><itemref idref="c1"/></spine>
</package>"""
    with zipfile.ZipFile(path, "w") as z:
        z.writestr("mimetype", "application/epub+zip", compress_type=zipfile.ZIP_STORED)
        z.writestr("META-INF/container.xml", """<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>""")
        z.writestr("OEBPS/content.opf", opf)
        z.writestr(
            "OEBPS/chapter1.xhtml",
            "<html xmlns='http://www.w3.org/1999/xhtml'><body><h1>x</h1></body></html>",
        )
        z.writestr("OEBPS/cover.png", PNG)


for volume in (1, 2, 3):
    make_epub(
        f"{root}/BLAME!/BLAME! Master Edition v0{volume}.kepub.epub",
        f"BLAME! Master Edition v0{volume}",
        "Tsutomu Nihei",
        series="BLAME! Master Edition",
        number=volume,
    )

make_epub(
    f"{root}/R.F. Kuang/Babel - Rebecca F. Kuang (2022).epub",
    "Babel",
    "R.F. Kuang",
)

make_epub(
    f"{root}/Oshi no Ko/Oshi no Ko v01.kepub.epub",
    "Oshi no Ko v01",
    "Aka Akasaka",
    layout="pre-paginated",
)
PY

"$BIN" > "$LOG" 2>&1 &
server=$!
trap 'kill $server 2>/dev/null || true' EXIT

for _ in $(seq 1 30); do
    if curl -fsS "$BASE/" > /dev/null 2>&1; then
        break
    fi
    sleep 1
done

if ! curl -fsS "$BASE/" > /dev/null 2>&1; then
    echo "::error title=smoke::server did not start"
    grep -iE "panic|error" "$LOG" 2>/dev/null | head -3 | while IFS= read -r line; do
        echo "::error title=server::$line"
    done
    tail -40 "$LOG" 2>/dev/null || true
    exit 1
fi

expect "root" "$(code "$BASE/")" 200
expect "no key" "$(code "$BASE/kobo/v1/user/profile")" 404
expect "wrong key" "$(code "$BASE/nope/kobo/v1/user/profile")" 404
expect "right key" "$(code "$BASE/$KEY/kobo/v1/user/profile")" 200
expect "doubled /v1" "$(code "$BASE/$KEY/kobo/v1/v1/initialization")" 404
expect "download, no key" "$(code "$BASE/download/x/EPUB/y.epub")" 404

sync=$(curl -s -D sync-headers.txt "$BASE/$KEY/kobo/v1/library/sync")
field() { printf '%s' "$sync" | jq -r "$1"; }

blame=$(printf '%s' "$sync" | jq '.[] | select(.NewEntitlement.BookMetadata.Title == "BLAME! Master Edition v01")')
bval() { printf '%s' "$blame" | jq -r "$1"; }

expect "books synced" "$(field 'length')" 5
expect "distinct ids" "$(field '[.[].NewEntitlement.BookEntitlement.Id] | unique | length')" 5
expect "kobo timestamp" "$(bval '.NewEntitlement.BookEntitlement.Created' | sed -E 's/[0-9]/N/g')" "NNNN-NN-NNTNN:NN:NNZ"
expect "series name" "$(bval '.NewEntitlement.BookMetadata.Series.Name')" "BLAME! Master Edition"
expect "series number" "$(bval '.NewEntitlement.BookMetadata.Series.Number')" 1
expect "series number float" "$(bval '.NewEntitlement.BookMetadata.Series.NumberFloat | test("^1(\\.0)?$")')" true
expect "series id" "$(bval '.NewEntitlement.BookMetadata.Series.Id | test("^[0-9a-f-]{36}$")')" true
expect "synctoken header" "$(grep -ci '^x-kobo-synctoken: ' sync-headers.txt)" 1
expect "gzip encoding" "$(curl -s -o /dev/null -D - -H 'Accept-Encoding: gzip' "$BASE/$KEY/kobo/v1/library/sync" | tr -d '\r' | awk 'tolower($1)=="content-encoding:"{print $2}')" gzip
token=$(tr -d '\r' < sync-headers.txt | awk 'tolower($1)=="x-kobo-synctoken:"{print $2}')
expect "incremental sync" "$(curl -s -H "x-kobo-synctoken: $token" "$BASE/$KEY/kobo/v1/library/sync" | jq 'length')" 0
expect "kepub format" "$(bval '.NewEntitlement.BookMetadata.DownloadUrls | map(.Format) | join(",")')" KEPUB

fixed=$(printf '%s' "$sync" | jq '.[] | select(.NewEntitlement.BookMetadata.Title == "Oshi no Ko v01")')
expect "fixed layout format" "$(printf '%s' "$fixed" | jq -r '.NewEntitlement.BookMetadata.DownloadUrls | map(.Format) | join(",")')" EPUB3FL
expect "series from title" "$(printf '%s' "$fixed" | jq -r '.NewEntitlement.BookMetadata.Series.Name')" "Oshi no Ko"

plain=$(printf '%s' "$sync" | jq '.[] | select(.NewEntitlement.BookMetadata.Title == "Babel")')
expect "no series" "$(printf '%s' "$plain" | jq '.NewEntitlement.BookMetadata | has("Series")')" false

book=$(bval '.NewEntitlement.BookEntitlement.Id')
url=$(bval '.NewEntitlement.BookMetadata.DownloadUrls[0].Url')

expect "metadata" "$(code "$BASE/$KEY/kobo/v1/library/$book/metadata")" 200
expect "state" "$(code "$BASE/$KEY/kobo/v1/library/$book/state")" 200
expect "delete" "$(code -X DELETE "$BASE/$KEY/kobo/v1/library/$book")" 200
expect "cover" "$(code "$BASE/$KEY/$book/149/223/false/image.jpg")" 200
expect "cover content type" \
    "$(curl -sI "$BASE/$KEY/$book/149/223/false/image.jpg" | tr -d '\r' | awk 'tolower($1)=="content-type:"{print $2}')" \
    "image/jpeg"
thumb=$(jpeg_dims "$BASE/$KEY/$book/149/223/70/False/image.jpg")
case "$thumb" in
    148x223 | 149x223) printf '%-22s %s\n' "cover thumb" "$thumb" ;;
    *)
        echo "::error title=smoke::cover thumb: unexpected size '$thumb'"
        exit 1
        ;;
esac
expect "cover large" "$(jpeg_dims "$BASE/$KEY/$book/1264/1680/80/False/image.jpg")" "800x1200"
expect "cover cached" "$(ls "$COVER_CACHE_DIR" | wc -l | tr -d ' ')" 3
expect "analytics" "$(code -X POST "$BASE/$KEY/kobo/v1/analytics/event")" 200
expect "download" "$(code "$url")" 200
expect "download content type" \
    "$(curl -sI "$url" | tr -d '\r' | awk 'tolower($1)=="content-type:"{print $2}')" \
    "application/x-kobo-epub+zip"

init=$(curl -s "$BASE/$KEY/kobo/v1/initialization")
expect "image_host" "$(printf '%s' "$init" | jq -r '.Resources.image_host')" "$BASE"
expect "image_url_template" "$(printf '%s' "$init" | jq -r '.Resources.image_url_template' | grep -c "$KEY")" 1

babel=$(printf '%s' "$sync" | jq '.[] | select(.NewEntitlement.BookMetadata.Title == "Babel")')
expect "epub formats" "$(printf '%s' "$babel" | jq -r '.NewEntitlement.BookMetadata.DownloadUrls | map(.Format) | join(",")')" "EPUB3,EPUB"
epub_url=$(printf '%s' "$babel" | jq -r '.NewEntitlement.BookMetadata.DownloadUrls[0].Url')
expect "epub download" "$(code "$epub_url")" 200
expect "epub content type" \
    "$(curl -sI "$epub_url" | tr -d '\r' | awk 'tolower($1)=="content-type:"{print $2}')" \
    "application/epub+zip"

sleep 4
refreshes=$(grep -c 'library refreshed' "$LOG" || true)
if [ "$refreshes" -lt 1 ]; then
    echo "::error title=smoke::refresh: expected at least one refresh, got $refreshes"
    exit 1
fi
printf '%-22s %s\n' "refresh logged" "$refreshes"
expect "api key logged" "$(grep -c "api key:      $KEY" "$LOG")" 1

echo "ok"
