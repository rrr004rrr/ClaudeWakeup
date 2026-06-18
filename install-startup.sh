#!/usr/bin/env bash
# Install (or remove) a LaunchAgent that starts ClaudeWakeup at login.
# Usage: ./install-startup.sh          -> install
#        ./install-startup.sh remove   -> remove
set -euo pipefail
cd "$(dirname "$0")"

LABEL="com.claudewakeup.app"
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"

if [ "${1:-}" = "remove" ]; then
    launchctl unload "$PLIST" 2>/dev/null || true
    rm -f "$PLIST" && echo "Removed login item." || echo "No login item."
    exit 0
fi

# Prefer the .app bundle (launched via `open`, so LSUIElement applies and there's
# no Dock icon); fall back to the bare release binary. Check /Applications first.
BIN="$(pwd)/target/release/ClaudeWakeup"
APP=""
for cand in "/Applications/ClaudeWakeup.app" "$(pwd)/dist/ClaudeWakeup.app"; do
    [ -d "$cand" ] && { APP="$cand"; break; }
done

mkdir -p "$HOME/Library/LaunchAgents"
if [ -n "$APP" ]; then
    PROG="<string>/usr/bin/open</string><string>-a</string><string>$APP</string>"
elif [ -x "$BIN" ]; then
    PROG="<string>$BIN</string>"
else
    echo "Build first: ./build.sh"
    exit 1
fi

cat > "$PLIST" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>$LABEL</string>
    <key>ProgramArguments</key><array>$PROG</array>
    <key>RunAtLoad</key><true/>
    <key>ProcessType</key><string>Interactive</string>
</dict>
</plist>
PLIST

launchctl unload "$PLIST" 2>/dev/null || true
launchctl load -w "$PLIST"
echo "Installed login item: $PLIST"
