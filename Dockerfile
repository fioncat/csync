# Accept build and base images as build arguments
ARG BUILD_IMAGE=rust:latest
ARG BASE_IMAGE=debian:latest

# Use BUILD_IMAGE for the builder stage
FROM ${BUILD_IMAGE} AS builder

WORKDIR /usr/src/csync
COPY . .

RUN cargo build --package csync-server --release --locked
RUN mv ./target/release/csync-server /usr/local/cargo/bin/csync-server

# Use BASE_IMAGE for the final stage
FROM ${BASE_IMAGE}

RUN apt update && apt install -y openssl
COPY --from=builder /usr/local/cargo/bin/csync-server /usr/local/bin

ENTRYPOINT [ "/usr/local/bin/csync-server" ]
