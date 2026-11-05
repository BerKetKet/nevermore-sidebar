#!/usr/bin/env python3
import time
import json
import subprocess
from pathlib import Path
import gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib

ROOT = Path("/home/bgg/projects/nevermore-sidebar")
SHOTS_DIR = ROOT / "docs" / "screenshots"
SHOTS_DIR.mkdir(parents=True, exist_ok=True)

MUSIC_STATE_PATH = Path("/home/bgg/.local/share/nevermore/plugins/music/music_ui_state.json")
MUSIC_STATE_PATH.parent.mkdir(parents=True, exist_ok=True)

def notify(summary, body, urgency="normal", timeout=5000):
    subprocess.run(["notify-send", "-u", urgency, "-t", str(timeout), summary, body])

def set_music_subview(subview_idx: int):
    MUSIC_STATE_PATH.write_text(json.dumps({"subview": subview_idx}))

def main():
    print("=== Nevermore Phase 13 (v16) Evidence Capture under Protocol 0.2 ===")

    # 1. 5s Advance Warning Notification
    print("1. Sending courtesy notification (5s warning)...")
    notify(
        "Nevermore Worker AI",
        "Capturing 7 verification screenshots for Phase 13 in 5 seconds — please pause mouse/keyboard",
        urgency="critical",
        timeout=6000
    )
    time.sleep(6.0)

    # 2. Close overview if any
    subprocess.run(["ydotool", "key", "1:1", "1:0"], capture_output=True)
    time.sleep(0.3)

    # 3. DND Inhibit
    print("2. Enabling DND Inhibit...")
    conn = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    params = GLib.Variant("(ssa{sv})", ("nevermore-v16-capture", "Screenshot Pass", {}))
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
        # Capture 1: Email Companion Side-Blade (16-email-companion-blade.png)
        print("Capturing 1: Email Companion Side-Blade...")
        subprocess.run(["/home/bgg/.local/bin/nevermore", "show", "email"], check=True)
        time.sleep(2.0)
        # Click on first email card to open companion blade
        subprocess.run(["ydotool", "mousemove", "-a", "1700", "150"], capture_output=True)
        time.sleep(0.2)
        subprocess.run(["ydotool", "click", "0xC0"], capture_output=True)
        time.sleep(2.0)
        out1 = SHOTS_DIR / "16-email-companion-blade.png"
        subprocess.run(["spectacle", "-b", "-n", "-o", str(out1)], check=True)
        print(f"Captured {out1.name} ({out1.stat().st_size} bytes)")

        # Capture 2: Music Now Playing (16-music-now-playing.png)
        print("Capturing 2: Music Now Playing...")
        set_music_subview(0)
        # Ensure song loaded
        subprocess.run(["mpc", "play", "1"], capture_output=True)
        time.sleep(0.5)
        subprocess.run(["/home/bgg/.local/bin/nevermore", "show", "music"], check=True)
        time.sleep(1.5)
        out2 = SHOTS_DIR / "16-music-now-playing.png"
        subprocess.run(["spectacle", "-b", "-n", "-o", str(out2)], check=True)
        print(f"Captured {out2.name} ({out2.stat().st_size} bytes)")

        # Capture 3: Music Queue (16-music-queue.png)
        print("Capturing 3: Music Queue...")
        set_music_subview(1)
        subprocess.run(["/home/bgg/.local/bin/nevermore", "show", "music"], check=True)
        time.sleep(1.5)
        out3 = SHOTS_DIR / "16-music-queue.png"
        subprocess.run(["spectacle", "-b", "-n", "-o", str(out3)], check=True)
        print(f"Captured {out3.name} ({out3.stat().st_size} bytes)")

        # Capture 4: Music Synced Lyrics (16-music-lyrics.png)
        print("Capturing 4: Music Synced Lyrics...")
        set_music_subview(2)
        subprocess.run(["/home/bgg/.local/bin/nevermore", "show", "music"], check=True)
        time.sleep(1.5)
        out4 = SHOTS_DIR / "16-music-lyrics.png"
        subprocess.run(["spectacle", "-b", "-n", "-o", str(out4)], check=True)
        print(f"Captured {out4.name} ({out4.stat().st_size} bytes)")

        # Capture 5: Music Playlists & Search (16-music-playlists-search.png)
        print("Capturing 5: Music Playlists & Search...")
        set_music_subview(3)
        subprocess.run(["/home/bgg/.local/bin/nevermore", "show", "music"], check=True)
        time.sleep(1.5)
        out5 = SHOTS_DIR / "16-music-playlists-search.png"
        subprocess.run(["spectacle", "-b", "-n", "-o", str(out5)], check=True)
        print(f"Captured {out5.name} ({out5.stat().st_size} bytes)")

        # Pause playback
        subprocess.run(["mpc", "pause"], capture_output=True)

        # Capture 6: Notepad Multi-Note (16-notepad-multinote.png)
        print("Capturing 6: Notepad Multi-Note...")
        subprocess.run(["/home/bgg/.local/bin/nevermore", "show", "notepad"], check=True)
        time.sleep(1.2)
        out6 = SHOTS_DIR / "16-notepad-multinote.png"
        subprocess.run(["spectacle", "-b", "-n", "-o", str(out6)], check=True)
        print(f"Captured {out6.name} ({out6.stat().st_size} bytes)")

        # Capture 7: Settings Defaults (16-settings-defaults.png)
        print("Capturing 7: Settings Defaults...")
        subprocess.run(["/home/bgg/.local/bin/nevermore", "show", "__settings__"], check=True)
        time.sleep(1.2)
        out7 = SHOTS_DIR / "16-settings-defaults.png"
        subprocess.run(["spectacle", "-b", "-n", "-o", str(out7)], check=True)
        print(f"Captured {out7.name} ({out7.stat().st_size} bytes)")

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
            "Phase 13 screenshot capture pass complete — thank you!",
            urgency="normal",
            timeout=4000
        )
        # Hide sidebar when done
        subprocess.run(["/home/bgg/.local/bin/nevermore", "hide"], capture_output=True)
        print("=== Complete! ===")

if __name__ == "__main__":
    main()
