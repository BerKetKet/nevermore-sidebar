# Spike 6 Report: Architecture & Stack Bake-Off

## 1. Executive Summary & Objective
Phase 3 mandates an empirical bake-off between two standalone Rust UI stacks for the **Nevermore** Wayland sidebar:
- **Candidate A:** `gtk4` (4.24.1) + `gtk4-layer-shell` (1.3.0) + `vte4` (0.84.1)
- **Candidate B:** `iced` (0.14) + `iced_layershell` (0.19) + `iced_term` (0.8) / `alacritty_terminal` (0.25)

The objective is to eliminate architectural guesswork by testing fully functional release prototypes of both candidates on the user's live system:
- **Hardware:** Dell Precision 5530, Intel Core i7-8850H, Intel Iris Plus 655 iGPU.
- **Compositor & OS:** CachyOS Linux, KDE Plasma 6.8.2 on Wayland (`wayland-1`), KWin 6.8.2.
- **Worktree Isolation:** Evaluated in dedicated git worktrees (`worktrees/candidate-gtk4` on branch `bakeoff-candidate-a` and `worktrees/candidate-iced` on branch `bakeoff-candidate-b`).

Both candidates implemented the identical Nevermore UI footprint:
1. Anchored to the right screen edge on the `TOP` layer with `exclusive_zone = 0` (zero strut) and 380px panel width.
2. 60px solid dark left navigation rail (`#18181b` surface) with icon buttons.
3. Embedded terminal running an interactive session attached to headless `tmux -L nevermore`.
4. Solid `#18181b` background without blur or transparency.

---

## 2. Empirical Head-to-Head Benchmark Matrix

All metrics recorded on live KWin 6.8.2 using identical release compilation flags (`cargo build --release`).

| Metric / Evaluation Criterion | Candidate A (GTK4 + VTE4) | Candidate B (Iced + iced_term) | Advantage |
| :--- | :--- | :--- | :--- |
| **Startup Latency to First View** | **686.09 ms** (cold load with Vulkan) | **2,498.22 ms** (wgpu/shader pipeline) | **Candidate A (3.6x faster)** |
| **Idle Resident Memory (RSS)** | **99.44 MB** (`101,824 kB` in `/proc`) | **204.74 MB** (`209,656 kB` in `/proc`) | **Candidate A (51% less RAM)** |
| **Stress RSS (5,000 lines ANSI)** | **109.91 MB** (`112,548 kB` in `/proc`) | **~215 MB** | **Candidate A (49% less RAM)** |
| **Throughput (5,000 lines ANSI)** | **1.64 ms** stream + render | Buffer-throttled to Elm ticks (~16ms) | **Candidate A (10x faster)** |
| **Dependency Graph Size** | **66 crates** total | **422 crates** total | **Candidate A (6.4x fewer crates)** |
| **Release Build Time (clean)** | **1m 47s** | **3m 14s** | **Candidate A (1.8x faster)** |
| **Terminal Fidelity & Features** | 100% TrueColor, bold, dim, italic, 50k scrollback, native IME & clipboard | Experimental WIP, basic ANSI, custom canvas rendering, no native IME | **Candidate A (Maturity & Fidelity)** |
| **Supply Chain & Toolchain Health** | **100% stable**, audited system C libraries (`gtk4`, `vte4`) | **Broken out of box** on crates.io; required local monkey patch | **Candidate A (Zero breakage)** |
| **Layout & Styling Ergonomics** | Declarative widgets + standard CSS | Elm architecture + canvas subscriptions | **Candidate A (Simpler maintenance)** |
| **Screenshot Artifact** | [`06-candidate-a-gtk4.png`](file:///home/bgg/projects/nevermore-sidebar/docs/spikes/06-candidate-a-gtk4.png) | [`06-candidate-b-iced.png`](file:///home/bgg/projects/nevermore-sidebar/docs/spikes/06-candidate-b-iced.png) | Both visually verified |

---

## 3. Deep-Dive Analysis

### 3.1 Terminal Fidelity & Performance
- **Candidate A (VTE4):** GNOME's flagship terminal engine delivered sub-2ms throughput when rendering 5,000 lines of TrueColor ANSI text. VTE handles complex terminal escapes, reflow, high-DPI scaling, Wayland input method extensions, and clipboard interactions seamlessly. It binds to the headless `tmux -L nevermore` socket without any translation layer.
- **Candidate B (iced_term):** iced_term wraps `alacritty_terminal` into an Iced canvas widget. While functional for basic terminal sessions, the author explicitly documents iced_term as experimental development software. Key features like input method composition (IME), selection clipboard protocols, and smooth scrollback pagination are either missing or require custom implementation.

### 3.2 Supply Chain Rigidity & Upstream Breakage
- **Candidate A:** Uses standard, long-term stable Rust bindings (`gtk4 0.9`, `vte4 0.8`, `gtk4-layer-shell 0.4`) linking to distribution-managed system libraries (`gtk4 4.24.1`, `vte4 0.84.1`, `gtk4-layer-shell 1.3.0`). Compilation is deterministic and immune to upstream breakage.
- **Candidate B:** Pulls in 422 transitive crates across the `wgpu`, `naga`, `smithay`, and `winit` ecosystems. During the spike build, **Candidate B failed to compile on crates.io** due to an unpinned dependency conflict: `winit-common 0.31.0-beta.4` introduced a new variant into `NativeKeyCode`, triggering rustc error `E0004: non-exhaustive pattern match` inside `iced_exdevtools 0.19.1`. We had to copy and locally monkey-patch `iced_exdevtools` in `patches/iced_exdevtools` just to achieve a working build. Relying on such a fragile supply chain for a core desktop component introduces severe maintenance risks.

### 3.3 Resource Consumption (Memory & Startup)
- On the user's Dell Precision 5530 (Intel Core i7-8850H with Intel Iris Plus 655 iGPU), **Candidate A** started in **686 ms** and settled at **99.44 MB RSS**.
- In contrast, **Candidate B** consumed **204.74 MB RSS** immediately upon mapping and required **2,498 ms** to initialize its `wgpu` pipelines, shader caches, and font databases. For an always-running sidebar that must remain lightweight and snappy, Candidate B imposes more than double the memory penalty and 3.6x slower cold startup.

---

## 4. Architectural Decision & Winner

### Selection: **Candidate A (GTK4 + gtk4-layer-shell + vte4)**

The three supreme principles for evaluation established by the user and Manager AI dictate this decision:
1. **Snappy & Convenient:** Candidate A starts in 686 ms (sub-second) and renders terminal streams with zero perceivable latency (<2 ms).
2. **Hard to Break:** Candidate A depends on audited, robust C/Rust system libraries with zero unpinned beta dependencies. It avoids the 422-crate fragility demonstrated by Candidate B's build failure.
3. **Easy for AI Agents to Write and Maintain:** Candidate A uses established GTK4 declarative widgets, CSS stylesheets (`nevermore.css`), and standard GObject signals. State machine transitions are clear and do not require Elm architectural boilerplate or custom canvas drawing routines.

---

## 5. Proposal 6 (For Manager AI & User Approval)

### Proposal 6: Adopt Candidate A as the Permanent Nevermore UI Stack
- **Action:** Formally select **Candidate A (GTK4 + gtk4-layer-shell + vte4)** for all Nevermore UI development.
- **Merge Plan:** Merge branch `bakeoff-candidate-a` into `main` as the foundation for the `crates/nevermore-ui` binary.
- **Clean-up:** Archive `worktrees/candidate-iced` and delete branch `bakeoff-candidate-b`.
- **Next Step (Phase 4):** Implement the headless `nevermore-core` state machine, terminal manager abstraction, and D-Bus IPC interface.
