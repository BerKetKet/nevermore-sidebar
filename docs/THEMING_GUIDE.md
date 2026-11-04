# Nevermore Sidebar Theming Guide

Nevermore features a **Semantic Theme Engine** with live reloading, solid dark palettes, OKLab seed derivation, and CLI control.

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

## 2. Seed Themes (`[seed]` Table) & OKLab Derivation

Starting in Phase 7.6, Nevermore supports minimal **3-key Seed Themes**. Instead of specifying 20+ color variables manually, you define only:

```toml
name = "cyber-neon"

[seed]
bg = "#0d1117"
fg = "#e6edf3"
accent = "#58a6ff"
```

The Nevermore theme engine utilizes pure-Rust **OKLab color space math** to automatically derive:
- `bg_secondary` & `bg_tertiary`: Lightened surfaces with smooth perceptual steps
- `border_color`: Subtle separator line derived from `bg`
- `text_secondary`: Contrast-adjusted 65% opacity equivalent mixed against `bg`
- `term_palette`: Full 16 ANSI terminal colors mapped to balanced lightness in OKLab
- Explicit overrides: You can override any derived value by defining it at the top level of the TOML file!

---

## 3. CLI Theme Commands

You can inspect, validate, expand, and switch themes live without restarting the sidebar:

```bash
# List all available themes (active theme marked with an asterisk)
nevermore theme list

# Switch to a theme live
nevermore theme set catppuccin-mocha

# Switch back
nevermore theme set nevermore-dark

# Validate a theme file before installing
nevermore theme check ~/.config/nevermore/themes/my-theme.toml
nevermore theme check ~/.config/nevermore/themes/my-theme.toml --json

# Expand a 3-key seed theme into a fully explicit 20+ field TOML palette
nevermore theme expand ~/.config/nevermore/themes/my-theme.toml
```

### Theme Validation Checks (`theme check`)
The validator enforces:
1. **Solid Hex Format:** All color values must be 6-digit `#rrggbb` hex codes. Transparent or alpha hex codes (`#rrggbbaa`) are strictly forbidden.
2. **Dark Ceiling:** `bg_primary` relative luminance MUST be below `0.15` (enforcing solid dark mode).
3. **WCAG Contrast Ratios:** `text_primary` on `bg_primary` must achieve at least `4.5:1` contrast ratio.
4. **Typo Detection:** Unknown keys are flagged with Levenshtein-distance suggestions for the closest valid key.

---

## 4. CSS Custom Properties (`--nm-*`) & Semantic Tone Classes

The theme engine automatically injects CSS variables into the GTK styling context on `window` and `.sidebar-panel`:

```css
var(--nm-bg-primary)
var(--nm-bg-secondary)
var(--nm-bg-tertiary)
var(--nm-border-color)
var(--nm-text-primary)
var(--nm-text-secondary)
var(--nm-accent)
var(--nm-danger)
var(--nm-success)
var(--nm-warning)
var(--nm-border-radius)
```

Additionally, utility tone classes are available for custom CSS and plugin widgets:
- `.nm-tone-primary` (color: `var(--nm-text-primary)`)
- `.nm-tone-secondary` (color: `var(--nm-text-secondary)`)
- `.nm-tone-muted` (color: `var(--nm-text-secondary)`, opacity: `0.6`)
- `.nm-tone-accent` (color: `var(--nm-accent)`)
- `.nm-tone-danger` (color: `var(--nm-danger)`)
- `.nm-tone-success` (color: `var(--nm-success)`)
- `.nm-tone-warning` (color: `var(--nm-warning)`)

---

## 5. User Override Stylesheet (`custom.css`)

For manual CSS tweaks, create `~/.config/nevermore/custom.css`.
When present, this stylesheet is automatically loaded with `STYLE_PROVIDER_PRIORITY_USER` on top of the dynamic theme CSS and re-applied whenever `nevermore theme set` is executed.
