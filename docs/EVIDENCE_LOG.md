# Nevermore Verification Evidence Log

This log records verifiable empirical evidence for all claims, checks, benchmarks, and tests in accordance with the Supreme Rule: **Verified Beats Claimed**.
All entries must record the exact command run, timestamp, stdout/stderr, measurement, or visual artifact path.

---

## Log Entries

| Date | Claim/Check | Command Run | Raw Output / Measurement / Screenshot Path | Status |
| :--- | :--- | :--- | :--- | :--- |
| 2026-11-03 11:28 | Antigravity CLI version check | `agy --version` | `1.1.5` | VERIFIED |
| 2026-11-03 11:28 | Antigravity CLI flags & subcommands | `agy --help < /dev/null` | Subcommands: `agent`, `agents`, `changelog`, `help`, `install`, `models`, `plugin`, `plugins`, `update`. Flags: `--add-dir`, `--agent`, `-c / --continue`, `--conversation`, `--dangerously-skip-permissions`, `--effort`, `-i / --prompt-interactive`, `--log-file`, `--mode`, `--model`, `--new-project`, `-p / --print`, `--print-timeout`, `--project`, `--prompt`, `--sandbox` | VERIFIED |
| 2026-11-03 11:28 | Subagent tool names audit | Inspected `~/.gemini/config/plugins/firebase/agents/firestore-rules-author.md` & `~/.gemini/config/plugins/flutter/agents/a11y_agent.md` | Verified tool names: `view_file`, `write_to_file`, `replace_file_content`, `multi_replace_file_content`, `grep_search`, `find_by_name`, `list_dir`, `run_command`, `send_message`, `schedule`, `manage_task`, `read_url_content`, `search_web`, `invoke_subagent`, `manage_subagents`, `ask_question` | VERIFIED |
| 2026-11-03 11:28 | `invoke_subagent` context modes | Inspected `invoke_subagent` tool schema | Supported modes in `Workspace` argument: `inherit` (default), `branch` (isolated worktree), `share` (shared repo dir) | VERIFIED |
| 2026-11-03 11:28 | Lifecycle hooks & sandbox support | Inspected `~/.gemini/config/config.json` & `/home/bgg/.gemini/antigravity/builtin/skills/agy-customizations/docs/hooks.md` | Hooks supported via `hooks.json` (`SessionStart`, `PreToolUse`, `PostToolUse`, `PreInvocation`, `PostInvocation`, `Stop`). Terminal sandbox flag in config is `"enableTerminalSandbox": false`. | VERIFIED |
| 2026-11-03 11:30 | Legacy repository read-only audit | Inspected `/home/bgg/projects/nevermore` (`nevermore-tmux.conf`, `nevermore-run`, `nevermore-raise`, `docs/spikes/terminal.md`) | Documented patterns and anti-patterns in `docs/LEGACY_LESSONS.md` on `main`. | VERIFIED |
| 2026-11-03 11:31 | System package dependency audit | `pacman -Qi kwin plasma-workspace wayland gtk4 gtk4-layer-shell vte4 tmux mpv yt-dlp cloudflare-warp-bin networkmanager intel-media-driver mesa` | Audited all 13 packages in `DEPENDENCIES.md`. Identified `vte4` is not yet installed (`vte4 0.84.1-1.1` in repos; install via `sudo pacman -S vte4`). | VERIFIED |
| 2026-11-03 11:32 | Rust compiler version check | `rustc --version` | `rustc 1.98.0 (88d9e12ae 2026-08-18)` | VERIFIED |
| 2026-11-03 11:36 | Cargo workspace build verification | `cargo build --workspace --verbose` | Compiling `nevermore-core v0.1.0`, `nevermore-ui v0.1.0`, `nevermore-cli v0.1.0`. Finished `dev` profile [unoptimized + debuginfo] target(s) in 24.82s. Exit code 0. | VERIFIED |
| 2026-11-03 11:36 | Automated unit tests verification | `cargo test --workspace --verbose` | 7 tests run, 7 passed, 0 failed: `test_cli_parsing` ok, `test_cli_toggle_behavior` ok, `test_click_inside_engages_keyboard_focus` ok, `test_edge_hover_leads_to_peek` ok, `test_initial_state_is_hidden` ok, `test_pinned_state_overrides_pointer_leave` ok, `test_tab_switching_on_show` ok. Finished in 0.70s. Exit code 0. | VERIFIED |
| 2026-11-03 11:36 | Rust code formatting verification | `cargo fmt --all -- --check` | Clean, 0 formatting errors. Exit code 0. | VERIFIED |
| 2026-11-03 11:36 | Clippy linter verification | `cargo clippy --workspace --all-targets -- -D warnings` | Clean, 0 warnings. Exit code 0. | VERIFIED |
