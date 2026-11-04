# [MANAGER AI DIRECTIVE — `v11_MANAGER`]: PHASES 8–11 APPROVED, FINAL RELEASE POLISH, LIVE SERVICE DEPLOYMENT, & IDLE STANDBY

**From:** Manager AI (Google AI Studio)  
**To:** Worker AI (Antigravity) & User (`bgg`)  
**Audited Code Commit on `main`:** `86a19ed`  
**Audited Docs Commit on `main`:** `06db684`  
**Communication Channel:** Google AI Studio DevTools Bridge Daemon  

---

## 1. FORMAL APPROVAL OF PHASES 8, 9, 11, AND 10

Phases 8, 9, 11, and 10 (`86a19ed` / `06db684`) are **APPROVED**.
All requirements from `v09_MANAGER` were fulfilled:
- Zero-Context Plugin SDK used across all 9 plugins.
- Headless plugin contract tests passing 5/5 in 0.00s.
- 41/41 workspace tests passing, clippy clean, fmt clean.
- Crash survival soak test passed with < 2s restart, 0 defuncts, tmux persistence.
- Sub-100ms show/hide transitions (39ms show / 84ms hide).
- Hidden idle CPU 0.0–0.2%.
- 100% WCAG 2.1 contrast compliance and 400px width lock across all tabs.

---

## 2. PHASE 10.1 FINAL RELEASE POLISH TASKS

Before entering final idle standby for `bgg`'s return, execute this 3-step Phase 10.1 Final Release Polish on `main` and then stop all automated tasks:

1. **Strict `=` Pin on `async-channel` (`crates/nevermore-ui/Cargo.toml`):**
   - Change `async-channel = "2.5.0"` to `async-channel = "=2.5.0"`.
   - Update System Dependencies in `DEPENDENCIES.md` to list `mpv`, `yt-dlp`, `cloudflare-warp-bin`, `networkmanager`, and `libsecret` with installed versions.
2. **Deploy Fresh Release Binary to Live `nevermore.service` Daemon:**
   - Run `cargo build --release --workspace --locked` and `cargo test --workspace --locked` (plus `DISPLAY="" WAYLAND_DISPLAY="" cargo test -p nevermore-ui --test plugin_contract`).
   - Ensure `~/.config/systemd/user/nevermore.service` (and `/home/bgg/.local/bin/nevermore` + `/home/bgg/.local/bin/nevermore-ui` if installed there) points to and runs the newly built Phase 8–11 release binaries.
   - Run `systemctl --user daemon-reload && systemctl --user restart nevermore.service`.
   - Reset active theme to `nevermore-dark` (`nevermore theme set nevermore-dark` and `nevermore hide`).
3. **Sync Final Artifacts to GitHub & Google Drive, Then Enter Standby:**
   - Commit the `Cargo.toml` / `DEPENDENCIES.md` polish to `main`, regenerate `docs/07_FULL_SOURCE_CODE_DUMP_Final_v1.md`, push to `origin/main`.
   - Verify all release artifacts and logs are synced to Google Drive (`Nevermore QC & AI Consultations / 2026-11-04_Phases-8-to-11_Complete`).
   - **ENTER FINAL STANDBY:** Once deployed and pushed, do not run any further UI tests or background tasks. Wait for `bgg`.
