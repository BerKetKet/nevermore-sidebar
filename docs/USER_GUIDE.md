# Nevermore Sidebar — Complete User Guide

Welcome to **Nevermore**, the right-edge Wayland sidebar and productivity hub for KDE Plasma 6.

---

## 1. Quick Start & Basic Interaction

### Mouse & Gesture Navigation
- **Right-Edge Hover:** Move your cursor to the extreme right edge of the primary display (`x = 1919..1920`) to reveal the sidebar in **Peek** mode (passive view, keyboard focus remains on your active window).
- **Click to Engage:** Clicking anywhere on the sidebar transitions it into **Engaged** mode, granting keyboard focus to the active tab (e.g., typing in the Terminal, Gemini composer, or Notepad).
- **Pin / Unpin:** Click the Pin icon button in the header bar. When pinned, the sidebar remains visible even if your mouse leaves the window.
- **Escape to Dismiss:** Pressing `Esc` while the sidebar is focused immediately hides it and returns focus to your desktop.

---

## 2. CLI Reference (`nevermore`)

Nevermore comes with a lightning-fast Unix domain socket client (`nevermore`):

### Visibility & Navigation
```bash
# Show sidebar on a specific tab
nevermore show terminal
nevermore show media
nevermore show gemini
nevermore show youtube
nevermore show vpn
nevermore show email
nevermore show notifications
nevermore show notepad
nevermore show widgets

# Hide sidebar
nevermore hide

# Toggle sidebar visibility
nevermore toggle

# Check sidebar state
nevermore status
```

### Theming Commands
```bash
# List all available themes (built-in and user-defined)
nevermore theme list

# Switch theme instantaneously
nevermore theme set nevermore-dark
nevermore theme set catppuccin-mocha

# Validate theme contrast and WCAG 2.1 conformance
nevermore theme check catppuccin-mocha

# Generate a complete 15-color theme from a single seed color using OKLab
nevermore theme expand "#89b4fa"
```

---

## 3. Tab Features & Workflows

### 3.1 Terminal Tab (`terminal`)
- Embedded VTE terminal connected to a background tmux session (`tmux -L nevermore`).
- Survives crashes, restarts, and logout without losing terminal buffers or running commands.
- Standard tmux shortcuts apply (`Ctrl-b c` for new window, `Ctrl-b %` for vertical split, `Ctrl-b "` for horizontal split).

### 3.2 Widgets Tab (`widgets`)
- Real-time CPU, RAM, and Swap utilization meters.
- High-precision digital clock and quick system status toggles.

### 3.3 Notifications Tab (`notifications`)
- Captures all system notifications via D-Bus without intercepting standard notification workflows.
- Rich metadata rendering, app desktop entry detection, and image-data preview.
- **Do Not Disturb (DND):** Click the Bell toggle in the header. DND uses native FreeDesktop `Inhibit` to prevent popups from disrupting your workflow while continuing to log notifications in the sidebar feed.

### 3.4 Notepad Tab (`notepad`)
- Fast scratchpad for quick notes, code snippets, and clipboard storage.
- Auto-saves atomically on every keystroke to `~/.local/share/nevermore/plugins/notepad/notes.txt`.
- Fully crash-resilient with backup restoration on corrupt read.

### 3.5 Media Player Tab (`media`)
- Scans `~/Music` and `~/Videos` for audio and video media files.
- High-efficiency playback powered by `mpv` with Intel VA-API hardware decoding (`--hwdec=vaapi --vo=gpu`).
- Complete playback controls (Play, Pause, Previous, Next, Seek slider, Volume).
- Remembers active playlist, track index, and playback position across restarts.

### 3.6 Gemini AI Tab (`gemini`)
- Conversational AI powered by Google Gemini API.
- Non-blocking asynchronous query execution with conversational history saved to `~/.local/share/nevermore/plugins/gemini/chat_history.json`.
- **API Key Configuration:**
  - Securely store your Gemini API key in Freedesktop Secret Service (recommended):
    ```bash
    secret-tool store --label="Gemini API Key" service nevermore account gemini_api_key
    ```
  - Alternatively, export `GEMINI_API_KEY="your-api-key"` in your shell environment.

### 3.7 YouTube Tab (`youtube`)
- Fast video search powered by `yt-dlp --flat-playlist`.
- Launch hardware-accelerated playback with zero browser CPU overhead using `mpv --hwdec=vaapi`.

### 3.8 VPN Tab (`vpn`)
- **Cloudflare WARP:** One-click connect/disconnect button reflecting real-time connection state (`warp-cli status`).
- **WireGuard / NetworkManager:** Discovers and activates configured NetworkManager WireGuard profiles with a single click.
- For instructions on importing `.conf` files, consult `docs/VPN_SETUP_GUIDE.md`.

### 3.9 Email Overview Tab (`email`)
- Displays unread message counts and recent inbox envelope summaries.
- One-click launch/raise button for your native desktop email client (`kmail`, `thunderbird`).
- Guaranteed 0.0% background idle CPU when hidden.

---

## 4. Configuration & File Locations

| File / Directory | Description |
|---|---|
| `~/.config/nevermore/themes/` | Custom user TOML theme definitions |
| `~/.local/share/nevermore/plugins/` | Persistent state storage per plugin |
| `~/.config/systemd/user/nevermore.service` | Systemd service definition |
| `$XDG_RUNTIME_DIR/nevermore/nevermore.sock` | Unix domain socket IPC endpoint |
| `$XDG_RUNTIME_DIR/nevermore/mpv.sock` | Media player IPC socket |
