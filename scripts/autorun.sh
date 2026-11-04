#!/usr/bin/env bash
set -euo pipefail
cd /home/bgg/projects/nevermore-sidebar

echo "=== 1. Running verify_icon_contrast.py ==="
python3 scripts/verify_icon_contrast.py

echo "=== 2. Running Part C Evidence Pass ==="
chmod +x scripts/run_evidence_pass.py
python3 scripts/run_evidence_pass.py

echo "=== 3. Evidence Pass Complete ==="
