use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

pub const BUNDLED_NEVERMORE_DARK: &str = include_str!("../resources/themes/nevermore-dark.toml");
pub const BUNDLED_CATPPUCCIN_MOCHA: &str =
    include_str!("../resources/themes/catppuccin-mocha.toml");
pub const BUNDLED_TOKYO_NIGHT: &str = include_str!("../resources/themes/tokyo-night.toml");

pub fn default_name() -> String {
    "nevermore-dark".to_string()
}
pub fn default_bg_primary() -> String {
    "#18181b".to_string()
}
pub fn default_bg_secondary() -> String {
    "#202024".to_string()
}
pub fn default_bg_tertiary() -> String {
    "#27272a".to_string()
}
pub fn default_border_color() -> String {
    "#2e2e32".to_string()
}
pub fn default_text_primary() -> String {
    "#f4f4f5".to_string()
}
pub fn default_text_secondary() -> String {
    "#a1a1aa".to_string()
}
pub fn default_accent_color() -> String {
    "#0a84ff".to_string()
}
pub fn default_danger_color() -> String {
    "#ef4444".to_string()
}
pub fn default_success_color() -> String {
    "#22c55e".to_string()
}
pub fn default_warning_color() -> String {
    "#f59e0b".to_string()
}
pub fn default_ui_font() -> String {
    "Inter".to_string()
}
pub fn default_mono_font() -> String {
    "Hack Nerd Font Mono".to_string()
}
pub fn default_mono_font_size() -> u32 {
    10
}
pub fn default_border_radius_px() -> u32 {
    8
}
pub fn default_term_bg() -> String {
    "#18181b".to_string()
}
pub fn default_term_fg() -> String {
    "#f4f4f5".to_string()
}
pub fn default_term_cursor() -> String {
    "#0a84ff".to_string()
}
pub fn default_term_palette() -> [String; 16] {
    [
        "#18181b".to_string(),
        "#ef4444".to_string(),
        "#22c55e".to_string(),
        "#f59e0b".to_string(),
        "#0a84ff".to_string(),
        "#a855f7".to_string(),
        "#06b6d4".to_string(),
        "#f4f4f5".to_string(),
        "#3f3f46".to_string(),
        "#f87171".to_string(),
        "#4ade80".to_string(),
        "#fbbf24".to_string(),
        "#60a5fa".to_string(),
        "#c084fc".to_string(),
        "#22d3ee".to_string(),
        "#ffffff".to_string(),
    ]
}

pub fn is_valid_solid_hex_color(s: &str) -> bool {
    if s.len() != 7 || !s.starts_with('#') {
        return false;
    }
    s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

pub fn sanitize_font(s: &str) -> String {
    s.replace(['"', ';', '{', '}', '\\'], "")
        .replace("/*", "")
        .replace("*/", "")
        .trim()
        .to_string()
}

pub fn validate_theme_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err(format!(
            "Theme name '{}' length must be between 1 and 64 characters",
            name
        ));
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(format!(
            "Theme name '{}' contains illegal path traversal characters",
            name
        ));
    }
    for ch in name.chars() {
        if !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') {
            return Err(format!(
                "Theme name '{}' contains invalid character '{}' (only ^[a-zA-Z0-9_-]{{1,64}}$ allowed)",
                name, ch
            ));
        }
    }
    Ok(())
}

pub fn contrast_text_color_for_hex(hex: &str) -> &'static str {
    if hex.len() == 7 && hex.starts_with('#') {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[1..3], 16),
            u8::from_str_radix(&hex[3..5], 16),
            u8::from_str_radix(&hex[5..7], 16),
        ) {
            let lum = 0.299 * (r as f64) + 0.587 * (g as f64) + 0.114 * (b as f64);
            if lum > 140.0 {
                return "#11111b";
            }
        }
    }
    "#ffffff"
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePalette {
    #[serde(default = "default_name")]
    pub name: String,

    #[serde(default = "default_bg_primary")]
    pub bg_primary: String,
    #[serde(default = "default_bg_secondary")]
    pub bg_secondary: String,
    #[serde(default = "default_bg_tertiary")]
    pub bg_tertiary: String,
    #[serde(default = "default_border_color")]
    pub border_color: String,
    #[serde(default = "default_text_primary")]
    pub text_primary: String,
    #[serde(default = "default_text_secondary")]
    pub text_secondary: String,
    #[serde(default = "default_accent_color")]
    pub accent_color: String,
    #[serde(default = "default_danger_color")]
    pub danger_color: String,
    #[serde(default = "default_success_color")]
    pub success_color: String,
    #[serde(default = "default_warning_color")]
    pub warning_color: String,

    #[serde(default = "default_ui_font")]
    pub ui_font: String,
    #[serde(default = "default_mono_font")]
    pub mono_font: String,
    #[serde(default = "default_mono_font_size")]
    pub mono_font_size: u32,
    #[serde(default = "default_border_radius_px")]
    pub border_radius_px: u32,

    #[serde(default = "default_term_bg")]
    pub term_bg: String,
    #[serde(default = "default_term_fg")]
    pub term_fg: String,
    #[serde(default = "default_term_cursor")]
    pub term_cursor: String,
    #[serde(default = "default_term_palette")]
    pub term_palette: [String; 16],
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self {
            name: default_name(),
            bg_primary: default_bg_primary(),
            bg_secondary: default_bg_secondary(),
            bg_tertiary: default_bg_tertiary(),
            border_color: default_border_color(),
            text_primary: default_text_primary(),
            text_secondary: default_text_secondary(),
            accent_color: default_accent_color(),
            danger_color: default_danger_color(),
            success_color: default_success_color(),
            warning_color: default_warning_color(),
            ui_font: default_ui_font(),
            mono_font: default_mono_font(),
            mono_font_size: default_mono_font_size(),
            border_radius_px: default_border_radius_px(),
            term_bg: default_term_bg(),
            term_fg: default_term_fg(),
            term_cursor: default_term_cursor(),
            term_palette: default_term_palette(),
        }
    }
}

impl ThemePalette {
    pub fn from_toml(content: &str) -> Result<Self, String> {
        Ok(Self::from_toml_str_lossy(content))
    }

    /// Hardened lossy parser: validates every field per-field and falls back safely to Nevermore Dark defaults.
    pub fn from_toml_str_lossy(content: &str) -> Self {
        let value = match toml::from_str::<toml::Value>(content) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "[nevermore-theme] warning: malformed TOML ({}), falling back to default theme",
                    e
                );
                return Self::default();
            }
        };

        let table = match value.as_table() {
            Some(t) => t,
            None => {
                eprintln!("[nevermore-theme] warning: TOML root is not a table, falling back to default theme");
                return Self::default();
            }
        };

        let mut palette = Self::default();

        // 1. Theme name
        if let Some(v) = table.get("name") {
            if let Some(s) = v.as_str() {
                let sanitized = s.replace("*/", "").replace('\n', "").trim().to_string();
                if !sanitized.is_empty() {
                    palette.name = sanitized;
                }
            } else {
                eprintln!("[nevermore-theme] warning: 'name' is not a string, using default");
            }
        }

        // Helper for color fields
        let parse_color = |key: &str, default_val: &str| -> String {
            if let Some(val) = table.get(key) {
                if let Some(s) = val.as_str() {
                    let trimmed = s.trim();
                    if is_valid_solid_hex_color(trimmed) {
                        return trimmed.to_ascii_lowercase();
                    } else {
                        eprintln!(
                            "[nevermore-theme] warning: invalid solid hex color '{}' for '{}', falling back to '{}'",
                            trimmed, key, default_val
                        );
                    }
                } else {
                    eprintln!(
                        "[nevermore-theme] warning: '{}' is not a string, falling back to '{}'",
                        key, default_val
                    );
                }
            }
            default_val.to_string()
        };

        palette.bg_primary = parse_color("bg_primary", &default_bg_primary());
        palette.bg_secondary = parse_color("bg_secondary", &default_bg_secondary());
        palette.bg_tertiary = parse_color("bg_tertiary", &default_bg_tertiary());
        palette.border_color = parse_color("border_color", &default_border_color());
        palette.text_primary = parse_color("text_primary", &default_text_primary());
        palette.text_secondary = parse_color("text_secondary", &default_text_secondary());
        palette.accent_color = parse_color("accent_color", &default_accent_color());
        palette.danger_color = parse_color("danger_color", &default_danger_color());
        palette.success_color = parse_color("success_color", &default_success_color());
        palette.warning_color = parse_color("warning_color", &default_warning_color());

        palette.term_bg = parse_color("term_bg", &default_term_bg());
        palette.term_fg = parse_color("term_fg", &default_term_fg());
        palette.term_cursor = parse_color("term_cursor", &default_term_cursor());

        // Fonts
        if let Some(v) = table.get("ui_font") {
            if let Some(s) = v.as_str() {
                let clean = sanitize_font(s);
                if !clean.is_empty() {
                    palette.ui_font = clean;
                }
            } else {
                eprintln!("[nevermore-theme] warning: 'ui_font' is not a string, using default");
            }
        }

        if let Some(v) = table.get("mono_font") {
            if let Some(s) = v.as_str() {
                let clean = sanitize_font(s);
                if !clean.is_empty() {
                    palette.mono_font = clean;
                }
            } else {
                eprintln!("[nevermore-theme] warning: 'mono_font' is not a string, using default");
            }
        }

        // Integers
        if let Some(v) = table.get("mono_font_size") {
            if let Some(n) = v.as_integer() {
                if (6..=36).contains(&n) {
                    palette.mono_font_size = n as u32;
                } else {
                    eprintln!("[nevermore-theme] warning: 'mono_font_size' out of range (6-36), using default");
                }
            } else {
                eprintln!(
                    "[nevermore-theme] warning: 'mono_font_size' is not an integer, using default"
                );
            }
        }

        if let Some(v) = table.get("border_radius_px") {
            if let Some(n) = v.as_integer() {
                if (0..=32).contains(&n) {
                    palette.border_radius_px = n as u32;
                } else {
                    eprintln!("[nevermore-theme] warning: 'border_radius_px' out of range (0-32), using default");
                }
            } else {
                eprintln!("[nevermore-theme] warning: 'border_radius_px' is not an integer, using default");
            }
        }

        // Terminal 16-color palette
        let default_palette = default_term_palette();
        let mut final_palette = default_palette.clone();
        if let Some(v) = table.get("term_palette") {
            if let Some(arr) = v.as_array() {
                if arr.len() != 16 {
                    eprintln!(
                        "[nevermore-theme] warning: 'term_palette' has {} items (expected 16), filling remainder with default palette",
                        arr.len()
                    );
                }
                for i in 0..16 {
                    if let Some(item) = arr.get(i) {
                        if let Some(s) = item.as_str() {
                            let trimmed = s.trim();
                            if is_valid_solid_hex_color(trimmed) {
                                final_palette[i] = trimmed.to_ascii_lowercase();
                            } else {
                                eprintln!(
                                    "[nevermore-theme] warning: invalid color '{}' at term_palette[{}], using default '{}'",
                                    trimmed, i, default_palette[i]
                                );
                            }
                        } else {
                            eprintln!(
                                "[nevermore-theme] warning: non-string item at term_palette[{}], using default '{}'",
                                i, default_palette[i]
                            );
                        }
                    }
                }
            } else {
                eprintln!(
                    "[nevermore-theme] warning: 'term_palette' is not an array, using default"
                );
            }
        }
        palette.term_palette = final_palette;

        palette
    }

    /// Generates the complete GTK4 CSS string for Nevermore Sidebar with solid colors.
    pub fn to_gtk_css(&self) -> String {
        let clean_name = self.name.replace("*/", "").replace('\n', "");
        let accent_fg = contrast_text_color_for_hex(&self.accent_color);

        format!(
            r#"/* Nevermore Dynamic GTK4 CSS — Theme: {name} */

window {{
    background-color: {bg_primary};
    color: {text_primary};
    font-family: "{ui_font}", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}}

.sidebar-panel {{
    background-color: {bg_primary};
}}

/* 2px Hot-Edge Strip when Hidden */
.hot-edge-strip {{
    background-color: rgba(24, 24, 27, 0.01);
    min-width: 2px;
}}

/* Nav Rail (60px) */
.nav-rail {{
    background-color: {bg_secondary};
    border-right: 1px solid {border_color};
    padding: 8px 4px;
}}

.nav-btn {{
    background: transparent;
    color: {text_secondary};
    border: none;
    border-radius: {border_radius_px}px;
    padding: 10px 4px;
    font-weight: 700;
    font-size: 11px;
    margin: 3px 0;
    transition: background 120ms ease, color 120ms ease;
}}

.nav-btn:hover {{
    background-color: {bg_tertiary};
    color: {text_primary};
}}

.nav-btn.active {{
    background-color: {bg_tertiary};
    color: {accent_color};
    border-left: 3px solid {accent_color};
}}

/* Header Bar */
.sidebar-header {{
    background-color: {bg_primary};
    border-bottom: 1px solid {border_color};
    padding: 8px 12px;
}}

.header-title {{
    font-weight: 800;
    font-size: 12px;
    letter-spacing: 0.5px;
    color: {text_secondary};
}}

.badge-state {{
    font-size: 10px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 4px;
    background-color: {bg_tertiary};
    color: {text_secondary};
}}

.badge-state.engaged {{
    background-color: {bg_tertiary};
    color: {accent_color};
    border: 1px solid {accent_color};
}}

.badge-state.pinned {{
    background-color: {accent_color};
    color: {accent_fg};
}}

.btn-header-pin {{
    background: transparent;
    color: {text_secondary};
    border: 1px solid {border_color};
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 11px;
    font-weight: 600;
}}

.btn-header-pin.pinned {{
    background-color: {accent_color};
    color: {accent_fg};
    border-color: {accent_color};
}}

.btn-header-close {{
    background: transparent;
    color: {text_secondary};
    border: none;
    padding: 4px 8px;
    font-size: 14px;
}}

/* Standardized Plugin CSS Classes */
.nm-card {{
    background-color: {bg_secondary};
    border: 1px solid {border_color};
    border-radius: {border_radius_px}px;
    padding: 10px;
}}

.nm-btn {{
    background-color: {bg_tertiary};
    color: {text_primary};
    border: 1px solid {border_color};
    border-radius: 6px;
    padding: 6px 12px;
    font-size: 11px;
    font-weight: 600;
}}

.nm-btn:hover {{
    background-color: {border_color};
}}

.nm-btn-primary {{
    background-color: {accent_color};
    color: {accent_fg};
    border: 1px solid {accent_color};
    border-radius: 6px;
    padding: 6px 12px;
    font-size: 11px;
    font-weight: 600;
}}

.nm-btn-danger {{
    background-color: {danger_color};
    color: #ffffff;
    border: 1px solid {danger_color};
    border-radius: 6px;
    padding: 6px 12px;
    font-size: 11px;
    font-weight: 600;
}}

.nm-heading {{
    color: {text_primary};
    font-weight: 700;
    font-size: 13px;
}}

.nm-muted {{
    color: {text_secondary};
    font-size: 11px;
}}

.nm-success {{
    color: {success_color};
}}

.nm-warning {{
    color: {warning_color};
}}

/* Terminal Tab */
.term-page-bar {{
    background-color: {bg_primary};
    border-bottom: 1px solid {border_color};
    padding: 4px 8px;
}}

.term-page-btn {{
    background-color: {bg_secondary};
    color: {text_secondary};
    border: 1px solid {border_color};
    border-radius: 6px;
    padding: 3px 6px;
    font-size: 10px;
    font-weight: 600;
}}

.term-page-btn:hover {{
    background-color: {bg_tertiary};
    color: {text_primary};
}}

.term-page-btn.active {{
    background-color: {accent_color};
    color: {accent_fg};
    border-color: {accent_color};
}}

.btn-term-ctrl {{
    background-color: {bg_secondary};
    color: {text_secondary};
    border: 1px solid {border_color};
    border-radius: 6px;
    padding: 3px 6px;
    font-size: 10px;
    font-weight: 600;
}}

.term-viewport {{
    background-color: {term_bg};
}}

.bg-sessions-section {{
    background-color: {bg_primary};
    border-top: 1px solid {border_color};
    padding: 6px 8px;
}}

.bg-sessions-header {{
    font-weight: 700;
    font-size: 10px;
    color: {text_secondary};
    letter-spacing: 0.5px;
}}

.bg-session-row {{
    background-color: {bg_secondary};
    border: 1px solid {border_color};
    border-radius: 4px;
    padding: 4px 8px;
    margin: 2px 0;
}}

.bg-session-label {{
    font-family: "{mono_font}", monospace;
    font-size: 10px;
    color: {text_primary};
}}

.btn-bg-attach {{
    background-color: {bg_tertiary};
    color: {accent_color};
    border: 1px solid {border_color};
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 10px;
    font-weight: 600;
}}

.btn-bg-kill {{
    background-color: {bg_tertiary};
    color: {danger_color};
    border: 1px solid {border_color};
    border-radius: 4px;
    padding: 2px 6px;
    font-size: 10px;
}}

/* Widgets Tab */
.widget-card {{
    background-color: {bg_secondary};
    border: 1px solid {border_color};
    border-radius: {border_radius_px}px;
    padding: 10px 12px;
}}

.widget-header {{
    font-weight: 700;
    font-size: 11px;
    color: {text_secondary};
    letter-spacing: 0.5px;
}}

.clock-time {{
    font-weight: 800;
    font-size: 24px;
    color: {text_primary};
    font-family: "{mono_font}", monospace;
}}

.clock-date {{
    font-size: 12px;
    color: {text_secondary};
}}

.media-title {{
    font-weight: 700;
    font-size: 12px;
    color: {text_primary};
}}

.media-artist {{
    font-size: 11px;
    color: {text_secondary};
}}

.btn-media {{
    background-color: {bg_secondary};
    color: {text_primary};
    border: 1px solid {border_color};
    border-radius: 6px;
    padding: 4px 10px;
}}

.btn-mute {{
    background: transparent;
    color: {text_secondary};
    border: none;
    font-size: 14px;
}}

/* Notifications Tab */
.notif-header-title {{
    font-weight: 700;
    font-size: 12px;
    color: {text_secondary};
}}

.notif-count-badge {{
    background-color: {bg_tertiary};
    color: {text_primary};
    border-radius: 10px;
    padding: 1px 6px;
    font-size: 10px;
    font-weight: 700;
}}

.btn-dnd {{
    background-color: {bg_secondary};
    color: {text_secondary};
    border: 1px solid {border_color};
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 10px;
    font-weight: 600;
}}

.btn-dnd.dnd-active {{
    background-color: {accent_color};
    color: {accent_fg};
    border-color: {accent_color};
}}

.btn-notif-clear {{
    background-color: {bg_secondary};
    color: {text_secondary};
    border: 1px solid {border_color};
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 10px;
    font-weight: 600;
}}

.notif-scroll {{
    background-color: {bg_primary};
}}

.notif-card {{
    background-color: {bg_secondary};
    border: 1px solid {border_color};
    border-radius: {border_radius_px}px;
    padding: 8px 10px;
}}

.notif-app-name {{
    font-weight: 700;
    font-size: 11px;
    color: {accent_color};
}}

.notif-time {{
    font-size: 10px;
    color: {text_secondary};
}}

.notif-summary {{
    font-weight: 600;
    font-size: 12px;
    color: {text_primary};
}}

.notif-body {{
    font-size: 11px;
    color: {text_secondary};
}}

.notif-empty {{
    font-size: 12px;
    color: {text_secondary};
    font-style: italic;
}}

/* Notepad Tab */
.notepad-header-title {{
    font-weight: 700;
    font-size: 12px;
    color: {text_secondary};
}}

.btn-notepad-clear {{
    background-color: {bg_secondary};
    color: {text_secondary};
    border: 1px solid {border_color};
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 10px;
    font-weight: 600;
}}

.notepad-view {{
    background-color: {bg_secondary};
    color: {text_primary};
    border: 1px solid {border_color};
    border-radius: {border_radius_px}px;
    font-family: "{mono_font}", monospace;
    font-size: {mono_font_size}pt;
    padding: 8px;
}}

.notepad-view text {{
    background-color: {bg_secondary};
    color: {text_primary};
}}

.notepad-status {{
    font-size: 10px;
    color: {text_secondary};
}}
"#,
            name = clean_name,
            bg_primary = self.bg_primary,
            bg_secondary = self.bg_secondary,
            bg_tertiary = self.bg_tertiary,
            border_color = self.border_color,
            text_primary = self.text_primary,
            text_secondary = self.text_secondary,
            accent_color = self.accent_color,
            accent_fg = accent_fg,
            danger_color = self.danger_color,
            success_color = self.success_color,
            warning_color = self.warning_color,
            ui_font = self.ui_font,
            border_radius_px = self.border_radius_px,
            term_bg = self.term_bg,
            mono_font = self.mono_font,
            mono_font_size = self.mono_font_size,
        )
    }
}

pub fn config_dir() -> PathBuf {
    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("nevermore")
    } else if let Some(home) = env::var_os("HOME") {
        PathBuf::from(home).join(".config").join("nevermore")
    } else {
        PathBuf::from("/tmp/nevermore-config")
    }
}

pub fn themes_dir() -> PathBuf {
    config_dir().join("themes")
}

pub fn custom_css_path() -> PathBuf {
    config_dir().join("custom.css")
}

pub fn config_file_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Auto-installs / refreshes built-in starter themes into $XDG_CONFIG_HOME/nevermore/themes/
pub fn install_bundled_themes() -> Result<(), String> {
    let t_dir = themes_dir();
    fs::create_dir_all(&t_dir).map_err(|e| format!("Failed to create themes dir: {}", e))?;

    let bundled = [
        ("nevermore-dark.toml", BUNDLED_NEVERMORE_DARK),
        ("catppuccin-mocha.toml", BUNDLED_CATPPUCCIN_MOCHA),
        ("tokyo-night.toml", BUNDLED_TOKYO_NIGHT),
    ];

    for (fname, content) in bundled {
        let p = t_dir.join(fname);
        fs::write(&p, content).map_err(|e| format!("Failed to write {}: {}", fname, e))?;
    }
    Ok(())
}

/// Lists all available theme names (without .toml extension).
pub fn list_available_themes() -> Vec<String> {
    let mut names = vec![
        "nevermore-dark".to_string(),
        "catppuccin-mocha".to_string(),
        "tokyo-night".to_string(),
    ];

    let t_dir = themes_dir();
    if let Ok(entries) = fs::read_dir(t_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let s = stem.to_string();
                    if !names.contains(&s) {
                        names.push(s);
                    }
                }
            }
        }
    }

    names.sort();
    names
}

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    #[serde(default)]
    theme: ThemeConfig,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ThemeConfig {
    name: Option<String>,
}

/// Reads the active theme name from config.toml (default "nevermore-dark").
pub fn get_active_theme_name() -> String {
    let cfg = config_file_path();
    if cfg.exists() {
        if let Ok(content) = fs::read_to_string(&cfg) {
            if let Ok(parsed) = toml::from_str::<AppConfig>(&content) {
                if let Some(name) = parsed.theme.name {
                    if !name.trim().is_empty() {
                        return name;
                    }
                }
            }
        }
    }
    "nevermore-dark".to_string()
}

/// Writes the active theme name atomically to config.toml while preserving existing config keys.
pub fn set_active_theme_name(theme_name: &str) -> Result<(), String> {
    validate_theme_name(theme_name)?;

    let c_dir = config_dir();
    fs::create_dir_all(&c_dir).map_err(|e| format!("Failed to create config dir: {}", e))?;

    let cfg_path = config_file_path();
    let mut table = if cfg_path.exists() {
        let s = fs::read_to_string(&cfg_path).unwrap_or_default();
        toml::from_str::<toml::Table>(&s).unwrap_or_default()
    } else {
        toml::Table::new()
    };

    let mut theme_table = table
        .get("theme")
        .and_then(|v| v.as_table().cloned())
        .unwrap_or_default();
    theme_table.insert(
        "name".to_string(),
        toml::Value::String(theme_name.to_string()),
    );
    table.insert("theme".to_string(), toml::Value::Table(theme_table));

    let content =
        toml::to_string_pretty(&table).map_err(|e| format!("Failed to format config: {}", e))?;
    let pid = std::process::id();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_path = c_dir.join(format!("config.{}.{}.tmp", pid, timestamp));

    fs::write(&tmp_path, content).map_err(|e| format!("Failed to write temp config: {}", e))?;
    fs::rename(&tmp_path, &cfg_path)
        .map_err(|e| format!("Failed to atomic rename config: {}", e))?;
    Ok(())
}

/// Loads a theme by name: checks filesystem in themes_dir, falls back to bundled constants.
pub fn load_theme(name: &str) -> Result<ThemePalette, String> {
    validate_theme_name(name)?;

    let t_path = themes_dir().join(format!("{}.toml", name));
    if t_path.exists() {
        let content = fs::read_to_string(&t_path)
            .map_err(|e| format!("Failed to read theme file '{}': {}", t_path.display(), e))?;
        return Ok(ThemePalette::from_toml_str_lossy(&content));
    }

    match name {
        "nevermore-dark" => Ok(ThemePalette::from_toml_str_lossy(BUNDLED_NEVERMORE_DARK)),
        "catppuccin-mocha" => Ok(ThemePalette::from_toml_str_lossy(BUNDLED_CATPPUCCIN_MOCHA)),
        "tokyo-night" => Ok(ThemePalette::from_toml_str_lossy(BUNDLED_TOKYO_NIGHT)),
        _ => Err(format!(
            "Theme '{}' not found in themes directory or bundled starter themes",
            name
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme_palette() {
        let palette = ThemePalette::default();
        assert_eq!(palette.name, "nevermore-dark");
        assert_eq!(palette.bg_primary, "#18181b");
        assert_eq!(palette.accent_color, "#0a84ff");
        assert_eq!(palette.term_palette.len(), 16);
    }

    #[test]
    fn test_theme_palette_from_toml() {
        let mocha = ThemePalette::from_toml(BUNDLED_CATPPUCCIN_MOCHA).expect("valid mocha toml");
        assert_eq!(mocha.name, "catppuccin-mocha");
        assert_eq!(mocha.bg_primary, "#1e1e2e");
        assert_eq!(mocha.accent_color, "#89b4fa");
    }

    #[test]
    fn test_to_gtk_css_contains_colors_and_contrast() {
        let mocha = ThemePalette::from_toml(BUNDLED_CATPPUCCIN_MOCHA).unwrap();
        let css = mocha.to_gtk_css();
        assert!(css.contains("#1e1e2e"));
        assert!(css.contains("#89b4fa"));
        assert!(css.contains(".nm-card"));
        assert!(css.contains(".nm-btn-primary"));
        assert!(css.contains(".nm-success"));
        assert!(css.contains(".nm-warning"));
        // Mocha accent #89b4fa is light -> contrast text should be #11111b
        assert!(css.contains("#11111b"));
    }

    #[test]
    fn test_list_themes() {
        let list = list_available_themes();
        assert!(list.contains(&"nevermore-dark".to_string()));
        assert!(list.contains(&"catppuccin-mocha".to_string()));
        assert!(list.contains(&"tokyo-night".to_string()));
    }

    #[test]
    fn test_validate_theme_names() {
        assert!(validate_theme_name("valid-theme_123").is_ok());
        assert!(validate_theme_name("").is_err());
        assert!(validate_theme_name("../bad").is_err());
        assert!(validate_theme_name("path/traversal").is_err());
        assert!(validate_theme_name("back\\slash").is_err());
        assert!(validate_theme_name("bad space").is_err());
    }

    #[test]
    fn test_theme_lossy_fallback_malformed_and_types() {
        let malformed = "this is not valid toml = = = ";
        let pal = ThemePalette::from_toml_str_lossy(malformed);
        assert_eq!(pal.name, "nevermore-dark");
        assert_eq!(pal.bg_primary, "#18181b");

        let bad_types = r#"
            name = "custom"
            bg_primary = 12345
            accent_color = true
            mono_font_size = "large"
            term_palette = "not an array"
        "#;
        let pal2 = ThemePalette::from_toml_str_lossy(bad_types);
        assert_eq!(pal2.name, "custom");
        assert_eq!(pal2.bg_primary, "#18181b");
        assert_eq!(pal2.accent_color, "#0a84ff");
        assert_eq!(pal2.mono_font_size, 10);
        assert_eq!(pal2.term_palette.len(), 16);
    }

    #[test]
    fn test_term_palette_15_items_and_invalid_hex() {
        let partial_palette = r##"
            name = "partial"
            term_palette = [
                "#000000", "#111111", "#222222", "#333333",
                "#444444", "#555555", "#666666", "#777777",
                "#888888", "#999999", "#aaaaaa", "#bbbbbb",
                "#cccccc", "#dddddd", "#eeeeee"
            ]
        "##;
        let pal = ThemePalette::from_toml_str_lossy(partial_palette);
        assert_eq!(pal.term_palette.len(), 16);
        assert_eq!(pal.term_palette[0], "#000000");
        assert_eq!(pal.term_palette[14], "#eeeeee");
        // 16th item should be filled with default
        assert_eq!(pal.term_palette[15], "#ffffff");

        let invalid_hex = r##"
            name = "invalid_hex"
            term_palette = [
                "rgba(0,0,0,1)", "transparent", "#123", "#12345678",
                "#invalid", "not-a-color", "#000000", "#111111",
                "#222222", "#333333", "#444444", "#555555",
                "#666666", "#777777", "#888888", "#999999"
            ]
        "##;
        let pal_inv = ThemePalette::from_toml_str_lossy(invalid_hex);
        assert_eq!(pal_inv.term_palette.len(), 16);
        // Invalid entries must fall back to valid solid hex default entries
        for color in &pal_inv.term_palette {
            assert!(
                is_valid_solid_hex_color(color),
                "Color '{}' must be valid 6-digit solid hex",
                color
            );
        }
    }

    #[test]
    fn test_rejection_of_rgba_transparent_and_injection() {
        let injection_toml = r#"
            name = "test*/ injection"
            ui_font = "Sans\"; color: red; /*"
            bg_primary = "rgba(255, 0, 0, 0.5)"
            accent_color = "transparent"
        "#;
        let pal = ThemePalette::from_toml_str_lossy(injection_toml);
        assert!(!pal.name.contains("*/"));
        assert!(!pal.ui_font.contains(';'));
        assert!(!pal.ui_font.contains('"'));
        assert_eq!(pal.bg_primary, "#18181b");
        assert_eq!(pal.accent_color, "#0a84ff");
    }
}
