# Nevermore Dependencies

This document tracks all external crate and system dependencies for the Nevermore Wayland sidebar workspace.

## System Dependencies (Arch / CachyOS Linux)
- `gtk4` (>= 4.14, installed `4.18.5`)
- `gtk4-layer-shell` (>= 0.8, installed `0.8.2`)
- `vte4` (>= 0.76, installed `0.84.1`)
- `tmux` (>= 3.3, installed `3.5a`)
- `mpv` (installed `0.41.0-7.1`, Intel Iris Plus 655 VA-API hardware decode)
- `yt-dlp` (installed `2026.10.26-1`, video search & stream extraction)
- `cloudflare-warp-bin` (installed `2026.8.2115-1`, `warp-cli` VPN integration)
- `networkmanager` (installed `1.58.1-3`, `nmcli` WireGuard profiles)
- `libsecret` (installed `0.21.8.2-1.1`, `secret-tool` Secret Service keyring integration)
- `intel-media-driver` (installed `26.3.5-1.1`, VA-API hardware decoding backend)
- `libnotify` / `notify-send` (desktop notifications)
- `spectacle` (KDE Plasma screenshot validation)

## Cargo Workspace Crate Dependencies (Exact Pinned Versions)

| Crate | Exact Version | Workspace / Crates Used | Purpose |
|---|---|---|---|
| `serde` | `=1.0.229` | Workspace, `nevermore-core`, `nevermore-cli`, `nevermore-ui` | Serialization / deserialization |
| `serde_json` | `=1.0.154` | Workspace, `nevermore-core`, `nevermore-cli`, `nevermore-ui` | IPC JSON wire protocol |
| `thiserror` | `=2.0.21` | Workspace, `nevermore-core` | Structured domain error handling |
| `clap` | `=4.6.7` | Workspace, `nevermore-cli` | Command-line argument parsing |
| `toml` | `=0.8.2` | Workspace, `nevermore-core` | Plugin manifest and theme parsing |
| `libc` | `=0.2.191` | Workspace, `nevermore-ui` | Linux `prctl(PR_SET_PDEATHSIG)` child process death signals |
| `gtk4` | `=0.11.5` | `nevermore-ui` | GTK4 Wayland GUI bindings |
| `gtk4-layer-shell` | `=0.8.1` | `nevermore-ui` | Wayland `wlr-layer-shell` integration |
| `vte4` | `=0.10.0` | `nevermore-ui` | Embedded virtual terminal emulator widget |
| `glib` | `=0.22.10` | `nevermore-ui` | GLib main loop, timeouts, and channels |
| `gio` | `=0.22.10` | `nevermore-ui` | GIO application framework |
| `async-channel` | `=2.5.0` | `nevermore-ui` | Lockless asynchronous IPC and notification queues |
