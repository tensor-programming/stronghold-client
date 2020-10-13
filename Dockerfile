# ------------------------------------------------------------------------------
# Cargo Build Stage
# ------------------------------------------------------------------------------


FROM rust:1.44-slim AS cargo-build

WORKDIR /usr/src/engine

RUN apt-get update

RUN apt-get install musl-tools build-essential gcc-multilib -y

RUN rustup target add x86_64-unknown-linux-musl

COPY Cargo.toml Cargo.toml
COPY crypto/ crypto/
COPY primitives/ primitives/
COPY random/ random/
COPY vault/ vault/
COPY snapshot/ snapshot/
COPY commandline/ commandline/

RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl

RUN cd crypto/fuzz && RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl

RUN cd vault/fuzz && RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl

# ------------------------------------------------------------------------------
# Crypto Fuzz Stage
# ------------------------------------------------------------------------------


FROM alpine:latest

RUN addgroup -g 1000 engine

RUN adduser -D -s /bin/sh -u 1000 -G engine engine

WORKDIR /home/engine/bin/

# Build Crypto Fuzzer
# COPY --from=cargo-build /usr/src/engine/crypto/fuzz/target/x86_64-unknown-linux-musl/release/fuzz .

# Build vault Fuzzer
COPY --from=cargo-build /usr/src/engine/vault/fuzz/target/x86_64-unknown-linux-musl/release/fuzz .

CMD ["sh", "-c", "./fuzz > fuzz.log"]