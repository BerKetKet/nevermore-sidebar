# Nevermore Sidebar Theming Guide

Nevermore features a **Semantic Theme Engine** with live reloading, solid dark palettes, and CLI control.

---

## 1. Built-In Themes

Nevermore bundles 3 solid dark themes automatically synchronized to `~/.config/nevermore/themes/`:

1. **`nevermore-dark`** (Default)
   - Background: `#18181b`
   - Cards / Rail: `#202024`
   - Accent: `#0a84ff` (macOS Blue)
   - Font: Inter 10pt / Hack Nerd Font Mono 10pt

2. **`catppuccin-mocha`**
   - Background: `#1e1e2e`
   - Cards / Rail: `#181825`
   - Accent: `#89b4fa` (High-contrast dark `#11111b` label text)
   - Danger: `#f38ba8`

3. **`tokyo-night`**
   - Background: `#1a1b26`
   - Cards / Rail: `#16161e`
   - Accent: `#7aa2f7` (Tokyo Blue)
   - Danger: `#f7768e`

---

## 2. CLI Theme Commands & Live Reloading

You can list and switch themes live without restarting the sidebar:

```bash
# List all available themes (active theme marked with an asterisk)
nevermore theme list

# Switch to a theme live
nevermore theme set catppuccin-mocha

# Switch back
nevermore theme set nevermore-dark
```

When switched via CLI or IPC:
1. `nevermore-ui` validates the theme name (`^[a-zA-Z0-9_-]{1,64}$`) to prevent path traversal.
2. The TOML palette file is parsed using the hardened per-field validator:
   - Any missing, malformed, or invalid fields fall back safely per-field to the default `nevermore-dark` values while emitting a logged warning.
   - All color values must be strict solid 6-digit `#rrggbb` hex codes (`rgba()`, `transparent`, and alpha-hex values are rejected to uphold the solid dark surface rules).
   - If `term_palette` contains fewer than 16 items or has invalid colors, fallback ANSI colors ensure that exactly 16 valid colors and `term_cursor` are passed to VTE.
3. The GTK4 `CssProvider` dynamically reloads CSS for all window surfaces and reloads `~/.config/nevermore/custom.css` if present.
4. Registered plugins receive `on_theme_changed(&palette)`, updating VTE embedded terminal foreground, background, cursor, ANSI 16-color palette, and fonts in real time.
5. The active theme name is saved atomically in `~/.config/nevermore/config.toml` (preserving any existing configuration keys).

---

## 3. Creating a Custom Theme

To create your own custom theme:
1. Create a `.toml` file in `~/.config/nevermore/themes/<my-theme>.toml`.
2. Fill in your chosen fields (all fields are optional; any omitted or invalid field automatically falls back to `nevermore-dark` defaults):

```toml
name = "my-custom-theme"
bg_primary = "#141416"
bg_secondary = "#1c1c1f"
bg_tertiary = "#26262b"
border_color = "#33333a"
text_primary = "#f0f0f5"
text_secondary = "#9090a0"
accent_color = "#38bdf8"
danger_color = "#f43f5e"
success_color = "#10b981"
warning_color = "#f59e0b"

ui_font = "Inter"
mono_font = "Hack Nerd Font Mono"
mono_font_size = 10
border_radius_px = 8

term_bg = "#141416"
term_fg = "#f0f0f5"
term_cursor = "#38bdf8"
term_palette = [
    "#141416", "#f43f5e", "#10b981", "#f59e0b",
    "#38bdf8", "#c084fc", "#2dd4bf", "#f0f0f5",
    "#3f3f46", "#fb7185", "#34d399", "#fbbf24",
    "#60a5fa", "#d8b4fe", "#5eead4", "#ffffff"
]
```

3. Test your new theme live:
```bash
nevermore theme set my-custom-theme
```

---

## 4. User Override Stylesheet (`custom.css`)

For manual CSS tweaks, create `~/.config/nevermore/custom.css`.
When present, this stylesheet is automatically loaded with `STYLE_PROVIDER_PRIORITY_USER` on top of the dynamic theme CSS and re-applied whenever `nevermore theme set` is executed.
