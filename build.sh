#!/usr/bin/env bash
# Build ClaudeWakeup for macOS (release) and assemble a menu-bar .app bundle.
# Usage: ./build.sh
set -euo pipefail
cd "$(dirname "$0")"

VERSION="$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')"

echo "Building release binary…"
cargo build --release

APP="dist/ClaudeWakeup.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp target/release/ClaudeWakeup "$APP/Contents/MacOS/ClaudeWakeup"
chmod +x "$APP/Contents/MacOS/ClaudeWakeup"

# LSUIElement=true → a pure menu-bar app (no Dock icon), matching the Windows tray app.
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>ClaudeWakeup</string>
    <key>CFBundleDisplayName</key><string>ClaudeWakeup</string>
    <key>CFBundleExecutable</key><string>ClaudeWakeup</string>
    <key>CFBundleIdentifier</key><string>com.claudewakeup.app</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>CFBundleVersion</key><string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key><string>10.14</string>
    <key>LSUIElement</key><true/>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

echo "Built: $APP"
echo "Recommended: move it out of any iCloud-synced folder, e.g."
echo "  mv \"$APP\" /Applications/"
echo "Run it with:  open /Applications/ClaudeWakeup.app   (or: open \"$APP\")"
echo "Autostart at login:  ./install-startup.sh"
