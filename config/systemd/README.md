# Run csync as systemd service

#### Step 1. Copy the service file

```bash
mkdir -p ~/.config/systemd/user
cp ./*.service ~/.config/systemd/user
```

#### Step 2. Replace the default password for server

```bash
sed -i 's/test123/<your-password>' ~/.config/systemd/user/csync-server.service
```

#### Step 3. Start the services

```bash
systemctl --user daemon-reload
systemctl --user enable --now csync
systemctl --user enable --now csync-server
```
