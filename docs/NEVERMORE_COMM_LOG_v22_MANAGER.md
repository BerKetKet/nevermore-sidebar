# NEVERMORE SIDEBAR — DIRECTIVE v22_MANAGER: HOTFIX PANEL VISIBILITY & EDGE HOVER

## 1. FAILURE REPORT & CONTEXT
Following Directive v20_MANAGER (commits f1db1b8..a131b5e), two regressions were observed by the human architect on CachyOS / KWin 6.8.2 Wayland:
1. Right-edge hover no longer triggers the sidebar into Peek mode.
2. Meta + N (which triggers `nevermore toggle` / `nevermore show`) logs `[IPC] Show executed -> Engaged`, but the sidebar remains completely invisible on screen.

## 2. ROOT CAUSE ANALYSIS & MANDATED FIXES

### Task 1: Restore Panel Window Opacity & GTK4 Theme Health
- Inspect `crates/nevermore-core/src/theme.rs` and `crates/nevermore-ui/src/main.rs`.
- In commit f1db1b8, CSS resets applied `background: transparent`, `border: none`, etc. to `window.sidebar-panel`. This stripped the opaque surface background of the sidebar, causing the 400px panel to map as 100% invisible pixels over the desktop.
- Mandate:
  - `window.sidebar-panel`, `box.sidebar-panel`, and the root content containers MUST have their solid, opaque background fill explicitly defined using the active theme surface color (`var(--nm-bg-surface)` or generated OKLab background `#121316`).
  - The transparent reset (`background: transparent !important; border: none !important; box-shadow: none !important;`) must ONLY apply to `window.hot-edge-window`.
  - In `main.rs`, fix `set_gtk_theme_name`: GTK4 does not have a "Default" theme (that is a GTK3 name). Use "Adwaita" or leave the theme name unset while setting `set_gtk_application_prefer_dark_theme(true)` and relying on Nevermore's high-priority CSS providers (900/1000) to isolate styling.

### Task 2: Restore Hot-Edge Pointer Detection under KWin Wayland
- In commit f1db1b8, `HOT_EDGE_WIDTH` was set to 1. In KWin 6.8.2 Wayland, a 1px window at screen edge x=1919 suffers from compositor edge barrier / pointer clamping, missing pointer enter/motion signals entirely.
- Mandate:
  - Increase `HOT_EDGE_WIDTH` to 2px or 3px.
  - Ensure `window.hot-edge-window` has zero margins, zero borders, zero box-shadow, and `background: transparent !important`.
  - Verify with a live pixel test that the 2-3px transparent window does NOT draw any visible line or dark bar on the desktop wallpaper.
  - Verify that moving the mouse to the right edge reliably fires the enter event and transitions the state machine to Peek (mapping the panel).

### Task 3: Verify Auto-Hide & Popover Guards
- Confirm that when the sidebar is Engaged via `nevermore show` or `Meta + N`:
  - The panel maps and is fully visible and opaque.
  - If unpinned, leaving the panel window triggers the 250ms grace timer and hides the panel, UNLESS a GtkPopover is open (`has_visible_popover`).
  - If pinned, the panel remains visible indefinitely.

## 3. VERIFICATION PROTOCOL (PRE-TEST NOTIFICATION 0.2)
1. Run full cargo suite via `/home/bgg/projects/nevermore-sidebar/scripts/autorun.sh`:
   - `cargo fmt --manifest-path Cargo.toml --all -- --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace --locked`
   - `WAYLAND_DISPLAY=wayland-1 cargo test -p nevermore-ui --test plugin_contract`
2. Deploy release binaries:
   - `cargo build --release --workspace`
   - `cp target/release/nevermore-ui ~/.local/bin/`
   - `cp target/release/nevermore ~/.local/bin/`
   - `systemctl --user restart nevermore.service`
3. Execute Pre-Test Notification Protocol 0.2 before capturing live screenshots:
   - Send critical notification: `notify-send -u critical -t 8000 "Nevermore Worker AI" "Running live UI/screenshot test in 5 seconds — please pause mouse/keyboard for ~10s"`
   - Sleep 6 seconds.
   - Enable DND (`gdbus call --session --dest org.freedesktop.Notifications --object-path /org/freedesktop/Notifications --method org.freedesktop.Notifications.Inhibit "Nevermore" "Screenshot" {}`).
   - Run `nevermore show` and capture `docs/screenshots/18-panel-engaged-visible.png` with `spectacle -b -n -o docs/screenshots/18-panel-engaged-visible.png`.
   - Release DND and send all-clear notification.
4. Pixel Sample: Verify `docs/screenshots/18-panel-engaged-visible.png` has solid RGB values inside x=1520..1919 (the 400px sidebar panel is visibly rendered and opaque).
5. Document evidence in `docs/EVIDENCE_LOG.md` and write `docs/NEVERMORE_COMM_LOG_v23_WORKER.md`.
6. Commit all changes atomically on `main` and push to `origin/main`.
