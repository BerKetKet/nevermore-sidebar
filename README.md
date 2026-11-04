# Nevermore Sidebar (`nevermore-sidebar`)

> **High-Performance Wayland Right-Edge Sidebar & Multi-Tab Hub for KDE Plasma 6**  
> Architected in Rust with GTK4 (`gtk4-layer-shell`), VTE, persistent tmux sessions, systemd task supervision, OKLab perceptual theming, and the Zero-Context Plugin SDK.

---

## 1. Architectural Highlights

- **Wayland Native Layer-Shell:** Anchored to the right edge with `Layer::Top` and `exclusive_zone = -1`, guaranteeing full 1080p height without stealing clicks from or occluding active fullscreen games.
- **Zero-Context Plugin SDK (`nevermore_ui::prelude::*`):** Any LLM or developer can write a self-contained ~50-line plugin in `crates/nevermore-ui/src/plugins/<id>/mod.rs` with automatic discovery (`build.rs`), automatic lifecycle-managed timers (`ctx.every`), background execution (`ctx.run`, `ctx.daemon`), and typed actor mailboxes (`ctx.mailbox`).
- **Text-Free Icon UI Chrome:** All navigation, header controls, and status badges use crisp, theme-tinted Lucide SVG icons (`nm_icon`, `nm_icon_button`, `Tone`) and accessible tooltips. Only actual user and system data is displayed as text (`.nm-data`).
- **Crash Resilience & Session Survival:** Sub-100ms show/hide transitions, auto-restarting systemd user service (`nevermore.service`), zero memory/FD leaks, and full terminal session preservation across UI restarts via detached `tmux -L nevermore`.
- **Perceptual OKLab Seed Theme Engine:** Generates complete 15-color WCAG 2.1-compliant GTK4 stylesheets from a single brand hex code or standard TOML palettes.

---

## 2. Workspace Crates

| Crate | Path | Responsibility |
|---|---|---|
| **`nevermore-core`** | `crates/nevermore-core` | State machine, Unix domain socket IPC, OKLab theme engine, tmux backend, systemd supervision, KWin shortcuts. |
| **`nevermore-ui`** | `crates/nevermore-ui` | GTK4 Layer-shell windows, text-free chrome, Lucide SVG renderer, Zero-Context Plugin SDK, and 9 built-in feature tabs. |
| **`nevermore` (CLI)** | `crates/nevermore-cli` | Zero-overhead IPC CLI for controlling the sidebar (`show`, `hide`, `toggle`, `status`, `theme`, `run`). |

---

## 3. Built-In Tabs & Plugins

1. **Terminal (`terminal`):** Embedded VTE terminal connected to a persistent `tmux -L nevermore` session with tab navigation, hotkey integration, and zero state loss across crashes.
2. **Widgets (`widgets`):** Real-time system monitoring (CPU, RAM, Swap), live clock, and quick system toggles.
3. **Notifications (`notifications`):** Live FreeDesktop notification daemon listener with image-data parsing, chronological feed, and native D-Bus Do Not Disturb (DND) suppression (`Inhibit`).
4. **Notepad (`notepad`):** Zero-friction text scratchpad with instantaneous atomic auto-save and corruption recovery.
5. **Media Player (`media`):** Hardware-accelerated audio/video playback via `mpv --hwdec=vaapi` (Intel Iris Plus 655 iGPU) with IPC playback controls, playlist scan (`~/Music`, `~/Videos`), and timestamp resume across restarts.
6. **Gemini AI (`gemini`):** Conversational AI assistant with Freedesktop Secret Service credential storage (`secret-tool`), persistent chat history, and non-blocking streaming.
7. **YouTube (`youtube`):** Background video search via `yt-dlp` and instant hardware-accelerated playback via `mpv`.
8. **VPN Manager (`vpn`):** Real-time Cloudflare WARP client status/toggle (`warp-cli`) and NetworkManager WireGuard profile switcher (`nmcli`).
9. **Gmail Inbox (`email`):** Official Gmail REST API v1 unread message inbox, unread badge, per-item cards with sender/subject/snippet, and one-click thread open in browser or mark-as-read.

---

## 4. Building & Running

### Prerequisites
- Linux with Wayland compositor (KDE Plasma 6 / KWin recommended)
- Rust toolchain (1.80+)
- System libraries: `gtk4`, `gtk4-layer-shell`, `vte4`, `tmux`, `mpv`, `yt-dlp`, `secret-tool`

### Build
```bash
cargo build --release
```

### Run via Systemd User Service
```bash
cp resources/systemd/nevermore.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now nevermore.service
```

### CLI Control
```bash
# Show specific tab
nevermore show terminal
nevermore show media
nevermore show vpn

# Toggle visibility
nevermore toggle

# Query status
nevermore status

# Theming
nevermore theme list
nevermore theme set catppuccin-mocha
nevermore theme check nevermore-dark
```

---

## 5. Verification & Testing

Nevermore maintains strict automated verification gates:

```bash
# 1. Plugin SDK Contract Verification (Manifests, Icons, Linter, Widget Tree, Timers)
DISPLAY="" WAYLAND_DISPLAY="" cargo test -p nevermore-ui --test plugin_contract

# 2. Workspace Integration & Unit Tests (41/41 passing)
cargo test --workspace --locked

# 3. Static Analysis & Linting (0 warnings enforced)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# 4. WCAG 2.1 Relative Luminance & Icon Contrast Verification
python3 scripts/verify_icon_contrast.py
```

---

## 6. Zero-Context Plugin Authoring

To create a new plugin, hand any external LLM or developer `docs/ZERO_CONTEXT_LLM_PROMPT.md`. Creating `crates/nevermore-ui/src/plugins/<id>/mod.rs` with `pub const SPEC: PluginSpec` and `pub fn build(ctx: &PluginContext) -> gtk4::Widget` is automatically discovered by `build.rs` and integrated without editing any registration files.
