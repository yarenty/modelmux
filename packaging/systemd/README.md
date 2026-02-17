# systemd Service (Linux)

ModelMux can run as a systemd daemon on Linux, separate from Homebrew. Two modes:

- **System service** — runs at boot, uses `/etc/modelmux/config.toml`
- **User service** — runs for your user, uses `~/.config/modelmux/config.toml`

## System service (system-wide)

Runs as a system daemon, starts at boot.

### 1. Install the binary

```bash
# From source
cargo install --path .
# Binary will be at ~/.cargo/bin/modelmux

# Or copy to system path
sudo cp ~/.cargo/bin/modelmux /usr/local/bin/
```

### 2. Create configuration

```bash
sudo mkdir -p /etc/modelmux
sudo modelmux config init
# Or copy your config:
# sudo cp ~/.config/modelmux/config.toml /etc/modelmux/
# sudo cp ~/.config/modelmux/service-account.json /etc/modelmux/
```

Edit `/etc/modelmux/config.toml` and set `auth.service_account_file` to `/etc/modelmux/service-account.json` if using a file.

### 3. Install and enable the service

```bash
sudo cp packaging/systemd/modelmux.service /etc/systemd/system/
# Edit ExecStart if your binary is elsewhere (e.g. ~/.cargo/bin/modelmux)
sudo systemctl daemon-reload
sudo systemctl enable modelmux
sudo systemctl start modelmux
sudo systemctl status modelmux
```

### 4. Manage the service

```bash
sudo systemctl start modelmux    # Start
sudo systemctl stop modelmux    # Stop
sudo systemctl restart modelmux # Restart
sudo journalctl -u modelmux -f  # View logs
```

## User service (per-user)

Runs under your user, uses your `~/.config/modelmux/` config. Does not start at boot unless you enable user lingering.

### 1. Install the binary

```bash
cargo install modelmux
# Or: cargo install --path .
```

### 2. Configure

```bash
modelmux config init
```

### 3. Install user service

```bash
mkdir -p ~/.config/systemd/user
cp packaging/systemd/modelmux-user.service ~/.config/systemd/user/modelmux.service
# modelmux-user.service uses PATH; ensure ~/.cargo/bin is in your login PATH
```

### 4. Enable and start

```bash
systemctl --user daemon-reload
systemctl --user enable modelmux
systemctl --user start modelmux
systemctl --user status modelmux
```

### 5. Run at login (optional)

User services run only when you're logged in. For a "always on" user service:

```bash
loginctl enable-linger $USER  # Service runs at boot, survives logout
```

### 6. Manage

```bash
systemctl --user start modelmux
systemctl --user stop modelmux
systemctl --user restart modelmux
journalctl --user -u modelmux -f
```

## Binary location

The default system unit uses `/usr/local/bin/modelmux`. Adjust if needed:

- **cargo install**: `~/.cargo/bin/modelmux`
- **Manual copy**: `/usr/local/bin/modelmux` or `/usr/bin/modelmux`

Edit the `ExecStart` line in the service file to match your install path.
