//! Entry views.

use std::ops::Deref;

use ratatui::widgets::Clear;

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
    /// Whether or not the entry has been read.
    pub has_been_read: bool,
    /// If the entry has been marked important.
    pub important: bool,
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
            has_been_read: false,
            important: false,
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
                    result.vertical_scroll.saturating_add(by as usize);
            } else {
                if result.vertical_scroll >= by.abs() as usize {
                    result.vertical_scroll = result
                        .vertical_scroll
                        .saturating_sub(by.abs() as usize);
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

impl Deref for DatabaseEntry {
    type Target = slipfeed::Entry;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

pub struct EntryViewWidget<'a> {
    entry: &'a mut DatabaseEntry,
    focus: &'a Focus,
}

impl<'a> EntryViewWidget<'a> {
    pub fn new(entry: &'a mut DatabaseEntry, focus: &'a Focus) -> Self {
        Self { entry, focus }
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
        let commands = self.entry.get_commands();
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
        .select(self.entry.result_selection_index)
        .highlight_style((Color::Black, Color::Blue))
        .render(tab_layouts[0], buf);
        match self.entry.get_result() {
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

        // Add tags:
        let mut tags: Vec<String> = self
            .0
            .tags()
            .iter()
            .map(|t| format!("<{t}>"))
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

    pub fn add(&mut self, entry: DatabaseEntry) -> Result<()> {
        if self.entries.len() < self.max_size {
            let db_id = entry.db_id;
            self.entries.push(entry);
            self.lookup.insert(db_id, self.entries.len() - 1);
            return Ok(());
        }
        bail!("Entry list at max length");
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn get(&mut self, db_id: EntryDbId) -> Option<&mut DatabaseEntry> {
        match self.lookup.get(&db_id) {
            Some(idx) => Some(&mut self.entries[*idx]),
            None => None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &DatabaseEntry> {
        self.entries.iter()
    }

    pub fn iter_entries(&self) -> impl Iterator<Item = &slipfeed::Entry> {
        self.entries.iter().map(|e| &e.entry)
    }

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
