# Wayland Launcher - Theming & Configuration

This guide describes how to configure and customize the appearance of the Wayland Launcher.

## Configuration File Location

The launcher looks for `config.toml` in the following priority order:

1. **Environment Variable**: `LAUNCHER_CONFIG=/path/to/config.toml`
2. **Development Mode**: A file named `config.toml` in the current working directory.
3. **User Configuration (XDG)**: `~/.config/launcher/config.toml` (Standard path)

> [!TIP]
> If no configuration file exists at any of these paths, the launcher will automatically generate a default, clean monotone configuration at `~/.config/launcher/config.toml`.

---

## Configuration Options

Here are the keys available in `config.toml`:

| Key | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `accent_color` | String | `"#ffffff"` | Primary color for selected items, caret/cursor, and active borders. Supports `#hex` or `rgb/rgba`. |
| `background_color` | String | `"rgb(22, 22, 22)"` | Base background color of the launcher. Supports `#hex` or `rgb/rgba`. |
| `background_opacity` | Float | `0.9` | Opacity of the background window card (`0.0` for transparent, `1.0` for opaque). |
| `border_radius_box` | Integer | `24` | Border radius of the main outer window (in pixels). |
| `border_radius_entry` | Integer | `14` | Border radius of the search entry field (in pixels). |
| `border_radius_row` | Integer | `12` | Border radius of the individual search result rows (in pixels). |
| `shadow_opacity` | Float | `0.6` | Box shadow strength of the window card (`0.0` to disable shadows, `1.0` for max depth). |
| `blur` | Boolean | `true` | Request blur matching the layer shell namespace `"launcher"`. |
| `font_family` | String | *(System default)* | Custom font name (e.g. `"Outfit"`, `"Inter"`). Leave commented out to use system UI font. |

---

## Example `config.toml`

To test different setups, copy [config.toml.example](file:///home/colin/Main/01_programming/01_projects/01_current_projects/launcher/config.toml.example) to your configuration path and uncomment the keys you want to change:

```toml
# Enable a custom font and vibrant blue highlight
accent_color = "#3b82f6"
font_family = "Outfit"

# Make the window card more transparent
background_opacity = 0.75
```

---

## Wayland Compositor Blur Integration

The launcher registers its Wayland windows under the layer-shell namespace `"launcher"`. To enable compositor-level background blur on **Hyprland**, add the following rules to your `hyprland.conf`:

```ini
# Enable blur for the launcher windows
layerrule = blur, launcher

# Prevent the blur from eating thin borders/shadow margins
layerrule = ignorealpha 0.2, launcher
```

---

## Troubleshooting & Error Reporting

If your configuration contains syntax errors or unknown fields, the launcher will:
1. Print a detailed error warning to `stderr` (visible in terminal or logs).
2. Gracefully fall back to the default monotone theme so the launcher still launches successfully without crashing.
