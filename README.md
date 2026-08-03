Disclaimer / About this project
This is a personal Linux app launcher built to fit my exact workflow. It was heavily vibe-coded using AI to get a working tool fast.

It works great on my setup!

The codebase might be a bit wild/unpolished under the hood.

A Wayland application launcher built with Rust, GTK4, and gtk4-layer-shell.

## Prerequisites

Requires GTK4 development libraries and Wayland layer-shell support.

- Arch Linux: `sudo pacman -S gtk4 gtk4-layer-shell`
- Debian/Ubuntu: `sudo apt install libgtk-4-dev libgtk4-layer-shell-dev`

## Install

Run the deployment script to build the release binary and install it to `~/.local/bin/launcher`:

```sh
./deploy.sh
```

Or build manually with Cargo:

```sh
cargo build --release
```

## Run

```sh
cargo run
```

Or run the installed binary:

```sh
~/.local/bin/launcher
```

## Configuration

Configuration is loaded from `~/.config/launcher/config.toml` (or `LAUNCHER_CONFIG`). If no config file exists, a default monotone theme is created automatically on first run.

Example `config.toml`:

```toml
accent_color = "#ffffff"
background_color = "rgb(22, 22, 22)"
background_opacity = 0.9
border_radius_box = 24
border_radius_entry = 14
border_radius_row = 12
shadow_opacity = 0.6
blur = true
# font_family = "Outfit"
```

For Hyprland background blur, add the following to `hyprland.conf`:

```ini
layerrule = blur, launcher
layerrule = ignorealpha 0.2, launcher
```
