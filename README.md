# Kobink

A Kobo can only download books from the Kobo store. Kobink runs on your own machine, answers at
that address instead, and hands out the files you already have. The books stay in a folder.
Nothing gets uploaded.

## How it works

The reader asks its store what it owns. Kobink replies with the files from your folder, and for
the Kobo it looks like a normal sync.

The address you type into the device contains a secret code. Without it, nothing is found. That
is the whole access control, so you can forward a public URL to it as long as the link stays with
you.

The code is printed when the container starts.

```bash
docker compose logs kobink | grep -A2 "api key"
```

## Setup

```yaml
services:
  kobink:
    image: ghcr.io/tswh0/kobink:latest
    restart: unless-stopped
    ports: ["3000:3000"]
    environment:
      PUBLIC_URL: http://192.168.1.10:3000
    volumes:
      - /path/to/books:/library:ro
      - ./data:/app/data
```

`PUBLIC_URL` has to be the address the reader can reach, because it ends up inside every download
link. If the device can open it in a browser, it is right.

Then edit `.kobo/Kobo/Kobo eReader.conf` on the Kobo:

```ini
[OneStoreServices]
api_endpoint=http://192.168.1.10:3000/PASTE-YOUR-KEY-HERE/kobo
```

Replace `PASTE-YOUR-KEY-HERE` with the code from the log. The path ends at `/kobo`; the device
appends `/v1` itself. Press Sync afterwards.

## What it does

- Covers are cut out of each book by the server, in the sizes the device asks for.
- Manga converted with [KCC](https://github.com/ciromattia/kcc) fills the screen, because the
  pages are fixed and the device is told so.
- Volumes arrive as a series with the right numbering. The name comes from the book's metadata,
  or from the title if the book has none (`Fire Punch v03`).
- A sync with nothing new costs a few kilobytes.
- The folder is read again every ten minutes. Drop a file in, wait, sync.
- No database. One container, two folders.

## Compared to upstream

Forked from [potatoeggy/kobink](https://github.com/potatoeggy/kobink). Upstream does the protocol
and stops there. Anyone who can reach the port can download the library, the tree does not build
(a source file is missing), and the download links point at the author's own domain.

This fork puts the key in the path, builds in Docker, renders covers, marks KCC books as fixed
layout, adds series and incremental sync and gzip, rescans the folder on a timer, and runs a test
against the real binary on every push. The SQL dependencies and demo endpoints are gone.

## Notes

- HTTP on your LAN is fine. Behind a reverse proxy use HTTPS and keep the link to yourself.
- `.epub` for books, `.kepub.epub` (KCC) for manga.
- Set `API_KEY` if you want to pick the code yourself instead of using the generated one.
- A book that refuses to update: delete it on the device and sync again.

License: AGPL-3.0, like the original.
