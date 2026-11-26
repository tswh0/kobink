#!/bin/sh
set -eu

dir=$(cd "$(dirname "$0")/.." && pwd)
font="$dir/assets/atarist-8x16.woff2"

if [ ! -f "$font" ]; then
  python3 - "$dir" <<'PY'
import base64, pathlib, sys
root = pathlib.Path(sys.argv[1])
src = root / "assets" / "atarist-8x16.woff2.b64"
out = root / "assets" / "atarist-8x16.woff2"
out.write_bytes(base64.b64decode(src.read_text()))
print("decoded", out)
PY
fi

cp "$font" "$dir/templates/atarist-8x16.woff2"
echo "copied font to templates/atarist-8x16.woff2"
