#!/bin/bash

# setup_hotkey.sh
# Automates adding a custom global shortcut in GNOME for test-popup-ai.

set -e

APP_PATH="$(pwd)/target/release/test-popup-ai"
COMMAND="$APP_PATH --ui"
NAME="AI Popup Assistant"
HOTKEY="<Primary>space" # GNOME's representation of Ctrl+Space

if [ ! -f "$APP_PATH" ]; then
    echo "❌ Error: compiled executable not found at $APP_PATH"
    echo "Please run 'cargo build --release' first."
    exit 1
fi

echo "✦ Setting up GNOME Custom Shortcut..."

# Base dconf path for custom shortcuts
BASE_PATH="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings"

# Generate a unique ID for our shortcut
SHORTCUT_ID="custom-test-popup-ai"
SHORTCUT_PATH="$BASE_PATH/$SHORTCUT_ID/"

# 1. Get current custom shortcuts array
CURRENT_BINDINGS=$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings)
if [ "$CURRENT_BINDINGS" = "@as []" ]; then
    NEW_BINDINGS="['$SHORTCUT_PATH']"
else
    # Remove our shortcut if it already exists to avoid duplicates, then append it
    CLEANED_BINDINGS=$(echo "$CURRENT_BINDINGS" | sed -e "s|'$SHORTCUT_PATH'||g" -e "s/, ,/,/g" -e "s/\[, /\[/g" -e "s/, ]/]/g")
    # Strip the trailing bracket and append our new one
    NEW_BINDINGS="${CLEANED_BINDINGS%]*}, '$SHORTCUT_PATH']"
fi

# 2. Add our shortcut to the list
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "$NEW_BINDINGS"

# 3. Create the actual shortcut details
dconf write "$SHORTCUT_PATH"name "'$NAME'"
dconf write "$SHORTCUT_PATH"command "'$COMMAND'"
dconf write "$SHORTCUT_PATH"binding "'$HOTKEY'"

echo "✅ Shortcut configured successfully!"
echo "Press 'Ctrl+Space' anywhere to summon the AI assistant."
echo "Note: If you are on KDE or another DE, please bind the following command manually:"
echo "      $COMMAND"
