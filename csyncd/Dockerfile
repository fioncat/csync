FROM rust:1.75-alpine3.19 AS builder

WORKDIR /usr/src/csync
COPY . .

RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.ustc.edu.cn/g' /etc/apk/repositories
RUN apk add --no-cache musl-dev git

RUN mv ./csyncd/docker-cargo.toml /usr/local/cargo/config
RUN cargo build --release --package csyncd --target x86_64-unknown-linux-musl --locked
RUN mv ./target/x86_64-unknown-linux-musl/release/csyncd /usr/local/cargo/bin/csyncd

FROM alpine:3.19

COPY --from=builder /usr/local/cargo/bin/csyncd /usr/local/bin

ENTRYPOINT [ "/usr/local/bin/csyncd" ]
