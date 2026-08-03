# Theming & Configuration

The launcher features a fully customizable CSS-based layout driven by a single TOML configuration file.

---

## 1. Config Lookup Priority

The configuration is resolved at launch in the following priority order:
1. **Env Var**: `LAUNCHER_CONFIG=/path/to/custom.toml`
2. **Local Path**: `./config.toml` (if running in dev/workspace directory)
3. **Standard Path**: `~/.config/launcher/config.toml` (Standard XDG configuration)

If no configuration file is found, a default, commented configuration file is automatically written to `~/.config/launcher/config.toml` (Self-Healing).

---

## 2. Config options

The following keys can be set inside `config.toml`:

```toml
# Accent color for selected row highlights, borders, and carets (Hex/RGB/RGBA supported)
accent_color = "#ffffff"

# Window card background color
background_color = "rgb(22, 22, 22)"

# Window background opacity (0.0 to 1.0)
background_opacity = 0.9

# Border radii (in pixels)
border_radius_box = 24
border_radius_entry = 14
border_radius_row = 12

# Box shadow opacity (0.0 to 1.0)
shadow_opacity = 0.6

# Request compositor blur matching namespace "launcher" (true or false)
blur = true

# Custom font family (uncomment to override, defaults to system UI font)
# font_family = "Outfit"
```

---

## 3. Dynamic CSS Generation

At startup, the configuration values are loaded and processed in [src/config.rs](file:///home/colin/Main/01_programming/01_projects/01_current_projects/launcher/src/config.rs#L149) into a global GTK CSS provider.

* **Hex-to-RGBA Parser**: To render selection overlays and borders with transparency, the launcher parses the `accent_color` (supporting `#rgb` and `#rrggbb`) and generates custom transparent versions (e.g., selected row gets `0.12` opacity, hover gets `0.06` opacity, active borders get `0.25` or `0.5` opacity).
* **Font Fallback**: If `font_family` is omitted or commented out, the launcher does not inject a `font-family` CSS rule, letting the system's GTK settings decide the default UI font.

---

## 4. Wayland Compositor Blur Integration

Wayland windows cannot request background blur directly. Instead, they register a **namespace identifier** with the compositor.

* The launcher sets the namespace on all window structures (Main window, Color Picker, and Translator):
  ```rust
  window.set_namespace(Some("launcher"));
  ```
* In your Hyprland configuration (`hyprland.conf`), you can target this namespace to apply blur:
  ```ini
  # Apply blur to the launcher namespace
  layerrule = blur, launcher

  # Ignore the 40px outer shadow margins of the window card
  layerrule = ignorealpha 0.2, launcher
  ```
