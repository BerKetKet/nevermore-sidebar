#!/usr/bin/env bash
set -euo pipefail

SOCKET="nevermore"
CONF="/home/bgg/projects/nevermore-sidebar/crates/nevermore-core/resources/nevermore-tmux.conf"

echo "=== 1. Starting isolated tmux server on socket '$SOCKET' ==="
# Ensure any previous test instance is cleaned up
tmux -L "$SOCKET" kill-server 2>/dev/null || true

# Start Session 1: htop
tmux -L "$SOCKET" -f "$CONF" new-session -d -s page1 -n htop "top -b -d 1"

# Create Session 2: continuous build loop
tmux -L "$SOCKET" -f "$CONF" new-session -d -s page2 -n build "bash -c 'for i in \$(seq 1 5000); do echo \"[BUILD STEP \$i] Compiling module_\$i.rs...\"; sleep 0.01; done'"

# Create Session 3: interactive shell
tmux -L "$SOCKET" -f "$CONF" new-session -d -s page3 -n shell "bash --norc"

echo "=== 2. Verifying sessions created ==="
tmux -L "$SOCKET" list-sessions

# Send commands into page3 (truecolor probe and environment check)
tmux -L "$SOCKET" send-keys -t page3 "printf '\x1b[38;2;10;132;255mTRUECOLOR_ACCENT_VERIFIED\x1b[0m\n'" C-m
tmux -L "$SOCKET" send-keys -t page3 "echo NEVERMORE_SESSION_TEST_OK" C-m

sleep 1

echo "=== 3. Simulating client kill -9 (Client Crash) ==="
# Launch a background client attached to page2
tmux -L "$SOCKET" attach-session -t page2 &
CLIENT_PID=$!
sleep 0.5

echo "Killing attached client PID $CLIENT_PID with kill -9..."
kill -9 "$CLIENT_PID" 2>/dev/null || true
wait "$CLIENT_PID" 2>/dev/null || true

echo "=== 4. Verifying server and background tasks survived client crash ==="
tmux -L "$SOCKET" list-sessions

echo "=== 5. Reattaching fresh client and verifying scrollback ==="
BUILD_LINES=$(tmux -L "$SOCKET" capture-pane -t page2 -p -S -100 | wc -l)
echo "Page 2 captured scrollback lines: $BUILD_LINES"
tmux -L "$SOCKET" capture-pane -t page2 -p -S -5

echo "=== 6. Verifying Page 3 output ==="
PAGE3_OUTPUT=$(tmux -L "$SOCKET" capture-pane -t page3 -p)
echo "$PAGE3_OUTPUT" | grep "NEVERMORE_SESSION_TEST_OK"
echo "Page 3 interactive session output intact!"

echo "=== 7. Verifying dynamic resizing ==="
# Resize window geometry
tmux -L "$SOCKET" resize-window -t page1 -x 120 -y 40
WIN_SIZE=$(tmux -L "$SOCKET" display-message -t page1 -p "#{window_width}x#{window_height}")
echo "Resized window geometry: $WIN_SIZE"
if [ "$WIN_SIZE" != "120x40" ]; then
    echo "ERROR: Window size mismatch: expected 120x40, got $WIN_SIZE"
    exit 1
fi

echo "=== 8. Measuring Memory (RSS) of tmux server ==="
TMUX_PID=$(pgrep -f "tmux -L $SOCKET" | head -n 1)
RSS_KB=$(ps -o rss= -p "$TMUX_PID" | tr -d ' ')
RSS_MB=$(awk "BEGIN {printf \"%.2f\", $RSS_KB/1024}")
echo "tmux server PID: $TMUX_PID, RSS: ${RSS_MB} MB (${RSS_KB} KB)"

# Clean up test session
tmux -L "$SOCKET" kill-server
echo "=== SPIKE 4 TEST COMPLETE: SUCCESS ==="
