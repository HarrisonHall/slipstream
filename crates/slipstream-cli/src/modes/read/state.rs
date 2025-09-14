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
    /// Inputs from the previous render.
    pub last_frame_inputs: LastFrameInputs,
}

impl TerminalState {
    pub fn get_paging_lines(&self) -> i16 {
        if self.size.0 > MIN_HOR_WIDTH {
            // Handle horizontal paging.
            return self.size.1 as i16 - (2 * SCROLL_WINDOW) as i16;
        } else {
            // Handle vertical paging.
            return (self.size.1 / 2) as i16 - (2 * SCROLL_WINDOW) as i16;
        }
    }
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            window: 0,
            has_focus: true,
            size: (0, 0),
            command_width: MIN_HOR_WIDTH,
            last_frame_inputs: LastFrameInputs::new(),
        }
    }
}

pub struct InteractionState {
    /// Which section should have focus.
    pub focus: Focus,
    /// Current selected entry index.
    pub selection: usize,
    /// Previous search.
    pub previous_search: Vec<DatabaseSearch>,
}

impl InteractionState {
    pub fn scroll(&mut self, amount: i16, entries: &DatabaseEntryList) {
        if amount > 0 {
            let max_index = entries.len().saturating_sub(1);
            self.selection = self
                .selection
                .saturating_add(amount as usize)
                .min(max_index);
        } else {
            self.selection =
                self.selection.saturating_sub(amount.abs() as usize);
        }
    }
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            focus: Focus::List,
            selection: 0,
            previous_search: Vec::new(),
        }
    }
}

/// Focus mode.
pub enum Focus {
    List,
    Entry,
    Menu {
        scroll: u16,
    },
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
            Focus::Menu { .. } => Focus::List,
            Focus::Command {
                command: _,
                message: _,
            } => Focus::List,
        };
    }

    pub fn toggle_menu(&mut self) {
        *self = match *self {
            Focus::Menu { .. } => Focus::List,
            _ => Focus::Menu { scroll: 0 },
        }
    }
}

pub struct LastFrameInputs {
    click: Option<(u16, u16)>,
    scroll: Option<ScrollDirection>,
}

impl LastFrameInputs {
    pub fn new() -> Self {
        Self {
            click: None,
            scroll: None,
        }
    }

    pub fn clear(&mut self) {
        self.click = None;
        self.scroll = None;
    }

    pub fn handle_event(&mut self, event: crossterm::event::MouseEvent) {
        match event.kind {
            event::MouseEventKind::Down(_button) => {
                self.click = Some((event.column, event.row));
            }
            event::MouseEventKind::ScrollDown => {
                self.scroll = Some(ScrollDirection::Down);
            }
            event::MouseEventKind::ScrollUp => {
                self.scroll = Some(ScrollDirection::Up);
            }
            _ => {}
        };
    }

    pub fn scrolled_up(&self) -> bool {
        if let Some(ScrollDirection::Up) = self.scroll {
            return true;
        }
        return false;
    }

    pub fn scrolled_down(&self) -> bool {
        if let Some(ScrollDirection::Down) = self.scroll {
            return true;
        }
        return false;
    }

    pub fn clicked(&self, area: Rect) -> bool {
        if let Some((x, y)) = self.click {
            return area.contains(ratatui::layout::Position { x, y });
        }
        return false;
    }
}

enum ScrollDirection {
    Up,
    Down,
}
