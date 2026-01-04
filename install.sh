#!/bin/sh
set -e

REPO="nickcramaro/relay"
INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="relay"

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Linux*)  OS="linux" ;;
    Darwin*) OS="macos" ;;
    *)       echo "Unsupported OS: $OS"; exit 1 ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64)  ARCH="x86_64" ;;
    aarch64) ARCH="aarch64" ;;
    arm64)   ARCH="aarch64" ;;
    *)       echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

ASSET_NAME="relay-${OS}-${ARCH}"
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ASSET_NAME}"

echo "Installing relay..."
echo "  OS: $OS"
echo "  Arch: $ARCH"
echo ""

# Create install directory if needed
mkdir -p "$INSTALL_DIR"

# Create temp directory
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

# Download binary
echo "Downloading from GitHub releases..."
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/$BINARY_NAME"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$DOWNLOAD_URL" -O "$TMP_DIR/$BINARY_NAME"
else
    echo "Error: curl or wget required"
    exit 1
fi

# Make executable and install
chmod +x "$TMP_DIR/$BINARY_NAME"
mv "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"

echo ""
echo "Successfully installed relay to $INSTALL_DIR/$BINARY_NAME"

# Check if install dir is in PATH
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        echo ""
        echo "Note: $INSTALL_DIR is not in your PATH."
        echo "Add it to your shell config:"
        echo ""
        echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
        ;;
esac
