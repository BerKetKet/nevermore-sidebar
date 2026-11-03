use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

pub const BUNDLED_NEVERMORE_DARK: &str = include_str!("../resources/themes/nevermore-dark.toml");
pub const BUNDLED_CATPPUCCIN_MOCHA: &str =
    include_str!("../resources/themes/catppuccin-mocha.toml");
pub const BUNDLED_TOKYO_NIGHT: &str = include_str!("../resources/themes/tokyo-night.toml");

fn default_name() -> String {
    "nevermore-dark".to_string()
}
fn default_bg_primary() -> String {
    "#18181b".to_string()
}
fn default_bg_secondary() -> String {
    "#202024".to_string()
}
fn default_bg_tertiary() -> String {
    "#27272a".to_string()
}
fn default_border_color() -> String {
    "#2e2e32".to_string()
}
fn default_text_primary() -> String {
    "#f4f4f5".to_string()
}
fn default_text_secondary() -> String {
    "#a1a1aa".to_string()
}
fn default_accent_color() -> String {
    "#0a84ff".to_string()
}
fn default_danger_color() -> String {
    "#ef4444".to_string()
}
fn default_success_color() -> String {
    "#22c55e".to_string()
}
fn default_warning_color() -> String {
    "#f59e0b".to_string()
}
fn default_ui_font() -> String {
    "Inter".to_string()
}
fn default_mono_font() -> String {
    "Hack Nerd Font Mono".to_string()
}
fn default_mono_font_size() -> u32 {
    10
}
fn default_border_radius_px() -> u32 {
    8
}
fn default_term_bg() -> String {
    "#18181b".to_string()
}
fn default_term_fg() -> String {
    "#f4f4f5".to_string()
}
fn default_term_cursor() -> String {
    "#0a84ff".to_string()
}
fn default_term_palette() -> [String; 16] {
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
        toml::from_str(content).map_err(|e| format!("Failed to parse theme toml: {}", e))
    }

    /// Generates the complete GTK4 CSS string for Nevermore Sidebar with solid colors.
    pub fn to_gtk_css(&self) -> String {
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
    color: #ffffff;
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
    background-color: #1e3a5f;
    color: #60a5fa;
}}

.badge-state.pinned {{
    background-color: {accent_color};
    color: #ffffff;
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
    color: #ffffff;
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
    color: #ffffff;
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
    color: #ffffff;
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

.term-scroll {{
    background-color: {term_bg};
}}

/* Widgets Tab */
.widget-card {{
    background-color: {bg_secondary};
    border: 1px solid {border_color};
    border-radius: {border_radius_px}px;
    padding: 10px 12px;
}}

.widget-title {{
    font-weight: 700;
    font-size: 11px;
    color: {text_secondary};
    letter-spacing: 0.5px;
}}

.stat-value {{
    font-weight: 800;
    font-size: 20px;
    color: {text_primary};
}}

.stat-sub {{
    font-size: 11px;
    color: {text_secondary};
}}

.stat-bar-container {{
    background-color: {bg_tertiary};
    border-radius: 4px;
}}

.stat-bar-fill {{
    background-color: {accent_color};
    border-radius: 4px;
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
    color: #ffffff;
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
            name = self.name,
            bg_primary = self.bg_primary,
            bg_secondary = self.bg_secondary,
            bg_tertiary = self.bg_tertiary,
            border_color = self.border_color,
            text_primary = self.text_primary,
            text_secondary = self.text_secondary,
            accent_color = self.accent_color,
            danger_color = self.danger_color,
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

/// Auto-installs built-in starter themes into $XDG_CONFIG_HOME/nevermore/themes/ if missing.
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
        if !p.exists() {
            fs::write(&p, content).map_err(|e| format!("Failed to write {}: {}", fname, e))?;
        }
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

/// Writes the active theme name atomically to config.toml.
pub fn set_active_theme_name(theme_name: &str) -> Result<(), String> {
    let c_dir = config_dir();
    fs::create_dir_all(&c_dir).map_err(|e| format!("Failed to create config dir: {}", e))?;

    let cfg_path = config_file_path();
    let content = format!("[theme]\nname = \"{}\"\n", theme_name);

    let tmp_path = cfg_path.with_extension("tmp");
    fs::write(&tmp_path, content).map_err(|e| format!("Failed to write temp config: {}", e))?;
    fs::rename(&tmp_path, &cfg_path)
        .map_err(|e| format!("Failed to atomic rename config: {}", e))?;
    Ok(())
}

/// Loads a theme by name: checks filesystem in themes_dir, falls back to bundled constants.
pub fn load_theme(name: &str) -> Result<ThemePalette, String> {
    let t_path = themes_dir().join(format!("{}.toml", name));
    if t_path.exists() {
        let content = fs::read_to_string(&t_path)
            .map_err(|e| format!("Failed to read theme file '{}': {}", t_path.display(), e))?;
        return ThemePalette::from_toml(&content);
    }

    match name {
        "nevermore-dark" => ThemePalette::from_toml(BUNDLED_NEVERMORE_DARK),
        "catppuccin-mocha" => ThemePalette::from_toml(BUNDLED_CATPPUCCIN_MOCHA),
        "tokyo-night" => ThemePalette::from_toml(BUNDLED_TOKYO_NIGHT),
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
    fn test_to_gtk_css_contains_colors() {
        let mocha = ThemePalette::from_toml(BUNDLED_CATPPUCCIN_MOCHA).unwrap();
        let css = mocha.to_gtk_css();
        assert!(css.contains("#1e1e2e"));
        assert!(css.contains("#89b4fa"));
        assert!(css.contains(".nm-card"));
        assert!(css.contains(".nm-btn-primary"));
    }

    #[test]
    fn test_list_themes() {
        let list = list_available_themes();
        assert!(list.contains(&"nevermore-dark".to_string()));
        assert!(list.contains(&"catppuccin-mocha".to_string()));
        assert!(list.contains(&"tokyo-night".to_string()));
    }
}
