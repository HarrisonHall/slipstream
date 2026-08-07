//! Read mode.

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

use ratatui::DefaultTerminal;
use ratatui::buffer::Buffer;
use ratatui::prelude::{
    Color, Constraint, Direction, Layout, Line, Rect, Span, Style, Text,
};
use ratatui::style::Stylize;
use ratatui::{
    crossterm::event::{self, Event},
    widgets::{Block, Paragraph, Widget},
};
use tokio::task::JoinHandle;

type Terminal = DefaultTerminal;

/// How often to refresh the screen without input.
const REFRESH_DELTA: f32 = 5.0;
/// Minimum height of the screen.
const MIN_VER_HEIGHT: u16 = 20;
/// The minimum terminal width to support horizontal mode.
const MIN_HOR_WIDTH: u16 = 120;
/// The C-c quit key event.
const CONTROL_C: KeyEvent =
    KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);

/// Perform the reader action.
pub async fn read_cli(
    config: Arc<Config>,
    task_manager_handle: TaskManagerHandle,
    cancel_token: CancellationToken,
) -> Result<()> {
    // Disable logging to stdout.
    get_logger().set_writing(false)?;

    // Show reader.
    let mut terminal = ratatui::init();
    let _kb_cap = MouseCapture::new()?;

    let mut reader =
        Reader::new(config.clone(), task_manager_handle, cancel_token)?;

    // Update reader on load.
    reader
        .handle_command_mode_command(&format!(
            "search {}",
            config.read.initial_search
        ))
        .await?;

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
    task_manager_handle: TaskManagerHandle,
    /// Refresh future.
    refresh: JoinHandle<DatabaseEntryList>,
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
        task_manager_handle: TaskManagerHandle,
        cancel_token: CancellationToken,
    ) -> Result<Self> {
        Ok(Self {
            config,
            task_manager_handle,
            refresh: tokio::spawn(std::future::pending()),
            entries: DatabaseEntryList::new(0),
            terminal_state: TerminalState::default(),
            interaction_state: InteractionState::default(),
            cancel_token,
        })
    }

    /// Run the reader.
    async fn run(&mut self, terminal: &mut Terminal) -> Result<()> {
        let mut task_receiver = self.task_manager_handle.receiver().await;
        let mut input_stream = crossterm::event::EventStream::new();

        'reader: loop {
            let previous_selection = self.interaction_state.selection;

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

            let input_fut = input_stream.next().fuse();
            let refresh_fut = tokio::time::sleep(
                tokio::time::Duration::from_secs_f32(REFRESH_DELTA),
            );
            tokio::select! {
                _ = refresh_fut => {
                    // Do nothing.
                    // This is present to sure the terminal is refreshed periodically.
                },
                input = input_fut => {
                    if let Some(input) = input {
                        match input {
                            Ok(input) => {
                                self.terminal_state.last_frame_inputs.clear();
                                self.handle_input(input, terminal).await?;
                            },
                            Err(e) => tracing::warn!("Failed to parse input: {e}"),
                        }
                    }
                },
                list_update_res = &mut self.refresh => {
                    match list_update_res {
                       Ok(entries)  => self.handle_list_update(Some(entries)).await,
                       Err(e) => {
                            tracing::error!("Failed to update entries: {}", e);
                           self.handle_list_update(None).await;
                       }
                    }
                },
                system_task = task_receiver.recv() => {
                    if let Ok(system_task) = system_task {
                        if self
                            .handle_system_tasks(system_task, terminal)
                            .await
                            .is_err()
                        {
                            self.cancel_token.cancel();
                            break 'reader Ok(());
                        }
                    }
                },
                _ = self.cancel_token.cancelled() => {
                    break 'reader Ok(());
                }
            };

            // Call read hooks on change.
            if self.interaction_state.selection != previous_selection {
                if self.interaction_state.selection < self.entries.len() {
                    self.task_manager_handle
                        .hook(
                            self.entries[self.interaction_state.selection]
                                .db_id
                                .into(),
                            Hook::OnRead,
                        )
                        .await;
                }
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

        None
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
        true
    }

    /// Handle input.
    /// Quits on error.
    async fn handle_input(
        &mut self,
        input: crossterm::event::Event,
        _terminal: &mut Terminal,
    ) -> Result<()> {
        match input {
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
                        let command = self.config.get_key_command(&key);
                        let ctx = match self.get_selected_entry_mut() {
                            Some(e) => tasks::Context::with_entry_id(e.db_id),
                            None => tasks::Context::default(),
                        };
                        self.task_manager_handle
                            .run_command(ctx, command)
                            .await;
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

    /// Handle system tasks.
    async fn handle_system_tasks(
        &mut self,
        task: tasks::BackgroundTask,
        terminal: &mut Terminal,
    ) -> Result<()> {
        match task {
            tasks::BackgroundTask::Update(u) => match u {
                tasks::BackgroundTaskUpdate::CommandUpdate { ctx, result } => {
                    if let (true, Some(id)) =
                        (result.command.save, &ctx.entry_id)
                    {
                        if let Some(entry) = self.entries.get_mut(*id) {
                            entry.add_result(result.into());
                        }
                    }
                }
                _ => {}
            },
            tasks::BackgroundTask::Execute(e) => match e {
                tasks::BackgroundTaskExecute::Command { ctx, commandish } => {
                    match commandish {
                        Commandish::Literal(lit) => {
                            self.run_command_literal(lit, terminal).await?
                        }
                        Commandish::CustomCommandRef(_) => {}
                        Commandish::CustomCommandFull(custom_command) => {
                            if let (true, Some(id)) =
                                (custom_command.save, &ctx.entry_id)
                            {
                                if let Some(entry) = self.entries.get_mut(*id) {
                                    entry.add_result(
                                        command::CommandResultContext::running(
                                            custom_command.clone(),
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
        }
        Ok(())
    }

    /// Run built-in command.
    async fn run_command_literal(
        &mut self,
        command: ReadCommand,
        terminal: &mut Terminal,
    ) -> Result<()> {
        match command {
            ReadCommand::None => {}
            ReadCommand::Quit => {
                if let Focus::Menu { .. } = &self.interaction_state.focus {
                    self.interaction_state.focus.toggle_menu();
                } else {
                    self.cancel_token.cancel();
                }
                return Ok(());
            }
            ReadCommand::Clear => {
                terminal.clear().ok();
                return Ok(());
            }
            ReadCommand::Update => {
                self.update_entries(
                    vec![DatabaseSearch::Latest],
                    OffsetCursor::LatestTimestamp,
                    false,
                )
                .await;
            }
            ReadCommand::Down => match self.interaction_state.focus {
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
            ReadCommand::Up => match self.interaction_state.focus {
                Focus::List => {
                    if (self.interaction_state.selection as isize) > 0 {
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
            ReadCommand::Left => {
                if self.interaction_state.selection < self.entries.len() {
                    self.entries[self.interaction_state.selection]
                        .cycle_result(-1);
                }
            }
            ReadCommand::Right => {
                if self.interaction_state.selection < self.entries.len() {
                    self.entries[self.interaction_state.selection]
                        .cycle_result(1);
                }
            }
            ReadCommand::PageDown => match self.interaction_state.focus {
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
            },
            ReadCommand::PageUp => match self.interaction_state.focus {
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
            ReadCommand::Swap => {
                self.interaction_state.focus.swap();
            }
            ReadCommand::Menu => {
                self.interaction_state.focus.toggle_menu();
            }
            ReadCommand::CommandMode => {
                self.interaction_state.focus = Focus::Command {
                    command: String::new(),
                    message: None,
                };
            }
            ReadCommand::SearchMode => {
                self.interaction_state.focus = Focus::Command {
                    command: "/".into(),
                    message: None,
                };
            }
            ReadCommand::PageForwards => {
                let offset = if let Some(entry) = self.entries.last() {
                    OffsetCursor::Before(entry.date().clone())
                } else {
                    OffsetCursor::LatestTimestamp
                };
                self.update_entries(
                    self.interaction_state.previous_search.clone(),
                    offset,
                    false,
                )
                .await;
            }
            ReadCommand::PageBackwards => {
                let offset = if let Some(entry) = self.entries.first() {
                    OffsetCursor::After(entry.date().clone())
                } else {
                    OffsetCursor::LatestTimestamp
                };
                self.update_entries(
                    self.interaction_state.previous_search.clone(),
                    offset,
                    false,
                )
                .await;
            }
            ReadCommand::Command(command) => {
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

    /// Handle completed updates.
    async fn handle_list_update(&mut self, entries: Option<DatabaseEntryList>) {
        // Update entries.
        if let Some(entries) = entries {
            self.entries = entries;
            if !self.interaction_state.repeat_previous
                || self.interaction_state.selection >= self.entries.len()
            {
                self.terminal_state.window = 0;
                self.interaction_state.selection = 0;
            }
        }
        self.refresh = tokio::spawn(std::future::pending());

        // Repeat search.
        if self.interaction_state.repeat_previous {
            self.interaction_state.next_delay =
                Some(tokio::time::Duration::from_secs_f32(5.0));
            self.update_entries(
                self.interaction_state.previous_search.clone(),
                self.interaction_state.previous_offset.clone(),
                true,
            )
            .await;
        }
    }

    /// Search for entries.
    async fn update_entries(
        &mut self,
        criteria: Vec<DatabaseSearch>,
        offset: OffsetCursor,
        repeat: bool,
    ) {
        // Check for new update.
        self.refresh.abort();

        self.refresh = {
            let delay = self.interaction_state.next_delay.take();
            let updater = self.task_manager_handle.clone();
            let criteria = criteria.clone();
            let offset = offset.clone();
            tokio::spawn(async move {
                if let Some(delay) = delay {
                    tokio::time::sleep(delay).await;
                }
                updater.search(criteria, offset).await
            })
        };
        self.interaction_state.repeat_previous = repeat;
        self.interaction_state.previous_search = criteria;
        self.interaction_state.previous_offset = offset;
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
                if !command.is_empty() {
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
                    OffsetCursor::LatestTimestamp,
                    false,
                )
                .await
            }
            command_mode::Command::SearchAny(search) => {
                let mut criteria: Vec<DatabaseSearch> = Vec::new();
                for tag in &search.tag {
                    criteria.push(DatabaseSearch::Tag(tag.clone()));
                }
                for not_tag in &search.not_tag {
                    criteria.push(DatabaseSearch::NotTag(not_tag.clone()));
                }
                for feed in &search.feed {
                    criteria.push(DatabaseSearch::Feed(feed.clone()));
                }
                for not_feed in &search.not_feed {
                    criteria.push(DatabaseSearch::NotFeed(not_feed.clone()));
                }
                for cmd in &search.command {
                    criteria.push(DatabaseSearch::Command(cmd.clone()));
                }
                for not_cmd in &search.not_command {
                    criteria.push(DatabaseSearch::NotCommand(not_cmd.clone()));
                }
                for raw_clause in &search.raw {
                    criteria.push(DatabaseSearch::Raw(raw_clause.clone()));
                }
                if let Some(text) = &search.text {
                    criteria.push(DatabaseSearch::Search(text.clone()));
                }
                tracing::info!("searchany: {:?}", criteria);
                self.update_entries(
                    criteria,
                    OffsetCursor::LatestTimestamp,
                    false,
                )
                .await
            }
            command_mode::Command::SearchNew => {
                self.update_entries(
                    vec![DatabaseSearch::New],
                    OffsetCursor::LatestId,
                    true,
                )
                .await
            }
            command_mode::Command::SearchLive => {
                self.update_entries(
                    vec![DatabaseSearch::Live],
                    OffsetCursor::LatestId,
                    true,
                )
                .await
            }
            command_mode::Command::TagAdd { tag } => {
                if self.interaction_state.selection < self.entries.len() {
                    let entry =
                        &mut self.entries[self.interaction_state.selection];
                    entry.entry.add_tag(&slipfeed::Tag::new(tag));
                    let tags: Vec<slipfeed::Tag> =
                        entry.entry.tags().iter().cloned().collect();
                    self.task_manager_handle
                        .update_tags(entry.db_id, tags)
                        .await;
                }
            }
            command_mode::Command::TagRemove { tag } => {
                let entry = &mut self.entries[self.interaction_state.selection];
                entry.entry.remove_tag(&slipfeed::Tag::new(tag));
                let tags: Vec<slipfeed::Tag> =
                    entry.entry.tags().iter().cloned().collect();
                self.task_manager_handle
                    .update_tags(entry.db_id, tags)
                    .await;
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
                    entry.entry.tags().iter().cloned().collect();
                self.task_manager_handle
                    .update_tags(entry.db_id, tags)
                    .await;
            }
            command_mode::Command::Command { command } => {
                let command = self.config.get_custom_command(&command);
                match &command {
                    Some(custom_command) => {
                        if custom_command.save {
                            self.entries[self.interaction_state.selection]
                                .add_result(
                                    command::CommandResultContext::running(
                                        custom_command.clone(),
                                    ),
                                );
                        }
                    }
                    None => {
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
                    OffsetCursor::LatestTimestamp
                };
                self.update_entries(
                    self.interaction_state.previous_search.clone(),
                    offset,
                    false,
                )
                .await;
            }
            command_mode::Command::PageBackwards => {
                let offset = if let Some(entry) = self.entries.first() {
                    OffsetCursor::After(entry.date().clone())
                } else {
                    OffsetCursor::LatestTimestamp
                };
                self.update_entries(
                    self.interaction_state.previous_search.clone(),
                    offset,
                    false,
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
                .constraints([Constraint::Min(1), Constraint::Percentage(100)])
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
                .constraints([
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
            .hovered(list_layout)
        {
            self.reader.interaction_state.focus = Focus::List;
        }
        if self
            .reader
            .terminal_state
            .last_frame_inputs
            .hovered(entry_layout)
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
                        width = (title_layout.width as usize),
                    ),
                    Style::new()
                        .bg(
                            match self.reader.interaction_state.repeat_previous
                            {
                                false => Color::Blue,
                                true => Color::LightYellow,
                            },
                        )
                        .fg(Color::Black),
                )
                .render(title_layout, buf);
            }
        }

        // Show previews.
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

                let selected: bool =
                    entry_num == self.reader.interaction_state.selection;
                let hovering: bool = self
                    .reader
                    .terminal_state
                    .last_frame_inputs
                    .hovering(line_layout);

                let mut indicators = Vec::new();

                // Find style by iterating through color rules.
                let mut line_style = Style::new();
                let mut entry_style = Style::new();
                for color_rule in &self.reader.config.read.tags.colors {
                    if color_rule.matches(entry) {
                        // Apply style.
                        color_rule.apply_style(&mut entry_style);

                        // Add indicator.
                        if indicators.len() > 3 {
                            continue;
                        }
                        if let Some(mut indicator) = color_rule.indicator() {
                            if hovering {
                                indicator =
                                    indicator.bg(Color::Gray).fg(Color::Black);
                            }
                            if selected {
                                indicator =
                                    indicator.bg(Color::Green).fg(Color::Black);
                            }
                            indicators.push(indicator);
                        }
                    }
                }

                // Selected, hovering, and focused state affects style.
                if hovering && self.reader.terminal_state.has_focus {
                    entry_style = match self.reader.interaction_state.focus {
                        Focus::Entry => {
                            Style::new().bg(Color::Black).fg(Color::Gray)
                        }
                        _ => Style::new().bg(Color::Gray).fg(Color::Black),
                    };
                    line_style = entry_style;
                }
                if selected {
                    entry_style = if self.reader.terminal_state.has_focus {
                        match self.reader.interaction_state.focus {
                            Focus::Entry => {
                                Style::new().bg(Color::Black).fg(Color::Green)
                            }
                            _ => Style::new().bg(Color::Green).fg(Color::Black),
                        }
                    } else {
                        Style::new().bg(Color::White).fg(Color::Black)
                    };
                    line_style = entry_style;
                }

                Line::raw(" ")
                    .style(line_style.not_underlined())
                    .render(line_layout, buf);
                let split_line_layout = self
                    .reader
                    .config
                    .read
                    .preview_format
                    .layout()
                    .split(line_layout);
                for (i, token) in
                    self.reader.config.read.preview_format.iter().enumerate()
                {
                    let last_token =
                        i == self.reader.config.read.preview_format.len() - 1;
                    match token {
                        PreviewToken::Summary => {
                            let summary_layout = if !last_token {
                                Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([
                                        Constraint::Fill(1),
                                        Constraint::Length(1),
                                    ])
                                    .split(split_line_layout[i])
                            } else {
                                Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([Constraint::Fill(1)])
                                    .split(split_line_layout[i])
                            };

                            Span::styled(entry.title(), entry_style)
                                .render(summary_layout[0], buf);
                        }
                        PreviewToken::Flags => {
                            let mut offset: u16 = 0;
                            let layout = split_line_layout[i];
                            for span in &indicators {
                                if offset >= 4 {
                                    break;
                                }
                                span.render(
                                    Rect {
                                        x: layout.x + offset,
                                        y: layout.y,
                                        width: layout.width - offset,
                                        height: layout.height,
                                    },
                                    buf,
                                );
                                offset += span.width() as u16;
                            }
                        }
                        PreviewToken::Feed => {
                            let feed: String = 'feed: {
                                if let Some(feed_ref) =
                                    entry.feeds().iter().next()
                                {
                                    break 'feed (*feed_ref.name).clone();
                                }
                                "???".to_owned()
                            };

                            let feed_layout = if !last_token {
                                Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([
                                        Constraint::Fill(1),
                                        Constraint::Length(1),
                                    ])
                                    .split(split_line_layout[i])
                            } else {
                                Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([Constraint::Fill(1)])
                                    .split(split_line_layout[i])
                            };

                            Span::styled(
                                format!(
                                    "[{:<width$}]",
                                    &feed[..feed.len().min(
                                        feed_layout[0].width as usize - 2
                                    )],
                                    width = feed_layout[0].width as usize - 2
                                ),
                                if !selected && !hovering {
                                    Style::new().fg(Color::LightCyan)
                                } else {
                                    entry_style
                                },
                            )
                            .render(feed_layout[0], buf);
                        }
                        PreviewToken::Date => {
                            let date_layout = if !last_token {
                                Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([
                                        Constraint::Fill(1),
                                        Constraint::Length(1),
                                    ])
                                    .split(split_line_layout[i])
                            } else {
                                Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([Constraint::Fill(1)])
                                    .split(split_line_layout[i])
                            };

                            let mut date = entry.entry.date().to_iso8601();
                            date = date.replace("T", " ");

                            Span::styled(
                                &date,
                                if !selected && !hovering {
                                    Style::new().fg(Color::LightYellow)
                                } else {
                                    entry_style
                                },
                            )
                            .render(date_layout[0], buf);
                        }
                        PreviewToken::Tag => {
                            let mut priority_tag: String = "".into();
                            for tag in &self.reader.config.read.tags.priority {
                                if entry.has_tag(tag) {
                                    priority_tag = tag.clone();
                                    break;
                                }
                            }
                            if priority_tag.is_empty() {
                                if let Some(tag) = entry.tags().iter().next() {
                                    priority_tag = tag.to_string();
                                }
                            }

                            let primary_tag_layout = if !last_token {
                                Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([
                                        Constraint::Fill(1),
                                        Constraint::Length(1),
                                    ])
                                    .split(split_line_layout[i])
                            } else {
                                Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([Constraint::Fill(1)])
                                    .split(split_line_layout[i])
                            };

                            Span::styled(
                                if !priority_tag.is_empty() {
                                    format!("#{priority_tag}")
                                } else {
                                    priority_tag
                                },
                                if selected || hovering {
                                    entry_style
                                } else {
                                    Style::new().fg(Color::Yellow)
                                },
                            )
                            .render(primary_tag_layout[0], buf);
                        }
                        PreviewToken::Author => {
                            let author_layout = if !last_token {
                                Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([
                                        Constraint::Fill(1),
                                        Constraint::Length(1),
                                    ])
                                    .split(split_line_layout[i])
                            } else {
                                Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([Constraint::Fill(1)])
                                    .split(split_line_layout[i])
                            };

                            Span::styled(entry.author(), entry_style)
                                .render(author_layout[0], buf);
                        }
                    }
                }
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
