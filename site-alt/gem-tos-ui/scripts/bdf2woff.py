#!/usr/bin/env python3
"""Convert a BDF bitmap font (8x16 Atari ST system font) to TTF + WOFF2.

Usage: python bdf2woff.py atarist-normal.bdf [out-basename]

Builds one square contour per horizontal run of pixels, so the outlines stay
pixel-exact at 16px / 32px / 48px. Requires fontTools (+ brotli for WOFF2).
"""
import sys
from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen


def parse_bdf(path):
    props, glyphs = {}, {}
    enc = None
    bbx = None
    rows = []
    in_bitmap = False
    for raw in Path(path).read_text(errors="replace").splitlines():
        line = raw.strip()
        if line.startswith("FONT_ASCENT"):
            props["ascent"] = int(line.split()[1])
        elif line.startswith("FONT_DESCENT"):
            props["descent"] = int(line.split()[1])
        elif line.startswith("FONTBOUNDINGBOX"):
            props["fbb"] = tuple(int(v) for v in line.split()[1:])
        elif line.startswith("STARTCHAR"):
            name = line.split(maxsplit=1)[1]
        elif line.startswith("ENCODING"):
            enc = int(line.split()[1])
        elif line.startswith("DWIDTH"):
            dwidth = int(line.split()[1])
        elif line.startswith("BBX"):
            bbx = tuple(int(v) for v in line.split()[1:])
        elif line == "BITMAP":
            in_bitmap, rows = True, []
        elif line == "ENDCHAR":
            if enc is not None and enc >= 0 and rows:
                glyphs[enc] = (name, bbx, dwidth, rows)
            enc, bbx, in_bitmap = None, None, False
        elif in_bitmap and line:
            rows.append(line)
    return props, glyphs


def runs(row_hex, width):
    bits = bin(int(row_hex, 16))[2:].zfill(width * 4 if len(row_hex) * 4 > width else width)
    bits = bits[-width:]
    out, start = [], None
    for x, b in enumerate(bits + "0"):
        if b == "1" and start is None:
            start = x
        elif b != "1" and start is not None:
            out.append((start, x))
            start = None
    return out


def main():
    src = sys.argv[1]
    base = sys.argv[2] if len(sys.argv) > 2 else Path(src).with_suffix("").name
    props, glyphs = parse_bdf(src)
    ascent = props.get("ascent", 14)
    descent = props.get("descent", 2)
    em = ascent + descent

    order = [".notdef"]
    cmap, advances = {}, {}
    pen_glyphs = {}

    pen = TTGlyphPen(None)
    for y in range(em):
        pen.moveTo((0, y)); pen.lineTo((8, y)); pen.lineTo((8, y + 1)); pen.lineTo((0, y + 1)); pen.closePath()
    pen_glyphs[".notdef"] = pen.glyph()

    for code in sorted(glyphs):
        name, (bw, bh, bx, by), dwidth, rows = glyphs[code]
        gname = name.replace(" ", "_") or f"uni{code:04X}"
        while gname in pen_glyphs:
            gname += "_"
        pen = TTGlyphPen(None)
        for r, rowhex in enumerate(rows):
            if r >= bh:
                break
            for x0, x1 in runs(rowhex, bw):
                x = bx + x0
                y = by + bh - r - 1
                pen.moveTo((x, y)); pen.lineTo((x1 - x0 + x, y))
                pen.lineTo((x1 - x0 + x, y + 1)); pen.lineTo((x, y + 1)); pen.closePath()
        pen_glyphs[gname] = pen.glyph()
        order.append(gname)
        cmap[code] = gname
        advances[gname] = dwidth

    fb = FontBuilder(em, isTTF=True)
    fb.setupGlyphOrder(order)
    fb.setupCharacterMap(cmap)
    fb.setupGlyf(pen_glyphs)
    metrics = {g: (advances.get(g, 8), 0) for g in order}
    fb.setupHorizontalMetrics(metrics)
    fb.setupHorizontalHeader(ascent=ascent, descent=-descent)
    fb.setupNameTable({
        "familyName": "Atarist 8x16",
        "styleName": "Regular",
        "uniqueFontIdentifier": "Atarist8x16-Regular",
        "fullName": "Atarist 8x16",
        "psName": "Atarist8x16-Regular",
        "version": "1.0",
        "copyright": "Bitmap font (c) 2015 ntwk, BSD-3-Clause. Converted to outlines.",
    })
    fb.setupOS2(sTypoAscender=ascent, sTypoDescender=-descent, sTypoLineGap=0,
                usWinAscent=ascent, usWinDescent=descent, sxHeight=8, sCapHeight=10,
                achVendID="NTRK", fsType=0)
    fb.setupPost()
    fb.save(f"{base}.ttf")
    fb.font.flavor = "woff2"
    fb.save(f"{base}.woff2")
    print(f"{base}.ttf / {base}.woff2  glyphs={len(order)} em={em} ascent={ascent}")


if __name__ == "__main__":
    main()
