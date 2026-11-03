# 📦 Nevermore System Dependencies

This document records the exact versions of all system packages, libraries, and language toolchains evaluated and pinned for the Nevermore project on CachyOS / KDE Plasma 6 (Wayland) on an Intel integrated GPU.

---

## 🛠️ Language Toolchain (Pinned)

| Tool | Version | Source | Status |
| :--- | :--- | :--- | :--- |
| **Rust Compiler (`rustc`)** | `1.98.0 (88d9e12ae 2026-08-18)` | `rustup` / `rust-toolchain.toml` | **Installed & Pinned** |
| **Cargo Package Manager** | `1.98.0 (797e8a9bc 2026-08-05)` | `rustup` / `rust-toolchain.toml` | **Installed & Pinned** |
| **Rust Edition** | `2021` | `Cargo.toml` | **Configured** |

---

## 🖥️ System Packages & Compositor Environment (`pacman -Qi`)

| Package Name | Installed Version | Repository | Role in Nevermore | Status |
| :--- | :--- | :--- | :--- | :--- |
| `kwin` | `6.8.2-1.1` | cachyos-extra-v3 | Primary target Wayland compositor | **Installed** |
| `plasma-workspace` | `6.8.2-1.1` | cachyos-extra-v3 | KDE Plasma 6 desktop environment | **Installed** |
| `wayland` | `1.26.0-1.1` | cachyos-extra-v3 | Display server protocol & client libraries | **Installed** |
| `gtk4` | `1:4.24.1-1.1` | cachyos-extra-v3 | Candidate GUI toolkit (Bake-Off Option A) | **Installed** |
| `gtk4-layer-shell` | `1.3.0-1.1` | cachyos-extra-v3 | Wayland `wlr-layer-shell` integration for GTK4 | **Installed** |
| `vte4` | `0.84.1-1.1` | extra / cachyos-extra-v3 | GTK4 terminal emulator widget | ⚠️ **Available, not yet installed** |
| `tmux` | `3.8-1.1` | cachyos-extra-v3 | Durable terminal session daemon (`tmux -L nevermore`) | **Installed** |
| `mpv` | `1:0.41.0-6.1` | cachyos-extra-v3 | Media playback engine backend | **Installed** |
| `yt-dlp` | `2026.10.26-1` | cachyos-extra-v3 | Media streaming & audio extraction helper | **Installed** |
| `cloudflare-warp-bin` | `2026.8.2115-1` | cachyos | VPN network toggle provider (`warp-cli`) | **Installed** |
| `networkmanager` | `1.58.1-2` | cachyos-extra-v3 | Network connection & Wi-Fi management | **Installed** |
| `intel-media-driver` | `26.3.5-1.1` | cachyos-extra-v3 | VAAPI hardware acceleration for Intel iGPU | **Installed** |
| `mesa` | `3:26.2.5-1` | cachyos-extra-v3 | OpenGL / Vulkan graphics drivers (Intel Iris Plus 655) | **Installed** |

---

## ⚠️ Missing Packages & Installation Command

The only evaluated package from the mandate that is not currently installed on the host is **`vte4`** (Virtual Terminal Emulator widget for GTK4).

To install it when needed for the Phase 3 Bake-Off:

```bash
sudo pacman -S vte4
```

---

## 🔒 Cargo Crate Dependency Pins (`Cargo.lock`)

All Cargo dependencies use exact `=` version pins in `Cargo.toml`:

| Crate | Exact Pin | Purpose |
| :--- | :--- | :--- |
| `nevermore-core` | `path = "crates/nevermore-core"` | Headless state machine, traits, types |
| `serde` | `=1.0.229` | Serializable state and configuration |
| `serde_json` | `=1.0.154` | JSON serialization for IPC and tasks |
| `thiserror` | `=2.0.21` | Strongly typed domain errors |
| `clap` | `=4.6.7` | Command-line interface argument parsing |
