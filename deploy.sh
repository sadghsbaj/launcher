#!/bin/bash
# Exit immediately if a command exits with a non-zero status
set -e

echo "=== Starting Wayland Launcher Deployment ==="

# 1. Build the release binary
echo "Step 1: Building launcher in release mode..."
cargo build --release

# 2. Ensure target directories exist
echo "Step 2: Creating local target directories..."
mkdir -p ~/.local/bin
mkdir -p ~/.config/launcher

# Terminate active launcher processes to release the file lock
WAS_RUNNING=0
if pgrep -x "launcher" > /dev/null; then
    echo "Terminating active launcher processes to release the file lock..."
    killall -q launcher || true
    sleep 0.5
    WAS_RUNNING=1
fi

# 3. Deploy the binary
echo "Step 3: Copying binary to ~/.local/bin/launcher..."
cp target/release/launcher ~/.local/bin/launcher
chmod +x ~/.local/bin/launcher

# 4. Deploy default configuration if not already present
CONFIG_DEST="$HOME/.config/launcher/config.toml"
WRITE_DEFAULT=0

if [ ! -f "$CONFIG_DEST" ]; then
    echo "Step 4: No config file found. Deploying default clean monotone config..."
    WRITE_DEFAULT=1
else
    # Check if they have the old default blue accent configuration
    if grep -q 'accent_color = "#3b82f6"' "$CONFIG_DEST"; then
        echo "Step 4: Old blue accent config detected. Backing it up to config.toml.bak and replacing with monotone white default..."
        cp "$CONFIG_DEST" "$CONFIG_DEST.bak"
        WRITE_DEFAULT=1
    else
        echo "Step 4: Existing custom configuration found at $CONFIG_DEST. Keeping it."
    fi
fi

if [ "$WRITE_DEFAULT" -eq 1 ]; then
    cat << 'EOF' > "$CONFIG_DEST"
# Wayland Launcher Configuration
# Customize the look and feel of your launcher

# Primary/Accent color used for selection highlight, borders, and carets (supports HEX or RGB/RGBA)
accent_color = "#ffffff"

# Window background color (supports HEX or RGB/RGBA)
background_color = "rgb(22, 22, 22)"

# Window background opacity (0.0 to 1.0)
background_opacity = 0.9

# Border radii (in pixels)
border_radius_box = 24
border_radius_entry = 14
border_radius_row = 12

# Box shadow opacity (0.0 to 1.0)
shadow_opacity = 0.6

# Request compositor blur rule matching namespace "launcher" (true or false)
# (For Hyprland, add: layerrule = blur, launcher)
blur = true

# Custom font family (set to a string like "Outfit" or leave commented out to use system default)
# font_family = "Outfit"
EOF
fi

# 5. Restart running launcher process if it was active
if [ "$WAS_RUNNING" -eq 1 ]; then
    echo "Step 5: Restarting launcher in the background..."
    ~/.local/bin/launcher &
fi

echo "=== Deployment Successful! ==="
