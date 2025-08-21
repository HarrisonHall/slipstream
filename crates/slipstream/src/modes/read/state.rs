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
}

pub struct UpdaterState {
    /// Slipstream updater handle.
    pub updater: Arc<Mutex<Updater>>,
    /// Timestamp of last slipfeed update.
    pub last_update: chrono::DateTime<chrono::Utc>,
    /// Updater future that returns a list of new entries.
    pub future: Option<BoxFuture<'static, Result<Vec<slipfeed::Entry>>>>,
}

impl UpdaterState {
    pub fn new(updater: Arc<Mutex<Updater>>) -> Self {
        Self {
            updater,
            last_update: chrono::Local::now().to_utc(),
            future: None,
        }
    }
}

impl Focus {
    pub fn swap(&mut self) {
        *self = match *self {
            Focus::List => Focus::Entry,
            Focus::Entry => Focus::List,
            Focus::Menu => Focus::List,
        };
    }

    pub fn toggle_menu(&mut self) {
        *self = match *self {
            Focus::Menu => Focus::List,
            _ => Focus::Menu,
        }
    }
}
