use serde::{Deserialize, Serialize};

/// The four explicit states of the Nevermore sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SidebarState {
    /// Thin invisible right-edge hot strip active; zero keyboard focus; ~0% CPU.
    Hidden,
    /// Hovering the hot edge slides panel in; NO keyboard focus so typing
    /// continues in the user's active window; auto-hides after pointer leaves.
    Peek,
    /// User clicked inside panel or invoked via keyboard; takes keyboard focus
    /// for typing in terminal/notepad; Escape or pointer leave returns to Hidden.
    Engaged,
    /// Pin button keeps panel open and overrides auto-hide until explicitly unpinned.
    Pinned,
}

impl SidebarState {
    /// Whether the panel body is currently visible on screen.
    pub fn is_visible(&self) -> bool {
        matches!(self, Self::Peek | Self::Engaged | Self::Pinned)
    }

    /// Whether the panel has keyboard interactivity enabled.
    pub fn takes_keyboard_focus(&self) -> bool {
        matches!(self, Self::Engaged | Self::Pinned)
    }
}

/// Events driving state transitions in the sidebar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SidebarEvent {
    /// Pointer hovered over the right screen edge hot strip.
    PointerEnterEdge,
    /// Pointer left the right screen edge without entering the panel.
    PointerLeaveEdge,
    /// Pointer entered the expanded sidebar panel body.
    PointerEnterPanel,
    /// Pointer clicked inside the sidebar panel body.
    PointerClickInside,
    /// Pointer left the sidebar panel body.
    PointerLeavePanel,
    /// Physical Escape key pressed.
    EscapePressed,
    /// Pin button toggled.
    TogglePinned,
    /// CLI: `nevermore show [tab]`
    CliShow { tab: Option<String> },
    /// CLI: `nevermore hide`
    CliHide,
    /// CLI: `nevermore toggle`
    CliToggle,
    /// CLI: `nevermore pin`
    CliPin,
}

/// Headless state machine governing the sidebar visibility, focus, and pinned state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarStateMachine {
    current_state: SidebarState,
    active_tab: String,
    was_pinned: bool,
}

impl Default for SidebarStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarStateMachine {
    pub fn new() -> Self {
        Self {
            current_state: SidebarState::Hidden,
            active_tab: "terminal".to_string(),
            was_pinned: false,
        }
    }

    pub fn state(&self) -> SidebarState {
        self.current_state
    }

    pub fn active_tab(&self) -> &str {
        &self.active_tab
    }

    pub fn set_active_tab(&mut self, tab: impl Into<String>) {
        self.active_tab = tab.into();
    }

    /// Transitions the state machine based on an incoming event and returns the new state.
    pub fn transition(&mut self, event: SidebarEvent) -> SidebarState {
        self.current_state = match (self.current_state, event) {
            // Hidden State
            (SidebarState::Hidden, SidebarEvent::PointerEnterEdge) => SidebarState::Peek,
            (SidebarState::Hidden, SidebarEvent::PointerClickInside) => SidebarState::Engaged,
            (SidebarState::Hidden, SidebarEvent::CliShow { tab }) => {
                if let Some(t) = tab {
                    self.active_tab = t;
                }
                SidebarState::Engaged
            }
            (SidebarState::Hidden, SidebarEvent::CliToggle) => SidebarState::Engaged,
            (SidebarState::Hidden, SidebarEvent::CliPin | SidebarEvent::TogglePinned) => {
                self.was_pinned = true;
                SidebarState::Pinned
            }
            (SidebarState::Hidden, _) => SidebarState::Hidden,

            // Peek State (Hovered edge, no keyboard focus)
            (SidebarState::Peek, SidebarEvent::PointerLeaveEdge) => SidebarState::Hidden,
            (SidebarState::Peek, SidebarEvent::PointerLeavePanel) => SidebarState::Hidden,
            (SidebarState::Peek, SidebarEvent::PointerClickInside) => SidebarState::Engaged,
            (SidebarState::Peek, SidebarEvent::EscapePressed) => SidebarState::Hidden,
            (SidebarState::Peek, SidebarEvent::TogglePinned | SidebarEvent::CliPin) => {
                self.was_pinned = true;
                SidebarState::Pinned
            }
            (SidebarState::Peek, SidebarEvent::CliHide) => SidebarState::Hidden,
            (SidebarState::Peek, SidebarEvent::CliToggle) => SidebarState::Hidden,
            (SidebarState::Peek, SidebarEvent::CliShow { tab }) => {
                if let Some(t) = tab {
                    self.active_tab = t;
                }
                SidebarState::Engaged
            }
            (SidebarState::Peek, _) => SidebarState::Peek,

            // Engaged State (Active keyboard input)
            (SidebarState::Engaged, SidebarEvent::EscapePressed) => {
                if self.was_pinned {
                    SidebarState::Pinned
                } else {
                    SidebarState::Hidden
                }
            }
            (SidebarState::Engaged, SidebarEvent::PointerLeavePanel) => {
                if self.was_pinned {
                    SidebarState::Pinned
                } else {
                    SidebarState::Hidden
                }
            }
            (SidebarState::Engaged, SidebarEvent::TogglePinned | SidebarEvent::CliPin) => {
                self.was_pinned = !self.was_pinned;
                if self.was_pinned {
                    SidebarState::Pinned
                } else {
                    SidebarState::Engaged
                }
            }
            (SidebarState::Engaged, SidebarEvent::CliHide) => {
                self.was_pinned = false;
                SidebarState::Hidden
            }
            (SidebarState::Engaged, SidebarEvent::CliToggle) => {
                self.was_pinned = false;
                SidebarState::Hidden
            }
            (SidebarState::Engaged, SidebarEvent::CliShow { tab }) => {
                if let Some(t) = tab {
                    self.active_tab = t;
                }
                SidebarState::Engaged
            }
            (SidebarState::Engaged, _) => SidebarState::Engaged,

            // Pinned State (Overrides auto-hide)
            (SidebarState::Pinned, SidebarEvent::TogglePinned | SidebarEvent::CliPin) => {
                self.was_pinned = false;
                SidebarState::Engaged
            }
            (SidebarState::Pinned, SidebarEvent::CliHide) => {
                self.was_pinned = false;
                SidebarState::Hidden
            }
            (SidebarState::Pinned, SidebarEvent::CliToggle) => {
                self.was_pinned = false;
                SidebarState::Hidden
            }
            (SidebarState::Pinned, SidebarEvent::CliShow { tab }) => {
                if let Some(t) = tab {
                    self.active_tab = t;
                }
                SidebarState::Pinned
            }
            (SidebarState::Pinned, SidebarEvent::EscapePressed) => {
                // Escape in pinned state focuses out without closing panel
                SidebarState::Pinned
            }
            (SidebarState::Pinned, SidebarEvent::PointerLeavePanel) => SidebarState::Pinned,
            (SidebarState::Pinned, _) => SidebarState::Pinned,
        };
        self.current_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_hidden() {
        let sm = SidebarStateMachine::new();
        assert_eq!(sm.state(), SidebarState::Hidden);
        assert!(!sm.state().is_visible());
        assert!(!sm.state().takes_keyboard_focus());
    }

    #[test]
    fn test_edge_hover_leads_to_peek() {
        let mut sm = SidebarStateMachine::new();
        sm.transition(SidebarEvent::PointerEnterEdge);
        assert_eq!(sm.state(), SidebarState::Peek);
        assert!(sm.state().is_visible());
        assert!(!sm.state().takes_keyboard_focus());

        // Pointer leaves edge -> returns to Hidden
        sm.transition(SidebarEvent::PointerLeaveEdge);
        assert_eq!(sm.state(), SidebarState::Hidden);
    }

    #[test]
    fn test_click_inside_engages_keyboard_focus() {
        let mut sm = SidebarStateMachine::new();
        sm.transition(SidebarEvent::PointerEnterEdge);
        assert_eq!(sm.state(), SidebarState::Peek);

        sm.transition(SidebarEvent::PointerClickInside);
        assert_eq!(sm.state(), SidebarState::Engaged);
        assert!(sm.state().takes_keyboard_focus());

        // Escape returns to Hidden
        sm.transition(SidebarEvent::EscapePressed);
        assert_eq!(sm.state(), SidebarState::Hidden);
    }

    #[test]
    fn test_pinned_state_overrides_pointer_leave() {
        let mut sm = SidebarStateMachine::new();
        sm.transition(SidebarEvent::CliShow {
            tab: Some("terminal".into()),
        });
        assert_eq!(sm.state(), SidebarState::Engaged);

        sm.transition(SidebarEvent::TogglePinned);
        assert_eq!(sm.state(), SidebarState::Pinned);

        // Pointer leave should NOT hide pinned panel
        sm.transition(SidebarEvent::PointerLeavePanel);
        assert_eq!(sm.state(), SidebarState::Pinned);

        // Unpinning allows auto-hide
        sm.transition(SidebarEvent::TogglePinned);
        assert_eq!(sm.state(), SidebarState::Engaged);
        sm.transition(SidebarEvent::PointerLeavePanel);
        assert_eq!(sm.state(), SidebarState::Hidden);
    }

    #[test]
    fn test_cli_toggle_behavior() {
        let mut sm = SidebarStateMachine::new();
        sm.transition(SidebarEvent::CliToggle);
        assert_eq!(sm.state(), SidebarState::Engaged);

        sm.transition(SidebarEvent::CliToggle);
        assert_eq!(sm.state(), SidebarState::Hidden);
    }

    #[test]
    fn test_tab_switching_on_show() {
        let mut sm = SidebarStateMachine::new();
        sm.transition(SidebarEvent::CliShow {
            tab: Some("tasks".into()),
        });
        assert_eq!(sm.state(), SidebarState::Engaged);
        assert_eq!(sm.active_tab(), "tasks");
    }
}
