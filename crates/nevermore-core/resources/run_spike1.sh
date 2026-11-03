#!/usr/bin/env bash
set -euo pipefail

echo "=== 1. Starting Spike 1 layer shell surface ==="
env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 /tmp/spike1_layer_shell > /tmp/spike1.log 2>&1 &
SPIKE_PID=$!
sleep 2

echo "=== 2. Verifying Initial State (Peek, Zero Focus) ==="
cat /tmp/spike1.log | grep "State: PEEK"

# Take screenshot of Peek state
mkdir -p /home/bgg/projects/nevermore-sidebar/docs/spikes
env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 spectacle -b -n -o /home/bgg/projects/nevermore-sidebar/docs/spikes/01-layer-shell-peek.png || true

echo "=== 3. Engaging focus via click inside surface ==="
# Move to x=1800, y=500 and click left button
ydotool mousemove --absolute 1800 500
sleep 0.2
ydotool click 0xC0
sleep 0.5

echo "=== 4. Verifying Engaged State ==="
cat /tmp/spike1.log | grep "State: ENGAGED"

echo "=== 5. Sending test keystrokes to verify input ==="
wtype "nevermore"
sleep 0.5
cat /tmp/spike1.log | grep -E "SPIKE 1 KEY EVENT" | head -n 3

echo "=== 6. Pressing Escape to drop focus ==="
wtype -k Escape
sleep 0.5
cat /tmp/spike1.log | grep "Focus Dropped via Escape"

# Take screenshot of final state
env WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/1000 spectacle -b -n -o /home/bgg/projects/nevermore-sidebar/docs/spikes/01-layer-shell-engaged.png || true

echo "=== 7. Cleaning up Spike 1 process ==="
kill "$SPIKE_PID" 2>/dev/null || true
wait "$SPIKE_PID" 2>/dev/null || true

echo "=== SPIKE 1 TEST COMPLETE: SUCCESS ==="
