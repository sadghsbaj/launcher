use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_accent_color")]
    pub accent_color: String,

    #[serde(default = "default_background_color")]
    pub background_color: String,

    #[serde(default = "default_background_opacity")]
    pub background_opacity: f32,

    #[serde(default = "default_border_radius_box")]
    pub border_radius_box: u32,

    #[serde(default = "default_border_radius_entry")]
    pub border_radius_entry: u32,

    #[serde(default = "default_border_radius_row")]
    pub border_radius_row: u32,

    #[serde(default = "default_shadow_opacity")]
    pub shadow_opacity: f32,

    #[serde(default = "default_blur")]
    pub blur: bool,

    #[serde(default = "default_font_family")]
    pub font_family: Option<String>,
}

fn default_accent_color() -> String { "#ffffff".to_string() }
fn default_background_color() -> String { "rgb(22, 22, 22)".to_string() }
fn default_background_opacity() -> f32 { 0.9 }
fn default_border_radius_box() -> u32 { 24 }
fn default_border_radius_entry() -> u32 { 14 }
fn default_border_radius_row() -> u32 { 12 }
fn default_shadow_opacity() -> f32 { 0.6 }
fn default_blur() -> bool { true }
fn default_font_family() -> Option<String> { None }

impl Default for Config {
    fn default() -> Self {
        Config {
            accent_color: default_accent_color(),
            background_color: default_background_color(),
            background_opacity: default_background_opacity(),
            border_radius_box: default_border_radius_box(),
            border_radius_entry: default_border_radius_entry(),
            border_radius_row: default_border_radius_row(),
            shadow_opacity: default_shadow_opacity(),
            blur: default_blur(),
            font_family: default_font_family(),
        }
    }
}

pub fn get_config_path() -> PathBuf {
    if let Ok(env_path) = std::env::var("LAUNCHER_CONFIG") {
        return PathBuf::from(env_path);
    }
    
    let local_path = PathBuf::from("config.toml");
    if local_path.exists() {
        return local_path;
    }
    
    if let Some(mut config_dir) = dirs::config_dir() {
        config_dir.push("launcher");
        config_dir.push("config.toml");
        return config_dir;
    }
    
    local_path
}

const DEFAULT_CONFIG_TEMPLATE: &str = r##"# Wayland Launcher Configuration
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
"##;

pub fn load_or_create() -> Config {
    let path = get_config_path();
    
    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::write(&path, DEFAULT_CONFIG_TEMPLATE) {
            eprintln!("Warning: Failed to write default config to {:?}: {}", path, e);
        }
        return Config::default();
    }
    
    match fs::read_to_string(&path) {
        Ok(content) => {
            match toml::from_str::<Config>(&content) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("====================================================================");
                    eprintln!("⚠️ CONFIG ERROR: Failed to parse configuration file at {:?}", path);
                    eprintln!("Error details: {}", err);
                    eprintln!("Falling back to default settings.");
                    eprintln!("====================================================================");
                    Config::default()
                }
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to read config file at {:?}: {}. Using default settings.", path, e);
            Config::default()
        }
    }
}

pub fn color_to_rgba(color: &str, alpha: f32) -> String {
    let trimmed = color.trim();
    if trimmed.starts_with("rgb") || trimmed.starts_with("rgba") {
        return trimmed.to_string();
    }
    
    let clean_hex = trimmed.trim_start_matches('#');
    let is_hex = clean_hex.chars().all(|c| c.is_ascii_hexdigit()) 
        && (clean_hex.len() == 3 || clean_hex.len() == 6);
        
    if is_hex {
        let mut rgb = (255, 255, 255);
        if clean_hex.len() == 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&clean_hex[0..1], 16),
                u8::from_str_radix(&clean_hex[1..2], 16),
                u8::from_str_radix(&clean_hex[2..3], 16),
            ) {
                rgb = (r * 17, g * 17, b * 17);
            }
        } else if clean_hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&clean_hex[0..2], 16),
                u8::from_str_radix(&clean_hex[2..4], 16),
                u8::from_str_radix(&clean_hex[4..6], 16),
            ) {
                rgb = (r, g, b);
            }
        }
        format!("rgba({}, {}, {}, {})", rgb.0, rgb.1, rgb.2, alpha)
    } else {
        trimmed.to_string()
    }
}

pub fn generate_css(config: &Config) -> String {
    let font_family_rule = if let Some(ref font) = config.font_family {
        format!("font-family: \"{}\", sans-serif;", font)
    } else {
        "".to_string()
    };

    let accent_selected = color_to_rgba(&config.accent_color, 0.12);
    let accent_hover = color_to_rgba(&config.accent_color, 0.06);
    let accent_border = color_to_rgba(&config.accent_color, 0.25);
    let accent_border_focus = color_to_rgba(&config.accent_color, 0.50);
    let accent_solid = color_to_rgba(&config.accent_color, 1.0);
    
    // Parse background color and apply opacity
    let bg_color = if config.background_color.trim().starts_with("rgb") {
        let clean = config.background_color.replace("rgb", "").replace("rgba", "").replace("(", "").replace(")", "");
        let parts: Vec<&str> = clean.split(',').map(|s| s.trim()).collect();
        if parts.len() >= 3 {
            format!("rgba({}, {}, {}, {})", parts[0], parts[1], parts[2], config.background_opacity)
        } else {
            format!("rgba(22, 22, 22, {})", config.background_opacity)
        }
    } else {
        color_to_rgba(&config.background_color, config.background_opacity)
    };

    format!(
        "
        window.launcher-window,
        window.launcher-window.background,
        window.launcher-window .background,
        window.launcher-window decoration,
        window.launcher-window .csd decoration,
        window.launcher-window .window-frame,
        window.launcher-window .solid-csd {{
            background-color: rgba(0, 0, 0, 0);
            background-image: none;
            box-shadow: none;
            border-style: none;
            border-width: 0px;
            margin-top: 0px;
            margin-bottom: 0px;
            margin-left: 0px;
            margin-right: 0px;
            padding-top: 0px;
            padding-bottom: 0px;
            padding-left: 0px;
            padding-right: 0px;
            {font_family_rule}
        }}

        .single-line-title {{
            font-weight: 600;
            font-size: 16px;
        }}

        .launcher-box {{
            background-color: {bg_color};
            border: 1px solid rgba(255, 255, 255, 0.08);
            border-radius: {border_radius_box}px;
            box-shadow: 0 12px 40px rgba(0, 0, 0, {shadow_opacity});
            padding-top: 12px;
            padding-bottom: 12px;
            padding-left: 12px;
            padding-right: 12px;
            margin-top: 30px;
            margin-bottom: 50px;
            margin-left: 40px;
            margin-right: 40px;
        }}

        window.launcher-window entry.search-entry {{
            background-color: rgba(255, 255, 255, 0.03);
            border: 1px solid rgba(255, 255, 255, 0.05);
            border-radius: {border_radius_entry}px;
            color: #ffffff;
            font-size: 18px;
            padding-top: 10px;
            padding-bottom: 10px;
            padding-left: 16px;
            padding-right: 16px;
            caret-color: {accent_solid};
            outline: none;
            outline-color: transparent;
            outline-style: none;
            outline-width: 0px;
            box-shadow: none;
            transition: all 200ms ease;
        }}

        window.launcher-window entry.search-entry:focus,
        window.launcher-window entry.search-entry:focus-within {{
            background-color: rgba(255, 255, 255, 0.05);
            border: 1px solid {accent_border_focus};
            box-shadow: none;
            outline: none;
            outline-color: transparent;
            outline-style: none;
            outline-width: 0px;
        }}

        .results-scroll {{
            margin-top: 10px;
        }}

        .results-list {{
            background-color: rgba(0, 0, 0, 0);
            padding-top: 4px;
            padding-bottom: 4px;
        }}

        .results-list row {{
            background-color: transparent;
            border-radius: {border_radius_row}px;
            margin-top: 2px;
            margin-bottom: 2px;
            margin-left: 0;
            margin-right: 0;
            padding-top: 8px;
            padding-bottom: 8px;
            padding-left: 12px;
            padding-right: 12px;
            color: #e0e0e0;
            transition: all 120ms cubic-bezier(0.25, 0.46, 0.45, 0.94);
        }}

        .results-list row:selected {{
            background-color: {accent_selected};
            color: #ffffff;
        }}

        .results-list row:hover {{
            background-color: {accent_hover};
            color: #ffffff;
        }}

        .result-title {{
            font-weight: 600;
            font-size: 15px;
        }}

        .result-description {{
            color: #888888;
            font-size: 12px;
        }}

        scrollbar, scrollbar.vertical, .results-scroll scrollbar {{
            opacity: 0;
            min-width: 0px;
            min-height: 0px;
            margin-top: 0px;
            margin-bottom: 0px;
            margin-left: 0px;
            margin-right: 0px;
            padding-top: 0px;
            padding-bottom: 0px;
            padding-left: 0px;
            padding-right: 0px;
        }}

        .picker-btn {{
            background-color: rgba(255, 255, 255, 0.05);
            border: 1px solid {accent_border};
            border-radius: 8px;
            color: #ffffff;
            font-size: 11px;
            padding-top: 4px;
            padding-bottom: 4px;
            padding-left: 10px;
            padding-right: 10px;
            transition: all 120ms ease;
        }}

        .picker-btn:hover {{
            background-color: {accent_hover};
            border: 1px solid {accent_border_focus};
        }}

        .picker-btn:active {{
            background-color: {accent_selected};
        }}
        ",
        font_family_rule = font_family_rule,
        bg_color = bg_color,
        border_radius_box = config.border_radius_box,
        border_radius_entry = config.border_radius_entry,
        border_radius_row = config.border_radius_row,
        shadow_opacity = config.shadow_opacity,
        accent_solid = accent_solid,
        accent_border_focus = accent_border_focus,
        accent_selected = accent_selected,
        accent_hover = accent_hover,
        accent_border = accent_border,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.accent_color, "#ffffff");
        assert_eq!(config.border_radius_box, 24);
        assert_eq!(config.blur, true);
    }

    #[test]
    fn test_color_to_rgba() {
        assert_eq!(color_to_rgba("#3b82f6", 0.5), "rgba(59, 130, 246, 0.5)");
        assert_eq!(color_to_rgba("3b82f6", 1.0), "rgba(59, 130, 246, 1)");
        assert_eq!(color_to_rgba("#fff", 0.1), "rgba(255, 255, 255, 0.1)");
        assert_eq!(color_to_rgba("rgb(1, 2, 3)", 0.8), "rgb(1, 2, 3)");
        assert_eq!(color_to_rgba("rgba(1, 2, 3, 0.4)", 0.8), "rgba(1, 2, 3, 0.4)");
        assert_eq!(color_to_rgba("red", 0.5), "red");
    }

    #[test]
    fn test_invalid_toml_fallback() {
        let invalid_toml = "invalid_field = 123\naccent_color = 12345";
        let parsed = toml::from_str::<Config>(invalid_toml);
        assert!(parsed.is_err());
    }
}

