FROM rust:1.75-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
COPY static ./static
COPY docs ./docs
COPY README.md WHITEPAPER.md ./

RUN cargo build --release

FROM ubuntu:22.04

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /var/lib/ghostcoin

COPY --from=builder /app/target/release/privacy_chain /usr/local/bin/privacy_chain
COPY --from=builder /app/static /app/static

WORKDIR /app

ENV GHOSTCOIN_SERVER=true
ENV GHOSTCOIN_P2P_PORT=8001
ENV GHOSTCOIN_WEB_PORT=8080
ENV GHOSTCOIN_DATA_DIR=/var/lib/ghostcoin

EXPOSE 8001 8080

CMD ["privacy_chain"]
