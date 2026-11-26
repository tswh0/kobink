#!/usr/bin/env python3
"""Report characters in HTML that the GEM font cannot render.

Usage: check-glyphs.py <file-or-dir> [font.woff2]

Needs fontTools (`pip install fonttools`). Run it after a build: a missing
glyph makes the browser fall back mid-line, and Safari can keep that fallback
for the rest of the run (typically the text after a link or inline code).
"""
import sys
from collections import Counter
from html.parser import HTMLParser
from pathlib import Path

from fontTools.ttLib import TTFont


class Text(HTMLParser):
    def __init__(self):
        super().__init__()
        self.out = []

    def handle_data(self, data):
        self.out.append(data)


def main():
    target = Path(sys.argv[1])
    font = Path(sys.argv[2]) if len(sys.argv) > 2 else target / "atarist-8x16.woff2"
    if not font.exists():
        font = target.parent / "atarist-8x16.woff2"
    cmap = TTFont(font).getBestCmap()
    files = [target] if target.is_file() else list(target.rglob("*.html"))
    missing = Counter()
    for f in files:
        t = Text()
        t.feed(f.read_text(encoding="utf-8", errors="replace"))
        for ch in "".join(t.out):
            if ch not in "\n\r\t" and ord(ch) not in cmap:
                missing[ch] += 1
    if not missing:
        print(f"ok: every character in {len(files)} file(s) is in {font.name}")
        return 0
    print(f"missing from {font.name}:")
    for ch, n in missing.most_common():
        print(f"  U+{ord(ch):04X}  {ch!r}  x{n}")
    print("\nNormalise: \u2019 \u2018 -> '; \u201c \u201d -> \"; \u2013 \u2014 -> -; \u2026 -> ...; "
          "\u2190 -> <-; \u2192 -> ->; box drawing -> + |-.")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
