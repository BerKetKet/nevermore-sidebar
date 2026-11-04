#!/usr/bin/env python3
"""
scripts/run_evidence_pass.py

Automated Single Comprehensive Evidence Pass for Nevermore Phase 7.5.2 (Part C).
Executes Protocol 0.2 courtesy notifications, D-Bus DND inhibition, hot-edge geometry
and grace timer tests without touching screen corners, clean toast-free screenshots
across all tabs and themes, terminal focus capture, and all-clear courtesy notification.
"""

import os
import subprocess
import time
import sys
from pathlib import Path
import gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib

ROOT = Path("/home/bgg/projects/nevermore-sidebar")
SPIKES_DIR = ROOT / "docs" / "spikes"
SPIKES_DIR.mkdir(parents=True, exist_ok=True)

def run(cmd, check=True):
    print(f"[EXEC] {' '.join(cmd) if isinstance(cmd, list) else cmd}")
    res = subprocess.run(cmd, shell=isinstance(cmd, str), capture_output=True, text=True)
    if check and res.returncode != 0:
        print(f"[FAIL] Return code: {res.returncode}\nSTDERR:\n{res.stderr}\nSTDOUT:\n{res.stdout}")
        raise RuntimeError(f"Command failed: {cmd}")
    return res

def notify(summary, body, urgency="normal", timeout=5000):
    subprocess.run(["notify-send", "-u", urgency, "-t", str(timeout), summary, body])

def main():
    print("================================================================================")
    print("PHASE 7.5.2 EVIDENCE PASS (C1 - C6)")
    print("================================================================================")

    # 1. Protocol 0.2: Mandatory Courtesy Notification
    print("\n--- Protocol 0.2 Courtesy Notification ---")
    notify(
        "Nevermore Worker AI",
        "Running live UI/screenshot test in 5 seconds — please pause mouse/keyboard for ~15s",
        urgency="critical",
        timeout=8000
    )
    print("Sleeping 6 seconds for warning toast to clear...")
    time.sleep(6.0)

    # Close any open overview or grid cleanly
    run("ydotool key 1:1 1:0")
    time.sleep(0.3)

    # 2. DND Inhibition (Toast-Free guarantee for screenshots)
    print("\n--- C2: Inhibit Desktop Notifications (DND ON) ---")
    conn = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    params = GLib.Variant("(ssa{sv})", ("nevermore-evidence", "Evidence Pass", {}))
    inhibit_reply = conn.call_sync(
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
        "Inhibit",
        params,
        GLib.VariantType.new("(u)"),
        Gio.DBusCallFlags.NONE,
        2000,
        None
    )
    cookie = inhibit_reply.get_child_value(0).get_uint32()
    print(f"Acquired DND Inhibit cookie: {cookie}")

    # Verify DND state on session bus
    res_dnd_on = run("busctl --user get-property org.freedesktop.Notifications /org/freedesktop/Notifications org.freedesktop.Notifications Inhibited")
    print(f"busctl Inhibited property check: {res_dnd_on.stdout.strip()}")
    assert "true" in res_dnd_on.stdout, f"Expected Inhibited == true, got {res_dnd_on.stdout}"

    try:
        # 3. Restart Nevermore with latest release binary
        print("\n--- Restarting nevermore.service ---")
        run("systemctl --user restart nevermore")
        time.sleep(1.5)

        st = run("/home/bgg/projects/nevermore-sidebar/target/release/nevermore status")
        print(f"Initial status:\n{st.stdout.strip()}")

        # 4. C3: Hot-Edge Geometry, Hover Hit Test & 1500ms Grace Timer (Corner-Free)
        print("\n--- C3: Hot-Edge Geometry & Grace Timer Test ---")
        run("/home/bgg/projects/nevermore-sidebar/target/release/nevermore hide")
        time.sleep(0.5)

        # Move away left
        run("ydotool mousemove -- -800 0")
        time.sleep(0.3)

        # Hit test: Edge hover entry
        print("Testing hover entry at right edge...")
        run("ydotool mousemove -- 1200 0")
        time.sleep(0.6)
        st_hit = run("/home/bgg/projects/nevermore-sidebar/target/release/nevermore status")
        print(f"Status at edge hit: {st_hit.stdout.strip()}")
        assert "Peek" in st_hit.stdout, f"Expected Peek at edge, got {st_hit.stdout}"

        # Grace timer test: Move away left and wait 1.8s
        print("Testing 1500ms Peek grace timer: moving left -800px and waiting 2.0s...")
        run("ydotool mousemove -- -800 0")
        time.sleep(2.0)
        st_grace = run("/home/bgg/projects/nevermore-sidebar/target/release/nevermore status")
        print(f"Status after grace timer: {st_grace.stdout.strip()}")
        assert "Hidden" in st_grace.stdout, f"Expected Hidden after grace timer, got {st_grace.stdout}"

        # 5. C2: Clean Toast-Free Screenshots
        print("\n--- C2: Capturing Screenshots of Tabs in nevermore-dark and catppuccin-mocha ---")
        run("/home/bgg/projects/nevermore-sidebar/target/release/nevermore theme set nevermore-dark")
        time.sleep(0.3)

        tabs_to_capture = [
            ("terminal", "07_5_2-tab-terminal.png"),
            ("widgets", "07_5_2-tab-widgets.png"),
            ("notifications", "07_5_2-tab-notifications.png"),
            ("notepad", "07_5_2-tab-notepad.png"),
            ("media", "07_5_2-tab-placeholders.png"),
        ]

        for tab_id, out_name in tabs_to_capture:
            print(f"Capturing tab '{tab_id}' -> {out_name}...")
            run(f"/home/bgg/projects/nevermore-sidebar/target/release/nevermore show {tab_id}")
            time.sleep(0.5)
            out_path = SPIKES_DIR / out_name
            run(f"spectacle -b -n -o {out_path}")
            assert out_path.exists() and out_path.stat().st_size > 1000, f"Screenshot {out_path} failed to capture"

        # Theme Catppuccin Mocha
        print("Switching theme to catppuccin-mocha...")
        run("/home/bgg/projects/nevermore-sidebar/target/release/nevermore theme set catppuccin-mocha")
        run("/home/bgg/projects/nevermore-sidebar/target/release/nevermore show terminal")
        time.sleep(0.5)
        mocha_path = SPIKES_DIR / "07_5_2-theme-mocha.png"
        run(f"spectacle -b -n -o {mocha_path}")
        assert mocha_path.exists() and mocha_path.stat().st_size > 1000, f"Screenshot {mocha_path} failed"

        # Reset theme to nevermore-dark
        run("/home/bgg/projects/nevermore-sidebar/target/release/nevermore theme set nevermore-dark")

        # 6. C5: Terminal Focus Proof on 'nevermore show terminal'
        print("\n--- C5: Terminal Focus Proof ---")
        run("/home/bgg/projects/nevermore-sidebar/target/release/nevermore hide")
        time.sleep(0.3)
        run("/home/bgg/projects/nevermore-sidebar/target/release/nevermore show terminal")
        time.sleep(0.5)

        # Type verification token
        token = "FOCUS_P752_OK"
        print(f"Typing verification command 'echo {token}' via ydotool...")
        run(f'ydotool type "echo {token}"')
        time.sleep(0.3)
        run("ydotool key 28:1 28:0") # Enter key
        time.sleep(0.8)

        # Capture tmux pane content across active sessions
        tmux_cap = run("tmux -L nevermore capture-pane -p -t main", check=False).stdout
        print(f"tmux pane output (main):\n{tmux_cap}")
        if token not in tmux_cap:
            for s in ["page_1", "page_2"]:
                cap = run(f"tmux -L nevermore capture-pane -p -t {s}", check=False).stdout
                if token in cap:
                    tmux_cap = cap
                    break
        assert token in tmux_cap, f"Token {token} not found in tmux pane output!"
        print(f"[SUCCESS] Focus proof passed! Terminal captured typing: {token}")

    finally:
        # 7. Release DND Inhibition
        print("\n--- Releasing DND Inhibit ---")
        unparams = GLib.Variant("(u)", (cookie,))
        conn.call_sync(
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "UnInhibit",
            unparams,
            None,
            Gio.DBusCallFlags.NONE,
            2000,
            None
        )
        print("DND Inhibit released.")

        res_dnd_off = run("busctl --user get-property org.freedesktop.Notifications /org/freedesktop/Notifications org.freedesktop.Notifications Inhibited")
        print(f"busctl Inhibited property check: {res_dnd_off.stdout.strip()}")

        # 8. Protocol 0.2: All-Clear Notification
        notify(
            "Nevermore Worker AI",
            "UI test complete — you can use the PC again!",
            urgency="normal",
            timeout=4000
        )
        print("All-clear notification sent.")

    print("\n[SUCCESS] Part C Evidence Pass Completed with 100% assertions verified!")

if __name__ == "__main__":
    main()
