FROM rust:1.99-alpine@sha256:44819b27be4c97b0e4d7203bd2e02c4820f97700211b6a37fb4c246a2de59383 AS build

RUN apk add --no-cache build-base \
    && mkdir -p /empty

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM python:3.12-slim-bookworm@sha256:935c3c5b12357a7e10caa9c1e6ca5692344d97cf229832ef584be2cec9fc03df AS conversion

RUN apt-get update \
    && apt-get install -y --no-install-recommends p7zip-full libgl1 libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

RUN pip install --no-cache-dir \
        "Pillow>=9.3.0" \
        "numpy>=1.22" \
        "natsort>=8.4" \
        "python-slugify>=1.2" \
        "requests>=2.31" \
        "packaging>=23.2" \
        "psutil>=5.9" \
        "PySide6-Essentials" \
        "PyMuPDF" \
        "mozjpeg-lossless-optimization>=1.2" \
    && pip install --no-cache-dir --no-deps \
        "https://github.com/ciromattia/kcc/archive/refs/tags/v12.1.3.tar.gz"

COPY --from=build /src/target/release/kobink /usr/local/bin/kobink
COPY --from=build /empty /library

WORKDIR /app
ENV LIBRARY_PATH=/library \
    PUBLIC_URL=http://localhost:3000 \
    RUST_LOG=info \
    KCC_BIN=kcc-c2e

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
