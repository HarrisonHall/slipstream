//! Entry views.

use std::ops::Deref;

use ratatui::{
    layout::Flex,
    widgets::{Clear, Wrap},
};

use super::*;

/// Entry view state.
#[derive(Debug, Clone)]
pub struct DatabaseEntry {
    /// Underlying entry.
    pub entry: slipfeed::Entry,
    /// Database id for the entry.
    pub db_id: EntryDbId,
    /// Index for which result is being displayed.
    /// Index 0 represent the "info" metadata pannel.
    pub result_selection_index: usize,
    /// List of unique binding/command results.
    command_results: Vec<CommandResultContext>,
    /// List of commands that were ran.
    ran_commands: Vec<Arc<String>>,
}

impl DatabaseEntry {
    /// Create a new entry view.
    pub fn new(entry: slipfeed::Entry, id: EntryDbId) -> Self {
        Self {
            entry,
            db_id: id,
            result_selection_index: 0,
            command_results: Vec::new(),
            ran_commands: Vec::new(),
        }
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
    pub fn scroll(&mut self, by: i16) {
        if self.result_selection_index == 0 {
            return;
        }

        if let Some(result) = self
            .command_results
            .get_mut(self.result_selection_index.wrapping_sub(1))
        {
            if by > 0 {
                result.vertical_scroll =
                    result.vertical_scroll.saturating_add(by as usize);
            } else {
                result.vertical_scroll =
                    result.vertical_scroll.saturating_sub(by.abs() as usize);
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
            if ctx.command.name == result.command.name {
                ctx.result = result.result;
                return;
            }
        }
        self.ran_commands.push(result.command.name.clone());
        self.command_results.push(result);
    }
}

impl EntryExt for DatabaseEntry {
    fn to_atom(&self, config: &Config) -> atom_syndication::Entry {
        let mut atom_entry = self.entry.to_atom(config);
        atom_entry.id = format!("{}", self.db_id);
        return atom_entry;
    }
}

impl Deref for DatabaseEntry {
    type Target = slipfeed::Entry;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

pub struct EntryViewWidget<'a> {
    entry: &'a mut DatabaseEntry,
    config: &'a Config,
    interaction_state: &'a InteractionState,
    terminal_state: &'a TerminalState,
}

impl<'a> EntryViewWidget<'a> {
    pub fn new(
        entry: &'a mut DatabaseEntry,
        config: &'a Config,
        interaction_state: &'a InteractionState,
        terminal_state: &'a TerminalState,
    ) -> Self {
        Self {
            entry,
            config,
            interaction_state,
            terminal_state,
        }
    }
}

impl<'a> Widget for EntryViewWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        // Render outline.
        let block =
            Block::bordered()
                .title(self.entry.title().as_str())
                .fg(match self.interaction_state.focus {
                    Focus::Entry => Color::Green,
                    _ => Color::White,
                });
        let inner_block = block.inner(area);
        block.render(area, buf);

        // Render loaded entry.
        let tab_layouts = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[Constraint::Min(1), Constraint::Percentage(100)])
            .split(inner_block);
        // let commands = self.entry.get_commands();
        let commands = ["info"]
            .iter()
            .map(|info| *info)
            .chain(self.entry.get_commands().iter().map(|tab| (*tab).as_str()));
        {
            let mut rect = tab_layouts[0];
            let mut idx = self.entry.result_selection_index;
            rect.height = 1;
            for (i, command) in commands.enumerate() {
                rect.width = command.len() as u16;
                if self.terminal_state.last_frame_inputs.clicked(rect) {
                    idx = i;
                    break;
                }
                rect.x += rect.width + 1;
            }
            self.entry.result_selection_index = idx;
        }

        let commands = ["info"]
            .iter()
            .map(|info| *info)
            .chain(self.entry.get_commands().iter().map(|tab| (*tab).as_str()));
        let tabs =
            ratatui::widgets::Tabs::new(commands.map(|tab| tab.to_uppercase()))
                .padding("", "")
                .divider(" ")
                // .bg(Color::Green)
                .select(self.entry.result_selection_index)
                .highlight_style((Color::Black, Color::Blue));
        tabs.render(tab_layouts[0], buf);
        match self.entry.get_result() {
            None => {
                EntryInfoWidget(self.entry, self.config)
                    .render(tab_layouts[1], buf);
            }
            Some(selected_result) => {
                selected_result.widget().render(tab_layouts[1], buf);
            }
        };
    }
}

/// Widget for displaying entry info.
struct EntryInfoWidget<'a>(&'a slipfeed::Entry, &'a Config);

impl<'a> Widget for EntryInfoWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        // Clear this space!
        Clear.render(area, buf);

        let layouts = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Max(5),
                Constraint::Fill(1),
                Constraint::Max(1),
            ])
            .flex(Flex::Legacy)
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

        // Add tags:
        let mut tags: Vec<String> = self
            .0
            .tags()
            .iter()
            .filter(|t| !self.1.read.tags.hidden.contains(t.as_ref()))
            .map(|t| format!("#{t}"))
            .collect::<Vec<String>>();
        tags.sort();
        top_lines.push(Line::styled(
            tags.join(", "),
            Style::default().fg(Color::Yellow),
        ));

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

        Paragraph::new(top_lines)
            .wrap(Wrap { trim: false })
            .render(layouts[0], buf);

        if !self.0.content().is_empty() {
            Paragraph::new(tui_markdown::from_str(self.0.content()))
                .left_aligned()
                .wrap(Wrap { trim: false })
                .render(layouts[1], buf);
        } else {
            Span::styled("---", Style::default().fg(Color::Gray))
                .render(layouts[1], buf);
        }

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

        Paragraph::new(bottom_lines).render(layouts[2], buf);
    }
}

/// In-memory state of all entries.
#[derive(Debug, Clone)]
pub struct DatabaseEntryList {
    max_size: usize,
    entries: Vec<DatabaseEntry>,
    lookup: HashMap<EntryDbId, usize>,
}

impl DatabaseEntryList {
    /// Generate empty EntryViews.
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            entries: Vec::new(),
            lookup: HashMap::new(),
        }
    }

    /// Add an entry to the list.
    pub fn add(&mut self, entry: DatabaseEntry) -> Result<()> {
        if self.entries.len() < self.max_size {
            let db_id = entry.db_id;
            self.entries.push(entry);
            self.lookup.insert(db_id, self.entries.len() - 1);
            return Ok(());
        }
        bail!("Entry list at max length ({}).", self.max_size);
    }

    /// Get the length of the list.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Get an entry in the list, by id.
    #[allow(unused)]
    pub fn get(&self, db_id: EntryDbId) -> Option<&DatabaseEntry> {
        match self.lookup.get(&db_id) {
            Some(idx) => Some(&self.entries[*idx]),
            None => None,
        }
    }

    /// Get a mutable entry in the list, by id.
    pub fn get_mut(&mut self, db_id: EntryDbId) -> Option<&mut DatabaseEntry> {
        match self.lookup.get(&db_id) {
            Some(idx) => Some(&mut self.entries[*idx]),
            None => None,
        }
    }

    /// Get the first entry in the list.
    pub fn first(&self) -> Option<&DatabaseEntry> {
        self.entries.first()
    }

    /// Get the last entry in the list.
    pub fn last(&self) -> Option<&DatabaseEntry> {
        self.entries.last()
    }

    /// Iterate the list.
    pub fn iter(&self) -> impl Iterator<Item = &DatabaseEntry> {
        self.entries.iter()
    }

    /// Iterate the list's slipfeed entries.
    pub fn iter_entries(&self) -> impl Iterator<Item = &slipfeed::Entry> {
        self.entries.iter().map(|e| &e.entry)
    }

    /// Turn list into an atom syndication.
    pub fn syndicate(&self, name: impl AsRef<str>, config: &Config) -> String {
        let mut syn = atom::FeedBuilder::default();
        syn.title(name.as_ref())
            .author(atom::PersonBuilder::default().name("slipstream").build());
        for entry in self.iter() {
            syn.entry(entry.to_atom(config));
        }
        syn.build().to_string()
    }
}

impl std::ops::Index<usize> for DatabaseEntryList {
    type Output = DatabaseEntry;

    fn index(&self, index: usize) -> &Self::Output {
        self.entries.index(index)
    }
}

impl std::ops::IndexMut<usize> for DatabaseEntryList {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.entries.index_mut(index)
    }
}
