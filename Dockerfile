FROM rust:1.99-slim-bookworm AS build

# hadolint ignore=DL3008
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim

RUN mkdir -p /app /library && : > /app/.env

COPY --from=build /src/target/release/kobink /usr/local/bin/kobink

WORKDIR /app
ENV LIBRARY_PATH=/library \
    PUBLIC_URL=http://localhost:3000 \
    RUST_LOG=info

EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/kobink", "--health"]

ENTRYPOINT ["/usr/local/bin/kobink"]
