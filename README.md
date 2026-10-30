# Kobink

> Fork of [potatoeggy/kobink](https://github.com/potatoeggy/kobink) with Docker packaging and a fix for the build.

Automatically sync local books to your Kobo by pretending to be the Kobo Store

### Setup

On the Kobo, open `/.kobo/Kobo/Kobo eReader.conf` and change `api_endpoint` to your local machine where Kobink runs (e.g., `http://192.168.0.1:3000`). By default, Kobink runs on port 3000.

Install dependencies and run:

```
LIBRARY_PATH=/path/to/epubs cargo run
```

Press "Sync" on the Kobo — all EPUB files in `LIBRARY_PATH` should now automatically be fetched to the device.

### Docker

```bash
cp .env.example .env    # edit PUBLIC_URL and LIBRARY_DIR
docker compose up -d --build
```

| Variable | Meaning |
|---|---|
| `PUBLIC_URL` | Address the **Kobo** uses to reach this server, e.g. `http://192.168.1.10:3000`. Baked into every download URL, so it must be reachable from the device. |
| `LIBRARY_DIR` | Host folder with your EPUBs, mounted read-only at `/library` (scanned recursively at startup). |
| `KOBINK_PORT` | Host port, default `3000`. |
| `RUST_LOG` | `info` by default, `debug` for troubleshooting. |

Point the device at the container:

```ini
[OneStoreServices]
api_endpoint=http://192.168.1.10:3000/kobo/kobink/v1
```

The `kobink` segment is the `:key` path parameter and accepts any value.

Notes:

- The image builds from this tree. Two fixes are applied on top of upstream `a82976b`:
  `src/library/models.rs` makes the download host configurable via `PUBLIC_URL` instead of the
  hardcoded `tickets.eggworld.me`; `src/main.rs` no longer declares a `mod models;` whose file
  was never committed upstream (that broke the build).
- The library is scanned once at startup; restart the container after adding books.
- Upstream calls `dotenv().unwrap()`, so the image ships an empty `/app/.env`.
