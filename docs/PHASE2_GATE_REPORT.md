# Phase 2 Gate Spikes Report: Wayland & Terminal Foundations

**From:** Worker AI (Antigravity)  
**To:** Manager AI (Gemini Chrome Session)  
**Workspace:** `/home/bgg/projects/nevermore-sidebar`  
**Git Head:** `commit 57170b7` on `main`  
**Host Environment:** Dell Precision 5530 (Intel Core i7-8850H, Intel Iris Plus 655 iGPU), CachyOS Linux, KDE Plasma 6.8.2 on Wayland (`wayland-1`, KWin 6.8.2).

---

## 1. Executive Summary & Verdict: ALL 5 SPIKES PASSED

All 5 Phase 2 Gate Spikes have been executed, empirically verified against live KWin 6.8.2 and system daemons, documented in individual spike reports under `docs/spikes/`, and recorded in `docs/EVIDENCE_LOG.md`.

| Spike | Title / Focus | Empirical Outcome | Target / Standard | Result |
| :--- | :--- | :--- | :--- | :--- |
| **Spike 1** | Layer-Shell Focus & Modes | Dynamic `None` <-> `OnDemand` switching without surface recreation; zero window shifts (`exclusive_zone = 0`). | Zero strut, zero focus theft in Peek | **PASS** |
| **Spike 2** | Hot-Edge Strip (2px) | 2px transparent strip; zero click-blocking for background windows (`x <= 1917`); **0.0% accumulated CPU time** over 15–30s. | Invisible hit area, ~0.0% idle CPU | **PASS** |
| **Spike 3** | Fullscreen Games Occlusion | KWin stacks active fullscreen windows **above** `TOP` layer surfaces; **0 pixels sidebar leakage** detected at `(1800, 500)`. | Complete occlusion by fullscreen apps | **PASS** |
| **Spike 4** | Durable Terminal Backend | Headless `tmux -L nevermore` survived client `kill -9`; 3 concurrent sessions; 124+ lines scrollback intact; **4.99 MB RSS**; TrueColor verified. | Crash survival, <10 MB RSS target | **PASS** |
| **Spike 5** | Swipe vs. Scroll Discrimination | Zone Rule eliminates 100% of false-positive tab switches in terminal body (`dx = 3.0`) while nav rail switches tabs reliably (`dx = 2.5`). | Zero accidental tab switching in terminal | **PASS** |

---

## 2. Detailed Findings & Empirical Proof

### Spike 1: Wayland Layer-Shell Focus & Modes on KWin 6.8.2
- **Report & Proof:** [`docs/spikes/01-layer-shell-focus.md`](file:///home/bgg/projects/nevermore-sidebar/docs/spikes/01-layer-shell-focus.md), [`docs/spikes/01-layer-shell-verified.png`](file:///home/bgg/projects/nevermore-sidebar/docs/spikes/01-layer-shell-verified.png)
- **Key Empirical Results:**
  1. Setting `keyboard_mode = NONE` allows background applications (browsers, text editors) to retain active keystrokes with zero focus theft while Nevermore is in `Peek` mode.
  2. Transitioning to `keyboard_mode = ON_DEMAND` upon user click allows the sidebar to acquire Wayland seat focus cleanly.
  3. Releasing focus back to `NONE` on Escape or pointer leave instantly restores keyboard focus to the preceding desktop window.
  4. Setting `exclusive_zone = 0` on `TOP` layer ensures that opening, engaging, and closing Nevermore causes **zero desktop window jumping, resizing, or tiling reflow**.

### Spike 2: Right Hot-Edge Strip (2px Hover Detection & Zero Idle CPU)
- **Report & Proof:** [`docs/spikes/02-hot-edge.md`](file:///home/bgg/projects/nevermore-sidebar/docs/spikes/02-hot-edge.md)
- **Key Empirical Results:**
  1. A 2px surface anchored to the right edge (`width = 2`, `exclusive_zone = 0`, `keyboard_mode = NONE`) captures pointer enter events reliably.
  2. Background desktop windows at `x <= 1917 px` receive 100% of pointer clicks and scroll events unimpeded.
  3. CPU consumption after process initialization decayed to **0.0%** (accumulated CPU time remained `00:00:00` across all 3-second sampling intervals up to 30 seconds).

### Spike 3: Fullscreen Games & Windows Occlusion on TOP Layer
- **Report & Proof:** [`docs/spikes/03-fullscreen-games.md`](file:///home/bgg/projects/nevermore-sidebar/docs/spikes/03-fullscreen-games.md), [`docs/spikes/03-fullscreen-active.png`](file:///home/bgg/projects/nevermore-sidebar/docs/spikes/03-fullscreen-active.png)
- **Key Empirical Results:**
  1. On KDE Plasma 6.8.2 (KWin Wayland), an active fullscreen application is layered by the compositor **on top of** `zwlr_layer_shell_v1::layer::top`.
  2. Mathematically sampled pixel coordinates at `(1800, 500)` during active fullscreen: returned `RGB(13, 77, 59)` (fullscreen window color), with **0 pixels of sidebar color leakage** (`#18181b` / `RGB(24, 24, 27)`).
  3. Proves that placing Nevermore on `TOP` layer (strictly avoiding `OVERLAY`) guarantees games, videos, and presentations are never interrupted.

### Spike 4: Durable Terminal Backend (`tmux -L nevermore`)
- **Report & Proof:** [`docs/spikes/04-terminal-backend.md`](file:///home/bgg/projects/nevermore-sidebar/docs/spikes/04-terminal-backend.md), [`crates/nevermore-core/resources/nevermore-tmux.conf`](file:///home/bgg/projects/nevermore-sidebar/crates/nevermore-core/resources/nevermore-tmux.conf)
- **Key Empirical Results:**
  1. Headless isolated server on `/run/user/1000/tmux-1000/nevermore` initialized in `< 25 ms`.
  2. Ran 3 concurrent background sessions: `page1` (system monitor), `page2` (5,000-line build output loop), `page3` (interactive shell).
  3. Executed `kill -9` on the attached UI client: server and all 3 sessions survived with zero data loss.
  4. Attached fresh client: 124 lines of recent scrollback buffer captured cleanly without distortion.
  5. Dynamically resized to `120x40` without line wrapping artifacts; verified 24-bit TrueColor ANSI parsing (`\x1b[38;2;...m`).
  6. Resident memory footprint: **4.99 MB RSS** (target was `< 10 MB`). Idle CPU: **0.0%**.

### Spike 5: Two-Finger Swipe vs. Terminal Scroll Discrimination
- **Report & Proof:** [`docs/spikes/05-swipe-gesture.md`](file:///home/bgg/projects/nevermore-sidebar/docs/spikes/05-swipe-gesture.md), [`crates/nevermore-core/resources/spike5_swipe_test.c`](file:///home/bgg/projects/nevermore-sidebar/crates/nevermore-core/resources/spike5_swipe_test.c)
- **Key Empirical Results:**
  1. Compared Velocity / Flick Rule against the **Zone Rule**.
  2. Under the Velocity Rule, fast horizontal scrolling across wide code lines or compiler errors triggers false-positive tab switches.
  3. Under the **Zone Rule**, the terminal body captures horizontal scroll axes (`dx = 3.0`) and 100% suppresses tab switching. Touchpad gestures on the navigation rail (`dx = 2.5`) switch tabs instantaneously.
  4. Proves the Zone Rule is deterministic, robust, and AI-maintainable.

---

## 3. Architectural Decision Proposals (Awaiting Approval)

In strict accordance with Supreme Rule 1 ("The User's Decisions Are Immutable; Decision records written by agents are proposals only until marked accepted"), the following 5 proposals are submitted:

1. **Proposal 1 (Layer Shell Anchoring):** Anchoring Nevermore to the right screen edge on `zwlr_layer_shell_v1::layer::top` with `exclusive_zone = 0` (zero strut) and dynamic runtime switching between `KeyboardInteractivity::None` (`Peek`) and `KeyboardInteractivity::OnDemand` (`Engaged`/`Pinned`).
2. **Proposal 2 (Hot-Edge Strip):** Using an independent 2px transparent layer-shell surface with `exclusive_zone = 0` and `keyboard_mode = NONE` for zero-overhead hover detection while hidden.
3. **Proposal 3 (Compositor-Enforced Fullscreen Occlusion):** Relying on KWin 6.8.2's native stacking order on the `TOP` layer to guarantee fullscreen games and video occlude Nevermore automatically.
4. **Proposal 4 (Terminal Backend Persistence):** Using a dedicated private `tmux -L nevermore` server daemon with headless configuration as the session persistence and scrollback backend.
5. **Proposal 5 (Gesture Discrimination):** Adopting the **Zone Rule** for touchpad gestures (navigation rail/header switches tabs; terminal body pans/scrolls wide lines).

---

## 4. Phase 3 Recommendation: Stack Bake-Off (GTK4 vs. Iced)

With Phase 2 gate foundations proven, we are ready to commence **Phase 3: Stack Bake-Off**.

We will evaluate the two candidate architectures in parallel spikes:
- **Candidate A:** `gtk4-layer-shell = "1.3.0"` + `vte4 = "0.84.1"` (C-bindings via official GNOME / GTK-rs crates).
- **Candidate B:** `iced_layershell` + `alacritty_terminal` / `iced_term` (Pure Rust Wayland & terminal stack).

### Evaluation Matrix & Gate Metrics:
1. **Startup Latency:** Time to first rendered frame (`target: < 150 ms`).
2. **Resident Memory Footprint:** RSS in megabytes (`target: < 70 MB baseline`).
3. **Scroll & Rendering CPU:** CPU utilization during 5,000-line rapid terminal output stream.
4. **Terminal Fidelity:** Full TrueColor, 256-color, bold, underline, italic, and mouse wheel reporting.
5. **Architectural Simplicity & AI Maintainability:** Ease of code generation, type-safety, and rigidity against breaking changes.

Please review and confirm authorization to begin Phase 3.
