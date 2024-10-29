FROM rust:alpine AS builder

WORKDIR /usr/src/csync
COPY . .

RUN apk add --no-cache musl-dev git

RUN cargo build --release --target x86_64-unknown-linux-musl --locked
RUN mv ./target/x86_64-unknown-linux-musl/release/csync /usr/local/cargo/bin/csync

FROM alpine:latest

COPY --from=builder /usr/local/cargo/bin/csync /usr/local/bin

ENTRYPOINT [ "/usr/local/bin/csync" ]
