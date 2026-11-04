# [MANAGER AI DIRECTIVE — `v09_MANAGER`]: PHASE 7.6 APPROVED → AUTONOMOUS EXECUTION OF PHASES 8, 9, 11, AND 10 USING THE NEW ZERO-CONTEXT SDK

**From:** Manager AI & User (`bgg`)  
**To:** Worker AI (Antigravity)  
**Current Repo State:** Phase 7.6 Verified on `main` (41/41 workspace tests passing, 5/5 `plugin_contract` tests passing, `Layer::Top` restored)  
**Repository:** `https://github.com/BerKetKet/nevermore-sidebar`  
**Communication Log File:** `docs/NEVERMORE_COMM_LOG_v09_MANAGER.md`

---

## 0. AUTONOMOUS OPERATING MODE WHILE USER IS AWAY (NO QC PAUSES)

1. **Phase 7.6 Sign-Off:** Phase 7.6 (`nevermore_ui::prelude::*`, `PluginSpec`, `build.rs` auto-discovery, 12 `UiKit` builders, `tests/plugin_contract.rs`, OKLab `[seed]` theme engine, and `Layer::Top` restoration) is **APPROVED**.
2. **Continuous Autonomous Execution (Do Not Pause for QC):**
   - `bgg` is stepping away and has instructed us to **skip external QC pauses** and advance the project continuously on `main` through **Phase 8 (Media, Gemini, YouTube)**, **Phase 9 (VPN)**, **Phase 11 (Email Spike + Tab)**, and **Phase 10 (Hardening, Profiling & User Docs)**.
   - Commit and `git push origin main` after each phase (recording the exact code and docs commit SHAs in `docs/EVIDENCE_LOG.md`), and log any assumed choices under **"Provisional Decisions Taken While Away (Pending User Final Sign-Off)"**.
   - **Every single plugin MUST be written using the new Phase 7.6 Zero-Context SDK (`use crate::prelude::*;`, `pub const SPEC: PluginSpec`, `pub fn build(ctx: &PluginContext) -> gtk4::Widget`) and MUST pass `DISPLAY="" WAYLAND_DISPLAY="" cargo test -p nevermore-ui --test plugin_contract`!**
3. **Keep Pre-Test Notification Protocol 0.2 + DND Toast Lock:**
   - Before any live UI/screenshot test, send the 5-second `notify-send` warning, sleep 6 seconds, enable DND (`Inhibit`) so no desktop toast photobombs the screenshot, run the test, disable DND, and send the all-clear notification.

---

## 1. PHASE 8: LOCAL MEDIA PLAYER, GEMINI CHAT, & YOUTUBE PLUGINS

### 1.1 Pre-Build Spikes (`docs/spikes/`)
1. **Spike 10 — Intel iGPU VA-API Hardware Decode (`docs/spikes/10-vaapi-media.md`):**
   - Verify `mpv --hwdec=vaapi --vo=gpu` (and audio/video IPC control via `--input-ipc-server=$XDG_RUNTIME_DIR/nevermore/mpv.sock`) on `bgg`'s Intel Iris Plus 655 iGPU (`intel-media-driver 26.3.5`). Never touch the discrete NVIDIA GPU. Confirm `Using hardware decoding (vaapi)` in logs and record in `docs/EVIDENCE_LOG.md`.
2. **Spike 11 — Secret Service Keyring Roundtrip (`docs/spikes/11-secret-service.md`):**
   - Verify storing and looking up a test key via `ctx.secret_set` / `ctx.secret_get` (`secret-tool` with secret passed via stdin) on the live session bus, then delete the test key. Confirm zero plaintext key leakage in files or `ps` argv.

### 1.2 Plugin 1: Local Media Player (`crates/nevermore-ui/src/plugins/media/mod.rs`)
Build using `use crate::prelude::*;` (`SPEC` + `build(ctx)`):
- **State Persistence (`ctx.load_json` / `ctx.save_json`):** Persist `MediaState { playlist: Vec<String>, current_idx: usize, position_sec: f64, volume: u32, speed: f64 }` in `playback_state.json` so track, playlist, and timestamp resume across UI restarts.
- **UI (`UiKit` Only, Zero Non-Data Text):**
  - Search/filter or directory scan bar (`nm_search_entry`) that scans `~/Music` and `~/Videos` (or entered directory/file path) via `ctx.run` for `.mp3`, `.flac`, `.wav`, `.ogg`, `.mp4`, `.mkv`, `.webm` files.
  - Now-playing `nm_card`: status dot (`Tone::Success` when playing, `Tone::Muted` when paused/idle), track filename (`nm_data_label`), progress/position row (`nm_metric_row("music", Tone::Accent)` showing `MM:SS / MM:SS`), and transport `nm_toolbar` with `nm_icon_button`s (`"skip-back"`, `"play"`/`"pause"` toggle, `"skip-forward"`, `"volume-2"`/`"volume-x"`, and speed cycle).
  - Playlist `nm_list` of tracks with play/remove `nm_icon_button` actions (`nm_empty_state("music", "No media files found")` when empty).
- **Backend:** Control `mpv --no-terminal --hwdec=vaapi --input-ipc-server=...` via `ctx.daemon(..., KeepOnHide)` / `ctx.run` so hardware decoding is used and position is polled every `2s` via `ctx.every` when visible.

### 1.3 Plugin 2: Gemini AI Chat (`crates/nevermore-ui/src/plugins/gemini/mod.rs`)
Build using `use crate::prelude::*;` (`SPEC` + `build(ctx)`):
- **Secret Service Key Management:**
  - Use `ctx.secret_get("gemini_api_key", ...)` on startup and `ctx.secret_set("gemini_api_key", ...)` when the user enters a key into the key input bar (`nm_search_entry` or password entry triggered by a `"key"` / `"layers"` icon button).
  - Show key status via `nm_status_dot` (`Tone::Success` when key is stored in Secret Service, `Tone::Warning` when missing).
- **Model Selector & History Persistence:**
  - Model toggle via `nm_segmented` or `nm_toggle_icon_button` + `nm_data_label` (`gemini-2.5-flash` vs `gemini-2.5-pro`).
  - Persist conversation history (`Vec<ChatMessage>`) atomically via `ctx.load_json("chat_history.json")` / `ctx.save_json("chat_history.json", &history)`.
  - Toolbar buttons (`"plus"` for New Chat, `"trash-2"` for Clear History).
- **Non-Blocking API Execution (Zero Key in `argv`):**
  - Send requests to `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent` using `ctx.run(Cmd::new("curl").arg("-sS").arg("-K").arg("-").stdin_bytes(...).timeout(Duration::from_secs(30)), ...)` so the `x-goog-api-key` header and JSON payload are passed over **stdin** (never visible in `ps aux`!).
  - Render messages inside `nm_card`s with `.nm-data` selectable/wrappable labels and a copy-to-clipboard icon button.

### 1.4 Plugin 3: YouTube Search & Player (`crates/nevermore-ui/src/plugins/youtube/mod.rs`)
Build using `use crate::prelude::*;` (`SPEC` + `build(ctx)`):
- **Isolated `yt-dlp` Binary Policy:**
  - Manage `~/.local/share/nevermore/bin/yt-dlp` (copying `/usr/bin/yt-dlp` on first run if absent).
  - Include toolbar `nm_icon_button`s to **Update Extractor** (downloads latest official `yt-dlp` release binary to `.new`, backs up current to `.prev`, atomically renames, and updates version `nm_data_label`) and **Rollback Extractor** (restores `.prev`).
- **Search & Hardware-Decoded Playback:**
  - Search/URL bar (`nm_search_entry`) that runs `~/.local/share/nevermore/bin/yt-dlp "ytsearch6:<query>" --flat-playlist --dump-json` via `ctx.run`.
  - Quality selector (`Audio Only`, `720p`, `1080p`) and result cards (`nm_list_row` showing video title & duration as `.nm-data` + play button) launching playback via `mpv --hwdec=vaapi --ytdl-raw-options=...`.

---

## 2. PHASE 9: VPN TAB PLUGIN (`crates/nevermore-ui/src/plugins/vpn/mod.rs`)

1. **Spike 12 — Cloudflare WARP & NetworkManager WireGuard (`docs/spikes/12-vpn-control.md`):**
   - Inspect `warp-cli status` and `warp-cli settings` on `bgg`'s machine. Record the current WARP mode and verify that `warp-cli connect` / `warp-cli disconnect` never alters the configured mode.
   - Inspect `nmcli -t -f NAME,UUID,TYPE,DEVICE,STATE connection show` for WireGuard/VPN profiles.
2. **Implement `crates/nevermore-ui/src/plugins/vpn/mod.rs` using `use crate::prelude::*;`:**
   - **Card 1 (Cloudflare WARP):** `nm_status_dot` (`Tone::Success` when Connected, `Tone::Warning` when Connecting, `Tone::Muted` when Disconnected), `nm_data_label` showing live WARP status & IP/mode, and `nm_toggle_icon_button` (`"vpn"`, `"vpn"`, `"Connect / Disconnect Cloudflare WARP"`) calling `warp-cli connect` / `warp-cli disconnect` via `ctx.run`.
   - **Card 2 (WireGuard / NetworkManager Profiles):** Lists discovered `wireguard` / `vpn` connections from `nmcli` using `nm_list_row` with a connect/disconnect `nm_toggle_icon_button` (`nmcli connection up <name>` / `down <name>`), or `nm_empty_state("vpn", "No WireGuard profiles configured")` if none exist.
   - Poll every `5s` while visible via `ctx.every(Duration::from_secs(5), ...)`.
   - Author `docs/VPN_SETUP_GUIDE.md` explaining how to import a WireGuard `.conf` into NetworkManager (`nmcli connection import type wireguard file <path>`).

---

## 3. PHASE 11: EMAIL FEASIBILITY SPIKE & EMAIL PLUGIN (`crates/nevermore-ui/src/plugins/email/mod.rs`)

1. **Spike 13 — Email Architecture Evaluation (`docs/spikes/13-email-feasibility.md`):**
   - Evaluate the 4 options from the Manager Brief: (A) Thunderbird integration, (B) `mbsync` + `notmuch` / `himalaya` CLI, (C) Rust read-only IMAP IDLE summary, (D) Akonadi.
   - Compare RAM/CPU footprint, setup complexity, credential safety (Secret Service), and AI maintainability, and record your recommended choice under **Provisional Decisions**.
2. **Implement `crates/nevermore-ui/src/plugins/email/mod.rs` using `use crate::prelude::*;`:**
   - Build a clean, zero-overhead Email Overview & Quick-Raise plugin that checks for local `himalaya` / unread mail summaries (configured via `ctx.config_dir/account.toml` or Secret Service) and provides one-click window raise/launch for the user's desktop mail client (`thunderbird` / `kmail` / webmail) plus unread message preview cards (`nm_list_row` / `nm_empty_state("email", "No unread messages or account configured")`).

---

## 4. PHASE 10: SYSTEM HARDENING, SOAK VERIFICATION, & USER DOCUMENTATION

Once all 9 tabs (`terminal`, `widgets`, `notifications`, `notepad`, `media`, `gemini`, `youtube`, `vpn`, `email`) are live and implemented:
1. **Contract & Static Gates:**
   - Run `DISPLAY="" WAYLAND_DISPLAY="" cargo test -p nevermore-ui --test plugin_contract` (must pass 100% across all plugins!).
   - Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --locked`.
2. **Crash Survival, Leak & Performance Soak Audit:**
   - Populate state across the sidebar (3 tmux terminal pages, notepad text, media playlist state, gemini chat history, custom theme), run `kill -9 $(pidof nevermore-ui)`, let `systemd --user` restart `nevermore.service`, and verify 100% state survival and 0 orphan/zombie processes (`ps aux | grep defunct`, `pgrep -a dbus-monitor`).
   - Measure and log in `docs/EVIDENCE_LOG.md`:
     - Warm `nevermore show` / `nevermore hide` latency (must be `≤ 100 ms`).
     - Hidden idle CPU % over 30 seconds (`0.0%`, with 0 active plugin timers when `Hidden`).
     - Resident memory (RSS in MB) and open FD count (`/proc/$(pidof nevermore-ui)/fd | wc -l`) after cycling through all 9 tabs.
3. **Toast-Free Visual & Contrast Proof of All New Tabs:**
   - Using Protocol 0.2 (5s `notify-send` warning -> sleep 6s -> DND `Inhibit` ON -> `spectacle` -> DND OFF -> all-clear notification), capture:
     - `docs/spikes/08-tab-media.png`
     - `docs/spikes/08-tab-gemini.png`
     - `docs/spikes/08-tab-youtube.png`
     - `docs/spikes/09-tab-vpn.png`
     - `docs/spikes/11-tab-email.png`
   - Run `python3 scripts/verify_icon_contrast.py` and verify 400px locked width (`x = 1520..1920`) on every tab.
4. **Final Documentation & Release Bundle:**
   - Write `README.md` and `docs/USER_GUIDE.md` covering CLI commands, hotkeys, theming (`nevermore theme set/check/expand`), plugin authoring (`docs/ZERO_CONTEXT_LLM_PROMPT.md`), and VPN/Email setup.
   - Commit code first, record exact SHA, commit docs & final dump (`docs/07_FULL_SOURCE_CODE_DUMP_Final_v1.md`) and `docs/NEVERMORE_COMM_LOG_v10_WORKER.md`, push to `origin/main`, and upload to Google Drive (`Nevermore QC & AI Consultations / 2026-11-04_Phases-8-to-11_Complete`).

Have a great time while you're away—whenever the Worker AI reports back in this thread (or pushes its phase commits to GitHub), I will review its work and keep guiding it all the way to completion!

thumb_up
thumb_down
