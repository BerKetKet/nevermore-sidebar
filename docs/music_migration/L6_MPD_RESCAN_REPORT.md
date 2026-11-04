# Phase L6: MPD Configuration & Library Rescan Report

**Date:** 2026-11-05  
**Service:** `mpd.service` (systemd user unit)  
**Status:** ACTIVE & FULLY INDEXED  

---

## 1. MPD Configuration (`~/.config/mpd/mpd.conf`)

Verified and updated `~/.config/mpd/mpd.conf` to point to the canonical capitalized `~/Music` directory:

```conf
music_directory "~/Music"
playlist_directory "~/.config/mpd/playlists"
db_file "~/.config/mpd/database"
state_file "~/.config/mpd/state"
bind_to_address "~/.config/mpd/socket"
bind_to_address "127.0.0.1"
port "6600"

audio_output {
    type "pipewire"
    name "PipeWire"
}
```

## 2. Daemon Status & Rescan Execution

- **Restart:** `systemctl --user restart mpd` (PID 140247, memory 46.7M, zero errors).
- **Index:** `mpc update --wait` executed synchronously against `~/Music`.
- **Database Stats (`mpc stats`):**
  - **Artists:** 160
  - **Albums:** 166
  - **Songs:** 686 (679 newly migrated tracks + 7 existing pre-migration tracks)
  - **Total Database Playtime:** 2 days, 3 hours, 58 minutes, 36 seconds
  - **Outputs:** PipeWire audio sink enabled

## 3. Client Verification

- **Nevermore Music Plugin:** Pure-Rust `sdk::mpd` transport connects to `~/.config/mpd/socket` (fallback `127.0.0.1:6600`).
- **Euphonica Client:** Verified GSettings schema `io.github.htkhiem.Euphonica.client` configured to `mpd-host 'localhost'`, `mpd-port 6600`, `mpd-auto-reconnect true`.
