---
name: write-spike-report
description: Instructions for producing standard empirical spike reports for Nevermore.
---

# Write Spike Report Skill

Use this skill when completing an empirical spike in Phase 2 or Phase 3. Spike reports evaluate feasibility, performance, memory usage, and compositor integration on KDE Plasma 6 (Wayland) before selecting implementations.

## Report Path
Save the report as: `docs/spikes/<spike-name>.md` on the `main` branch.

## Mandatory Report Structure
Every spike report must include these 6 sections:

### 1. Objective & Hypothesis
- What technical capability or integration is being tested?
- What was the hypothesis before testing?

### 2. Constraints & Supreme Rules Check
- Explicit verification that all Hard Prohibitions in `AGENTS.md` are respected:
  - Terminal drawn inside Nevermore UI (not external window).
  - No Konsole or external emulator launched.
  - Zero blur / solid `#18181b` surface.
  - Intel iGPU Iris Plus 655 only (NVIDIA GPU untouched).
  - Zero strut / `exclusive_zone = 0` on `TOP` layer.

### 3. Test Environment & Setup
- Host hardware: Dell Precision 5530, i7-8850H, Intel Iris Plus 655 / UHD 630.
- OS & Compositor: CachyOS, Linux kernel version, KDE Plasma 6.8.x on Wayland (KWin).
- Toolchain: Exact Rust compiler version (`rustc --version`).
- Dependencies: Exact crate versions and system package versions tested.

### 4. Empirical Measurements (Recorded Proof)
Provide raw command outputs and measurements:
- **Startup Latency**: Time to first frame (ms).
- **Idle Resource Usage**: Resident memory (RSS in MB via `ps` or `smem`) and CPU usage % when hidden vs peek vs engaged.
- **Rendering Performance**: Frame time / FPS during slide-in and slide-out.
- **Input Responsiveness**: Latency for keyboard typing in terminal or text inputs.
- **Wayland Protocol Verification**: Layer-shell namespace, surface role, keyboard focus acquisition.

### 5. Failure Modes & Edge Cases
- Document any crashes, glitching, frame drops, or protocol misbehaviors.
- Multi-monitor behavior, screen scaling (100% vs fractional), and resolution changes.

### 6. Decision & Recommendation
- State clearly whether the approach is viable or rejected.
- **All agent decisions are proposals only** until the user explicitly accepts them.
