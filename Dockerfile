# syntax=docker/dockerfile:1

FROM rust:1-slim-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets
RUN cargo build --release --bin jsonic-rpc

FROM debian:bookworm-slim

RUN groupadd --gid 10001 jsonic \
    && useradd --uid 10001 --gid 10001 --home-dir /nonexistent --shell /usr/sbin/nologin --no-create-home jsonic \
    && mkdir -p /data \
    && chown jsonic:jsonic /data

WORKDIR /app
COPY --from=builder /app/target/release/jsonic-rpc /usr/local/bin/jsonic-rpc

ENV JSONIC_RPC_ADDR=0.0.0.0:8080
ENV JSONIC_RPC_DATA_DIR=/data

EXPOSE 8080
VOLUME ["/data"]

USER jsonic
CMD ["jsonic-rpc"]
