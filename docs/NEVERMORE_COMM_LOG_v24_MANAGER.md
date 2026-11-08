# NEVERMORE SIDEBAR — DIRECTIVE v24_MANAGER: POPOVER LEAVE-TIMER POLLING, AUTOSTART TOGGLE, & DOCS SYNC

## 0. INVARIANTS & REPOSITORY RULES
- Repository: /home/bgg/projects/nevermore-sidebar (branch: main)
- Run all commands via `bash /home/bgg/projects/nevermore-sidebar/scripts/autorun.sh` to avoid permission popups.
- Pinned toolchain and workspace crates only. Never add unpinned dependencies.
- Commit atomically on main with standard conventional commit messages (one commit per task).

---

## 1. TASK 1: CONVERT `leave_source` IN `main.rs` TO POPOVER-AWARE REPEATING TIMER

Edit `crates/nevermore-ui/src/main.rs`:
1. In `panel_motion.connect_leave`:
   - Replace `glib::timeout_add_local_once` with `glib::timeout_add_local(Duration::from_millis(250), move || { ... })`.
   - Store the returned `SourceId` into `*ctx_timer.leave_source.borrow_mut() = Some(source_id)`.
   - Inside the repeating timer closure:
     a. If `ctx_timer.panel_hovered.get()` is true:
        `*ctx_timer.leave_source.borrow_mut() = None;`
        return `glib::ControlFlow::Break;`
     b. If `has_visible_popover(&ctx_timer.panel_window)` is true:
        return `glib::ControlFlow::Continue;` (keep polling every 250ms).
     c. Otherwise (pointer outside panel and no popovers active):
        if `sm.state() != SidebarState::Pinned` {
            sm.transition(SidebarState::Hidden);
            apply_state(&ctx_timer);
        }
        `*ctx_timer.leave_source.borrow_mut() = None;`
        return `glib::ControlFlow::Break;`
2. In `panel_motion.connect_enter`:
   - Set `ctx.panel_hovered.set(true);`
   - If `let Some(source_id) = ctx.leave_source.borrow_mut().take()` {
       safe_source_remove(source_id);
     }
3. Commit as: `fix(ui): convert leave-timer to popover-aware repeating polling loop`

---

## 2. TASK 2: IMPLEMENT "AUTOSTART AT LOGIN" TOGGLE IN `settings_page.rs`

1. **Verify Unit File:**
   Ensure `resources/systemd/nevermore.service` contains:
   ```ini
   [Unit]
   Description=Nevermore Wayland Right-Edge Sidebar
   PartOf=graphical-session.target
   After=graphical-session.target

   [Service]
   Type=simple
   ExecStart=%h/.local/bin/nevermore-ui
   Restart=on-failure
   RestartSec=1s

   [Install]
   WantedBy=graphical-session.target
   ```
2. **Settings UI Row (`crates/nevermore-ui/src/settings_page.rs`):**
   - Inside the General / System section of `settings_page.rs`:
   - Create an "Autostart at Login" row with `nm_field_label("Autostart at Login")` and a `gtk4::Switch`.
   - Create `let is_syncing_autostart = Rc::new(Cell::new(false));`
   - Set switch insensitive initially: `sw_autostart.set_sensitive(false);`
3. **Async State Query on Startup:**
   - Spawn off-thread query via `gio::spawn_blocking`:
   - Run `systemctl --user is-enabled nevermore.service`.
   - Check exit status and stdout. If output trimmed is `"enabled"` or `"enabled-runtime"`, result is `true`, else `false`.
   - On completion, dispatch to `glib::MainContext::default().spawn_local`:
     - `is_syncing_autostart.set(true);`
     - `sw_autostart.set_active(is_enabled);`
     - `sw_autostart.set_state(is_enabled);`
     - `is_syncing_autostart.set(false);`
     - `sw_autostart.set_sensitive(true);`
4. **Async State-Set Handler:**
   - Connect via `sw_autostart.connect_state_set`:
   - If `is_syncing_autostart.get()` is true, return `glib::Propagation::Proceed`.
   - Disable switch: `sw.set_sensitive(false);`
   - Capture `desired_state`.
   - In `gio::spawn_blocking`:
     - If `desired_state == true`:
       - Ensure `~/.config/systemd/user/nevermore.service` exists. If not, write it from `resources/systemd/nevermore.service`.
       - Execute `systemctl --user daemon-reload`.
       - Execute `systemctl --user enable nevermore.service` (do NOT pass `--now`).
     - If `desired_state == false`:
       - Execute `systemctl --user disable nevermore.service` (do NOT pass `--now`).
   - On completion in `glib::MainContext::default().spawn_local`:
     - If command succeeded:
       - `is_syncing_autostart.set(true);`
       - `sw.set_state(desired_state);`
       - `is_syncing_autostart.set(false);`
     - If command failed:
       - Log error to stderr.
       - `is_syncing_autostart.set(true);`
       - `sw.set_active(!desired_state);`
       - `sw.set_state(!desired_state);`
       - `is_syncing_autostart.set(false);`
     - Restore sensitivity: `sw.set_sensitive(true);`
     - Return `glib::Propagation::Stop`.
5. Commit as: `feat(settings): add async systemd autostart toggle with unit auto-install`

---

## 3. TASK 3: SYNC README.md DOCUMENTATION
In `README.md`:
- Under Prerequisites, replace `mpv`, `yt-dlp`, and `secret-tool` with `mpd` and `mpc`.
- Under CLI Control, replace `nevermore show media` and `nevermore show vpn` with `nevermore show music` and `nevermore show notepad`.
- Under 5. Verification & Testing, update the contract test command to:
  `WAYLAND_DISPLAY=wayland-1 cargo test -p nevermore-ui --test plugin_contract`
- Commit as: `docs: sync README prerequisites, CLI commands, and test invocations`

---

## 4. VERIFICATION GATE & EMPIRICAL TESTING
1. Run formatting, lints, and test suite:
   - `cargo fmt --manifest-path Cargo.toml --all -- --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace --locked`
   - `WAYLAND_DISPLAY=wayland-1 cargo test -p nevermore-ui --test plugin_contract`
2. Build and deploy release binaries:
   - `cargo build --release --workspace`
   - `cp target/release/nevermore-ui ~/.local/bin/`
   - `cp target/release/nevermore ~/.local/bin/`
   - `systemctl --user restart nevermore.service`
3. Live Functional Testing:
   - Open Settings dropdown (e.g. font or theme selector), move cursor outside the 400px panel, dismiss the dropdown. Confirm the sidebar auto-hides ~250ms after the popover closes.
   - Toggle "Autostart at Login" switch ON. Verify `systemctl --user is-enabled nevermore.service` returns `enabled` and `nevermore-ui` remains running.
   - Toggle switch OFF. Verify `systemctl --user is-enabled nevermore.service` returns `disabled` and `nevermore-ui` remains running.
4. Pre-Test Notification Protocol 0.2:
   - Run: `notify-send -u critical -t 8000 "Nevermore Worker AI" "Running live UI/screenshot test in 5 seconds — please pause mouse/keyboard for ~10s"`
   - Sleep 6 seconds.
   - Enable DND via DBus Inhibit.
   - Open Settings page and capture `docs/screenshots/20-settings-autostart-toggle.png` with `spectacle -b -n -o docs/screenshots/20-settings-autostart-toggle.png`.
   - Release DND and send all-clear notification.
5. Record empirical verification in `docs/EVIDENCE_LOG.md` and write `docs/NEVERMORE_COMM_LOG_v25_WORKER.md`.
6. Commit deployment and proof artifacts as: `test,deploy: verify repeating leave-timer, live autostart toggle, and deploy v24 release`
7. Push all commits to `origin/main`.
