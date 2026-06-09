#!/usr/bin/env bash
# Build a self-contained AppImage for healthctl with all companion binaries
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
test -d "$PROJECT_ROOT/target/appimage" && rm -rf "$PROJECT_ROOT/target/appimage"
APPDIR="$PROJECT_ROOT/target/appimage/healthctl.AppDir"
ARCH="$(uname -m)"

echo "==> Building release binaries..."
cargo build --release -p healthctl -p healthctl-daemon -p healthctl-dashboard

echo "==> Creating AppDir structure..."
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"

echo "==> Copying binaries..."
cp "$PROJECT_ROOT/target/release/healthctl" "$APPDIR/usr/bin/"
cp "$PROJECT_ROOT/target/release/healthctl-daemon" "$APPDIR/usr/bin/"
cp "$PROJECT_ROOT/target/release/healthctl-dashboard" "$APPDIR/usr/bin/"

echo "==> Copying icon..."
cp "$PROJECT_ROOT/crates/healthctl/healthctl.png" "$APPDIR/healthctl.png"
cp "$PROJECT_ROOT/crates/healthctl/healthctl.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/healthctl.png"

echo "==> Copying desktop entry..."
cp "$PROJECT_ROOT/crates/healthctl/healthctl.desktop" "$APPDIR/healthctl.desktop"

echo "==> Creating AppRun..."
cat > "$APPDIR/AppRun" << 'EOF'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin:${PATH}"
exec "${HERE}/usr/bin/healthctl" "$@"
EOF
chmod +x "$APPDIR/AppRun"

echo "==> Building AppImage..."
cd "$PROJECT_ROOT/target/appimage"

# Check if appimagetool is available
if ! command -v appimagetool &> /dev/null; then
    echo "Error: appimagetool not found. Please install it from:"
    echo "  https://github.com/AppImage/AppImageKit/releases"
    exit 1
fi

# Set ARCH for appimagetool (it expects this env var)
export ARCH
appimagetool "$APPDIR" "healthctl-${ARCH}.AppImage"

echo "==> Done! AppImage created at:"
echo "    $PROJECT_ROOT/target/appimage/healthctl-${ARCH}.AppImage"
ln -s "healthctl-${ARCH}.AppImage" "$PROJECT_ROOT/target/appimage/healthctl"
