set -e

INSTALL_DIR="$HOME/.local/bin"
REPO="talhabw/nlsh"

echo "Installing nlsh (latest release)..."

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux) PLATFORM="linux" ;;
    Darwin) PLATFORM="macos" ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="arm64" ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

ASSET="nlsh-${PLATFORM}-${ARCH}"
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"

mkdir -p "$INSTALL_DIR"

echo "Downloading ${ASSET}..."
if ! curl -fsSL "$DOWNLOAD_URL" -o "$INSTALL_DIR/nlsh"; then
    echo "Failed to download ${ASSET}."
    echo "Check the latest release assets at:"
    echo "https://github.com/${REPO}/releases/latest"
    exit 1
fi

chmod +x "$INSTALL_DIR/nlsh"

setup_shell() {
    local rc_file="$1"
    touch "$rc_file"
    
    if ! grep -q '.local/bin' "$rc_file" 2>/dev/null; then
        echo '' >> "$rc_file"
        echo '# nlsh - PATH' >> "$rc_file"
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$rc_file"
    fi
    
    if grep -q 'nlsh # auto-start' "$rc_file" 2>/dev/null; then
        sed -i '' '/nlsh # auto-start/d' "$rc_file" 2>/dev/null || sed -i '/nlsh # auto-start/d' "$rc_file" 2>/dev/null
    fi
    if grep -q 'nlsh - auto-start' "$rc_file" 2>/dev/null; then
        sed -i '' '/nlsh - auto-start/d' "$rc_file" 2>/dev/null || sed -i '/nlsh - auto-start/d' "$rc_file" 2>/dev/null
    fi
}

case "$(basename "$SHELL")" in
  zsh)
    [ -f "$HOME/.zshrc" ] && setup_shell "$HOME/.zshrc"
    ;;
  bash)
    [ -f "$HOME/.bashrc" ] && setup_shell "$HOME/.bashrc"
    ;;
esac

export PATH="$HOME/.local/bin:$PATH"

echo ""
echo "nlsh installed successfully!"
echo ""
echo "Open a new terminal to start using nlsh, or run: nlsh"
echo ""
