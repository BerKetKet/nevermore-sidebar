#!/usr/bin/env bash
set -euo pipefail
cd /home/bgg/projects/nevermore-sidebar

# Pre-test notification courtesy
notify-send -u critical -t 8000 "Nevermore Worker AI" "Running live UI/screenshot test in 5 seconds — please pause mouse/keyboard for ~10s"
sleep 5

echo "=== 1. Testing Single notify-send Immediate Emission & 340px Clamp ==="
nevermore show notifications
sleep 1

# Send a single notification with desktop-entry hint
notify-send -a "Discord" -h "string:desktop-entry:discord" "Phase 7.5.1 Audit" "Deterministic parser immediate emission verified!"
sleep 1

# Capture screenshot of notifications tab
spectacle -b -n -o docs/spikes/07_5_1-notifications-feed.png
sleep 0.5

echo "=== 2. Testing Theme Switch & High-Contrast Accents ==="
nevermore theme set catppuccin-mocha
sleep 1
spectacle -b -n -o docs/spikes/07_5_1-theme-mocha-contrast.png
sleep 0.5

nevermore theme set nevermore-dark
sleep 1

echo "=== 3. Testing DND Inhibit Property on Session D-Bus ==="
python3 -c "
import dbus
bus = dbus.SessionBus()
obj = bus.get_object('org.freedesktop.Notifications', '/org/freedesktop/Notifications')
props = dbus.Interface(obj, 'org.freedesktop.DBus.Properties')
val = props.Get('org.freedesktop.Notifications', 'Inhibited')
print('Current D-Bus Inhibited status:', bool(val))
"

echo "=== 4. Resetting Sidebar to Hidden ==="
nevermore hide
sleep 1

# All-clear notification
notify-send -u normal -t 4000 "Nevermore Worker AI" "UI test complete — you can use the PC again!"
echo "=== Test script finished successfully ==="
