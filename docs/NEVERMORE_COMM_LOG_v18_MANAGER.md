# [MANAGER AI DIRECTIVE — `v18_MANAGER`]: CRITICAL RUNTIME STABILITY & POPOVER FOCUS FIXES

**From:** Manager AI & User (`bgg`)  
**To:** Worker AI (Antigravity)  
**Base Commit on `main`:** `7b9b338`  
**Repository:** `https://github.com/BerKetKet/nevermore-sidebar`  
**Communication Log File:** `docs/NEVERMORE_COMM_LOG_v18_MANAGER.md`  

---

## Directive Tasks

1. **FIX 1 — ELIMINATE SIGABRT CRASH FROM STALE `glib::SourceId::remove()` CALLS ACROSS ALL MODULES:**
   - Root cause from coredump (PID 331742): In `crates/nevermore-ui/src/plugins/widgets/pane.rs`, the 50ms `vol_debounce` timeout returned `glib::ControlFlow::Break` without clearing `*p.vol_debounce.borrow_mut() = None`. The next time `audio_rx` called `a_scale.set_value(v * 100.0)`, `vol_scale.connect_value_changed` called `src.remove()` on the already-expired `SourceId`, which panics in glib 0.22 ("Source with ID ... was not found") across the C FFI boundary (`value_changed_trampoline` -> `panic_cannot_unwind` -> SIGABRT).
   - Required changes:
     * Define a universal helper in `crates/nevermore-ui/src/sdk/lifecycle.rs` (and re-export in `nevermore_ui`):
       ```rust
       pub fn safe_source_remove(id: glib::SourceId) {
           if glib::MainContext::default().find_source_by_id(&id).is_some() {
               id.remove();
           }
       }
       ```
     * Replace EVERY raw `.remove()` call on a `glib::SourceId` across the entire codebase (`crates/nevermore-ui/src/main.rs`, `plugins/widgets/pane.rs`, `plugins/notepad/pane.rs`, `plugins/music/mod.rs`, `sdk/lifecycle.rs`, `sdk/ui_kit.rs`) with `safe_source_remove(id)`.
     * In `crates/nevermore-ui/src/plugins/widgets/pane.rs`:
       - Clear `*p_timer.vol_debounce.borrow_mut() = None` inside the 50ms debounce closure before returning `glib::ControlFlow::Break`.
       - Add an `is_syncing_vol: Rc<Cell<bool>>` guard around `a_scale.set_value(v * 100.0)` in the `audio_rx` loop and check `if is_syncing_vol.get() { return; }` at the top of `vol_scale.connect_value_changed` so programmatic volume updates never trigger `vol_debounce` or spawn `wpctl`.

2. **FIX 2 — STOP SIDEBAR FROM DISAPPEARING WHEN OPENING DROPDOWNS / POPOVERS IN SETTINGS (`crates/nevermore-ui/src/main.rs` & `settings_page.rs`):**
   - Root cause:
     (a) When a user clicks any `gtk4::DropDown` in Settings (Theme, UI Font, Monospace Font, or Choice setting), GTK4 maps a child `GtkPopover` (Wayland `xdg_popup` subsurface) which grabs pointer focus, causing Wayland to emit `wl_pointer.leave` on `panel_window` (`panel_motion.connect_leave`). 250ms later, `AUTO_HIDE_DELAY_MS` fires `PointerLeavePanel` and hides `panel_window` while the user is looking at the dropdown options!
     (b) `click_inside.connect_pressed` on `panel_window` (`main.rs` line 428) unconditionally calls `apply_state()` -> `panel_window.present()` on every click even when already in `Engaged` or `Pinned` state, stealing focus from child popovers.
     (c) In `panel_motion.connect_leave` (lines 469–472) and `blade_ctrl.connect_leave` (lines 503–506), calling `ctx_timer.leave_source.borrow_mut().take()` before returning `glib::ControlFlow::Continue` leaves an untracked timer ticking every 250ms forever.
   - Required changes in `crates/nevermore-ui/src/main.rs`:
     * In `click_inside.connect_pressed`: check `let cur_state = ctx_click.state_machine.lock().unwrap().state();` and ONLY transition / call `apply_click()` when `cur_state == SidebarState::Peek || cur_state == SidebarState::Hidden`. When already in `Engaged` or `Pinned`, do NOT call `apply_state()` or `panel_window.present()` on mouse clicks.
     * Add `panel_hovered: Cell<bool>` to `UiContext` (set true in `panel_motion.connect_enter`, false in `panel_motion.connect_leave`).
     * Add a helper function `has_visible_popover(root: &impl IsA<gtk4::Widget>) -> bool` that recursively walks the widget tree (including `GtkPopover` children attached to `DropDown`/`MenuButton` widgets, or checking `panel_window.focus_widget()`) to detect any open popover, AND track `active_tab`.
     * Suppress pointer-leave auto-hide whenever:
       - `ctx_timer.panel_hovered.get()` is true, OR
       - `ctx_timer.companion_blade.is_hovered()` is true, OR
       - `ctx_timer.companion_blade.is_open()` is true, OR
       - `has_visible_popover(&ctx_timer.panel_window)` is true, OR
       - `sm.active_tab() == "__settings__"` (while the user is on the Settings page configuring dropdowns and inputs, never auto-hide on pointer leave; the user exits Settings via tab switch, Escape, or the close button).
     * Never return `glib::ControlFlow::Continue` after `.take()`ing `leave_source`; always return `glib::ControlFlow::Break` when the timer finishes.

3. **FIX 3 — AUDIT & HARDEN ALL FFI SIGNAL CLOSURES AGAINST PANICS:**
   - Audit all `connect_value_changed`, `connect_change_value`, `connect_clicked`, `connect_selected_notify`, and `connect_activate` closures in `settings_page.rs`, `widgets/pane.rs`, `notepad/pane.rs`, `notifications/pane.rs`, `terminal/pane.rs`, `music/mod.rs`, `email/mod.rs`, and `gemini/mod.rs` to ensure none of them can panic across the GTK C FFI boundary.

4. **FIX 4 — LIVE VERIFICATION & DEPLOYMENT:**
   - Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, and `WAYLAND_DISPLAY=wayland-1 cargo test -p nevermore-ui --test plugin_contract`.
   - Build the release binary (`cargo build --release --workspace --locked`), deploy to `/home/bgg/.local/bin/nevermore-ui` (and ensure `~/.config/systemd/user/nevermore.service` `ExecStart` points to the updated release binary), and restart `nevermore.service`.
   - Perform a live automated test:
     (1) Switch to the Widgets tab 5 times while adjusting volume to verify 0 SIGABRT crashes in `journalctl --user -u nevermore.service`.
     (2) Open `__settings__`, click the Theme DropDown and Font DropDowns, wait 2 seconds (> 250ms auto-hide delay), and verify `panel_window` remains visible (`nevermore status` reports `Visible: true`) and options can be selected cleanly.
