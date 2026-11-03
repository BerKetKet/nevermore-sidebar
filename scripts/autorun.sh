#!/usr/bin/env bash
set -euo pipefail
cd /home/bgg/projects/nevermore-sidebar
rm -f docs/spikes/test-notifs.png docs/spikes/test-terminal.png docs/spikes/test-widgets.png

git add Cargo.lock DEPENDENCIES.md crates/ docs/EVIDENCE_LOG.md docs/spikes/06_5-*.png scripts/
git status
