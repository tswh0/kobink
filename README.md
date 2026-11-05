# Kobink

**Your own Kobo shop, running at home.**

A Kobo only knows how to fetch books from the Kobo store. Kobink answers at that address, but
hands out the files sitting in a folder on your own machine. Point the reader at it once, press
Sync, and your books appear — covers, series and all.

No account, no upload, nothing leaves your network.

## How it works

The reader regularly calls the store to ask "what do I own?". Kobink sits at that address and
replies with your files, described the way the device expects. As far as the Kobo can tell, it is
talking to Kobo.

The address contains a long secret code that is printed once, when the server starts:

```
http://192.168.1.10:3000/9f3c1d0c8b2e4a76b1d5f0a2c7e93b48/kobo
```

Anything asked for without that code simply does not exist. You can therefore put it on a public
address without anyone being able to browse your shelf — the full link is the key.

## What it does

- **Your folder is the shop.** Drop EPUBs in a directory and they show up on the device.
- **Covers that fit the screen.** The reader asks for cover images in several sizes; Kobink cuts
  them out of the book itself, so shelves are never grey.
- **Manga looks like manga.** Comics converted with [KCC](https://github.com/ciromattia/kcc) are
  fixed pages. Kobink tells the device so, and pages fill the screen instead of floating in a
  text layout with bands around them.
- **Volumes arrive as a series.** Series info is read from the book, or from the title when the
  book has none (`Fire Punch v03`), so the reader groups and numbers them.
- **Quick repeat syncs.** Once everything is on the device, the next sync is a few kilobytes.
  Add or change a file and it is picked up on the following pass.
- **New books appear by themselves.** The folder is re-read every ten minutes, no restart needed.
- **One small container.** No database, no account, one folder to mount.

## What is different from the original

This is a fork of [potatoeggy/kobink](https://github.com/potatoeggy/kobink). Upstream is the bare
protocol: it serves files and trusts anyone who can reach it. This fork adds:

- a secret code in the URL, generated on first start and printed in the log, so the server can be
  exposed through a reverse proxy without giving your library away
- a build that works: upstream does not compile (a source file is missing from the repo) and
  hardcodes the author's domain into download links
- covers rendered by the server, with a cache on disk
- correct handling of KCC manga, which upstream advertises wrongly and the device then letterboxes
- series grouping, incremental sync, gzip, and a periodic rescan of the folder
- no database: the Rust dependencies for SQL were dropped along with the leftover demo endpoints
- a test that runs the real binary and checks the whole protocol on every push

## Running it

```yaml
services:
  kobink:
    image: ghcr.io/tswh0/kobink:latest
    ports: ["3000:3000"]
    environment:
      PUBLIC_URL: http://192.168.1.10:3000
      LIBRARY_DIR: /books
    volumes:
      - /path/to/books:/library:ro
      - ./data:/app/data
```

`PUBLIC_URL` must be the address the **reader** uses to reach the machine, not the container.

Start it, then look up the code and the ready-made address:

```bash
docker compose logs kobink | grep -A2 "api key"
```

On the Kobo, edit `.kobo/Kobo/Kobo eReader.conf` and set:

```ini
[OneStoreServices]
api_endpoint=http://192.168.1.10:3000/9f3c1d0c8b2e4a76b1d5f0a2c7e93b48/kobo
```

The path ends at `/kobo` — the device adds `/v1` itself. Press Sync.

Other settings: `LIBRARY_REFRESH_SECONDS` (default `600`, `0` disables the rescan), `API_KEY`
(pin your own code instead of the generated one), `KOBINK_PORT` (default `3000`), `RUST_LOG`.

## Good to know

- Plain HTTP on your own network is fine. If you put it behind a reverse proxy, use HTTPS and
  treat the link like a password — whoever has it can read the library.
- Books stay as they are: `.epub` for normal books, `.kepub.epub` (KCC) for manga.
- If an old version of a book sticks on the device, delete it there and sync again.

## License

AGPL-3.0, same as the original.
