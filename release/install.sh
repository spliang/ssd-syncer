#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# SSD-Syncer Installer for macOS / Linux
# Place this script alongside the binary and run:
#   chmod +x install.sh && ./install.sh
# After installation, you can use "sync <command>" globally.
# ============================================================

echo ""
echo "  === SSD-Syncer Installer ==="
echo ""

# -- Determine the directory where this script resides --
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# -- Detect OS and pick the right binary --
OS="$(uname -s)"
case "$OS" in
    Darwin)
        BINARY_NAME="ssd-syncer-macos"
        ;;
    Linux)
        BINARY_NAME="ssd-syncer-linux"
        ;;
    *)
        echo "[ERROR] Unsupported OS: $OS"
        echo "This installer supports macOS and Linux only."
        echo "For Windows, please use install.bat"
        exit 1
        ;;
esac

BINARY_PATH="$SCRIPT_DIR/$BINARY_NAME"

if [ ! -f "$BINARY_PATH" ]; then
    echo "[ERROR] Binary not found: $BINARY_PATH"
    echo "Please place this script in the same directory as $BINARY_NAME"
    exit 1
fi

# -- Target installation directory --
INSTALL_DIR="$HOME/.ssd-syncer/bin"

echo "OS:                $OS"
echo "Binary:            $BINARY_NAME"
echo "Install directory: $INSTALL_DIR"
echo ""

# -- Create install directory --
mkdir -p "$INSTALL_DIR"

# -- Copy binary --
echo "Copying $BINARY_NAME ..."
cp "$BINARY_PATH" "$INSTALL_DIR/ssd-syncer"
chmod +x "$INSTALL_DIR/ssd-syncer"

# macOS: 清除 quarantine/provenance 属性并重新 ad-hoc 签名
if [ "$OS" = "Darwin" ]; then
    xattr -cr "$INSTALL_DIR/ssd-syncer" 2>/dev/null || true
    codesign --force --sign - "$INSTALL_DIR/ssd-syncer" 2>/dev/null || true
fi
echo "  Done."

# -- Create sync wrapper script --
echo "Creating sync wrapper ..."
cat > "$INSTALL_DIR/sync" << 'WRAPPER'
#!/usr/bin/env bash
SELF_DIR="$(cd "$(dirname "$0")" && pwd)"
exec "$SELF_DIR/ssd-syncer" "$@"
WRAPPER
chmod +x "$INSTALL_DIR/sync"
echo "  Done."

# -- Add to PATH in shell profiles --
echo ""
echo "Checking shell profiles ..."

PATH_LINE="export PATH=\"\$HOME/.ssd-syncer/bin:\$PATH\""
ADDED=false

add_to_profile() {
    local profile="$1"
    if [ -f "$profile" ]; then
        if grep -qF '.ssd-syncer/bin' "$profile" 2>/dev/null; then
            echo "  $profile: already configured"
        else
            echo "" >> "$profile"
            echo "# SSD-Syncer" >> "$profile"
            echo "$PATH_LINE" >> "$profile"
            echo "  $profile: updated"
            ADDED=true
        fi
    fi
}

# Try common shell profiles
if [ "$OS" = "Darwin" ]; then
    # macOS: default shell is zsh since Catalina
    if [ -f "$HOME/.zshrc" ]; then
        add_to_profile "$HOME/.zshrc"
    else
        # Create .zshrc if no shell profile exists
        echo "$PATH_LINE" > "$HOME/.zshrc"
        echo "  Created $HOME/.zshrc"
        ADDED=true
    fi
    # Also add to .bash_profile if it exists
    [ -f "$HOME/.bash_profile" ] && add_to_profile "$HOME/.bash_profile"
else
    # Linux
    add_to_profile "$HOME/.bashrc"
    [ -f "$HOME/.zshrc" ] && add_to_profile "$HOME/.zshrc"
    [ -f "$HOME/.profile" ] && add_to_profile "$HOME/.profile"

    # If none of the profiles existed, create .bashrc
    if [ "$ADDED" = false ] && [ ! -f "$HOME/.bashrc" ]; then
        echo "$PATH_LINE" > "$HOME/.bashrc"
        echo "  Created $HOME/.bashrc"
        ADDED=true
    fi
fi

echo ""
echo "  === Installation Complete ==="
echo ""
echo "  Binary:  $INSTALL_DIR/ssd-syncer"
echo "  Wrapper: $INSTALL_DIR/sync"
echo ""
echo "  IMPORTANT: Run the following command or open a new terminal"
echo "  for PATH changes to take effect:"
echo ""
echo "    source ~/.zshrc    # macOS (zsh)"
echo "    source ~/.bashrc   # Linux (bash)"
echo ""
echo "  Usage:"
echo "    sync list"
echo "    sync init --name \"my-machine\""
echo "    sync add --local /path/to/local --ssd \"SYNC_FOLDER\""
echo "    sync sync /Volumes/MySSD      # macOS"
echo "    sync sync /mnt/ssd            # Linux"
echo ""
