# ModelMux systemd Service Setup

This guide covers setting up ModelMux as a systemd service on Linux. We provide configurations for both system-wide and user-specific deployments.

## Prerequisites

- ModelMux v1.0.0 or later
- Google Cloud project with Vertex AI API enabled
- Service account with "Vertex AI User" role
- Ubuntu/Debian or other systemd-based Linux distribution

## Quick Start (System-wide)

For most users, the system-wide installation is recommended:

```bash
# 1. Build and install latest version
cargo build --release
sudo cp target/release/modelmux /usr/bin/

# 2. Create configuration directory
sudo mkdir -p /etc/modelmux

# 3. Copy example config and edit with your settings
sudo cp packaging/systemd/config.toml.example /etc/modelmux/config.toml
sudo chmod 644 /etc/modelmux/config.toml
sudo vi /etc/modelmux/config.toml  # Edit with your project details

# 4. Install and start service
sudo cp packaging/systemd/modelmux.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now modelmux

# 5. Verify it's working
sudo systemctl status modelmux
modelmux doctor
```

## Configuration

ModelMux reads configuration from `/etc/modelmux/config.toml` (system) or `~/.config/modelmux/config.toml` (user).

### Required Configuration

Edit `/etc/modelmux/config.toml` with your Google Cloud details:

```toml
[server]
port = 3000
log_level = "info"

[auth]
service_account_json = "{\"type\": \"service_account\", \"project_id\": \"your-project-id\", ...}"

[vertex]
project = "your-gcp-project-id"
region = "us-central1"
location = "us-central1"
publisher = "anthropic"
model = "claude-3-5-sonnet-20241022"

[streaming]
mode = "auto"
```

### Getting Your Service Account JSON

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Navigate to **IAM & Admin** → **Service Accounts**
3. Create a service account with "Vertex AI User" role
4. Download the JSON key file
5. Copy the entire JSON content as a single line into the `service_account_json` field

## System Service (Recommended)

Runs at boot, available system-wide, uses `/etc/modelmux/config.toml`.

### Installation

```bash
# Install binary to system location
sudo cp target/release/modelmux /usr/bin/
# Or from cargo: sudo cp ~/.cargo/bin/modelmux /usr/bin/

# Create system configuration
sudo mkdir -p /etc/modelmux
sudo cp packaging/systemd/config.toml.example /etc/modelmux/config.toml
sudo chmod 644 /etc/modelmux/config.toml

# Edit configuration with your settings
sudo vi /etc/modelmux/config.toml

# Install service
sudo cp packaging/systemd/modelmux.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable modelmux
sudo systemctl start modelmux
```

### Management

```bash
sudo systemctl start modelmux     # Start service
sudo systemctl stop modelmux      # Stop service
sudo systemctl restart modelmux   # Restart service
sudo systemctl status modelmux    # Check status
sudo journalctl -u modelmux -f    # View live logs
sudo journalctl -u modelmux -n 50 # View last 50 log entries
```

### Configuration Validation

```bash
modelmux doctor                    # Check configuration health
modelmux validate                  # Validate configuration file
```

## User Service

Runs under your user account, uses `~/.config/modelmux/config.toml`.

### Installation

```bash
# Install binary (cargo install or copy to PATH)
cargo install --path .

# Create user configuration
mkdir -p ~/.config/modelmux
cp packaging/systemd/config.toml.example ~/.config/modelmux/config.toml
vi ~/.config/modelmux/config.toml  # Edit with your settings

# Install user service
mkdir -p ~/.config/systemd/user
cp packaging/systemd/modelmux-user.service ~/.config/systemd/user/modelmux.service
systemctl --user daemon-reload
systemctl --user enable modelmux
systemctl --user start modelmux
```

### Management

```bash
systemctl --user start modelmux
systemctl --user stop modelmux
systemctl --user restart modelmux
systemctl --user status modelmux
journalctl --user -u modelmux -f
```

### Auto-start at boot (optional)

```bash
loginctl enable-linger $USER  # Service starts at boot, survives logout
```

## Testing

After installation, test the service:

```bash
# Check service status
sudo systemctl status modelmux

# Check configuration
modelmux doctor

# Test API endpoint
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer dummy-key" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 50
  }'
```

Expected response:
```json
{
  "id": "chatcmpl-...",
  "object": "chat.completion", 
  "created": 1234567890,
  "model": "claude-3-5-sonnet-20241022",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 8,
    "total_tokens": 18
  }
}
```

## Security

The system service runs as root by default. For production environments, consider:

1. **Dedicated user**: Create a `modelmux` user and update the service file
2. **File permissions**: Ensure config files are not world-readable if they contain sensitive data
3. **Firewall**: Restrict access to port 3000 if needed
4. **TLS**: Use a reverse proxy (nginx, Apache) for HTTPS termination

```bash
# Create dedicated user (optional)
sudo useradd -r -s /bin/false modelmux
sudo chown modelmux:modelmux /etc/modelmux/config.toml
# Edit service file: User=modelmux, Group=modelmux
```

## Troubleshooting

### Service won't start

```bash
# Check logs
sudo journalctl -u modelmux --no-pager -n 20

# Check configuration
modelmux doctor

# Verify binary location
which modelmux
ls -la /usr/bin/modelmux
```

### Authentication errors

```bash
# Verify service account JSON is valid
echo 'YOUR_JSON' | jq .  # Should parse without errors

# Check GCP project and permissions
gcloud auth activate-service-account --key-file=/etc/modelmux/service-account.json
gcloud projects list
```

### Common issues

| Error | Solution |
|-------|----------|
| `Configuration error: No service account configuration found` | Add `service_account_json` to `[auth]` section |
| `Permission denied` | Check file permissions: `sudo chmod 644 /etc/modelmux/config.toml` |
| `Binary not found` | Verify binary location matches `ExecStart` in service file |
| `Port already in use` | Change `port` in config or stop conflicting service |

## Log Levels

Configure logging detail in `config.toml`:

- `trace` - Very detailed debugging
- `debug` - Debugging information  
- `info` - General information (default)
- `warn` - Warning messages
- `error` - Error messages only

## Files

- **System config**: `/etc/modelmux/config.toml`
- **User config**: `~/.config/modelmux/config.toml`  
- **System service**: `/etc/systemd/system/modelmux.service`
- **User service**: `~/.config/systemd/user/modelmux.service`
- **Logs**: `journalctl -u modelmux` or `journalctl --user -u modelmux`

## Support

For issues and questions:
- GitHub: https://github.com/yarenty/modelmux
- Check service logs: `sudo journalctl -u modelmux -f`
- Run diagnostics: `modelmux doctor`
