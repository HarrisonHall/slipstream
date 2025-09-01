//! Reader focus state.

use super::*;

pub struct TerminalState {
    /// The window.
    pub window: usize,
    /// Whether or not the terminal has focus.
    pub has_focus: bool,
    /// Full terminal window size.
    pub size: (u16, u16),
    /// Command width.
    pub command_width: u16,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            window: 0,
            has_focus: true,
            size: (0, 0),
            command_width: MIN_HORIZONTAL_WIDTH,
        }
    }
}

pub struct InteractionState {
    /// Which section should have focus.
    pub focus: Focus,
    /// Current selected entry index.
    pub selection: usize,
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            focus: Focus::List,
            selection: 0,
        }
    }
}

/// Focus mode.
pub enum Focus {
    List,
    Entry,
    Menu,
    Command {
        command: String,
        message: Option<String>,
    },
}

impl Focus {
    pub fn swap(&mut self) {
        *self = match *self {
            Focus::List => Focus::Entry,
            Focus::Entry => Focus::List,
            Focus::Menu => Focus::List,
            Focus::Command {
                command: _,
                message: _,
            } => Focus::List,
        };
    }

    pub fn toggle_menu(&mut self) {
        *self = match *self {
            Focus::Menu => Focus::List,
            _ => Focus::Menu,
        }
    }
}
