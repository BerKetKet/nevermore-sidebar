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

#[derive(Debug, Clone, Copy)]
pub struct Oklab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

pub fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

pub fn rgb_to_oklab(r: u8, g: u8, b: u8) -> Oklab {
    let lr = srgb_to_linear(r as f64 / 255.0);
    let lg = srgb_to_linear(g as f64 / 255.0);
    let lb = srgb_to_linear(b as f64 / 255.0);

    let l = 0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb;
    let m = 0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb;
    let s = 0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    Oklab {
        l: 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        a: 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        b: 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    }
}

pub fn oklab_to_rgb(lab: Oklab) -> (u8, u8, u8) {
    let l_ = lab.l + 0.3963377774 * lab.a + 0.2158037573 * lab.b;
    let m_ = lab.l - 0.1055613458 * lab.a - 0.0638541728 * lab.b;
    let s_ = lab.l - 0.0894841775 * lab.a - 1.2914855480 * lab.b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let lr = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let lg = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let lb = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    let r = (linear_to_srgb(lr).clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (linear_to_srgb(lg).clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (linear_to_srgb(lb).clamp(0.0, 1.0) * 255.0).round() as u8;

    (r, g, b)
}

pub fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let clean = hex.trim().strip_prefix('#')?;
    if clean.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
    let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
    let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
    Some((r, g, b))
}

pub fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

pub fn mix_hex_oklab(hex1: &str, hex2: &str, t: f64) -> String {
    let (r1, g1, b1) = hex_to_rgb(hex1).unwrap_or((0, 0, 0));
    let (r2, g2, b2) = hex_to_rgb(hex2).unwrap_or((255, 255, 255));
    let o1 = rgb_to_oklab(r1, g1, b1);
    let o2 = rgb_to_oklab(r2, g2, b2);

    let o_mixed = Oklab {
        l: o1.l + (o2.l - o1.l) * t,
        a: o1.a + (o2.a - o1.a) * t,
        b: o1.b + (o2.b - o1.b) * t,
    };
    let (r, g, b) = oklab_to_rgb(o_mixed);
    rgb_to_hex(r, g, b)
}

pub fn adjust_lightness_oklab(hex: &str, delta_l: f64) -> String {
    let (r, g, b) = hex_to_rgb(hex).unwrap_or((0, 0, 0));
    let mut o = rgb_to_oklab(r, g, b);
    o.l = (o.l + delta_l).clamp(0.0, 1.0);
    let (ro, go, bo) = oklab_to_rgb(o);
    rgb_to_hex(ro, go, bo)
}

pub fn relative_luminance(hex: &str) -> f64 {
    if let Some((r, g, b)) = hex_to_rgb(hex) {
        let lr = srgb_to_linear(r as f64 / 255.0);
        let lg = srgb_to_linear(g as f64 / 255.0);
        let lb = srgb_to_linear(b as f64 / 255.0);
        0.2126 * lr + 0.7152 * lg + 0.0722 * lb
    } else {
        0.0
    }
}

pub fn contrast_ratio(hex1: &str, hex2: &str) -> f64 {
    let l1 = relative_luminance(hex1);
    let l2 = relative_luminance(hex2);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

#[allow(clippy::needless_range_loop)]
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0; b_chars.len() + 1]; a_chars.len() + 1];

    for i in 0..=a_chars.len() {
        dp[i][0] = i;
    }
    for j in 0..=b_chars.len() {
        dp[0][j] = j;
    }

    for i in 1..=a_chars.len() {
        for j in 1..=b_chars.len() {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a_chars.len()][b_chars.len()]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedConfig {
    pub bg: String,
    pub fg: String,
    pub accent: String,
    #[serde(default)]
    pub danger: Option<String>,
    #[serde(default)]
    pub success: Option<String>,
    #[serde(default)]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeCheckResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub contrast_ratios: ThemeContrastRatios,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeContrastRatios {
    pub text_primary_on_bg: f64,
    pub text_secondary_on_bg_secondary: f64,
    pub accent_on_bg_tertiary: f64,
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

    #[serde(default)]
    pub seed: Option<SeedConfig>,

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
            seed: None,
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
    pub fn from_seed(seed: &SeedConfig) -> Self {
        let bg = seed.bg.trim().to_ascii_lowercase();
        let fg = seed.fg.trim().to_ascii_lowercase();
        let accent = seed.accent.trim().to_ascii_lowercase();
        let danger = seed
            .danger
            .as_deref()
            .unwrap_or("#ef4444")
            .trim()
            .to_ascii_lowercase();
        let success = seed
            .success
            .as_deref()
            .unwrap_or("#22c55e")
            .trim()
            .to_ascii_lowercase();
        let warning = seed
            .warning
            .as_deref()
            .unwrap_or("#f59e0b")
            .trim()
            .to_ascii_lowercase();

        let bg_primary = bg.clone();
        let bg_secondary = mix_hex_oklab(&bg, &fg, 0.05);
        let bg_tertiary = mix_hex_oklab(&bg, &fg, 0.10);
        let border_color = mix_hex_oklab(&bg, &fg, 0.16);
        let text_primary = fg.clone();
        let text_secondary = mix_hex_oklab(&fg, &bg, 0.35);

        let term_bg = bg.clone();
        let term_fg = fg.clone();
        let term_cursor = accent.clone();

        let mut term_palette = default_term_palette();
        term_palette[0] = bg;
        term_palette[1] = danger.clone();
        term_palette[2] = success.clone();
        term_palette[3] = warning.clone();
        term_palette[4] = accent.clone();
        term_palette[5] = mix_hex_oklab(&accent, &danger, 0.45);
        term_palette[6] = mix_hex_oklab(&accent, &success, 0.45);
        term_palette[7] = mix_hex_oklab(&fg, &bg_primary, 0.15);
        term_palette[8] = border_color.clone();
        term_palette[9] = adjust_lightness_oklab(&danger, 0.10);
        term_palette[10] = adjust_lightness_oklab(&success, 0.10);
        term_palette[11] = adjust_lightness_oklab(&warning, 0.10);
        term_palette[12] = adjust_lightness_oklab(&accent, 0.10);
        term_palette[13] = adjust_lightness_oklab(&term_palette[5], 0.10);
        term_palette[14] = adjust_lightness_oklab(&term_palette[6], 0.10);
        term_palette[15] = fg;

        Self {
            name: "seed-derived".to_string(),
            seed: Some(seed.clone()),
            bg_primary,
            bg_secondary,
            bg_tertiary,
            border_color,
            text_primary,
            text_secondary,
            accent_color: accent,
            danger_color: danger,
            success_color: success,
            warning_color: warning,
            ui_font: default_ui_font(),
            mono_font: default_mono_font(),
            mono_font_size: default_mono_font_size(),
            border_radius_px: default_border_radius_px(),
            term_bg,
            term_fg,
            term_cursor,
            term_palette,
        }
    }

    pub fn from_toml(content: &str) -> Result<Self, String> {
        Ok(Self::from_toml_str_lossy(content))
    }

    /// Hardened lossy parser: validates every field per-field and falls back safely to Nevermore Dark defaults or [seed] derivations.
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

        // 0. Seed configuration
        if let Some(seed_val) = table.get("seed") {
            if let Some(seed_tab) = seed_val.as_table() {
                let bg = seed_tab
                    .get("bg")
                    .and_then(|v| v.as_str())
                    .unwrap_or("#18181b");
                let fg = seed_tab
                    .get("fg")
                    .and_then(|v| v.as_str())
                    .unwrap_or("#f4f4f5");
                let accent = seed_tab
                    .get("accent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("#0a84ff");
                let danger = seed_tab.get("danger").and_then(|v| v.as_str());
                let success = seed_tab.get("success").and_then(|v| v.as_str());
                let warning = seed_tab.get("warning").and_then(|v| v.as_str());

                let seed = SeedConfig {
                    bg: bg.to_string(),
                    fg: fg.to_string(),
                    accent: accent.to_string(),
                    danger: danger.map(|s| s.to_string()),
                    success: success.map(|s| s.to_string()),
                    warning: warning.map(|s| s.to_string()),
                };
                palette = Self::from_seed(&seed);
            }
        }

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
        let parse_color = |key: &str, current_val: &str| -> String {
            if let Some(val) = table.get(key) {
                if let Some(s) = val.as_str() {
                    let trimmed = s.trim();
                    if is_valid_solid_hex_color(trimmed) {
                        return trimmed.to_ascii_lowercase();
                    } else {
                        eprintln!(
                            "[nevermore-theme] warning: invalid solid hex color '{}' for '{}', falling back to '{}'",
                            trimmed, key, current_val
                        );
                    }
                } else {
                    eprintln!(
                        "[nevermore-theme] warning: '{}' is not a string, falling back to '{}'",
                        key, current_val
                    );
                }
            }
            current_val.to_string()
        };

        palette.bg_primary = parse_color("bg_primary", &palette.bg_primary);
        palette.bg_secondary = parse_color("bg_secondary", &palette.bg_secondary);
        palette.bg_tertiary = parse_color("bg_tertiary", &palette.bg_tertiary);
        palette.border_color = parse_color("border_color", &palette.border_color);
        palette.text_primary = parse_color("text_primary", &palette.text_primary);
        palette.text_secondary = parse_color("text_secondary", &palette.text_secondary);
        palette.accent_color = parse_color("accent_color", &palette.accent_color);
        palette.danger_color = parse_color("danger_color", &palette.danger_color);
        palette.success_color = parse_color("success_color", &palette.success_color);
        palette.warning_color = parse_color("warning_color", &palette.warning_color);

        palette.term_bg = parse_color("term_bg", &palette.term_bg);
        palette.term_fg = parse_color("term_fg", &palette.term_fg);
        palette.term_cursor = parse_color("term_cursor", &palette.term_cursor);

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
        let default_palette = palette.term_palette.clone();
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

:root {{
    --nm-bg-primary: {bg_primary};
    --nm-bg-secondary: {bg_secondary};
    --nm-bg-tertiary: {bg_tertiary};
    --nm-border-color: {border_color};
    --nm-text-primary: {text_primary};
    --nm-text-secondary: {text_secondary};
    --nm-accent-color: {accent_color};
    --nm-accent-fg: {accent_fg};
    --nm-danger-color: {danger_color};
    --nm-success-color: {success_color};
    --nm-warning-color: {warning_color};
    --nm-font-sans: "{ui_font}", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    --nm-font-mono: "{mono_font}", monospace;
    --nm-font-size: {mono_font_size}pt;
    --nm-border-radius: {border_radius_px}px;
}}

/* Tone Utility Classes for Dynamic CSS Recoloring */
.nm-tone-primary {{
    color: {text_primary};
}}
.nm-tone-secondary {{
    color: {text_secondary};
}}
.nm-tone-muted {{
    color: {text_secondary};
}}
.nm-tone-accent {{
    color: {accent_color};
}}
.nm-tone-danger {{
    color: {danger_color};
}}
.nm-tone-success {{
    color: {success_color};
}}
.nm-tone-warning {{
    color: {warning_color};
}}

/* Dynamic Button State Interactions */
button:hover .nm-tone-secondary,
button:hover .nm-tone-muted {{
    color: {text_primary};
}}

button.active .nm-tone-secondary,
button.active .nm-tone-muted,
button:checked .nm-tone-secondary,
button:checked .nm-tone-muted {{
    color: {accent_color};
}}

button:disabled .nm-tone-primary,
button:disabled .nm-tone-secondary,
button:disabled .nm-tone-accent {{
    color: {border_color};
}}

window {{
    background-color: {bg_primary};
    color: {text_primary};
    font-family: "{ui_font}", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}}

.sidebar-panel {{
    background-color: {bg_primary};
    border-left: 1px solid {border_color};
}}

.sidebar-panel.state-engaged {{
    border-left: 2px solid {accent_color};
}}

.sidebar-panel.state-pinned {{
    border-left: 2px solid {accent_color};
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

.nav-btn.active .nm-icon {{
    color: {accent_color};
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

.btn-header-pin.pinned .nm-icon {{
    color: {accent_fg};
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

.nm-section-header {{
    font-size: 11px;
    font-weight: 700;
    color: {text_secondary};
    letter-spacing: 0.5px;
    margin-top: 4px;
    margin-bottom: 2px;
}}

.nm-field-label {{
    font-size: 10px;
    font-weight: 600;
    color: {text_secondary};
    margin-bottom: 2px;
}}

.nm-data {{
    font-size: 11px;
    color: {text_primary};
}}

.nm-data.nm-prose {{
    font-size: 11px;
    color: {text_primary};
    line-height: 1.4;
}}

.nm-prompt-entry {{
    background-color: {bg_tertiary};
    border: 1px solid {border_color};
    border-radius: {border_radius_px}px;
    padding: 4px 8px;
}}

.nm-prompt-entry entry {{
    background: transparent;
    border: none;
    box-shadow: none;
    color: {text_primary};
    font-size: 11px;
}}

.nm-search-entry {{
    background-color: {bg_tertiary};
    border: 1px solid {border_color};
    border-radius: {border_radius_px}px;
    padding: 4px 8px;
}}

.nm-search-entry entry {{
    background: transparent;
    border: none;
    box-shadow: none;
    color: {text_primary};
    font-size: 11px;
}}

.icon-btn {{
    background: transparent;
    border: none;
    border-radius: 4px;
    padding: 4px;
}}

.icon-btn:hover {{
    background-color: {bg_tertiary};
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

pub fn check_theme_file(path: &std::path::Path) -> Result<ThemeCheckResult, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let toml_val: toml::Value = toml::from_str(&content)
        .map_err(|e| format!("Invalid TOML syntax in {}: {}", path.display(), e))?;
    let table = toml_val
        .as_table()
        .ok_or_else(|| "Root of TOML is not a table".to_string())?;

    let mut errors = Vec::new();
    let warnings = Vec::new();

    let known_keys = [
        "name",
        "seed",
        "bg_primary",
        "bg_secondary",
        "bg_tertiary",
        "border_color",
        "text_primary",
        "text_secondary",
        "accent_color",
        "danger_color",
        "danger",
        "success_color",
        "success",
        "warning_color",
        "warning",
        "ui_font",
        "mono_font",
        "mono_font_size",
        "border_radius_px",
        "term_bg",
        "term_fg",
        "term_cursor",
        "term_palette",
    ];

    let seed_known_keys = ["bg", "fg", "accent", "danger", "success", "warning"];

    for key in table.keys() {
        if !known_keys.contains(&key.as_str()) {
            let mut best_match = "";
            let mut min_dist = usize::MAX;
            for candidate in known_keys {
                let dist = levenshtein_distance(key, candidate);
                if dist < min_dist {
                    min_dist = dist;
                    best_match = candidate;
                }
            }
            if min_dist <= 3 {
                errors.push(format!(
                    "Unknown key '{}', did you mean '{}'?",
                    key, best_match
                ));
            } else {
                errors.push(format!("Unknown key '{}'", key));
            }
        }
    }

    if let Some(seed_val) = table.get("seed") {
        if let Some(seed_tab) = seed_val.as_table() {
            for key in seed_tab.keys() {
                if !seed_known_keys.contains(&key.as_str()) {
                    let mut best_match = "";
                    let mut min_dist = usize::MAX;
                    for candidate in seed_known_keys {
                        let dist = levenshtein_distance(key, candidate);
                        if dist < min_dist {
                            min_dist = dist;
                            best_match = candidate;
                        }
                    }
                    if min_dist <= 3 {
                        errors.push(format!(
                            "Unknown key '[seed].{}', did you mean '[seed].{}'?",
                            key, best_match
                        ));
                    } else {
                        errors.push(format!("Unknown key '[seed].{}'", key));
                    }
                }
            }
            for required in ["bg", "fg", "accent"] {
                if !seed_tab.contains_key(required) {
                    errors.push(format!("Missing required key '[seed].{}'", required));
                }
            }
        } else {
            errors.push("'[seed]' must be a table".to_string());
        }
    }

    // Validate solid hex formats for top-level keys
    let color_keys = [
        "bg_primary",
        "bg_secondary",
        "bg_tertiary",
        "border_color",
        "text_primary",
        "text_secondary",
        "accent_color",
        "danger_color",
        "success_color",
        "warning_color",
        "term_bg",
        "term_fg",
        "term_cursor",
    ];
    for k in color_keys {
        if let Some(val) = table.get(k) {
            if let Some(s) = val.as_str() {
                if !is_valid_solid_hex_color(s.trim()) {
                    errors.push(format!("Invalid solid hex color '{}' for key '{}'", s, k));
                }
            } else {
                errors.push(format!("Key '{}' must be a solid hex string", k));
            }
        }
    }

    let palette = ThemePalette::from_toml_str_lossy(&content);

    // Dark-theme luminance ceiling
    let lum_bg = relative_luminance(&palette.bg_primary);
    if lum_bg >= 0.15 {
        errors.push(format!(
            "bg_primary relative luminance is {:.3} (exceeds dark-theme ceiling 0.150)",
            lum_bg
        ));
    }

    // Contrast checks
    let c_text = contrast_ratio(&palette.text_primary, &palette.bg_primary);
    if c_text < 4.5 {
        errors.push(format!(
            "text_primary on bg_primary contrast is {:.2}:1 (fails WCAG AA 4.5:1)",
            c_text
        ));
    }

    let c_sec = contrast_ratio(&palette.text_secondary, &palette.bg_secondary);
    if c_sec < 3.0 {
        errors.push(format!(
            "text_secondary on bg_secondary contrast is {:.2}:1 (fails WCAG UI 3.0:1)",
            c_sec
        ));
    }

    let c_acc = contrast_ratio(&palette.accent_color, &palette.bg_tertiary);
    if c_acc < 3.0 {
        errors.push(format!(
            "accent_color on bg_tertiary contrast is {:.2}:1 (fails WCAG UI 3.0:1)",
            c_acc
        ));
    }

    let contrast_ratios = ThemeContrastRatios {
        text_primary_on_bg: c_text,
        text_secondary_on_bg_secondary: c_sec,
        accent_on_bg_tertiary: c_acc,
    };

    Ok(ThemeCheckResult {
        valid: errors.is_empty(),
        errors,
        warnings,
        contrast_ratios,
    })
}

pub fn expand_theme_file(path: &std::path::Path) -> Result<String, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let palette = ThemePalette::from_toml_str_lossy(&content);
    toml::to_string_pretty(&palette)
        .map_err(|e| format!("Failed to serialize expanded theme: {}", e))
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
        match toml::from_str::<toml::Table>(&s) {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "[nevermore-theme] warning: existing config.toml failed to parse ({}), backing up to config.toml.bak",
                    e
                );
                let bak_path = cfg_path.with_extension("toml.bak");
                let _ = fs::copy(&cfg_path, &bak_path);
                toml::Table::new()
            }
        }
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

/// Reads the [appearance] table from config.toml. Returns (ui_font, mono_font, mono_font_size).
pub fn get_appearance_config() -> (Option<String>, Option<String>, Option<f64>) {
    let cfg = config_file_path();
    if cfg.exists() {
        if let Ok(content) = fs::read_to_string(&cfg) {
            if let Ok(table) = toml::from_str::<toml::Table>(&content) {
                if let Some(app) = table.get("appearance").and_then(|v| v.as_table()) {
                    let ui = app
                        .get("ui_font")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let mono = app
                        .get("mono_font")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let size = app
                        .get("mono_font_size")
                        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)));
                    return (ui, mono, size);
                }
            }
        }
    }
    (None, None, None)
}

/// Sets [appearance] values in config.toml atomically while preserving other sections.
pub fn set_appearance_config(
    ui_font: &str,
    mono_font: &str,
    mono_font_size: f64,
) -> Result<(), String> {
    let c_dir = config_dir();
    fs::create_dir_all(&c_dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    let cfg_path = config_file_path();
    let mut table = if cfg_path.exists() {
        let s = fs::read_to_string(&cfg_path).unwrap_or_default();
        toml::from_str::<toml::Table>(&s).unwrap_or_default()
    } else {
        toml::Table::new()
    };

    let mut app_table = table
        .get("appearance")
        .and_then(|v| v.as_table().cloned())
        .unwrap_or_default();
    app_table.insert(
        "ui_font".to_string(),
        toml::Value::String(ui_font.to_string()),
    );
    app_table.insert(
        "mono_font".to_string(),
        toml::Value::String(mono_font.to_string()),
    );
    app_table.insert(
        "mono_font_size".to_string(),
        toml::Value::Float(mono_font_size),
    );
    table.insert("appearance".to_string(), toml::Value::Table(app_table));

    let toml_str =
        toml::to_string_pretty(&table).map_err(|e| format!("Serialization error: {}", e))?;
    let pid = std::process::id();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp = c_dir.join(format!("config.{}.{}.tmp", pid, timestamp));
    fs::write(&tmp, toml_str).map_err(|e| format!("Failed to write tmp: {}", e))?;
    fs::rename(&tmp, &cfg_path).map_err(|e| format!("Failed to rename: {}", e))?;
    Ok(())
}

/// Reads [plugins] disabled array from config.toml.
pub fn get_disabled_plugins() -> Vec<String> {
    let cfg = config_file_path();
    if cfg.exists() {
        if let Ok(content) = fs::read_to_string(&cfg) {
            if let Ok(table) = toml::from_str::<toml::Table>(&content) {
                if let Some(arr) = table
                    .get("plugins")
                    .and_then(|p| p.get("disabled"))
                    .and_then(|d| d.as_array())
                {
                    return arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }
        }
    }
    Vec::new()
}

/// Sets whether a plugin is disabled in config.toml.
pub fn set_plugin_disabled(plugin_id: &str, disabled: bool) -> Result<(), String> {
    let c_dir = config_dir();
    fs::create_dir_all(&c_dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    let cfg_path = config_file_path();
    let mut table = if cfg_path.exists() {
        let s = fs::read_to_string(&cfg_path).unwrap_or_default();
        toml::from_str::<toml::Table>(&s).unwrap_or_default()
    } else {
        toml::Table::new()
    };

    let mut plugins_table = table
        .get("plugins")
        .and_then(|v| v.as_table().cloned())
        .unwrap_or_default();
    let mut current_disabled: Vec<String> = plugins_table
        .get("disabled")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if disabled {
        if !current_disabled.contains(&plugin_id.to_string()) {
            current_disabled.push(plugin_id.to_string());
        }
    } else {
        current_disabled.retain(|id| id != plugin_id);
    }

    let arr_values = current_disabled
        .into_iter()
        .map(toml::Value::String)
        .collect();
    plugins_table.insert("disabled".to_string(), toml::Value::Array(arr_values));
    table.insert("plugins".to_string(), toml::Value::Table(plugins_table));

    let toml_str =
        toml::to_string_pretty(&table).map_err(|e| format!("Serialization error: {}", e))?;
    let pid = std::process::id();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp = c_dir.join(format!("config.{}.{}.tmp", pid, timestamp));
    fs::write(&tmp, toml_str).map_err(|e| format!("Failed to write tmp: {}", e))?;
    fs::rename(&tmp, &cfg_path).map_err(|e| format!("Failed to rename: {}", e))?;
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

    #[test]
    fn test_seed_derivation_oklab() {
        let seed_toml = r##"
            name = "seed-test"
            [seed]
            bg = "#121214"
            fg = "#e4e4e7"
            accent = "#38bdf8"
        "##;
        let pal = ThemePalette::from_toml_str_lossy(seed_toml);
        assert_eq!(pal.name, "seed-test");
        assert_eq!(pal.bg_primary, "#121214");
        assert_eq!(pal.text_primary, "#e4e4e7");
        assert_eq!(pal.accent_color, "#38bdf8");
        assert_eq!(pal.term_palette.len(), 16);
        assert_ne!(pal.bg_secondary, pal.bg_primary);
        assert_ne!(pal.bg_tertiary, pal.bg_secondary);
        assert_ne!(pal.text_secondary, pal.text_primary);

        // Explicit override over seed
        let override_toml = r##"
            name = "seed-override"
            accent_color = "#f43f5e"
            [seed]
            bg = "#121214"
            fg = "#e4e4e7"
            accent = "#38bdf8"
        "##;
        let pal_ov = ThemePalette::from_toml_str_lossy(override_toml);
        assert_eq!(pal_ov.accent_color, "#f43f5e");
    }

    #[test]
    fn test_theme_check_and_expand() {
        let temp_dir =
            std::env::temp_dir().join(format!("nevermore_test_theme_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let valid_file = temp_dir.join("valid.toml");
        std::fs::write(
            &valid_file,
            r##"
            name = "valid-seed"
            [seed]
            bg = "#18181b"
            fg = "#f4f4f5"
            accent = "#0a84ff"
            "##,
        )
        .unwrap();

        let check_res = check_theme_file(&valid_file).unwrap();
        assert!(check_res.valid, "Errors: {:?}", check_res.errors);
        assert!(check_res.contrast_ratios.text_primary_on_bg >= 4.5);
        assert!(check_res.contrast_ratios.text_secondary_on_bg_secondary >= 3.0);
        assert!(check_res.contrast_ratios.accent_on_bg_tertiary >= 3.0);

        let expanded = expand_theme_file(&valid_file).unwrap();
        assert!(expanded.contains("term_palette"));
        assert!(expanded.contains("#0a84ff"));

        // Test typo suggestion with Levenshtein distance
        let typo_file = temp_dir.join("typo.toml");
        std::fs::write(
            &typo_file,
            r##"
            name = "typo"
            accent_colr = "#0a84ff"
            bg_primary = "#ffffff"
            "##,
        )
        .unwrap();

        let typo_res = check_theme_file(&typo_file).unwrap();
        assert!(!typo_res.valid);
        assert!(typo_res
            .errors
            .iter()
            .any(|e| e.contains("did you mean 'accent_color'?")));
        // Should also fail dark ceiling
        assert!(typo_res
            .errors
            .iter()
            .any(|e| e.contains("exceeds dark-theme ceiling")));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
