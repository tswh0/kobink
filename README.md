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
| `LIBRARY_DIR` | Host folder with your EPUBs, mounted read-only at `/library`. |
| `LIBRARY_REFRESH_SECONDS` | How often the library is rescanned, default `600` (10 min); `0` disables it. |
| `API_KEY` | Optional. Pins the API key; leave empty to have one generated on first start. |
| `API_KEY_FILE` | Where the generated key is stored, default `/app/data/api-key` (mounted at `./data`). |
| `KOBINK_PORT` | Host port, default `3000`. |
| `RUST_LOG` | `info` by default, `debug` for troubleshooting. |

### Authentication

Every request below the API key is rejected (404), so the endpoint can be exposed through a
reverse proxy without granting access to the library.

On first start a key is generated, written to `API_KEY_FILE` and printed in the logs:

```bash
docker compose logs kobink | grep -A2 "api key"
```

```
--------------------------------------------------------------
 api key:      9f3c1d0c8b2e4a76b1d5f0a2c7e93b48
 api_endpoint: http://192.168.1.10:3000/9f3c1d0c8b2e4a76b1d5f0a2c7e93b48/kobo/v1
--------------------------------------------------------------
```

Point the device at the container, with the key in the path:

```ini
[OneStoreServices]
api_endpoint=http://192.168.1.10:3000/9f3c1d0c8b2e4a76b1d5f0a2c7e93b48/kobo/v1
```

Set `API_KEY` in `.env` instead if you would rather manage the key yourself (e.g. supply a
reverse proxy config before the first start).

Notes:

- The image builds from this tree. Two fixes are applied on top of upstream `a82976b`:
  `src/library/models.rs` makes the download host configurable via `PUBLIC_URL` instead of the
  hardcoded `tickets.eggworld.me`; `src/main.rs` no longer declares a `mod models;` whose file
  was never committed upstream (that broke the build).
- The library is rescanned every `LIBRARY_REFRESH_SECONDS` (default 10 minutes); add books and
  wait for the next pass, no restart needed.
- `.github/workflows/build.yml` runs `cargo check` on every push.
