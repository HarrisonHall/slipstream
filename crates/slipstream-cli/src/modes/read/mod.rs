//!!utput
//! Fields: time (relative),

use super::*;

mod command;
mod command_mode;
mod config;
mod entry;
mod keyboard;
mod menu;
mod state;

pub use command::*;
pub use config::*;
pub use entry::*;
pub use keyboard::*;
pub use state::*;

use std::time::Duration;

use ratatui::DefaultTerminal;
use ratatui::buffer::Buffer;
use ratatui::style::Stylize;
use ratatui::{
    crossterm::event::{self, Event},
    widgets::{Block, Paragraph, Widget},
};
use ratatui::{
    prelude::{
        Color, Constraint, Direction, Layout, Line, Rect, Span, Style, Text,
    },
    // style::Stylize,
};
use tokio::task::JoinHandle;

type Terminal = DefaultTerminal;

/// How often to refresh the screen without input.
const REFRESH_DELTA: f32 = 0.25;
/// Minimum height of the screen.
const MIN_VER_HEIGHT: u16 = 20;
/// The minimum terminal width to support horizontal mode.
const MIN_HOR_WIDTH: u16 = 120;
/// The C-c quit key event.
const CONTROL_C: KeyEvent =
    KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);

/// Perform the reader action.
pub async fn read(
    config: Arc<Config>,
    updater: UpdaterHandle,
    cancel_token: CancellationToken,
) -> Result<()> {
    // Disable logging to stdout.
    get_logger().set_writing(false)?;

    // Show reader.
    let mut terminal = ratatui::init();
    let _kb_cap = MouseCapture::new()?;

    let mut reader = Reader::new(config, updater, cancel_token)?;

    // Update reader on load.
    reader
        .update_entries(vec![DatabaseSearch::Latest], OffsetCursor::Latest)
        .await;

    // Run loop.
    let result = reader.run(&mut terminal).await;

    // Restore terminal.
    ratatui::restore();
    get_logger().set_writing(true)?;
    result
}

/// Terminal reader.
struct Reader {
    /// Slipstream configuration.
    config: Arc<Config>,
    /// State of the updating logic.
    updater: UpdaterHandle,
    /// Refresh future.
    refresh: Option<JoinHandle<DatabaseEntryList>>,
    /// Futures for binding commands run on entries.
    command_futures:
        tokio::task::JoinSet<(EntryDbId, command::CommandResultContext)>,
    /// Entries.
    entries: DatabaseEntryList,
    /// Cached of the terminal.
    terminal_state: TerminalState,
    /// State of user interaction.
    interaction_state: InteractionState,
    /// Cancellation token.
    cancel_token: CancellationToken,
}

impl Reader {
    /// Create a new reader.
    fn new(
        config: Arc<Config>,
        updater: UpdaterHandle,
        cancel_token: CancellationToken,
    ) -> Result<Self> {
        Ok(Self {
            config,
            updater,
            refresh: None,
            command_futures: tokio::task::JoinSet::new(),
            entries: DatabaseEntryList::new(0),
            terminal_state: TerminalState::default(),
            interaction_state: InteractionState::default(),
            cancel_token,
        })
    }

    /// Run the reader.
    async fn run(&mut self, terminal: &mut Terminal) -> Result<()> {
        'reader: loop {
            // Check if quitting.
            if self.cancel_token.is_cancelled() {
                break 'reader Ok(());
            }

            // Draw reader.
            terminal.draw(|f| {
                let area = f.area();
                let buf = f.buffer_mut();

                // Update components.
                self.terminal_state.size = (area.width, area.height);

                // Do not render below minimum size.
                if !self.check_size_or_render(buf) {
                    return;
                }

                // Render the correct widget.
                match &self.interaction_state.focus {
                    Focus::Menu { .. } => {
                        menu::MenuWidget::new(self).render(area, buf);
                    }
                    _ => {
                        ReaderWidget::new(self).render(area, buf);
                    }
                };
            })?;

            // Poll input.
            self.terminal_state.last_frame_inputs.clear();
            if self.handle_input().await.is_err() {
                self.cancel_token.cancel();
                break 'reader Ok(());
            }

            // Manage updater.
            self.check_for_updates().await;

            // Sync current with db.
            if self.interaction_state.selection < self.entries.len() {
                // TODO: Read hook!
                // let entry = &mut self.entries[self.interaction_state.selection];
                // self.updater.toggle_read(entry.db_id, true).await;
                // self.updater.update_view(entry).await;
            }
        }
    }
}

// Draw logic.
impl Reader {
    /// Get the selected entry.
    fn get_selected_entry_mut(&mut self) -> Option<&mut DatabaseEntry> {
        if self.interaction_state.selection < self.entries.len() {
            return Some(&mut self.entries[self.interaction_state.selection]);
        }

        return None;
    }

    /// Check the size.
    /// If the buffer size is too small, this returns false and renders a notification.
    fn check_size_or_render(&mut self, buf: &mut Buffer) -> bool {
        if self.terminal_state.size.0 < MIN_VER_HEIGHT
            || self.terminal_state.size.1
                < (2 * self.config.read.scroll_buffer as u16) + 5
        {
            let area = buf.area;
            ratatui::widgets::Paragraph::new("Too Small")
                .block(
                    ratatui::widgets::Block::new()
                        .style(Style::new().bg(Color::White).fg(Color::Black))
                        .padding(ratatui::widgets::Padding::new(
                            0,
                            0,
                            area.height / 2,
                            0,
                        )),
                )
                .render(area, buf);
            return false;
        }
        return true;
    }

    /// Handle input.
    /// Quits on error.
    async fn handle_input(&mut self) -> Result<()> {
        // Wait for input for REFRESH_DELTA.
        if terminal_input_ready(REFRESH_DELTA).await {
            // It's guaranteed that the `read()` won't block when the `poll()`
            // function returns `true`.
            match event::read()? {
                Event::FocusGained => self.terminal_state.has_focus = true,
                Event::FocusLost => self.terminal_state.has_focus = false,
                Event::Key(key) => {
                    if key == CONTROL_C {
                        self.cancel_token.cancel();
                        return Ok(());
                    }
                    match &self.interaction_state.focus {
                        Focus::Command { .. } => {
                            self.handle_command_mode_input(&key).await?;
                        }
                        _ => {
                            let command =
                                self.config.read.get_key_command(&key);
                            self.run_command(command).await?;
                        }
                    }
                }
                Event::Mouse(event) => {
                    self.terminal_state.last_frame_inputs.handle_event(event);
                }
                Event::Resize(width, height) => {
                    self.terminal_state.size = (width, height);
                }
                _ => {}
            }
        }

        // Handle queued input.
        if self.terminal_state.last_frame_inputs.scrolled_up() {
            let scroll = -(self.config.read.scroll as i16);
            match self.interaction_state.focus {
                Focus::List => {
                    self.interaction_state.scroll(scroll, &self.entries);
                }
                Focus::Entry => {
                    if let Some(entry) = self.get_selected_entry_mut() {
                        entry.scroll(scroll);
                    }
                }
                _ => {}
            }
        }
        if self.terminal_state.last_frame_inputs.scrolled_down() {
            let scroll = self.config.read.scroll as i16;
            match self.interaction_state.focus {
                Focus::List => {
                    self.interaction_state.scroll(scroll, &self.entries);
                }
                Focus::Entry => {
                    if let Some(entry) = self.get_selected_entry_mut() {
                        entry.scroll(scroll);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Run command.
    async fn run_command(&mut self, command: Commandish) -> Result<()> {
        match command {
            Commandish::CustomCommandRef(name) => {
                tracing::error!("Invalid command name: {}", name.as_str());
            }
            Commandish::CustomCommandFull(custom_command) => {
                if custom_command.save {
                    self.entries[self.interaction_state.selection].add_result(
                        command::CommandResultContext::new(
                            custom_command.clone(),
                        ),
                    );
                }
                self.command_futures.spawn(Reader::run_shell_command(
                    custom_command,
                    self.entries[self.interaction_state.selection].clone(),
                    self.terminal_state.command_width,
                ));
            }
            Commandish::Literal(command) => {
                self.run_command_literal(command).await?
            }
        }
        Ok(())
    }

    /// Run built-in command.
    async fn run_command_literal(
        &mut self,
        command: ReadCommandLiteral,
    ) -> Result<()> {
        match command {
            ReadCommandLiteral::None => {}
            ReadCommandLiteral::Quit => {
                if let Focus::Menu { .. } = &self.interaction_state.focus {
                    self.interaction_state.focus.toggle_menu();
                } else {
                    self.cancel_token.cancel();
                }
                return Ok(());
            }
            ReadCommandLiteral::Update => {
                self.update_entries(
                    vec![DatabaseSearch::Latest],
                    OffsetCursor::Latest,
                )
                .await;
            }
            ReadCommandLiteral::Down => match self.interaction_state.focus {
                Focus::List => {
                    if self.interaction_state.selection + 1 < self.entries.len()
                    {
                        self.interaction_state.selection += 1;
                    }
                }
                Focus::Entry => {
                    if self.interaction_state.selection < self.entries.len() {
                        self.entries[self.interaction_state.selection]
                            .scroll(1);
                    }
                }
                Focus::Menu { scroll } => {
                    self.interaction_state.focus = Focus::Menu {
                        scroll: scroll.saturating_add(1),
                    };
                }
                Focus::Command { .. } => {}
            },
            ReadCommandLiteral::Up => match self.interaction_state.focus {
                Focus::List => {
                    if (self.interaction_state.selection as isize) - 1 >= 0 {
                        self.interaction_state.selection -= 1;
                    }
                }
                Focus::Entry => {
                    if self.interaction_state.selection < self.entries.len() {
                        self.entries[self.interaction_state.selection]
                            .scroll(-1);
                    }
                }
                Focus::Menu { scroll } => {
                    self.interaction_state.focus = Focus::Menu {
                        scroll: scroll.saturating_sub(1),
                    };
                }
                Focus::Command { .. } => {}
            },
            ReadCommandLiteral::Left => {
                if self.interaction_state.selection < self.entries.len() {
                    self.entries[self.interaction_state.selection]
                        .cycle_result(-1);
                }
            }
            ReadCommandLiteral::Right => {
                if self.interaction_state.selection < self.entries.len() {
                    self.entries[self.interaction_state.selection]
                        .cycle_result(1);
                }
            }
            ReadCommandLiteral::PageDown => {
                match self.interaction_state.focus {
                    Focus::List => {
                        self.interaction_state.scroll(
                            self.terminal_state.get_paging_lines(&self.config),
                            &self.entries,
                        );
                    }
                    Focus::Entry => {
                        let paging_lines =
                            self.terminal_state.get_paging_lines(&self.config);
                        if let Some(entry) = self.get_selected_entry_mut() {
                            entry.scroll(paging_lines);
                        }
                    }
                    Focus::Menu { .. } => {}
                    Focus::Command { .. } => {}
                }
            }
            ReadCommandLiteral::PageUp => match self.interaction_state.focus {
                Focus::List => {
                    self.interaction_state.scroll(
                        -self.terminal_state.get_paging_lines(&self.config),
                        &self.entries,
                    );
                }
                Focus::Entry => {
                    let paging_lines =
                        -self.terminal_state.get_paging_lines(&self.config);
                    if let Some(entry) = self.get_selected_entry_mut() {
                        entry.scroll(paging_lines);
                    }
                }
                Focus::Menu { .. } => {}
                Focus::Command { .. } => {}
            },
            ReadCommandLiteral::Swap => {
                self.interaction_state.focus.swap();
            }
            ReadCommandLiteral::Menu => {
                self.interaction_state.focus.toggle_menu();
            }
            ReadCommandLiteral::CommandMode => {
                self.interaction_state.focus = Focus::Command {
                    command: String::new(),
                    message: None,
                };
            }
            ReadCommandLiteral::SearchMode => {
                self.interaction_state.focus = Focus::Command {
                    command: "/".into(),
                    message: None,
                };
            }
            ReadCommandLiteral::PageForwards => {
                let offset = if let Some(entry) = self.entries.last() {
                    OffsetCursor::Before(entry.date().clone())
                } else {
                    OffsetCursor::Latest
                };
                self.update_entries(
                    self.interaction_state.previous_search.clone(),
                    offset,
                )
                .await;
            }
            ReadCommandLiteral::PageBackwards => {
                let offset = if let Some(entry) = self.entries.first() {
                    OffsetCursor::After(entry.date().clone())
                } else {
                    OffsetCursor::Latest
                };
                self.update_entries(
                    self.interaction_state.previous_search.clone(),
                    offset,
                )
                .await;
            }
            ReadCommandLiteral::Command(command) => {
                if self.interaction_state.selection < self.entries.len() {
                    if let Err(e) =
                        self.handle_command_mode_command(&command).await
                    {
                        tracing::error!("Failed to run command: {}", e);
                    }
                }
            }
        };

        Ok(())
    }

    /// Run a custom shell command.
    /// This replaces select substrings of the shell command with values from the
    /// entry.
    async fn run_shell_command(
        custom_command: CustomCommand,
        entry: DatabaseEntry,
        width: u16,
    ) -> (EntryDbId, command::CommandResultContext) {
        // Build command.
        let mut shell_command: Vec<String> = (*custom_command.command).clone();

        for command in shell_command.iter_mut() {
            // Add links.
            *command = command.replace("{{link.url}}", &entry.source().url);
            let mut link_count: usize = 0;
            if !entry.source().url.is_empty() {
                link_count += 1;
                *command = command.replace(
                    &format!("{{{{link.url{}}}}}", link_count),
                    &entry.source().url,
                );
            }
            if !entry.comments().url.is_empty() {
                link_count += 1;
                *command = command.replace(
                    &format!("{{{{link.url{}}}}}", link_count),
                    &entry.comments().url,
                );
            }
            for i in 0..entry.other_links().len() {
                link_count += 1;
                *command = command.replace(
                    &format!("{{{{link.url{}}}}}", link_count),
                    &entry.other_links()[i].url,
                );
            }

            // Add link name.
            if command.contains("{{link.name}}")
                || command.contains("{{link.name_}}")
            {
                let link_name = entry
                    .title()
                    .clone()
                    .replace(
                        &['(', ')', ',', '\"', '.', ';', ':', '\''][..],
                        "",
                    )
                    .replace(" ", "_")
                    .to_lowercase();
                *command = command.replace("{{link.name}}", &link_name);
                *command = command.replace("{{link.name_}}", &link_name);
            }
            if command.contains("{{link.name-}}") {
                let link_name = entry
                    .title()
                    .clone()
                    .replace(
                        &['(', ')', ',', '\"', '.', ';', ':', '\''][..],
                        "",
                    )
                    .replace(" ", "-")
                    .to_lowercase();
                *command = command.replace("{{link.name-}}", &link_name);
            }

            // Add terminal settings.
            *command =
                command.replace("{{terminal.width}}", &format!("{}", width));
        }

        // Log final command.
        tracing::trace!("Command: {:?}", &shell_command);

        // Build subprocess.
        let mut subproc = tokio::process::Command::new(&shell_command[0]);
        subproc.args(&shell_command[1..]);

        // Run subprocess.
        let mut ctx = CommandResultContext::new(custom_command.clone());
        match subproc.output().await {
            Ok(output) => {
                let exit: i32 = output.status.code().unwrap_or(1);
                let output: String = match exit {
                    0 => String::from_utf8(output.stdout)
                        .unwrap_or_else(|_| String::new()),
                    _ => {
                        String::from_utf8(output.stderr).unwrap_or_else(|_| {
                            format!(
                                "Failed to execute command: {:?}",
                                shell_command
                            )
                        })
                    }
                };
                tracing::info!("Command:\n{:?}", &custom_command.command);
                tracing::info!("Output:\n{}", output);
                ctx.update(Arc::new(output), exit == 0);
                (entry.db_id, ctx)
            }
            Err(e) => {
                ctx.update(
                    Arc::new(format!("Failed to create subprocess: {}", e)),
                    false,
                );
                (entry.db_id, ctx)
            }
        }
    }

    /// Check for an update and handle completed updates.
    async fn check_for_updates(&mut self) {
        // Check for new update.
        if let Some(entries_fut) = &mut self.refresh {
            if entries_fut.is_finished() {
                match entries_fut.await {
                    Ok(entries) => {
                        self.entries = entries;
                        self.terminal_state.window = 0;
                        self.interaction_state.selection = 0;
                    }
                    Err(e) => {
                        tracing::error!("Failed to update entries: {}", e);
                    }
                }
                self.refresh = None;
            }
        }

        // Check for loaded entries.
        while let Some(res) = self.command_futures.try_join_next() {
            if let Ok((entry_id, context)) = res {
                if context.command.save {
                    self.updater.save_command(entry_id, &context).await;
                    if let Some(entry) = self.entries.get_mut(entry_id) {
                        entry.add_result(context);
                    }
                }
            }
        }
    }

    /// Search for entries.
    async fn update_entries(
        &mut self,
        criteria: Vec<DatabaseSearch>,
        offset: OffsetCursor,
    ) {
        // Check for new update.
        if let Some(entries_fut) = &mut self.refresh {
            entries_fut.abort();
        }
        self.refresh = None;

        self.refresh = Some({
            let updater = self.updater.clone();
            let criteria = criteria.clone();
            tokio::spawn(async move { updater.search(criteria, offset).await })
        });
        self.interaction_state.previous_search = criteria;
    }

    async fn handle_command_mode_input(
        &mut self,
        key: &KeyEvent,
    ) -> Result<()> {
        // Update command.
        let (mut command, is_error) = match &self.interaction_state.focus {
            Focus::Command { command, message } => {
                (command.clone(), message.is_some())
            }
            _ => (String::new(), false),
        };

        // Go back to list if menu pressed.
        if *key == MENU {
            self.interaction_state.focus = Focus::List;
            return Ok(());
        }

        // If an error, clear and let the user continue typing.
        if is_error {
            self.interaction_state.focus = Focus::Command {
                command,
                message: None,
            };
            return Ok(());
        }

        match key.code {
            KeyCode::Char(c) => {
                command.push(c);
            }
            KeyCode::Backspace => {
                if command.len() > 0 {
                    command.pop();
                } else {
                    self.interaction_state.focus = Focus::List;
                    return Ok(());
                }
            }
            KeyCode::Enter => {
                match self.handle_command_mode_command(&command).await {
                    Ok(_) => {
                        self.interaction_state.focus = Focus::List;
                    }
                    Err(e) => {
                        self.interaction_state.focus = Focus::Command {
                            command,
                            message: Some(e.to_string()),
                        };
                    }
                }
                return Ok(());
            }
            _ => {}
        }

        self.interaction_state.focus = Focus::Command {
            command,
            message: None,
        };

        Ok(())
    }

    async fn handle_command_mode_command(
        &mut self,
        command: &str,
    ) -> Result<()> {
        let parsed_command =
            match command_mode::CommandParser::parse_command(command) {
                Ok(parsed) => parsed,
                Err(_) => bail!("Invalid command: {command}"),
            };
        match parsed_command.command {
            command_mode::Command::Quit => self.cancel_token.cancel(),
            command_mode::Command::SearchLatest => {
                self.update_entries(
                    vec![DatabaseSearch::Latest],
                    OffsetCursor::Latest,
                )
                .await
            }
            command_mode::Command::SearchAny(search) => {
                let mut criteria: Vec<DatabaseSearch> = Vec::new();
                for tag in &search.tag {
                    criteria.push(DatabaseSearch::Tag(tag.clone()));
                }
                for feed in &search.feed {
                    criteria.push(DatabaseSearch::Feed(feed.clone()));
                }
                for cmd in &search.command {
                    criteria.push(DatabaseSearch::Command(cmd.clone()));
                }
                for raw_clause in &search.raw {
                    criteria.push(DatabaseSearch::Raw(raw_clause.clone()));
                }
                if let Some(text) = &search.text {
                    criteria.push(DatabaseSearch::Search(text.clone()));
                }
                self.update_entries(criteria, OffsetCursor::Latest).await
            }
            command_mode::Command::TagAdd { tag } => {
                if self.interaction_state.selection < self.entries.len() {
                    let entry =
                        &mut self.entries[self.interaction_state.selection];
                    entry.entry.add_tag(&slipfeed::Tag::new(tag));
                    let tags: Vec<slipfeed::Tag> =
                        entry.entry.tags().iter().map(|t| t.clone()).collect();
                    self.updater.update_tags(entry.db_id, tags).await;
                }
            }
            command_mode::Command::TagRemove { tag } => {
                let entry = &mut self.entries[self.interaction_state.selection];
                entry.entry.remove_tag(&slipfeed::Tag::new(tag));
                let tags: Vec<slipfeed::Tag> =
                    entry.entry.tags().iter().map(|t| t.clone()).collect();
                self.updater.update_tags(entry.db_id, tags).await;
            }
            command_mode::Command::TagToggle { tag } => {
                let tag = slipfeed::Tag::new(tag);
                let entry = &mut self.entries[self.interaction_state.selection];
                if entry.entry.tags().contains(&tag) {
                    entry.entry.remove_tag(&tag);
                } else {
                    entry.entry.add_tag(&tag);
                }
                let tags: Vec<slipfeed::Tag> =
                    entry.entry.tags().iter().map(|t| t.clone()).collect();
                self.updater.update_tags(entry.db_id, tags).await;
            }
            command_mode::Command::Command { command } => {
                let command = self.config.read.get_custom_command(&command);
                match command {
                    Commandish::CustomCommandFull(custom_command) => {
                        if custom_command.save {
                            self.entries[self.interaction_state.selection]
                                .add_result(
                                    command::CommandResultContext::new(
                                        custom_command.clone(),
                                    ),
                                );
                        }
                        self.command_futures.spawn(Reader::run_shell_command(
                            custom_command,
                            self.entries[self.interaction_state.selection]
                                .clone(),
                            self.terminal_state.command_width,
                        ));
                    }
                    _ => {
                        tracing::warn!(
                            "Command mode commands do not support command: {command:?}."
                        );
                    }
                }
            }
            command_mode::Command::PageForwards => {
                let offset = if let Some(entry) = self.entries.last() {
                    OffsetCursor::Before(entry.date().clone())
                } else {
                    OffsetCursor::Latest
                };
                self.update_entries(
                    self.interaction_state.previous_search.clone(),
                    offset,
                )
                .await;
            }
            command_mode::Command::PageBackwards => {
                let offset = if let Some(entry) = self.entries.first() {
                    OffsetCursor::After(entry.date().clone())
                } else {
                    OffsetCursor::Latest
                };
                self.update_entries(
                    self.interaction_state.previous_search.clone(),
                    offset,
                )
                .await;
            }
        };

        Ok(())
    }
}

/// Widget to render the reader.
struct ReaderWidget<'a> {
    reader: &'a mut Reader,
}

impl<'a> ReaderWidget<'a> {
    fn new(reader: &'a mut Reader) -> Self {
        Self { reader }
    }
}

impl<'a> Widget for ReaderWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // Render the entry list:

        // Compute layout.
        let title_layout;
        let list_layout;
        let entry_layout;
        if area.width > MIN_HOR_WIDTH {
            let vert_layouts = Layout::default()
                .direction(Direction::Vertical)
                .constraints(&[Constraint::Min(1), Constraint::Percentage(100)])
                .split(area);
            title_layout = vert_layouts[0];
            let hor_layouts = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(vert_layouts[1]);
            list_layout = hor_layouts[0];
            entry_layout = hor_layouts[1];
        } else {
            let layouts = Layout::default()
                .direction(Direction::Vertical)
                .constraints(&[
                    Constraint::Min(1),
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(area);
            title_layout = layouts[0];
            list_layout = layouts[1];
            entry_layout = layouts[2];
        }

        // Update window based on layout.
        if (self.reader.interaction_state.selection as isize)
            < self.reader.terminal_state.window as isize
                + self.reader.config.read.scroll_buffer as isize
                - 1
        {
            self.reader.terminal_state.window =
                (self.reader.interaction_state.selection as isize
                    - self.reader.config.read.scroll_buffer as isize
                    + 1)
                .max(0) as usize;
        }
        if self.reader.interaction_state.selection
            > self.reader.terminal_state.window + list_layout.height as usize
                - self.reader.config.read.scroll_buffer as usize
        {
            self.reader.terminal_state.window =
                (self.reader.interaction_state.selection as isize
                    + self.reader.config.read.scroll_buffer as isize
                    - list_layout.height as isize)
                    .max(0) as usize;
        }

        // Update focus based on mouse.
        if self
            .reader
            .terminal_state
            .last_frame_inputs
            .clicked(list_layout)
        {
            self.reader.interaction_state.focus = Focus::List;
        }
        if self
            .reader
            .terminal_state
            .last_frame_inputs
            .clicked(entry_layout)
        {
            self.reader.interaction_state.focus = Focus::Entry;
        }

        // Show slipstream header.
        match &self.reader.interaction_state.focus {
            Focus::Command { command, message } => match message {
                Some(message) => {
                    Line::from(vec![
                        Span::styled("! ", Style::new().bold()),
                        Span::styled(message, Style::new().fg(Color::Black)),
                    ])
                    .bg(Color::Red)
                    .render(title_layout, buf);
                }
                None => {
                    Line::from(vec![
                        Span::styled(":", Style::new()),
                        Span::styled(command, Style::new().fg(Color::Blue)),
                        Span::styled("█", Style::new()),
                    ])
                    .bg(Color::Black)
                    .render(title_layout, buf);
                }
            },
            _ => {
                Text::styled(
                    format!(
                        "{:<width$}",
                        format!(
                            "slipstream {}/{}",
                            self.reader.interaction_state.selection + 1,
                            self.reader.entries.len()
                        ),
                        width = &(title_layout.width as usize),
                    ),
                    Style::new().bg(Color::Blue).fg(Color::Black),
                )
                .render(title_layout, buf);
            }
        }

        // Show titles.
        self.reader
            .entries
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                *i >= self.reader.terminal_state.window
                    && *i
                        < self.reader.terminal_state.window
                            + list_layout.height as usize
            })
            .enumerate()
            .for_each(|(line_num, (entry_num, entry))| {
                let feed: String = 'feed: {
                    for feed_ref in entry.feeds().iter() {
                        break 'feed (*feed_ref.name).clone();
                    }
                    "???".to_owned()
                };

                let selected: bool =
                    entry_num == self.reader.interaction_state.selection;

                let style = if selected {
                    if self.reader.terminal_state.has_focus {
                        match self.reader.interaction_state.focus {
                            Focus::Entry => {
                                Style::new().bg(Color::Black).fg(Color::Green)
                            }
                            _ => Style::new().bg(Color::Green).fg(Color::Black),
                        }
                    } else {
                        Style::new().bg(Color::White).fg(Color::Black)
                    }
                } else {
                    let mut style = Style::new();
                    for color_rule in &self.reader.config.read.tags.colors {
                        if color_rule.matches(entry) {
                            color_rule.apply_style(&mut style);
                        }
                    }
                    style
                };

                let line_layout = Rect {
                    x: list_layout.x,
                    y: list_layout.y + (line_num as u16),
                    width: list_layout.width,
                    height: 1,
                };

                if self
                    .reader
                    .terminal_state
                    .last_frame_inputs
                    .clicked(line_layout)
                {
                    self.reader.interaction_state.selection = entry_num;
                }

                Line::from(vec![
                    Span::styled(
                        format!("[{:<10}]", &feed[..10.min(feed.len())]),
                        if selected {
                            Style::new().bg(Color::Green).fg(Color::Black)
                        } else {
                            Style::new().fg(Color::Cyan)
                        },
                    ),
                    Span::from(" "),
                    Span::styled(entry.title(), style),
                ])
                .style(if selected { style } else { Style::new() })
                .render(line_layout, buf);
            });

        // Render the selection:
        self.reader.terminal_state.command_width = entry_layout.width - 6;
        if self.reader.interaction_state.selection < self.reader.entries.len() {
            let entry = &mut self.reader.entries
                [self.reader.interaction_state.selection];
            EntryViewWidget::new(
                entry,
                &self.reader.config,
                &self.reader.interaction_state,
                &self.reader.terminal_state,
            )
            .render(entry_layout, buf);
        }
    }
}

/// Check if a terminal event has happened.
async fn terminal_input_ready(poll_time: f32) -> bool {
    let check_fut = tokio::task::spawn_blocking(move || {
        event::poll(Duration::from_secs_f32(poll_time))
    });
    match check_fut.await {
        Ok(t) => match t {
            Ok(ready) => ready,
            Err(e) => {
                tracing::error!("Ratatui failed to check input: {}", e);
                false
            }
        },
        Err(e) => {
            tracing::error!("Failed to check if input is ready: {}", e);
            false
        }
    }
}
