#!/usr/bin/env sh
set -e

REPO="Gerharddc/litterbox"
INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="litterbox"
DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$BINARY_NAME"

ARCH="$(uname -m)"

# --- ARCH CHECK (only x86-64 supported) ---
case "$ARCH" in
  x86_64|amd64)
    # OK
    ;;
  *)
    echo "⚠ Unsupported architecture detected: $ARCH"
    echo ""
    echo "Only x86-64 builds of 'litterbox' are currently available."
    echo "Please build from source instead:"
    echo ""
    echo "  git clone https://github.com/$REPO.git"
    echo "  cd litterbox"
    echo "  cargo build --release"
    echo ""
    echo "Exiting."
    exit 1
    ;;
esac
# ------------------------------------------

echo "Installing $BINARY_NAME from latest $REPO release..."
echo ""

# Ensure install dir exists
if [ ! -d "$INSTALL_DIR" ]; then
  echo "Creating $INSTALL_DIR ..."
  mkdir -p "$INSTALL_DIR"
fi

# Download binary
echo "Downloading $BINARY_NAME..."
curl -fL "$DOWNLOAD_URL" -o "$INSTALL_DIR/$BINARY_NAME"

# Make executable
chmod +x "$INSTALL_DIR/$BINARY_NAME"

echo ""
echo "✔ Installed to $INSTALL_DIR/$BINARY_NAME"
echo ""

# PATH check
case ":$PATH:" in
  *:"$INSTALL_DIR":*)
    echo "✔ $INSTALL_DIR is already in PATH."
    ;;
  *)
    echo "⚠ $INSTALL_DIR is not in PATH."
    echo ""
    echo "Add it to your shell profile:"
    echo ""
    echo "bash/zsh:"
    echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
    echo "  # or ~/.zshrc"
    echo ""
    echo "fish:"
    echo "  fish_add_path \$HOME/.local/bin"
    echo ""
    echo "After updating PATH, restart your shell."
    ;;
esac

echo "Litterbox installed!"
