FROM ubuntu:22.04 AS builder

RUN apt-get update
RUN apt-get install -y libx11-xcb-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
RUN apt-get install -y git gcc curl

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /tmp/rustup-init.sh \
	&& chmod u+x /tmp/rustup-init.sh \
	&& sh /tmp/rustup-init.sh -y \
	&& rm /tmp/rustup-init.sh

WORKDIR /usr/src/sync
COPY . .

RUN ~/.cargo/bin/cargo build --release --package csync --target x86_64-unknown-linux-gnu --locked
RUN mv ./target/x86_64-unknown-linux-gnu/release/csync ~/.cargo/bin/csync

FROM ubuntu:22.04

COPY --from=builder /root/.cargo/bin/csync /usr/local/bin

ENTRYPOINT [ "/usr/local/bin/csync" ]
