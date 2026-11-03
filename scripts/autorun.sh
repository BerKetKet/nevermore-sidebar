#!/usr/bin/env bash
set -euo pipefail
cd /home/bgg/projects/nevermore-sidebar

git add crates/ docs/ scripts/
git status

git commit -m "feat(phase7): implement notification spikes and live widgets, notifications, and notepad tabs

- Step 7A Spikes:
  - Spike 07: D-Bus listen-only notification monitor via BecomeMonitor without intercepting plasmashell (docs/spikes/07-notification-listen.md).
  - Spike 08: Click-to-raise application window mapping via KWin scripting DBus (docs/spikes/08-notification-click.md).
  - Spike 09: DND toggle evaluation proving Inhibited property is read-only and adopting standard FreeDesktop Inhibit/UnInhibit lifecycle (docs/spikes/09-dnd-toggle.md).
- Step 7B Tabs Implementation:
  - Widgets tab: Live CPU and RAM metrics from /proc, PipeWire volume and mute toggle via wpctl, MPRIS2 media controller, and clock/date card (docs/spikes/07-tab-widgets.png).
  - Notifications tab: Chronological notification feed, live D-Bus listen-only capture, click-to-raise window, and DND toggle (docs/spikes/07-tab-notifications.png).
  - Notepad tab: Persistent multi-line scratchpad, crash-proof atomic auto-saving to $XDG_DATA_HOME/nevermore/notes.txt, character and word counters (docs/spikes/07-tab-notepad.png).
- Strict 400px width lock empirically verified across all new tabs (left edge = 1520, width = 400px).
- Update docs/EVIDENCE_LOG.md with empirical logs and benchmarks."

git push origin main
