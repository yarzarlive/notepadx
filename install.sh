#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building NotepadX release binary..."
cargo build --release

echo "Symlinking binary to ~/.local/bin/notepadx..."
mkdir -p "$HOME/.local/bin"
ln -sf "$SCRIPT_DIR/target/release/notepadx" "$HOME/.local/bin/notepadx"

echo "Installing desktop icon..."
mkdir -p "$HOME/.local/share/icons/hicolor/scalable/apps"
cp icons/com.notepadx.NotepadX.svg "$HOME/.local/share/icons/hicolor/scalable/apps/com.notepadx.NotepadX.svg"

echo "Installing desktop launcher..."
mkdir -p "$HOME/.local/share/applications"
rm -f "$HOME/.local/share/applications/com.notepadx.NotepadX.desktop"
cp "$SCRIPT_DIR/notepadx.desktop" "$HOME/.local/share/applications/notepadx.desktop"


if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo "NotepadX installation complete! You can run 'notepadx' or launch it from your desktop application menu."


