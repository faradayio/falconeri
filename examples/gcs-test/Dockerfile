# Note: No platform specified - uses native architecture of build environment (minikube VM)
FROM rust:latest AS builder

RUN apt-get update && \
    apt-get install -y libpq-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY falconeri-worker ./falconeri-worker
COPY falconeri_common ./falconeri_common
COPY falconeri ./falconeri
COPY falconerid ./falconerid

RUN cargo build -p falconeri-worker

FROM ubuntu:25.10

RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        libpq5 \
        && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/debug/falconeri-worker /usr/local/bin/
ADD word-frequencies.sh /usr/local/bin/

