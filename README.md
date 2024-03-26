<div align="center">
<h1>📄 csync</h1>
</div>

---

Csync offers an easy CLI to share your clipboard between different devices. This is done through network, you should prepare a server that all your devices can access it.

Csync is written by Rust and it is very fast and lightweight!

---

## Installation

#### Download from release

You can find all bianry files from [GitHub Release Page](https://github.com/fioncat/csync/releases).

#### Use Cargo to install it

```bash
cargo install --git https://github.com/fioncat/csync
```

## Usage

#### Prepare a server

You need to prepare a server that all devices can access to perform data exchange. Run the following command in your server:

```bash
csync serve --bind "<bind-addr>" --password "<your-password>"
```

Arguements:

- `--bind`: The server bind address, default is `0.0.0.0:7703`.
- `--password`: Optional, if provided, all data will be encrypted using [AES](https://en.wikipedia.org/wiki/Advanced_Encryption_Standard). Your clipboard data will be safety exchanged in network. The client should configure the same password otherwise it won't be able to send or receive data from server.

#### Sync clipboard in device

Add a csync config file `~/.config/csync.toml`:

<details>
<summary>Linux on Wayland</summary>

Require package `wl-clipboard`.

```toml
# ~/.config/csync.toml

server = "127.0.0.1:7703"  # The server address
device = "test-device"     # The current device name, should be unique
password = "test password" # The server password

watch = ["ucloud-mac"]  # The other devices to watch

[read]
notify = true
interval = 200

[write]
text_cmd = ["wl-copy"]
download_image = true
```

Run the following command to watch clipboard events and notify them to csync:

```bash
wl-paste --no-newline --watch csync notify
```

</details>

<details>
<summary>MacOS</summary>

```toml
# ~/.config/csync.toml

server = "127.0.0.1:7703"  # The server address
device = "test-device"     # The current device name, should be unique
password = "test password" # The server password

watch = ["ucloud-mac"]  # The other devices to watch

[read]
cmd = ["pbpaste"]
interval = 200

[write]
text_cmd = ["pbcopy"]
download_image = true
```

</details>

Run the following command to start syncing:

```bash
csync watch
```

You can manually send something to other devices:

```bash
csync send "Some text"
csync send -f /path/to/file
```

---

## Special thanks

- [tokio](https://github.com/tokio-rs/tokio): The basic async runtime and network framework.
- [tokio-miniredis](https://github.com/tokio-rs/mini-redis): I referd to its tcp stream IO logic and protocol implement.
- [arboard](https://github.com/1Password/arboard): Although I use external programs to interact with clipboard, but `arboard` is still a good cross-platform library to call clipboard using Rust. But sadly it does not support Wayland natively.
- [clipboard-master](https://github.com/DoumanAsh/clipboard-master): Together with `arboard`, this is usually used to monitor clipboard events. But I still did not use this in csync since it does not support Wayland natively.
