# [MANAGER AI DIRECTIVE — `v20_MANAGER`]: GTK THEME ISOLATION, 1PX HOT-EDGE, CLEAN 5-TAB ARCHITECTURE & QUEUE MATH

**From:** Manager AI & User (`bgg`)  
**To:** Worker AI (Antigravity)  
**Base Commit on `main`:** `17556ea`  
**Repository:** `https://github.com/BerKetKet/nevermore-sidebar`  
**Communication Log File:** `docs/NEVERMORE_COMM_LOG_v20_MANAGER.md`  

---

## Directive Tasks

==================================================================
COMMIT 1 — TASK 1: SYSTEM GTK THEME ISOLATION & 1PX TRANSPARENT HOT-EDGE FIX
==================================================================

1. Lock GTK4 Settings to Built-In "Default" Theme on Startup (crates/nevermore-ui/src/main.rs):
   - Immediately inside app.connect_activate (before creating any CssProvider or windows), lock gtk4::Settings so third-party system themes never inflate or round Nevermore surfaces:
     if let Some(settings) = gtk4::Settings::default() {
         settings.set_gtk_theme_name(Some("Default"));
         settings.set_gtk_application_prefer_dark_theme(true);
     }

2. Elevate Nevermore CSS Provider Priority Above PRIORITY_USER (crates/nevermore-ui/src/main.rs):
   - Register css_provider at gtk4::STYLE_PROVIDER_PRIORITY_USER + 100 (900) and user_provider (~/.config/nevermore/custom.css) at gtk4::STYLE_PROVIDER_PRIORITY_USER + 200 (1000) so ~/.config/gtk-4.0/gtk.css (loaded at 800) can never override Nevermore styles.

3. Harden hot_edge_window and panel_window Geometry & CSS (crates/nevermore-ui/src/main.rs & crates/nevermore-core/src/theme.rs):
   - In crates/nevermore-ui/src/main.rs:
     * Change pub const HOT_EDGE_WIDTH: i32 = 1;
     * Add hot_edge_window.add_css_class("hot-edge-window"); and hot_edge_window.set_decorated(false);
     * Add panel_window.set_decorated(false);
     * Explicitly set hot_edge_window.set_margin(Edge::Right, 0); and panel_window.set_margin(Edge::Right, 0);
     * Log the allocated width and height of hot_edge_window after map/allocate so we can verify [HOT_EDGE] Allocated geometry: window width=1.
   - In crates/nevermore-core/src/theme.rs (ThemePalette::to_gtk_css()):
     * Explicitly reset window.hot-edge-window and all its descendants so no system theme can paint a background, border, shadow, padding, or radius on it:
       window.hot-edge-window,
       window.hot-edge-window.background,
       window.hot-edge-window.csd,
       window.hot-edge-window.solid-csd,
       window.hot-edge-window > *,
       .hot-edge-strip {
           background: transparent;
           background-color: rgba(0, 0, 0, 0.0);
           border: none;
           border-width: 0px;
           border-radius: 0px;
           box-shadow: none;
           outline: none;
           margin: 0px;
           padding: 0px;
           min-width: 1px;
           min-height: 0px;
       }
     * Explicitly reset window.sidebar-panel, window.sidebar-panel.background, window.sidebar-panel.csd, window.sidebar-panel.solid-csd with:
       border-radius: 0px;
       box-shadow: none;
       outline: none;
       margin: 0px;
       padding: 0px;
       border-top: none;
       border-bottom: none;
       border-right: none;

==================================================================
COMMIT 2 — TASK 2: REMOVE GMAIL (email) & GEMINI (gemini) PLUGINS AND COMPANION BLADE
==================================================================

1. Remove Plugin Directories & CompanionBlade:
   - Run git rm -r crates/nevermore-ui/src/plugins/email and git rm -r crates/nevermore-ui/src/plugins/gemini.
   - Run git rm crates/nevermore-ui/src/companion_blade.rs and remove all companion_blade references from crates/nevermore-ui/src/main.rs, lib.rs, plugin_api.rs, and prelude.rs so no extra layer-shell window is created at startup.

2. Clean Up Canonical Tabs, Configs, & Docs:
   - Update CANONICAL_TABS in crates/nevermore-core/src/types.rs (and any unit tests in nevermore-core or nevermore-ui) to reflect the clean 5-tab set: ["terminal", "widgets", "notifications", "notepad", "music"].
   - Remove stale ~/.config/nevermore/plugins/email.toml and ~/.config/nevermore/plugins/gemini.toml if present on disk, and clear any stored email/gemini secrets from secret-tool.
   - Update README.md, docs/USER_GUIDE.md, and docs/ZERO_CONTEXT_LLM_PROMPT.md to reflect the 5 active tabs + bottom-pinned settings page.

==================================================================
COMMIT 3 — TASK 3: FIX SETTINGS AUTO-HIDE, PIN ICON STATE, & MUSIC QUEUE "PLAY NEXT" MATH
==================================================================

1. Task 3.1 — Fix Settings Tab Pointer-Leave Auto-Hide & Pin Icon State (crates/nevermore-ui/src/main.rs):
   - In panel_motion.connect_leave: remove || sm.active_tab() == "__settings__" so the unpinned Settings page auto-hides normally when the user moves the pointer back to the desktop.
   - Handle open dropdown popovers cleanly in the 250ms leave timer:
     * If ctx_timer.panel_hovered.get() is true: take leave_source and return glib::ControlFlow::Break.
     * If has_visible_popover(&ctx_timer.panel_window) is true: do NOT take leave_source yet; return glib::ControlFlow::Continue so the timer re-checks every 250ms while the dropdown popover is open (allowing connect_enter to cancel leave_source if the pointer re-enters panel_window, and automatically hiding 250ms after the popover closes if the pointer remains outside panel_window).
     * Once both panel_hovered and has_visible_popover are false: take leave_source, transition SidebarEvent::PointerLeavePanel if state != SidebarState::Pinned, call apply_timer_cb(), and return glib::ControlFlow::Break.
   - In on_theme_updated and on_appearance_updated (main.rs): check whether state_machine.lock().unwrap().state() == SidebarState::Pinned and set btn_pin's icon to "pin-off" (with pal.accent_color) when pinned vs "pin" (with pal.text_secondary) when unpinned.

2. Task 3.2 — Fix Music Queue "Play Next" Position Math (crates/nevermore-ui/src/plugins/music/mod.rs):
   - In MusicMsg::QueueUpdated: use current_pos (Option<u32>) and each song's s.pos (Option<u32>) when wiring the "Play Next" button:
     * If current_pos is Some(cur_pos) and s.pos is Some(song_pos):
       - When song_pos > cur_pos, target_pos is cur_pos + 1.
       - When song_pos < cur_pos, target_pos is cur_pos (because removing the song from before cur_pos shifts cur_pos down by 1).
       - When song_pos == cur_pos, do nothing (return early).
     * If current_pos is None, target_pos is 0.
   - In MusicMsg::SearchResultsUpdated ("Play Next" button): query c.status()?.song_pos, add the track via let id = c.add_id(&u)?, and if song_pos is Some(cur_pos), call c.playlist_move_id(id, cur_pos + 1)?.

==================================================================
COMMIT 4 — TASK 4: VERIFICATION, DEPLOYMENT, & HIDDEN RIGHT-EDGE PROOF
==================================================================

1. Run cargo fmt --all -- --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace --locked, and WAYLAND_DISPLAY=wayland-1 cargo test -p nevermore-ui --test plugin_contract (must pass 100% with the 5 active plugins: terminal, widgets, notifications, notepad, music).
2. Build release binaries (cargo build --release --workspace --locked), deploy to /home/bgg/.local/bin/, and restart nevermore.service.
3. Verify in journalctl --user -u nevermore.service that [HOT_EDGE] Allocated geometry reports window width=1.
4. Capture a full-screen 1920x1080 screenshot in Hidden state (docs/screenshots/17-hidden-right-edge-clean.png) and sample pixels across x = 1900..1919, y = 540 to prove the ~0.5 cm right-edge bar is 100% gone.
5. Verify live that opening unpinned Settings, opening a DropDown (sidebar stays visible), closing the DropDown, and moving the pointer to the desktop auto-hides the sidebar cleanly after ~250ms.
6. Commit all evidence to docs/EVIDENCE_LOG.md and docs/NEVERMORE_COMM_LOG_v21_WORKER.md and push to origin/main.
