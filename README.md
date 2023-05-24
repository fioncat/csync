# csync

csync is a simple tool for synchronizing the clipboards of multiple machines.

## Install

For Linux, the X11 library is needed, you can install it like this:

```bash
sudo apt-get install xorg-dev
```

You can download csync binary from [release page](https://github.com/fioncat/csync/releases).

If you have Rust installed, you can build csync from source:

```bash
git clone https://github.com/fioncat/csync.git /path/to/csync
cd /path/to/csync
cargo install --path .
```

## Usage

Suppose you want to synchronize the clipboards of two machines, the network between them must be reachable, and the IP addresses are "192.168.0.1" and "192.168.0.2".

Run the command in "192.168.0.1":

```bash
csync --target "192.168.0.2"
```
Run the command in "192.168.0.2":

```bash
csync --target "192.168.0.1"
```

All done! csync will automatically watch your system clipboard and synchronize to the peer.
