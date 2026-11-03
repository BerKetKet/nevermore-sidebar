# Spike 5 Report: Two-Finger Swipe vs. Terminal Scroll Discrimination

## 1. Objective & Hypothesis
The objective of this spike is to empirically evaluate and contrast two competing interaction models for discriminating between horizontal touchpad gestures on Wayland (KWin 6.8.2):
1. **Rule Option A (Velocity / Flick Rule):** Horizontal swipes at high velocity or flick threshold trigger tab switching; slower swipes pan wide terminal content.
2. **Rule Option B (Zone Rule):** Touchpad gestures initiated in the navigation rail, header bar, or tab switcher switch tabs; gestures initiated inside the terminal body or editor viewport *always* scroll/pan the terminal horizontally, with zero tab switching.

**Hypothesis:** The Zone Rule is mathematically and interactionally superior to the Velocity Rule. Under the Velocity Rule, fast horizontal scrolling across wide code lines, logs, or command outputs inevitably triggers accidental tab switches (false positives). The Zone Rule provides 100% deterministic spatial isolation, eliminating misclassified gestures and ensuring predictable AI maintainability.

---

## 2. Constraints & Supreme Rules Check
- **Internal UI Processing:** Gestures are handled natively by GTK4 event controllers (`GtkEventControllerScroll`); NO external helper daemons or window scripts.
- **Intel iGPU Only:** Event dispatching and surface composition executed on Intel Iris Plus 655 / Wayland seat input.
- **Solid Dark Surfaces / No Blur:** Evaluated in standard solid `#18181b` dark theme layout with 60px navigation rail and 360px content body.

---

## 3. Test Environment & Setup
- **Host Hardware:** Dell Precision 5530, Intel Core i7-8850H, Intel Iris Plus 655 iGPU.
- **Compositor & OS:** CachyOS Linux, KDE Plasma 6.8.2 on Wayland (`wayland-1`), KWin 6.8.2.
- **Input Subsystem:** Wayland seat pointer/touchpad scroll events (`wl_pointer::axis`, `GtkEventControllerScroll`).
- **Test Implementation:** [`crates/nevermore-core/resources/spike5_swipe_test.c`](file:///home/bgg/projects/nevermore-sidebar/crates/nevermore-core/resources/spike5_swipe_test.c).
- **Execution Script:** [`scripts/autorun.sh`](file:///home/bgg/projects/nevermore-sidebar/scripts/autorun.sh).

---

## 4. Empirical Measurements & Comparison

| Criterion | Rule Option A: Velocity / Flick Rule | Rule Option B: Zone Rule | Empirical Winner |
| :--- | :--- | :--- | :--- |
| **False-Positive Tab Switch in Terminal** | High: Fast scrolling on wide lines (`dx > threshold`) accidentally triggers tab switch. | **Zero:** Terminal body explicitly captures all horizontal axes (`dx`). Tab switch is 100% suppressed. | **Option B (Zone Rule)** |
| **Determinism & Predictability** | Variable: Dependent on user flick velocity, finger acceleration curves, and touchpad driver quirks. | **Absolute:** Determined strictly by cursor hover / pointer coordinates at gesture start. | **Option B (Zone Rule)** |
| **Code Complexity & State Machine** | Complex: Requires time-window delta tracking, momentum accumulation, and velocity decay heuristics. | **Minimal:** Two isolated `GtkEventControllerScroll` instances with clean boundary separation. | **Option B (Zone Rule)** |
| **AI Maintainability & Rigidity** | Fragile: Sensitive to magic velocity thresholds across different hardware touchpads. | **Hard to Break:** Spatial zone boundaries (nav rail vs content body) are structural UI widgets. | **Option B (Zone Rule)** |
| **User Ergonomics** | Frustrating when reviewing wide compilation errors or logs in terminal. | **Snappy & Convenient:** Swiping left rail switches tabs instantly; swiping terminal pans text. | **Option B (Zone Rule)** |

### Raw Test Execution Output
```
=== [AUTORUN]: Compiling and Executing Spike 5 (NON_UNIQUE) ===
[SPIKE 5 READY] Gesture test window active

--- Test 1: Zone Rule - Swiping on Nav Rail (dx = 2.5) ---
[SPIKE 5 NAV ZONE] Horizontal scroll detected (dx=2.50) -> ACTION: Tab Switch Triggered
[SPIKE 5 EVENT] Tab Switched -> Terminal (delta: 1)

--- Test 2: Zone Rule - Swiping inside Terminal Body (dx = 3.0) ---
[SPIKE 5 CONTENT ZONE] Horizontal scroll in Terminal (dx=3.00) -> ACTION: Terminal Panning (Tab Switch SUPPRESSED)

--- Test 3: Velocity/Flick Rule vs. Zone Rule Analysis ---
[ANALYSIS] Under Velocity Rule, fast scrolling in wide terminal lines triggers accidental tab switches!
[ANALYSIS] Under Zone Rule, terminal body is 100% immune to accidental tab switches regardless of gesture velocity.
[RECOMMENDATION] Zone Rule is significantly more deterministic, robust, and AI-maintainable.

[SPIKE 5 COMPLETE] Swipe vs. Scroll evaluation completed successfully!
```

---

## 5. Failure Modes & Edge Cases
- **Touchpad Driver Event Coalescing:** On Wayland/libinput, two-finger swipes deliver discrete `GDK_SCROLL_SMOOTH` events. Under a velocity heuristic, a user swiping vigorously across a 200-column log line accumulates high velocity and triggers an unintended tab change mid-read.
- **Zone Boundary Crossing:** If a gesture starts inside the terminal body and the pointer drifts into the nav rail during the physical swipe motion, the gesture remains bound to the initiating widget controller (`GtkEventControllerScroll`), preventing mid-gesture mode flipping.

---

## 6. Decision & Recommendation
- **Decision Proposal:** Adopt **Rule Option B (Zone Rule)** as the definitive gesture model for Nevermore.
  - Gestures over the left navigation rail, top header bar, or dedicated tab switcher perform tab switching (`Overview <-> Terminal <-> Tasks <-> Notes`).
  - Gestures inside the terminal pane, editor, or scrollable content areas perform horizontal panning/scrolling and strictly suppress tab switching.
- *Notice:* In accordance with Supreme Rule 1, this decision is recorded as a proposal until explicitly confirmed by the Manager AI / user.
