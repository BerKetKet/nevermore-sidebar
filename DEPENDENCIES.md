# Nevermore Dependencies

This document tracks all external crate and system dependencies for the Nevermore Wayland sidebar workspace.

## System Dependencies (Arch / CachyOS Linux)
- `gtk4` (>= 4.14)
- `gtk4-layer-shell` (>= 0.8)
- `vte4` (>= 0.76)
- `tmux` (>= 3.3)
- `playerctl` (for MPRIS media control)
- `wireplumber` / `wpctl` (for PipeWire audio control)
- `libnotify` / `notify-send` (desktop notifications)
- `spectacle` / `grim` / `slurp` (screenshot validation)

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
