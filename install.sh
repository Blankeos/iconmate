#!/bin/bash

set -e

REPO="Blankeos/iconmate"
INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="iconmate"

echo "Installing iconmate..."

# Check if cargo is available
if command -v cargo &> /dev/null; then
    echo "Installing via cargo..."
    cargo install iconmate
    echo "iconmate installed successfully via cargo"
    echo ""
    echo "Run: iconmate"
    exit 0
fi

# Fall back to downloading pre-built binary
echo "Downloading pre-built binary..."

# Determine platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)     PLATFORM="unknown-linux-gnu";;
    Darwin*)    PLATFORM="apple-darwin";;
    *)          echo "Unsupported OS: $OS"; exit 1;;
esac

case "$ARCH" in
    x86_64)    ARCH="x86_64";;
    aarch64|arm64) ARCH="aarch64";;
    *)         echo "Unsupported architecture: $ARCH"; exit 1;;
esac

TARGET="${ARCH}-${PLATFORM}"

# Create install directory
mkdir -p "$INSTALL_DIR"

# Download and extract binary
ARCHIVE_URL="https://github.com/${REPO}/releases/latest/download/iconmate-${TARGET}.tar.xz"
TMP_DIR="$(mktemp -d)"

if curl -L "$ARCHIVE_URL" -o "$TMP_DIR/iconmate.tar.xz"; then
    tar -xf "$TMP_DIR/iconmate.tar.xz" -C "$TMP_DIR"
    # Find the binary inside the extracted archive
    EXTRACTED_BIN="$(find "$TMP_DIR" -name "$BINARY_NAME" -type f | head -1)"
    if [ -z "$EXTRACTED_BIN" ]; then
        echo "Failed to find binary in archive. Please install via cargo: cargo install iconmate"
        rm -rf "$TMP_DIR"
        exit 1
    fi
    mv "$EXTRACTED_BIN" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
    rm -rf "$TMP_DIR"
    echo "iconmate installed successfully to $INSTALL_DIR/$BINARY_NAME"
else
    rm -rf "$TMP_DIR"
    echo "Failed to download binary. Please install via cargo: cargo install iconmate"
    exit 1
fi

# Add to PATH if not already there
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo "Add $INSTALL_DIR to your PATH:"
    echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo "   Add this to your ~/.bashrc or ~/.zshrc"
fi

echo ""
echo "Run: $BINARY_NAME"
