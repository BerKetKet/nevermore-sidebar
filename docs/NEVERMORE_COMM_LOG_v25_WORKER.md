# [WORKER AI VERIFICATION REPORT — `v25_WORKER`]: POPOVER LEAVE-TIMER POLLING, AUTOSTART TOGGLE, & DOCS SYNC VERIFIED

**From:** Worker AI (Antigravity)  
**To:** Manager AI & User (`bgg`)  
**Base Commit on `main`:** `f159ec1`  
**Repository:** `https://github.com/BerKetKet/nevermore-sidebar`  
**Directive Reference:** `docs/NEVERMORE_COMM_LOG_v24_MANAGER.md`  
**Verification Date:** 2026-11-08  

---

## 1. Executive Summary & Verification Highlights

All tasks outlined in Directive `v24_MANAGER` have been implemented, tested across unit and Wayland integration test suites, verified empirically through live system execution, and deployed to `nevermore.service`:

1. **Task 1 — Popover-Aware Repeating Leave-Timer:**
   Converted `panel_motion.connect_leave` in `main.rs` from a one-shot delay to an asynchronous repeating polling loop (`glib::timeout_add_local(Duration::from_millis(250), ...)`). When a GTK4 popover or popover menu is visible (`has_visible_popover` returns `true`), the closure returns `glib::ControlFlow::Continue` to maintain visibility while interacting with dropdowns. Once dismissed and pointer is confirmed outside the panel, it transitions to `SidebarState::Hidden` and breaks cleanly. Exported `has_visible_popover` in `nevermore_ui::lib` with dedicated unit test coverage.
   
2. **Task 2 — Async "Autostart at Login" Toggle with Unit Auto-Install:**
   Added an "Autostart at Login" row with `nm_field_label` and a `gtk4::Switch` to the General / System card in `settings_page.rs`. The switch initializes off-thread via `gio::spawn_blocking` querying `systemctl --user is-enabled nevermore.service`. When toggled, an asynchronous state-set handler writes `resources/systemd/nevermore.service` to `~/.config/systemd/user/nevermore.service` if missing, reloads systemd user daemon, and enables or disables the unit without killing the running process. Guarded with `is_syncing_autostart` to prevent re-entrant feedback loops.

3. **Task 3 — Documentation Sync in README.md:**
   Updated prerequisites to replace deprecated `mpv`, `yt-dlp`, and `secret-tool` with `mpd` and `mpc`. Updated CLI commands to replace obsolete `media` and `vpn` tabs with `music` and `notepad`. Updated automated contract test command to specify `WAYLAND_DISPLAY=wayland-1`.

4. **Task 4 — Verification Gate, Protocol 0.2, & Release Deployment:**
   - 100% pass on `cargo fmt --check`, `cargo clippy -D warnings`, 52/52 workspace unit tests, and 4/4 Wayland plugin contract tests.
   - Deployed release binary to `~/.local/bin/nevermore-ui` and `~/.local/bin/nevermore`.
   - Executed Protocol 0.2 (critical pre-test notification, 6s sleep, DND Inhibit lock, spectacle capture, DND release, all-clear notification) to generate `docs/screenshots/20-settings-autostart-toggle.png`.
   - Verified live service persistence during autostart enable/disable transitions.

---

## 2. Implemented Code Changes

### A. Task 1: Repeating Leave-Timer Polling Loop (`crates/nevermore-ui/src/main.rs` & `lib.rs`)
- In `crates/nevermore-ui/src/lib.rs`:
  ```rust
  pub fn has_visible_popover(root: &impl gtk4::prelude::IsA<gtk4::Widget>) -> bool {
      fn check_widget(w: &gtk4::Widget) -> bool {
          if w.is_visible() && (w.is::<gtk4::Popover>() || w.is::<gtk4::PopoverMenu>()) {
              return true;
          }
          let mut child = w.first_child();
          while let Some(c) = child {
              if check_widget(&c) {
                  return true;
              }
              child = c.next_sibling();
          }
          false
      }
      check_widget(root.as_ref())
  }
  ```
- In `crates/nevermore-ui/src/main.rs`:
  ```rust
  let source_id = glib::timeout_add_local(
      std::time::Duration::from_millis(AUTO_HIDE_DELAY_MS as u64),
      move || {
          if ctx_timer.panel_hovered.get() {
              *ctx_timer.leave_source.borrow_mut() = None;
              return glib::ControlFlow::Break;
          }
          if has_visible_popover(&ctx_timer.panel_window) {
              return glib::ControlFlow::Continue;
          }
          let mut sm = ctx_timer.state_machine.lock().unwrap();
          if sm.state() != SidebarState::Pinned {
              sm.transition(SidebarEvent::PointerLeavePanel);
              drop(sm);
              apply_timer_cb();
          }
          *ctx_timer.leave_source.borrow_mut() = None;
          glib::ControlFlow::Break
      },
  );
  *ctx_timer_leave.leave_source.borrow_mut() = Some(source_id);
  ```

### B. Task 2: Async Systemd Autostart Toggle (`crates/nevermore-ui/src/settings_page.rs`)
- Added `CARD 3: SYSTEM & GENERAL` card with switch and field label.
- Startup initialization:
  ```rust
  let is_syncing_init = is_syncing_autostart.clone();
  let sw_init_weak = sw_autostart.downgrade();
  glib::MainContext::default().spawn_local(async move {
      let is_enabled = gio::spawn_blocking(|| {
          let output = std::process::Command::new("systemctl")
              .args(["--user", "is-enabled", "nevermore.service"])
              .output();
          match output {
              Ok(out) => {
                  let text = String::from_utf8_lossy(&out.stdout);
                  let trimmed = text.trim();
                  trimmed == "enabled" || trimmed == "enabled-runtime"
              }
              Err(_) => false,
          }
      })
      .await
      .unwrap_or(false);

      if let Some(sw) = sw_init_weak.upgrade() {
          is_syncing_init.set(true);
          sw.set_active(is_enabled);
          sw.set_state(is_enabled);
          is_syncing_init.set(false);
          sw.set_sensitive(true);
      }
  });
  ```
- Asynchronous state change handler with fallback template:
  ```rust
  let is_syncing_handler = is_syncing_autostart.clone();
  sw_autostart.connect_state_set(move |sw, desired_state| {
      if is_syncing_handler.get() {
          return glib::Propagation::Proceed;
      }
      sw.set_sensitive(false);
      let sw_weak = sw.downgrade();
      let is_syncing = is_syncing_handler.clone();

      glib::MainContext::default().spawn_local(async move {
          let res = gio::spawn_blocking(move || -> Result<(), String> {
              if desired_state {
                  let user_service_dir = dirs::config_dir().join("systemd/user");
                  let user_service_file = user_service_dir.join("nevermore.service");
                  if !user_service_file.exists() {
                      let _ = fs::create_dir_all(&user_service_dir);
                      let content = include_str!("../../../resources/systemd/nevermore.service");
                      let _ = fs::write(&user_service_file, content);
                  }
                  let _ = std::process::Command::new("systemctl").args(["--user", "daemon-reload"]).status();
                  let enable_status = std::process::Command::new("systemctl").args(["--user", "enable", "nevermore.service"]).status();
                  // evaluate status...
              } else {
                  let disable_status = std::process::Command::new("systemctl").args(["--user", "disable", "nevermore.service"]).status();
                  // evaluate status...
              }
          }).await;
          // sync UI state and restore sensitivity...
      });
      glib::Propagation::Stop
  });
  ```

---

## 3. Empirical Verification Results

### A. Live Systemctl Autostart Verification
Empirical lifecycle test of `systemctl --user enable/disable nevermore.service`:
1. Initial status:
   `systemctl --user is-enabled nevermore.service` -> `disabled` (exit code 1).
2. Enable execution:
   `systemctl --user enable nevermore.service` -> `Created symlink .../graphical-session.target.wants/nevermore.service -> .../nevermore.service`.
   `is-enabled` verification -> `enabled` (exit code 0).
   `is-active` verification -> `active` (PID 387557 remained running without interruption).
3. Disable execution:
   `systemctl --user disable nevermore.service` -> `Removed .../graphical-session.target.wants/nevermore.service`.
   `is-enabled` verification -> `disabled` (exit code 1).
   `is-active` verification -> `active` (PID 387557 remained running without interruption).

### B. Screenshot Verification (`docs/screenshots/20-settings-autostart-toggle.png`)
- Protocol 0.2 executed cleanly without cursor or toast interference.
- Image resolution: `1920x1080` (PNG, 147,985 bytes).
- Pixel analysis:
  - Header: `SYSTEM & GENERAL` rendered at `y = 505..510`, `x = 1592..1907`.
  - Field label: `Autostart at Login` rendered at `y = 535..545`, `x = 1592..1750`.
  - Switch: GTK4 Switch widget rendered at `x = 1850..1896`, `y = 520..562`.

---

## 4. Test Suite Execution

All test commands passed cleanly:
```bash
cargo fmt --manifest-path Cargo.toml --all -- --check
# Result: Clean (0 diffs)

cargo clippy --workspace --all-targets -- -D warnings
# Result: Clean (0 warnings)

cargo test --workspace --locked
# Result: 52/52 tests passing (nevermore-cli: 2/2, nevermore-core: 25/25, nevermore-ui: 25/25)

WAYLAND_DISPLAY=wayland-1 cargo test -p nevermore-ui --test plugin_contract
# Result: 4/4 contract tests passing
```

---

## 5. Deployment Status

- Release binaries built: `target/release/nevermore-ui`, `target/release/nevermore`.
- Installed to: `/home/bgg/.local/bin/nevermore-ui` and `/home/bgg/.local/bin/nevermore`.
- Service status:
  ```
  ● nevermore.service - Nevermore Wayland Right-Edge Sidebar
       Loaded: loaded (/home/bgg/.config/systemd/user/nevermore.service; disabled; preset: enabled)
       Active: active (running) since Sun 2026-11-08 05:33:42 EAT; Main PID: 387557
  ```
- Commit history on `main`:
  1. `011e32a` — `fix(ui): convert leave-timer to popover-aware repeating polling loop`
  2. `60a2d6e` — `feat(settings): add async systemd autostart toggle with unit auto-install`
  3. `f159ec1` — `docs: sync README prerequisites, CLI commands, and test invocations`
  4. Final deployment & proof commit staged.
