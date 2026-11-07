# Nevermore Sidebar — Complete User Guide

Welcome to **Nevermore**, the right-edge Wayland sidebar and productivity hub for KDE Plasma 6.

---

## 1. Quick Start & Basic Interaction

### Mouse & Gesture Navigation
- **Right-Edge Hover:** Move your cursor to the extreme right edge of the primary display (`x = 1919..1920`) to reveal the sidebar in **Peek** mode (passive view, keyboard focus remains on your active window).
- **Click to Engage:** Clicking anywhere on the sidebar transitions it into **Engaged** mode, granting keyboard focus to the active tab (e.g., typing in the Terminal or Notepad).
- **Pin / Unpin:** Click the Pin icon button in the header bar. When pinned, the sidebar remains visible even if your mouse leaves the window.
- **Escape to Dismiss:** Pressing `Esc` while the sidebar is focused immediately hides it and returns focus to your desktop.

---

## 2. CLI Reference (`nevermore`)

Nevermore comes with a lightning-fast Unix domain socket client (`nevermore`):

### Visibility & Navigation
```bash
# Show sidebar on a specific tab
nevermore show terminal
nevermore show widgets
nevermore show notifications
nevermore show notepad
nevermore show music
nevermore show __settings__

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

### 3.5 Music Player Tab (`music`)
- High-performance MPD controller with 4 views (Now Playing, Queue, Lyrics, Library).
- Live album art rendering with dominant color palette derivation.
- Synchronized `.lrc` and plain-text lyrics viewer with LRCLIB auto-fetching.
- Full 100-track queue view with reordering, "Play Next", shuffle, and save-to-playlist.
- Library search across Artist, Album, Title with instant add/replace actions.

### 3.6 Settings Page (`__settings__`)
- Bottom-pinned settings icon on the navigation rail.
- Live theme selector with OKLab palette generation from custom hex seeds.
- UI font and monospace font configuration with live previews.
- Plugin management and status toggles.

---

## 4. Configuration & File Locations

| File / Directory | Description |
|---|---|
| `~/.config/nevermore/themes/` | Custom user TOML theme definitions |
| `~/.local/share/nevermore/plugins/` | Persistent state storage per plugin |
| `~/.config/systemd/user/nevermore.service` | Systemd service definition |
| `$XDG_RUNTIME_DIR/nevermore/nevermore.sock` | Unix domain socket IPC endpoint |
