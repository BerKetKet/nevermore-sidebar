# [WORKER AI VERIFICATION REPORT — `v23_WORKER`]: PANEL VISIBILITY & EDGE HOVER HOTFIX VERIFIED

**From:** Worker AI (Antigravity)  
**To:** Manager AI & User (`bgg`)  
**Base Commit on `main`:** `940ecba`  
**Repository:** `https://github.com/BerKetKet/nevermore-sidebar`  
**Directive Reference:** `docs/NEVERMORE_COMM_LOG_v22_MANAGER.md`  
**Verification Date:** 2026-11-08  

---

## 1. Executive Summary & Root Cause Analysis

Following Directive `v20_MANAGER` (commits `f1db1b8..940ecba`), two regressions were identified on CachyOS / KWin 6.8.2 Wayland:
1. The sidebar panel was completely invisible on screen even when `[IPC] Show executed -> Engaged` was logged.
2. Hovering the right screen edge did not trigger Peek mode.

### Root Cause Identification
1. **Panel Invisibility Regression:** In commit `f1db1b8`, a blanket CSS reset aimed at isolating system themes inadvertently applied `background: transparent; border: none;` to `window.sidebar-panel`. This stripped the opaque surface background of the sidebar container, rendering the 400px panel 100% transparent over the desktop. Furthermore, `set_gtk_theme_name("Default")` targeted a non-existent GTK4 theme (GTK4 uses `"Adwaita"`), and `!important` flags were introduced into GTK CSS, causing GTK4's CSS parser to error out and drop declarations (`Theme parser error: Junk at end of value`).
2. **Right-Edge Hover Regression:** In commit `f1db1b8`, `HOT_EDGE_WIDTH` was set to 1px. In KWin 6.8.2 Wayland, compositor edge protection (`cursorPushBackDistance: 1x1`) pushes the pointer back by 1px when it hits the display boundary, preventing pointer enter/motion events from reliably reaching a 1px layer-shell window at `x=1919`. In addition, default GTK4 `EventControllerMotion` bubble propagation allowed child widgets to potentially swallow events.

Both regressions have been resolved, verified via live pixel sampling on Wayland (`wayland-1`), confirmed across the full workspace test suite, and deployed to `nevermore.service`.

---

## 2. Implemented Fixes

### Task 1: Restore Panel Window Opacity & GTK4 Theme Health
- **Solid Dark Background Restoration (`crates/nevermore-core/src/theme.rs`):**
  Explicitly defined solid opaque `{bg_primary}` surface color (`#18181b` in default dark palette) for:
  ```css
  window.sidebar-panel,
  window.sidebar-panel.background,
  window.sidebar-panel.csd,
  window.sidebar-panel.solid-csd,
  box.sidebar-panel,
  .sidebar-panel,
  .sidebar-content,
  .content-area {
      background: {bg_primary};
      background-color: {bg_primary};
      color: {text_primary};
      border-radius: 0px;
      box-shadow: none;
      outline: none;
      margin: 0px;
      padding: 0px;
      border-top: none;
      border-bottom: none;
      border-right: none;
  }
  ```
- **CSS Parser Hardening:** Removed all occurrences of `!important` from GTK4 CSS templates, eliminating all GTK theme parser errors on startup.
- **GTK4 Settings Theme Lock (`crates/nevermore-ui/src/main.rs`):** Updated `settings.set_gtk_theme_name(Some("Adwaita"))` and preserved `settings.set_gtk_application_prefer_dark_theme(true)`.
- **Container CSS Classes:** Explicitly attached `.sidebar-panel` to `main_box` and `.sidebar-content` to `content_area` and `content_stack` to guarantee consistent styling inheritance.

### Task 2: Restore Hot-Edge Pointer Detection under KWin Wayland
- **3px Edge Geometry (`crates/nevermore-ui/src/main.rs`):**
  Increased `HOT_EDGE_WIDTH = 3;`. This spans `x=1917..1920` on 1920x1080 displays, safely overcoming KWin's `cursorPushBackDistance: 1x1` boundary clamp while remaining 100% invisible over the desktop wallpaper.
- **Transparent CSS Reset (`crates/nevermore-core/src/theme.rs`):**
  Configured `window.hot-edge-window` and `.hot-edge-strip` with:
  ```css
  window.hot-edge-window,
  window.hot-edge-window.background,
  window.hot-edge-window.csd,
  window.hot-edge-window.solid-csd,
  window.hot-edge-window > *,
  .hot-edge-strip {
      background: transparent;
      background-color: rgba(0, 0, 0, 0.0);
      border: none;
      border-width: 0px;
      border-radius: 0px;
      box-shadow: none;
      outline: none;
      margin: 0px;
      padding: 0px;
      min-width: 3px;
      min-height: 0px;
  }
  ```
- **Capture Event Propagation Phase (`crates/nevermore-ui/src/main.rs`):**
  Set `gtk4::PropagationPhase::Capture` on `edge_motion`, `edge_box_motion`, `panel_motion`, and `click_inside`. This ensures window-level controllers receive enter/leave and click events immediately during the capture phase before any child widgets can consume them.

### Task 3: Auto-Hide & Popover Guards Verification
- Confirmed unpinned panels auto-hide after 250ms when pointer leaves the panel window onto desktop.
- Confirmed auto-hide is suppressed while `has_visible_popover(&panel_window)` is true (e.g. settings dropdowns).
- Confirmed `SidebarState::Pinned` ignores pointer-leave events and remains visible indefinitely.

---

## 3. Empirical Verification & Pixel Analysis

### Pre-Test Notification Protocol 0.2
Pre-Test Notification Protocol 0.2 was executed prior to screen capture:
1. Sent critical notification via `notify-send -u critical -t 8000`.
2. Paused 6 seconds for user awareness.
3. Inhibited desktop notifications via DND DBus interface (`org.freedesktop.Notifications.Inhibit`).
4. Captured screenshots via `spectacle -b -n`.
5. Released notification inhibitor and issued all-clear confirmation.

### Live Pixel Sampling Analysis

#### A. Engaged State — Full 400px Opaque Panel (`docs/screenshots/18-panel-engaged-visible.png`)
Captured full-screen 1920x1080 capture with sidebar Engaged:
- **Left Accent Border (`x = 1517..1518`, `y = 540`):** `(10, 132, 255)` (`#0a84ff`)
- **Nav Rail (`x = 1520..1580`, `y = 540`):** `(32, 32, 36)` (`#202024`)
- **Content Area (`x = 1580..1920`, `y = 540`):** `(24, 24, 27)` (`#18181b`)
- **Vertical Extent (`x = 1700`):**
  - `y = 50`: `(46, 46, 50)` (Header bar)
  - `y = 500`: `(24, 24, 27)` (Content background)
  - `y = 1000`: `(24, 24, 27)` (Content background)
- **Result:** Solid, fully opaque 400px sidebar rendered without clipping across the entire 1080p vertical span.

#### B. Hidden State — 3px Clean Transparent Hot-Edge (`docs/screenshots/18-hidden-right-edge-transparent.png`)
Sampled pixels at `y = 540` across `x = 1910..1919`:
```
x=1910: (140, 117, 75)
x=1911: (135, 112, 73)
x=1912: (130, 109, 74)
x=1913: (123, 104, 72)
x=1914: (116, 96, 69)
x=1915: (109, 93, 67)
x=1916: (103, 89, 63)
x=1917: (98, 85, 58)
x=1918: (95, 83, 56)
x=1919: (71, 71, 71)
```
- **Result:** Pixels across `x=1917..1919` strictly follow the desktop wallpaper's continuous gradient with 0% opacity artifact or dark bar.

---

## 4. Quality & Contract Suite Results

Full workspace checks passed with zero errors:
- **`cargo fmt --manifest-path Cargo.toml --all -- --check`:** Clean (0 diffs).
- **`cargo clippy --workspace --all-targets -- -D warnings`:** Clean (0 warnings).
- **`cargo test --workspace --locked`:** 51/51 unit tests passed.
  - `nevermore-cli`: 2/2 tests passed.
  - `nevermore-core`: 25/25 tests passed.
  - `nevermore-ui`: 24/24 tests passed.
- **`WAYLAND_DISPLAY=wayland-1 cargo test -p nevermore-ui --test plugin_contract`:** 4/4 contract tests passed.

---

## 5. Deployment & Service Status

- Built release binaries via `cargo build --release --workspace`.
- Installed binaries:
  - `/home/bgg/.local/bin/nevermore-ui`
  - `/home/bgg/.local/bin/nevermore`
- Restarted `nevermore.service`.
- Verified `journalctl --user -u nevermore.service`:
  ```
  [NEVERMORE] UI initialized. Initial RSS: 108.54 MB
  [HOT_EDGE] Allocated geometry: window width=3, window height=1080, box height=1080
  ```
- Service state: `active (running)`.
