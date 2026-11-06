import os
import struct
import sys
import zipfile
import zlib


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


def make_epub(path, title, author, layout="reflowable", series=None, number=None):
    os.makedirs(os.path.dirname(path), exist_ok=True)

    series_meta = ""
    if series is not None:
        series_meta = (
            f'<meta name="calibre:series" content="{series}"/>\n'
            f"    <meta name=\"calibre:series_index\" content=\"{number}\"/>"
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

    with zipfile.ZipFile(path, "w") as epub:
        epub.writestr("mimetype", "application/epub+zip", compress_type=zipfile.ZIP_STORED)
        epub.writestr(
            "META-INF/container.xml",
            """<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>""",
        )
        epub.writestr("OEBPS/content.opf", opf)
        epub.writestr(
            "OEBPS/chapter1.xhtml",
            "<html xmlns='http://www.w3.org/1999/xhtml'><body><h1>x</h1></body></html>",
        )
        epub.writestr("OEBPS/cover.png", PNG)


def make_cbz(path, pages=2):
    os.makedirs(os.path.dirname(path), exist_ok=True)

    with zipfile.ZipFile(path, "w") as cbz:
        for index in range(pages):
            cbz.writestr(f"{index:03d}.png", PNG)


if sys.argv[1] == "--manga":
    out = sys.argv[2]
    make_cbz(f"{out}/cbz/Fire Punch v03.cbz")
    make_cbz(f"{out}/cbz/fail.cbz")
    make_epub(
        f"{out}/stub/Fire Punch v03.epub",
        "Fire Punch v03",
        "Tatsuki Fujimoto",
        series="Fire Punch",
        number=3,
    )
    print(f"manga fixtures in {out}")
    sys.exit(0)


root = sys.argv[1]

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

print(f"fixtures: 5 books in {root}")
