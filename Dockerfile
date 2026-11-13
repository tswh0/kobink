FROM rust:1.99-alpine@sha256:44819b27be4c97b0e4d7203bd2e02c4820f97700211b6a37fb4c246a2de59383 AS build

RUN apk add --no-cache build-base \
    && mkdir -p /empty

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM python:3.15-slim-bookworm@sha256:96599df822d0e1c6bb700014a05c7f3c8cabc6e0f7ff3c4f9f69cee033b4b5d9 AS conversion

RUN apt-get update \
    && apt-get install -y --no-install-recommends p7zip-full \
    && rm -rf /var/lib/apt/lists/*

COPY docker/requirements.txt /tmp/requirements.txt
COPY docker/qt-stub /usr/local/lib/python3.12/site-packages/

RUN pip install --no-cache-dir --no-deps -r /tmp/requirements.txt \
    && pip install --no-cache-dir --no-deps \
        "https://github.com/ciromattia/kcc/archive/ae8079212bc0b3d6f65b0a734c84f9ae68ddc724.tar.gz#sha256=35fa3bf2b40fd715482d7ed212d349a66e7b17d92ae91ee7a9503214561c445c" \
    && find /usr/local/lib/python3.12 -name "__pycache__" -prune -exec rm -rf {} + \
    && rm -f /tmp/requirements.txt

COPY --from=build /src/target/release/kobink /usr/local/bin/kobink
COPY --from=build /empty /library

WORKDIR /app
ENV LIBRARY_PATH=/library \
    PUBLIC_URL=http://localhost:3000 \
    RUST_LOG=info \
    KCC_BIN=kcc-c2e \
    PYTHONUNBUFFERED=1

EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/kobink", "--health"]

ENTRYPOINT ["/usr/local/bin/kobink"]

FROM gcr.io/distroless/static-debian12@sha256:d75cdd72874d4790092fcb1b058493ecf6bb5bf2b2b897045b00ff01d91843f2 AS runtime

COPY --from=build /src/target/release/kobink /usr/local/bin/kobink
COPY --from=build /empty /library

WORKDIR /app
ENV LIBRARY_PATH=/library \
    PUBLIC_URL=http://localhost:3000 \
    RUST_LOG=info

EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/kobink", "--health"]

ENTRYPOINT ["/usr/local/bin/kobink"]
