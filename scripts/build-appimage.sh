#!/usr/bin/env bash
set -e

APP_DIR="AppDir"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/usr/bin"
mkdir -p "$APP_DIR/usr/lib"
mkdir -p "$APP_DIR/usr/share/icons/hicolor/512x512/apps"

# Copy binary
cp target/x86_64-unknown-linux-gnu/release/serialforge "$APP_DIR/usr/bin/"

# Copy desktop file
cp scripts/serialforge.desktop "$APP_DIR/"

# Copy icon to AppDir root AND standard icon path
cp assets/serialforge.png "$APP_DIR/serialforge.png"
cp assets/serialforge.png "$APP_DIR/usr/share/icons/hicolor/512x512/apps/serialforge.png"

# Create AppRun entrypoint
cat << 'EOF' > "$APP_DIR/AppRun"
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH}"
exec "${HERE}/usr/bin/serialforge" "$@"
EOF

chmod +x "$APP_DIR/AppRun"

echo "Bundling AppImage..."
ARCH=x86_64 appimagetool-bin "$APP_DIR" "SerialForge-x86_64.AppImage"