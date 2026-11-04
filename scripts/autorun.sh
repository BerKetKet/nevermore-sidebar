#!/usr/bin/env bash
set -euo pipefail
cd /home/bgg/projects/nevermore-sidebar

SUBFOLDER_ID="1y4ZJ9aaGr6_VEgo0VSqzmu1UUf_IwyQo"

echo "=== Uploading Deliverables to Subfolder $SUBFOLDER_ID ==="
gdrive mutate upload docs/NEVERMORE_COMM_LOG_v04_WORKER.md --parent "$SUBFOLDER_ID"
gdrive mutate upload docs/04_FULL_SOURCE_CODE_DUMP_Phase-7_5_1.md --parent "$SUBFOLDER_ID"
gdrive mutate upload docs/PLUGIN_SDK.md --parent "$SUBFOLDER_ID"
gdrive mutate upload docs/THEMING_GUIDE.md --parent "$SUBFOLDER_ID"
gdrive mutate upload docs/spikes/07_5_1-notifications-feed.png --parent "$SUBFOLDER_ID"
gdrive mutate upload docs/spikes/07_5_1-theme-mocha-contrast.png --parent "$SUBFOLDER_ID"

echo "=== Verifying Subfolder Contents ==="
gdrive readonly ls "$SUBFOLDER_ID"

echo "=== Google Drive Upload Complete ==="
