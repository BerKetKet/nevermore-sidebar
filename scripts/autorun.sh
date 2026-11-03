#!/usr/bin/env bash
set -uo pipefail
cd /home/bgg/projects/nevermore-sidebar

git add -A
git status
git commit -m "feat(ui): implement Phase 5 app shell and Phase 6 embedded terminal with tmux backend"
git checkout main
git merge phase5-6-app-shell-terminal
git log -n 5 --oneline
