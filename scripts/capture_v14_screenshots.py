#!/usr/bin/env python3
import time
import subprocess
from pathlib import Path
import gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib

ROOT = Path("/home/bgg/projects/nevermore-sidebar")
SHOTS_DIR = ROOT / "docs" / "screenshots"
SHOTS_DIR.mkdir(parents=True, exist_ok=True)

def notify(summary, body, urgency="normal", timeout=5000):
    subprocess.run(["notify-send", "-u", urgency, "-t", str(timeout), summary, body])

def main():
    print("=== Nevermore v14 Evidence Capture under Protocol 0.2 ===")

    # 1. Warning notification
    print("1. Sending courtesy notification (5s warning)...")
    notify(
        "Nevermore Worker AI",
        "Capturing 4 full-height verification screenshots in 5 seconds — please pause mouse/keyboard",
        urgency="critical",
        timeout=6000
    )
    time.sleep(6.0)

    # 2. Close any overview
    subprocess.run(["ydotool", "key", "1:1", "1:0"], capture_output=True)
    time.sleep(0.3)

    # 3. DND Inhibit
    print("2. Enabling DND Inhibit...")
    conn = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    params = GLib.Variant("(ssa{sv})", ("nevermore-v14-capture", "Screenshot Pass", {}))
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

    try:
        targets = [
            ("music", SHOTS_DIR / "14-full-height-music.png"),
            ("gemini", SHOTS_DIR / "14-live-gemini-verified.png"),
            ("email", SHOTS_DIR / "14-live-gmail-verified.png"),
            ("__settings__", SHOTS_DIR / "14-full-height-settings.png"),
        ]

        for tab_id, out_path in targets:
            print(f"Showing tab '{tab_id}' -> {out_path.name}...")
            subprocess.run(["/home/bgg/.local/bin/nevermore", "show", tab_id], check=True)
            time.sleep(1.2)
            subprocess.run(["spectacle", "-b", "-n", "-o", str(out_path)], check=True)
            assert out_path.exists() and out_path.stat().st_size > 10000, f"Capture failed for {out_path}"
            print(f"Captured {out_path.name} ({out_path.stat().st_size} bytes)")

    finally:
        print("Releasing DND Inhibit...")
        conn.call_sync(
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "UnInhibit",
            GLib.Variant("(u)", (cookie,)),
            None,
            Gio.DBusCallFlags.NONE,
            2000,
            None
        )
        notify(
            "Nevermore Worker AI",
            "Screenshot capture pass complete — thank you!",
            urgency="normal",
            timeout=4000
        )
        print("=== Complete! ===")

if __name__ == "__main__":
    main()
