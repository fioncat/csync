<div align="center">
<h1>📄 Csync</h1>
</div>

csync is a powerful clipboard synchronization tool that allows you to share clipboard content seamlessly across multiple devices. It provides both command-line interface and GUI system tray for easy access to clipboard history.

Csync is written by Rust, it is very fast and lightweight!

---

## Features

- Real-time clipboard synchronization across devices
- Secure data transmission with HTTPS and AES encryption support
- System tray GUI for quick access to clipboard history
- Command-line interface for clipboard management
- Support for both text and file clipboard content
- Role-based access control

## Dependencies

On Linux systems, csync requires `libwebkit2gtk-4.1` for the system tray GUI. You can install it using your distribution's package manager:

### Ubuntu/Debian

```bash
sudo apt install libwebkit2gtk-4.1-dev
```

### Fedora

```bash
sudo dnf install webkit2gtk4.1-devel
```

### Arch Linux

```bash
sudo pacman -S webkit2gtk-4.1
```

## Installation

You can download the pre-built binaries from the [releases](https://github.com/fioncat/csync/releases) page.

## Building from Source

### Dependencies

- Rust toolchain
- libglib2.0-dev, libgtk-3-dev, libwebkit2gtk-4.1-dev (for Linux)
- Make

### Build Options

1. Full Build (with GUI support):

```bash
make release
```

2. Minimal Build (CLI only, no GUI):

```bash
make minimal
```

## Components

csync consists of several components:

- Server: The central server that all devices connect to for clipboard synchronization. All devices that need to share clipboard data must have network access to this server.
- Daemon: A background service that runs on each device, managing clipboard synchronization through the system's clipboard manager.
- System Tray: A GUI application that provides quick access to clipboard history and management through a system tray icon.
- CLI: Command-line interface for direct interaction with clipboard history using commands like `csync get` and `csync read`.

## Server Deployment

Prepare the server configuration file at `/etc/csync/server.toml`. You can use the example from [testdata/config/server/server.toml](testdata/config/server/server.toml) as a reference.

### Option 1: Systemd Service

Set up the systemd service using the example from [deploy/systemd/csync-server.service](deploy/systemd/csync-server.service).

### Option 2: Docker Compose

Use the provided Docker Compose configuration in [deploy/docker-compose/docker-compose.yml](deploy/docker-compose/docker-compose.yml).

## Client Setup

1. Create a client configuration file at `~/.config/csync/client.toml`. You can use the example from [testdata/config/client/client.toml](testdata/config/client/client.toml) as a reference.

2. Start the daemon and system tray:

```bash
csync daemon  # Start the daemon
csync tray    # Start the system tray
```

## Usage

For detailed command usage:

```bash
csync --help
```

## Security

Since csync transmits clipboard data over the network, security is a top priority. The following security measures are implemented:

- HTTPS support for secure server communication
- AES encryption for clipboard data
- Role-based access control
- Token-based authentication

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
