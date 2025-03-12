<div align="center">
<h1>📄 Csync</h1>
</div>

Csync is a powerful clipboard synchronization tool that allows you to share clipboard content seamlessly across multiple devices. It provides both command-line interface and GUI system tray for easy access to clipboard history.

Csync is written by Rust, it is very fast and lightweight!

---

## Features

- Real-time clipboard synchronization across devices
- Secure data transmission with HTTPS
- System tray GUI for quick access to clipboard history
- Command-line interface for clipboard management
- Support for both text and PNG image clipboard content

## Installation

### Arch Linux (AUR)

```bash
yay -S csync-release
```

### macOS (homebrew)

```bash
brew install fioncat/apps/csync
```

## Usage

Csync has 3 binary files:

- `csync-server`: A HTTP(S) server to save and exchange clipboard data. All devices should be able to access it.
- `csyncd`: A daemon to access system clipboard manager and synchronous with other devices. It will also start a system tray GUI to view clipboard history.
- `csynctl`: A command to control csync server. You can use this command to read/write clipboard data, manage server RBAC, etc.

Please refer to `<cmd> --help` for more usage.

## Security

Since csync transmits clipboard data over the network, security is a top priority. The following security measures are implemented:

- HTTPS support for secure server communication
- Token-based authentication

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
