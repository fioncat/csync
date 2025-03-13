# Csync albert plugin

## Installation

### Linux

```bash
git clone https://github.com/fioncat/csync /path/to/csync
mkdir -p ~/.local/share/albert/python/plugins
ln -s /path/to/csync/albert ~/.local/share/albert/python/plugins/csync
```

### macOS

```bash
git clone https://github.com/fioncat/csync /path/to/csync
mkdir -p ~/Library/Application\ Support/albert/python/plugins
ln -s /path/to/csync/albert ~/Library/Application\ Support/albert/python/plugins/csync
```

## Debug

```bash
QT_LOGGING_RULES='albert.python.csync=true' albert
```
