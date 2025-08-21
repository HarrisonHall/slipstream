//! Entry views.

use ratatui::widgets::Clear;

use super::*;

/// Entry view state.
pub struct EntryView {
    /// Index for which result is being displayed.
    /// Index 0 represent the "info" metadata pannel.
    pub result_selection_index: usize,
    /// List of unique binding/command results.
    command_results: Vec<CommandResultContext>,
    /// List of commands that were ran.
    ran_commands: Vec<Arc<String>>,
    /// Whether or not the entry has been read.
    pub has_been_read: bool,
    /// If the entry has been marked important.
    pub important: bool,
}

impl EntryView {
    /// Create a new entry view.
    fn new() -> Self {
        Self {
            result_selection_index: 0,
            command_results: Vec::new(),
            ran_commands: Vec::new(),
            has_been_read: false,
            important: false,
        }
    }

    /// Set the entry as read.
    pub fn set_read(&mut self) {
        self.has_been_read = true;
    }

    /// Set the entry as important.
    pub fn set_imporant(&mut self, important: bool) {
        self.important = important;
    }

    /// Get the currently selected result.
    pub fn get_result(&self) -> Option<&CommandResultContext> {
        self.command_results
            .get(self.result_selection_index.wrapping_sub(1))
    }

    /// Get a list of the executed commands.
    pub fn get_commands(&self) -> &Vec<Arc<String>> {
        &self.ran_commands
    }

    /// Cycle the selected result.
    pub fn scroll(&mut self, by: isize) {
        if self.result_selection_index == 0 {
            return;
        }

        if let Some(result) = self
            .command_results
            .get_mut(self.result_selection_index.wrapping_sub(1))
        {
            if by > 0 {
                result.vertical_scroll =
                    result.vertical_scroll.wrapping_add(by as usize);
            } else {
                if result.vertical_scroll >= by.abs() as usize {
                    result.vertical_scroll =
                        result.vertical_scroll.wrapping_sub(by.abs() as usize);
                }
            }
        }
    }

    /// Cycle the selected result.
    pub fn cycle_result(&mut self, by: i8) {
        if !self.command_results.is_empty() {
            if by > 0 {
                self.result_selection_index =
                    self.result_selection_index.wrapping_add(by as usize);
                if self.result_selection_index >= self.command_results.len() + 1
                {
                    self.result_selection_index = 0;
                }
            } else {
                self.result_selection_index =
                    self.result_selection_index.wrapping_sub(by.abs() as usize);
                if self.result_selection_index >= self.command_results.len() + 1
                {
                    self.result_selection_index = self.command_results.len();
                }
            }
        }
    }

    /// Add result to entry.
    /// This replaces a previous result with the same name.
    pub fn add_result(&mut self, result: CommandResultContext) {
        for ctx in self.command_results.iter_mut() {
            if ctx.binding_name == result.binding_name {
                ctx.result = result.result;
                return;
            }
        }
        self.ran_commands.push(result.binding_name.clone());
        self.command_results.push(result);
    }
}

pub struct EntryViewWidget<'a> {
    view: &'a mut EntryView,
    entry: &'a slipfeed::Entry,
    focus: &'a Focus,
}

impl<'a> EntryViewWidget<'a> {
    pub fn new(
        view: &'a mut EntryView,
        entry: &'a slipfeed::Entry,
        focus: &'a Focus,
    ) -> Self {
        Self { view, entry, focus }
    }
}

impl<'a> Widget for EntryViewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        // Render outline.
        let block = Block::bordered().title(self.entry.title().as_str()).fg(
            match *self.focus {
                Focus::Entry => Color::Green,
                _ => Color::White,
            },
        );
        let inner_block = block.inner(area);
        block.render(area, buf);

        // Render loaded entry.
        let tab_layouts = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[Constraint::Min(1), Constraint::Percentage(100)])
            .split(inner_block);
        let commands = self.view.get_commands();
        ratatui::widgets::Tabs::new(
            ["info"]
                .iter()
                .map(|info| *info)
                .chain(commands.iter().map(|tab| (*tab).as_str()))
                .map(|tab| tab.to_uppercase()),
        )
        .padding("", "")
        .divider(" ")
        // .bg(Color::Green)
        .select(self.view.result_selection_index)
        .highlight_style((Color::Black, Color::Blue))
        .render(tab_layouts[0], buf);
        match self.view.get_result() {
            None => {
                EntryInfoWidget(self.entry).render(tab_layouts[1], buf);
            }
            Some(selected_result) => {
                selected_result.widget().render(tab_layouts[1], buf);
            }
        };
    }
}

/// Widget for displaying entry info.
struct EntryInfoWidget<'a>(&'a slipfeed::Entry);

impl<'a> Widget for EntryInfoWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        // Clear this space!
        Clear.render(area, buf);

        let layouts = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Max(1)])
            .split(area);

        // All text lines.
        let mut top_lines: Vec<Line> = Vec::new();

        // Add author:
        top_lines.push(
            Span::styled(
                format!(
                    "Author: {}",
                    match !self.0.author().is_empty() {
                        true => self.0.author().as_str(),
                        false => "---",
                    }
                ),
                Style::default().fg(Color::LightGreen),
            )
            .into(),
        );

        // Add links:
        let mut link_count = 0;
        if !self.0.source().url.is_empty() {
            link_count += 1;
            top_lines.push(
                Span::styled(
                    format!("[{}] {}", link_count, self.0.source().url),
                    Style::default().fg(Color::Red),
                )
                .into(),
            );
        }
        if !self.0.comments().url.is_empty() {
            link_count += 1;
            top_lines.push(
                Span::styled(
                    format!("[{}] {}", link_count, self.0.comments().url),
                    Style::default().fg(Color::Red),
                )
                .into(),
            );
        }
        for i in 0..self.0.other_links().len() {
            link_count += 1;
            top_lines.push(
                Span::styled(
                    format!("[{}] {}", link_count, self.0.other_links()[i].url),
                    Style::default().fg(Color::Red),
                )
                .into(),
            );
        }

        top_lines.push(
            Span::styled(
                if !self.0.content().is_empty() {
                    self.0.content().as_str()
                } else {
                    "---"
                },
                Style::default().fg(Color::White),
            )
            .into(),
        );

        Paragraph::new(top_lines).render(layouts[0], buf);

        // Bottom text lines.
        let mut bottom_lines: Vec<Line> = Vec::new();

        // Add date:
        bottom_lines.push(
            Line::from(Span::styled(
                format!("{}", self.0.date()),
                Style::default(),
            ))
            .right_aligned(),
        );

        Paragraph::new(bottom_lines).render(layouts[1], buf);
    }
}

/// All command results, stored by entry.
/// Entries can have multiple commands, but only one of each binding.
pub struct EntryViews {
    views: HashMap<slipfeed::Entry, EntryView>,
}

impl EntryViews {
    /// Generate empty EntryViews.
    pub fn new() -> Self {
        Self {
            views: HashMap::new(),
        }
    }

    /// Get the view for an entry.
    pub fn get<'a>(&'a mut self, entry: &slipfeed::Entry) -> &'a mut EntryView {
        if self.views.contains_key(entry) {
            return self.views.get_mut(entry).unwrap();
        }
        self.views.insert(entry.clone(), EntryView::new());
        self.views.get_mut(entry).unwrap()
    }

    // /// Get the view context for an entry.
    // pub fn displayed_results(
    //     &mut self,
    //     entry: &slipfeed::Entry,
    // ) -> Option<&mut CommandResultContext> {
    //     if let Some(view) = self.views.get(entry) {
    //         if view.result_selection_index < view.command_results.len() {
    //             return Some(
    //                 &mut view.command_results[view.result_selection_index],
    //             );
    //         }
    //     }
    //     None
    // }

    // pub fn run_commands<'a>(
    //     &'a mut self,
    //     entry: &slipfeed::Entry,
    // ) -> &'a Vec<Arc<String>> {
    //     if let Some(view) = self.views.get(entry) {
    //         return &view.ran_commands;
    //     }
    //     &self.no_commands
    // }

    // pub fn add_result(
    //     &mut self,
    //     entry: &slipfeed::Entry,
    //     result: CommandResultContext,
    // ) {
    //     if let Some(view) = self.views.get_mut(entry) {
    //         view.add_result(result);
    //     } else {
    //         let mut view = EntryView::new();
    //         view.add_result(result);
    //         self.views.insert(entry.clone(), view);
    //     }
    // }
}
