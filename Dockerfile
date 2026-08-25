FROM rust:1.98-slim

RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \
    ca-certificates \
    curl \
    gcc \
    libc6-dev \
    gcc-mingw-w64-x86-64 \
    && rm -rf /var/lib/apt/lists/*

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

RUN cargo binstall -y cargo-audit --version 0.22.2 \
    && cargo binstall -y cargo-nextest --version 0.9.88

RUN rustup component add clippy rustfmt \
    && rustup target add x86_64-pc-windows-gnu
