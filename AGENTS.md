# NEVERMORE AGENT PROTOCOL & OPERATING MANDATE

## Role & Mandate
You are working on **Nevermore**, a standalone, always-running right-edge Wayland sidebar for KDE Plasma 6 (CachyOS / Linux, Wayland compositor) written in Rust.
The legacy Plasma plasmoid repository (`https://github.com/berketket/nevermore`) is permanently frozen as a read-only reference.

---

## 1. Supreme Rules (Immutable)

1. **The User's Decisions Are Immutable.**
   You may NEVER change, replace, or quietly work around any fixed decision. If an experiment or spike shows a fixed decision is infeasible, you must STOP, record the exact command output and evidence, present at least two real options with trade-offs (prioritizing: 1. snappy and convenient, 2. hard to break, 3. easy for AI agents to write and maintain), and wait for the user's explicit decision. Decision records written by agents are **proposals only** until the user marks them accepted.

2. **Single Unchanging Command Execution Protocol:**
   To maintain seamless unattended execution without repeated authorization popups, all terminal executions MUST be written to `/home/bgg/projects/nevermore-sidebar/scripts/autorun.sh` and executed exclusively with the single unchanging command string:
   `bash /home/bgg/projects/nevermore-sidebar/scripts/autorun.sh`
   Never use ad-hoc command strings, chaining (`&&`), or arguments in `run_command`.

2. **Hard Prohibitions (Zero Exceptions):**
   - NEVER put the terminal in a separate, floating, popped-out, or docked external window.
   - NEVER add a "Show Terminal" or "Pop out" button.
   - NEVER launch Konsole or any external terminal emulator.
   - NEVER manage external terminal windows via KWin scripts.
   - NEVER use Qt or QML (this project is strictly Rust).
   - NEVER use transparency/blur effects or light themes (solid dark surfaces only: `#18181b` background, `#0a84ff` blue accent, red for destructive actions).
   - NEVER replace or take over KDE Plasma's notification daemon.
   - NEVER touch, test on, or target the discrete NVIDIA Quadro P2000 GPU (Intel iGPU Iris Plus 655 only).
   - NEVER build or plan around the password manager (KeePassXC) or authenticator (both are on hold).

3. **Verified Beats Claimed.**
   A task is only "done" or "working" when backed by recorded evidence in `docs/EVIDENCE_LOG.md` on the `main` branch (exact command run, stdout/stderr, timestamp, latency/memory/CPU numbers, or a `spectacle` screenshot path). Anything without recorded proof must be labeled **UNVERIFIED**.

---

## 2. Fixed Architectural Decisions

1. **Language & Toolkit:** Rust. Toolkit is decided through an empirical bake-off between GTK4 and Iced.
2. **Repository Architecture:** Brand-new Cargo workspace. Three crates:
   - `crates/nevermore-core`: Headless library containing state machine, traits, types (zero GUI dependencies).
   - `crates/nevermore-cli`: IPC client binary (`toggle`, `show <tab>`, `hide`, `pin`, `status`).
   - `crates/nevermore-ui`: GUI service process.
3. **Compositor Boundary:** KDE Plasma 6.8 on Wayland (KWin) first. Layer-shell setup, hot-edge strip, and window raise/focus must sit behind a clean `EdgeAdapter` trait in `nevermore-core`.
4. **Window Geometry & Layer:** Right-edge overlay using `wlr-layer-shell` on the `TOP` layer (NOT `OVERLAY`, so fullscreen games cover it) with zero strut (`exclusive_zone = 0`) by default so desktop windows never shift or resize.
5. **Visual Identity:** Dark theme only, solid surfaces (`#18181b` bg, `#0a84ff` blue accent, red for destructive actions), zero blur, `Inter` font for UI, configurable monospace font for terminal (`Hack Nerd Font` default), bundled open-licensed icons.
6. **Embedded Terminal:** A tab inside the sidebar containing multiple live PTY pages drawn inside Nevermore's own UI, plus a background session list.
7. **Disposable UI over Durable Backends:**
   - UI runs as resident `systemd --user` service with D-Bus/socket activation and ~0% CPU when hidden.
   - If UI process is killed (`kill -9`), terminal sessions survive in a dedicated `tmux` server on a private socket (`tmux -L nevermore`).
   - Background tasks survive as `systemd-run --user` transient units.
   - Notes, settings, and chat history survive via atomic file writes (temp file + rename) in XDG directories.
   - Secrets (Gemini API key) live in the system Secret Service.
8. **Strict Dependency Pinning:**
   - Commit `Cargo.lock`.
   - Exact `=` version pins in all `Cargo.toml` files.
   - Compiler pinned in `rust-toolchain.toml`.
   - All host system packages audited in `DEPENDENCIES.md`.
9. **Explicit Sidebar State Machine:**
   - `Hidden`: Only a thin invisible right-edge hot strip exists; takes pointer hover, zero keyboard focus, ~0% idle CPU.
   - `Peek`: Hovering the hot edge slides panel in; takes NO keyboard focus so typing continues in active app; auto-hides after pointer leaves.
   - `Engaged`: Clicking inside panel grants keyboard focus for terminal/notepad; `Escape` or leaving transitions back to `Hidden`.
   - `Pinned`: Pin button keeps panel open and overrides auto-hide until explicitly unpinned.
10. **Browser Interaction & Manager Bridge Architecture:**
    - All interaction with Chrome and Manager AI MUST route exclusively through the persistent user daemon (`nevermore-manager-bridge.service`) via Unix domain socket `/tmp/nevermore_bridge.sock` or `scripts/manager-cli`.
    - Never open direct connections to Chrome's remote debugging port (port 9222) to avoid prompting the user.
    - Handled via dedicated subagent `browser-bridge`.
   - `Pinned`: Pin button keeps panel open and overrides auto-hide until unpinned.

---

## 3. Subagent Roles & Guidelines

When subagents are dispatched:
- **`rust-implementer`**: Works in an isolated `branch` workspace. Implements Rust code against pre-defined traits. Must run `cargo check` and `cargo test` before submitting work.
- **`spike-runner`**: Works in an isolated `branch` workspace. Executes a time-boxed spike and writes an empirical evaluation report into `docs/spikes/<spike-name>.md` on `main`.
- **`reviewer`**: Read-only auditor (`view_file`, `grep_search`, `find_by_name`, `list_dir`). Reviews diffs against `AGENTS.md`. Rejects forbidden patterns, unpinned dependencies, or unverified claims.
- **`verifier`**: Runs `cargo build --locked`, `cargo clippy`, `cargo fmt --check`, `cargo test`, captures visual evidence, and updates `docs/EVIDENCE_LOG.md`.
