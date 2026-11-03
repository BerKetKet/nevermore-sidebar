# 📚 Lessons & Technical Patterns from Legacy Repository

This document summarizes the technical lessons, proven patterns, and cautionary failures from the legacy plasmoid repository (`https://github.com/berketket/nevermore`, frozen as a read-only reference).

---

## 1. Durable Terminal Multiplexer Backend (`tmux`)

**Source:** `spikes/terminal-poc/nevermore-tmux.conf` on `phase3a-spike-fixes`

The legacy spike proved that terminal sessions can survive UI crashes and restarts when hosted in an isolated, private tmux server instance (`tmux -L nevermore`).

### Key Configuration Options:
- **`status off`**: Completely removes the tmux status bar, ensuring the embedded terminal appears as a seamless, clean shell prompt within Nevermore.
- **`unbind C-b; set -g prefix None`**: Disables the default tmux command prefix (`Ctrl+B`), allowing 100% of physical keystrokes, shortcuts, and keybindings to pass through directly to shell, Vim, Emacs, and CLI utilities.
- **`setw -g window-size latest`**: Automatically adjusts the terminal geometry dynamically whenever the client window resizes.
- **`set -s escape-time 0`**: Eliminates key latency on `Escape` keypresses, critical for snappy Vim/vi modal editing and shell navigation.
- **`set -g default-terminal "tmux-256color"`** + RGB overrides: Enables full 24-bit TrueColor rendering.
- **`set -g mouse on`**: Enables mouse scrolling, clicking, and native text selection.
- **`bind -T copy-mode MouseDragEnd1Pane send-keys -X copy-pipe-and-cancel "wl-copy"`**: Seamlessly copies mouse-selected text to the system Wayland clipboard.

---

## 2. Transient Background Tasks (`systemd-run --user`)

**Source:** `scripts/nevermore-run`

The legacy runner evolved from a fragile one-shot shell runner into an honest, durable background task manager using native systemd user services.

### Key Architecture:
- **Unit Prefixing:** Units are named with a predictable prefix and timestamp: `nevermore-task-$(date +%s%N | cut -b1-13)`.
- **Durable Lifecycle:** Launched with `--remain-after-exit` and `--working-directory="$TARGET_DIR"`. This ensures finished tasks remain visible with their exit codes rather than disappearing immediately upon completion.
- **JSON Querying:** State is queried via `systemctl --user list-units "nevermore-task-*" --all --output=json`, providing structured, machine-readable status (`running`, `exited`, `failed`).
- **Exit Code Inspection:** Process return codes are inspected via `systemctl --user show "$UNIT" --property=Id,ExecMainStatus`.
- **Logs Inspection:** Live and past logs are fetched via `journalctl --user -u "$UNIT" -n 200 --no-pager`.
- **Cleanup / Dismissal:** Completed units are dismissed using `systemctl --user stop "$UNIT"` followed by `systemctl --user reset-failed "$UNIT"`.

---

## 3. Compositor Window Raising & Activation

**Source:** `scripts/nevermore-raise`

When interacting with notifications that do not provide default click actions, Nevermore activates the corresponding application window using KWin scripting.

### Key Architecture:
- **Desktop Entry Matching:** Case-insensitive match on `desktopFileName` (with or without `.desktop`) and fallback to `resourceClass` / `resourceName`.
- **Unminimize Before Focus:** Explicitly sets `w.minimized = false;` before setting `workspace.activeWindow = w;`. In KWin Wayland, setting `activeWindow` on a minimized window without unminimizing it leaves the window hidden.
- **Strict Validation:** Input desktop entry name is validated against `^[A-Za-z0-9_.-]+$` to prevent script injection.

---

## 4. Cautionary Architecture Lesson: External Window vs. Embedded Terminal

**Source:** `docs/spikes/terminal.md`

In Phase 3b of the legacy plasmoid, an external agent unlawfully replaced the user's explicit embedded terminal requirement with a separate external Konsole window docked to the right edge via KWin placement scripts (`scripts/nevermore-term`).

### Why That Approach Was Rejected:
1. **Direct Violation of User Mandate:** The user explicitly requested an embedded terminal drawn inside Nevermore's own interface. Popping out a separate window violates the core UX of an integrated, all-in-one control center sidebar.
2. **Fragile Window Management:** Relying on KWin placement scripts to position, resize, strip borders, and toggle an external emulator window is prone to compositor race conditions, tiling window manager interference (Krohnkite/BTree), and focus routing desyncs.
3. **Double Surface Problem:** Having both a sidebar surface and an adjacent terminal window creates awkward window overlap, double shadows, and competing shortcuts.

### The Correct Rust Architecture:
In the new Rust architecture, the terminal is rendered **strictly inside Nevermore's own sidebar window** (embedded VTE / PTY terminal widget), backed by durable sessions in `tmux -L nevermore`. There are **no separate windows, no Konsole invocations, and no KWin window placement hacks**.

---

## 5. Legacy Widget Feature Matrix (For Parity Tracking)

| Widget | Legacy Implementation | New Rust Target |
| :--- | :--- | :--- |
| **Calendar** | 7x6 month grid, today highlight, month navigation | Native sidebar widget with local event indicators |
| **CPU Telemetry** | `ksysguard` sensor: `cpu/all/total/usagePercent` | Real-time `/proc/stat` or `sysinfo` sensor (Intel iGPU target) |
| **RAM Telemetry** | `memory/physical/used` and total | Real-time `/proc/meminfo` or `sysinfo` |
| **Disk Telemetry** | `disk/all/usedPercent` | Native `statvfs` on mounted filesystems |
| **Network Throughput**| `network/all/rate` upload & download | Live `/proc/net/dev` rate polling |
| **Media Player (MPRIS)** | D-Bus `org.mpris.MediaPlayer2.*` multi-player switcher | Native MPRIS2 D-Bus client with player cycling |
| **Audio Volume** | PipeWire/PulseAudio sliders and mute toggle | Native PulseAudio / PipeWire D-Bus or libpulse client |
| **Notification Center** | `org.kde.notificationmanager` read-only feed | `org.freedesktop.Notifications` inspection feed (read-only) |
| **Terminal** | (External Konsole window — rejected) | **Embedded PTY/VTE tab** inside sidebar + `tmux -L nevermore` |
| **Background Tasks** | `systemd-run --user` list, logs, stop, dismiss | Native `systemd-run --user` task management tab |
