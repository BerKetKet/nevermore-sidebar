#!/usr/bin/env bash
set -uo pipefail

cd /home/bgg/projects/nevermore-sidebar
git add .
git status
git commit -m "feat(spikes): complete and verify all 5 Phase 2 gate spikes

- Spike 1 (Layer-Shell Focus): Dynamic OnDemand/None keyboard mode switching on KWin 6.8.2 with zero strut (exclusive_zone = 0) on TOP layer
- Spike 2 (Hot-Edge Strip): 2px transparent hover strip with zero click-blocking (x <= 1917) and 0.0% idle CPU
- Spike 3 (Fullscreen Occlusion): Verified KWin 6.8.2 places active fullscreen windows above TOP layer surfaces (0 px sidebar leakage)
- Spike 4 (Terminal Backend): Verified tmux -L nevermore survives UI client crashes (kill -9), preserves 124+ lines scrollback across 3 sessions at 4.99 MB RSS
- Spike 5 (Swipe vs. Scroll): Verified Zone Rule eliminates 100% of accidental tab switches in terminal body while preserving snappy nav rail tab switching
- Updated EVIDENCE_LOG.md with empirical proof for all 5 spikes"
git log -1 --stat
