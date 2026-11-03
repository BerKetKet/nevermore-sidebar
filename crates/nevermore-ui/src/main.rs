//! nevermore-ui: UI binary stub for Nevermore Wayland sidebar.
//! GUI toolkit integration (GTK4 or Iced) will be implemented after the Phase 3 Bake-Off.

use nevermore_core::SidebarStateMachine;

fn main() {
    println!("Nevermore UI stub (Phase 1). Awaiting Phase 3 toolkit bake-off.");
    let sm = SidebarStateMachine::new();
    println!("Initial sidebar state: {:?}", sm.state());
}
