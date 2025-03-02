#!/bin/bash
set -e

# This script must be run as a non-root user.
if [ "$EUID" -eq 0 ]; then
  echo "Please run the script as a regular user, not as root."
  exit 1
fi

# Detect architecture
ARCH=$(uname -m)
echo "Detected architecture: $ARCH"

case "$ARCH" in
  x86_64)
    BINARY_URL="https://github.com/guzun-corp/tsight-agent/releases/download/v0.1.0/tsight_agent-linux-x86_64"
    ;;
  # aarch64)
  #   BINARY_URL="https://github.com/guzun-corp/tsight-agent/releases/download/v0.1.0/tsight_agent-linux-aarch64"
  #   ;;
  *)
    echo "Architecture $ARCH is not supported."
    exit 1
    ;;
esac

# Define installation paths for user-level service
BIN_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/tsight_agent"
SERVICE_DIR="$HOME/.config/systemd/user"
SERVICE_FILE="$SERVICE_DIR/tsight-agent.service"

# Create binary directory if it doesn't exist
mkdir -p "$BIN_DIR"

# Download the binary
echo "Downloading binary..."
curl -L -o "$BIN_DIR/tsight_agent" "$BINARY_URL"
chmod +x "$BIN_DIR/tsight_agent"

# Create configuration directory and file
echo "Creating configuration directory: $CONFIG_DIR"
mkdir -p "$CONFIG_DIR"
CONFIG_FILE="$CONFIG_DIR/config.yaml"
if [ ! -f "$CONFIG_FILE" ]; then
  echo "# Tsight Agent Configuration" > "$CONFIG_FILE"
  echo "# Please check the guide for configuration options: https://tsight.app/onboarding" >> "$CONFIG_FILE"
  echo "# For more information, see README.md: https://github.com/guzun-corp/tsight-agent/blob/main/README.md#configuration" >> "$CONFIG_FILE"
fi

# Create directory for systemd unit files (user-level)
mkdir -p "$SERVICE_DIR"

# Create unit file for the user service
echo "Creating systemd user service unit file..."
cat <<EOF > "$SERVICE_FILE"
[Unit]
Description=Tsight Agent User Service
After=network.target
StartLimitIntervalSec=60

[Service]
Environment="RUST_LOG=info"
ExecStart=$BIN_DIR/tsight_agent --config $CONFIG_FILE
Restart=on-failure
RestartSec=5
StartLimitBurst=3

[Install]
WantedBy=default.target
EOF

# Reload the user systemd daemon and start the service
echo "Reloading user-level systemd daemon..."
systemctl --user daemon-reload

echo "Enabling and starting tsight-agent service..."
systemctl --user enable tsight-agent

# Final instructions highlighted in green
GREEN="\033[32m"
RESET="\033[0m"
echo -e "${GREEN}Installation complete. To start the service, follow these steps:"
echo "1. Edit the configuration file:"
echo "   nano $CONFIG_FILE"
echo "2. Start the service:"
echo "   systemctl --user start tsight-agent"
echo "3. Check the service status:"
echo "   systemctl --user status tsight-agent"
