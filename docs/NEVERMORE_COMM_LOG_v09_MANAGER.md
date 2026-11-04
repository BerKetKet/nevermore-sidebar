# [MANAGER AI DIRECTIVE / SIGN-OFF — `v09_MANAGER`]: PHASE 7.6 FORMAL VERIFICATION & SIGN-OFF (ZERO-CONTEXT PLUGIN SDK & THEME ENGINE)

**From:** Manager AI (Google AI Studio)  
**To:** Worker AI (Antigravity), User (`bgg`), QC Partner AI (Claude)  
**Audited State on `main`:** `main @ d6b806d` / `5f07a5d`  
**Communication Channel:** Google AI Studio (`https://aistudio.google.com/prompts/13YvHFFS5vO79vKlLp_BV7TRIryaj_r2O`) via DevTools Bridge Daemon  

---

## 1. Live Verification of `main` on GitHub (Phase 7.6 Complete)

From `docs/EVIDENCE_LOG.md`, `crates/nevermore-cli/src/main.rs`, and `crates/nevermore-ui/src/main.rs` on `main`:

1. **Hotfix 7.6.0 (`Layer::Top` Restored):**
   - Both `hot_edge_window.set_layer(Layer::Top)` and `panel_window.set_layer(Layer::Top)` are restored on `main` (with `set_exclusive_zone(-1)`), protecting fullscreen games.

2. **Zero-Context Plugin SDK (`crates/nevermore-ui/src/sdk/`):**
   - Implemented `use nevermore_ui::prelude::*;`, `PluginSpec`, `Tone`, `KeepOnHide`, `Tx<Msg>` (`ctx.mailbox`), `Cmd` builder (with process-group isolation and watchdog timeout), `PluginLifecycle` (auto-starting/stopping `ctx.every` timers on show/hide), and all 12 UiKit builders.

3. **`build.rs` Auto-Registration & `_template`:**
   - `crates/nevermore-ui/build.rs` scans `src/plugins/*/mod.rs` for `pub const SPEC: PluginSpec` and auto-generates `$OUT_DIR/plugins_gen.rs`—zero manual edits to `plugins/mod.rs` needed.
   - Converted `media`, `gemini`, `youtube`, `vpn`, `email`, and `_template/mod.rs` (~40 lines) to the new `SPEC` + `build(ctx)` SDK.

4. **Headless Contract Test Harness (`tests/plugin_contract.rs`):**
   - `DISPLAY="" WAYLAND_DISPLAY="" cargo test -p nevermore-ui --test plugin_contract` passes 5/5 contract checks in 0.00s (static source linter blocking raw `Command::new`/`timeout_add`/`tokio`/emojis, manifest & icon check, widget tree `.nm-data` walk, lifecycle timer leak check, and `ZERO_CONTEXT_LLM_PROMPT.md` sync check).
   - 41/41 total workspace tests passing with 0 clippy warnings.

5. **OKLab `[seed]` Theme Engine & CLI Validator:**
   - `nevermore theme check <path> [--json]` and `nevermore theme expand <path>` are live in `nevermore-cli`.

6. **1-Page Prompt Generated:**
   - The Worker AI generated `/home/bgg/projects/nevermore-sidebar/docs/ZERO_CONTEXT_LLM_PROMPT.md` (and uploaded `NEVERMORE_COMM_LOG_v08_WORKER.md` + `ZERO_CONTEXT_LLM_PROMPT.md` to your Google Drive folder `Nevermore QC & AI Consultations` / `2026-11-04_Phase-7_6_SDK`).

---

## 2. Ready for Live Experiment!

Everything is in place to test zero-context plugin and theme generation.
