#!/usr/bin/env bash
# ModelMux systemd installation script
# Installs ModelMux as a system service with proper configuration
#
# Usage:
#   ./install.sh                    # Interactive installation
#   ./install.sh --config-only      # Only setup configuration (binary already installed)
#   ./install.sh --help             # Show help

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BINARY_PATH="/usr/bin/modelmux"
CONFIG_DIR="/etc/modelmux"
CONFIG_FILE="$CONFIG_DIR/config.toml"
SERVICE_FILE="/etc/systemd/system/modelmux.service"

print_header() {
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE} ModelMux systemd Installation${NC}"
    echo -e "${BLUE}================================${NC}"
    echo ""
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        print_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

check_binary() {
    if [[ -f "$PROJECT_ROOT/target/release/modelmux" ]]; then
        print_info "Found built binary at $PROJECT_ROOT/target/release/modelmux"
        return 0
    fi

    if command -v modelmux >/dev/null 2>&1; then
        print_info "Found modelmux in PATH: $(which modelmux)"
        return 0
    fi

    print_warning "No modelmux binary found. Building from source..."
    cd "$PROJECT_ROOT"
    cargo build --release
    if [[ -f "target/release/modelmux" ]]; then
        print_success "Built modelmux binary"
        return 0
    else
        print_error "Failed to build modelmux binary"
        exit 1
    fi
}

install_binary() {
    if [[ -f "$PROJECT_ROOT/target/release/modelmux" ]]; then
        print_info "Installing binary to $BINARY_PATH"
        cp "$PROJECT_ROOT/target/release/modelmux" "$BINARY_PATH"
        chmod +x "$BINARY_PATH"
        print_success "Binary installed"
    elif command -v modelmux >/dev/null 2>&1; then
        local existing_binary=$(which modelmux)
        if [[ "$existing_binary" != "$BINARY_PATH" ]]; then
            print_info "Copying binary from $existing_binary to $BINARY_PATH"
            cp "$existing_binary" "$BINARY_PATH"
            chmod +x "$BINARY_PATH"
            print_success "Binary copied"
        else
            print_success "Binary already installed at correct location"
        fi
    else
        print_error "No modelmux binary found to install"
        exit 1
    fi
}

create_config() {
    print_info "Creating configuration directory"
    mkdir -p "$CONFIG_DIR"

    if [[ -f "$CONFIG_FILE" ]]; then
        print_warning "Configuration file already exists at $CONFIG_FILE"
        read -p "Overwrite existing config? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "Keeping existing configuration"
            return 0
        fi
    fi

    print_info "Copying example configuration"
    cp "$SCRIPT_DIR/config.toml.example" "$CONFIG_FILE"
    chmod 644 "$CONFIG_FILE"
    print_success "Configuration file created"

    print_warning "You must edit $CONFIG_FILE with your Google Cloud settings"
    print_info "Required changes:"
    echo "  - Set your Google Cloud project ID in [vertex] section"
    echo "  - Set your Vertex AI region in [vertex] section"
    echo "  - Add your service account JSON in [auth] section"
    echo ""
    echo "Example service account setup:"
    echo "  1. Go to https://console.cloud.google.com/"
    echo "  2. IAM & Admin → Service Accounts"
    echo "  3. Create account with 'Vertex AI User' role"
    echo "  4. Download JSON key and paste into config file"
}

install_service() {
    print_info "Installing systemd service"
    cp "$SCRIPT_DIR/modelmux.service" "$SERVICE_FILE"

    # Update binary path if needed
    if [[ "$BINARY_PATH" != "/usr/bin/modelmux" ]]; then
        sed -i "s|/usr/bin/modelmux|$BINARY_PATH|" "$SERVICE_FILE"
    fi

    print_success "Service file installed"
}

reload_systemd() {
    print_info "Reloading systemd"
    systemctl daemon-reload
    print_success "systemd reloaded"
}

prompt_service_enable() {
    echo ""
    read -p "Enable and start ModelMux service now? (Y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Nn]$ ]]; then
        print_info "Service not started. You can start it later with:"
        echo "  sudo systemctl enable --now modelmux"
        return 0
    fi

    print_info "Enabling and starting service"
    systemctl enable modelmux
    systemctl start modelmux

    sleep 2

    if systemctl is-active --quiet modelmux; then
        print_success "Service is running"

        # Check if configuration is valid
        if "$BINARY_PATH" doctor >/dev/null 2>&1; then
            print_success "Configuration is valid"
        else
            print_warning "Configuration needs attention. Run: modelmux doctor"
        fi
    else
        print_error "Service failed to start"
        print_info "Check logs with: sudo journalctl -u modelmux --no-pager -n 20"
        return 1
    fi
}

show_status() {
    echo ""
    print_info "Installation complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Edit configuration: sudo vi $CONFIG_FILE"
    echo "  2. Test configuration: modelmux doctor"
    echo "  3. Start service: sudo systemctl start modelmux"
    echo "  4. Check status: sudo systemctl status modelmux"
    echo "  5. View logs: sudo journalctl -u modelmux -f"
    echo ""
    echo "Test API endpoint:"
    echo "  curl -X POST http://localhost:3000/v1/chat/completions \\"
    echo "    -H 'Content-Type: application/json' \\"
    echo "    -H 'Authorization: Bearer dummy' \\"
    echo "    -d '{\"model\":\"claude-3-5-sonnet-20241022\",\"messages\":[{\"role\":\"user\",\"content\":\"Hello!\"}],\"max_tokens\":50}'"
    echo ""
    echo "Configuration location: $CONFIG_FILE"
    echo "Service management: sudo systemctl {start|stop|restart|status} modelmux"
    echo "Logs: sudo journalctl -u modelmux -f"
}

show_help() {
    echo "ModelMux systemd installation script"
    echo ""
    echo "Usage:"
    echo "  $0                    Interactive installation"
    echo "  $0 --config-only      Only setup configuration (binary already installed)"
    echo "  $0 --help             Show this help"
    echo ""
    echo "This script will:"
    echo "  1. Install ModelMux binary to $BINARY_PATH"
    echo "  2. Create configuration at $CONFIG_FILE"
    echo "  3. Install systemd service"
    echo "  4. Enable and start the service (optional)"
    echo ""
    echo "Prerequisites:"
    echo "  - Run as root (sudo)"
    echo "  - Google Cloud project with Vertex AI enabled"
    echo "  - Service account with 'Vertex AI User' role"
}

main() {
    local config_only=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --config-only)
                config_only=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                echo "Use --help for usage information"
                exit 1
                ;;
        esac
    done

    print_header

    check_root

    if [[ "$config_only" == "false" ]]; then
        check_binary
        install_binary
    else
        if [[ ! -f "$BINARY_PATH" ]]; then
            print_error "Binary not found at $BINARY_PATH"
            print_info "Install binary first or run without --config-only"
            exit 1
        fi
        print_success "Using existing binary at $BINARY_PATH"
    fi

    create_config

    if [[ "$config_only" == "false" ]]; then
        install_service
        reload_systemd

        # Only prompt to start if we're doing full install
        if [[ -f "$CONFIG_FILE" ]]; then
            prompt_service_enable
        fi
    fi

    show_status
}

# Run main function with all arguments
main "$@"
