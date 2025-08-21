//!!utput
//! Fields: time (relative),

use super::*;

mod command;
mod config;
mod entry;
mod keyboard;
mod state;

pub use command::*;
pub use config::*;
use entry::*;
pub use keyboard::*;
use ratatui::buffer::Buffer;
pub use state::*;

use std::time::Duration;

use futures::FutureExt;
use futures::future::BoxFuture;
use ratatui::DefaultTerminal;
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

type Terminal = DefaultTerminal;

/// The minimum number of displayed entries before supporting scroll.
const SCROLL_WINDOW: usize = 3;
/// How often to refresh the screen without input.
const REFRESH_DELTA: f32 = 0.5;
/// Minimum time to poll the terminal for queued inputs.
const INPUT_DELTA: f32 = 0.01;
/// The minimum terminal width to support horizontal mode.
const MIN_HORIZONTAL_WIDTH: u16 = 120;

/// Perform the reader action.
pub async fn read(
    config: Arc<Config>,
    updater: Arc<Mutex<Updater>>,
) -> Result<()> {
    // Disable logging to stdout.
    get_logger().set_writing(false)?;

    // Show reader.
    let mut terminal = ratatui::init();
    let mut reader = Reader::new(config, updater)?;
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
    updater_state: UpdaterState,
    /// Futures for binding commands run on entries.
    command_futures:
        tokio::task::JoinSet<(slipfeed::Entry, command::CommandResultContext)>,
    /// Current scroll.
    entries: Arc<RwLock<Vec<slipfeed::Entry>>>,
    /// Cached results form binding commands.
    views: EntryViews,
    /// Cached of the terminal.
    terminal_state: TerminalState,
    /// State of user interaction.
    interaction_state: InteractionState,
}

impl Reader {
    /// Create a new reader.
    fn new(config: Arc<Config>, updater: Arc<Mutex<Updater>>) -> Result<Self> {
        // let size = terminal.size()?;
        // size: (size.width, size.height),
        // let focus = terminal.hide_cursor()?;
        Ok(Self {
            config,
            updater_state: UpdaterState::new(updater),
            command_futures: tokio::task::JoinSet::new(),
            entries: Arc::new(RwLock::new(Vec::new())),
            views: EntryViews::new(),
            terminal_state: TerminalState::default(),
            interaction_state: InteractionState::default(),
        })
    }

    /// Run the reader.
    async fn run(&mut self, terminal: &mut Terminal) -> Result<()> {
        let entries = self.entries.clone();
        'reader: loop {
            {
                let entries = entries.read().await;
                terminal.draw(|f| {
                    ReaderWidget::new(self, &entries)
                        .render(f.area(), f.buffer_mut());
                })?;
            }

            // Poll input.
            if self.handle_input().await.is_err() {
                break 'reader Ok(());
            }

            // Manage updater.
            self.check_for_update(false).await;
        }
    }
}

// Draw logic.
impl Reader {
    /// Check the size.
    /// If the buffer size is too small, this returns false and renders a notification.
    fn check_size_or_render(&mut self, buf: &mut Buffer) -> bool {
        if self.terminal_state.size.0 < 20
            || self.terminal_state.size.1 < (2 * SCROLL_WINDOW as u16) + 5
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
        for i in 0..5 {
            let poll_time: f32 =
                if i == 0 { REFRESH_DELTA } else { INPUT_DELTA };
            if event::poll(Duration::from_secs_f32(poll_time))? {
                // It's guaranteed that the `read()` won't block when the `poll()`
                // function returns `true`.
                match event::read()? {
                    Event::FocusGained => self.terminal_state.has_focus = true,
                    Event::FocusLost => self.terminal_state.has_focus = false,
                    Event::Key(key) => {
                        tracing::debug!("Key press: {:?}", key);
                        let command = self.config.read.get_key_command(&key);
                        self.run_command(command).await?;
                    }
                    Event::Mouse(event) => tracing::debug!("Mouse {:?}", event),
                    Event::Resize(width, height) => {
                        self.terminal_state.size = (width, height);
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Run command.
    async fn run_command(&mut self, command: ReadCommand) -> Result<()> {
        match command {
            ReadCommand::CustomCommandRef(name) => {
                tracing::error!("Invalid command name: {}", name.as_str());
            }
            ReadCommand::CustomCommandFull { name, command } => {
                let entries = self.entries.read().await;
                self.views
                    .get(&entries[self.interaction_state.selection])
                    .add_result(command::CommandResultContext::new(
                        name.clone(),
                    ));
                self.command_futures.spawn(Reader::run_shell_command(
                    name.clone(),
                    entries[self.interaction_state.selection].clone(),
                    (*command).clone(),
                    self.terminal_state.command_width,
                ));
            }
            ReadCommand::Literal(command) => {
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
                bail!("Quit.");
            }
            ReadCommandLiteral::Update => {
                self.check_for_update(true).await;
            }
            ReadCommandLiteral::Down => match self.interaction_state.focus {
                Focus::List => {
                    if self.interaction_state.selection + 1
                        < self.entries.read().await.len()
                    {
                        self.interaction_state.selection += 1;
                    }
                }
                Focus::Entry => {
                    let entries = self.entries.read().await;
                    if self.interaction_state.selection < entries.len() {
                        self.views
                            .get(&entries[self.interaction_state.selection])
                            .scroll(1);
                    }
                }
                Focus::Menu => {}
            },
            ReadCommandLiteral::Up => match self.interaction_state.focus {
                Focus::List => {
                    if (self.interaction_state.selection as isize) - 1 >= 0 {
                        self.interaction_state.selection -= 1;
                    }
                }
                Focus::Entry => {
                    let entries = self.entries.read().await;
                    if self.interaction_state.selection < entries.len() {
                        self.views
                            .get(&entries[self.interaction_state.selection])
                            .scroll(-1);
                    }
                }
                Focus::Menu => {}
            },
            ReadCommandLiteral::Left => {
                let entries = self.entries.read().await;
                if self.interaction_state.selection < entries.len() {
                    self.views
                        .get(&entries[self.interaction_state.selection])
                        .cycle_result(-1);
                }
            }
            ReadCommandLiteral::Right => {
                let entries = self.entries.read().await;
                if self.interaction_state.selection < entries.len() {
                    self.views
                        .get(&entries[self.interaction_state.selection])
                        .cycle_result(1);
                }
            }
            ReadCommandLiteral::PageDown => {
                match self.interaction_state.focus {
                    Focus::List => {
                        let entry_count =
                            self.entries.read().await.len() as isize;
                        let page_down = self.terminal_state.size.1 as isize
                            - (2 * SCROLL_WINDOW) as isize;
                        if self.interaction_state.selection as isize + page_down
                            < entry_count
                        {
                            self.interaction_state.selection +=
                                page_down as usize;
                        } else {
                            self.interaction_state.selection =
                                (entry_count - 1).max(0) as usize;
                        }
                    }
                    Focus::Entry => {
                        let entries = self.entries.read().await;
                        if self.interaction_state.selection < entries.len() {
                            self.views
                                .get(&entries[self.interaction_state.selection])
                                .scroll(1);
                        }
                    }
                    Focus::Menu => {}
                }
            }
            ReadCommandLiteral::PageUp => match self.interaction_state.focus {
                Focus::List => {
                    let entry_count = self.entries.read().await.len() as isize;
                    let page_up = self.terminal_state.size.1 as isize
                        - (2 * SCROLL_WINDOW) as isize;
                    if self.interaction_state.selection as isize - page_up >= 0
                    {
                        self.interaction_state.selection -=
                            page_up.min(entry_count) as usize;
                    } else {
                        self.interaction_state.selection = 0;
                    }
                }
                Focus::Entry => {
                    let entries = self.entries.read().await;
                    if self.interaction_state.selection < entries.len() {
                        self.views
                            .get(&entries[self.interaction_state.selection])
                            .scroll(-1);
                    }
                }
                Focus::Menu => {}
            },
            ReadCommandLiteral::Swap => {
                self.interaction_state.focus.swap();
            }
            ReadCommandLiteral::Menu => {
                self.interaction_state.focus.toggle_menu();
            }
            ReadCommandLiteral::ToggleImportant => {
                let entries = self.entries.read().await;
                if self.interaction_state.selection < entries.len() {
                    let important = self
                        .views
                        .get(&entries[self.interaction_state.selection])
                        .important;
                    self.views
                        .get(&entries[self.interaction_state.selection])
                        .set_imporant(!important);
                }
            }
        };

        Ok(())
    }

    /// Run a custom shell command.
    /// This replaces select substrings of the shell command with values from the
    /// entry.
    async fn run_shell_command(
        binding_name: Arc<String>,
        entry: slipfeed::Entry,
        shell_command: Vec<String>,
        width: u16,
    ) -> (slipfeed::Entry, command::CommandResultContext) {
        // Build command.
        let mut shell_command = shell_command;

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
                tracing::info!(
                    "Other links {} {}",
                    i,
                    &entry.other_links()[i].url
                );
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
            tracing::debug!("terminal width: {}", width);
            *command =
                command.replace("{{terminal.width}}", &format!("{}", width));
        }

        // Log final command.
        tracing::trace!("Command: {:?}", &shell_command);

        // Build subprocess.
        let mut subproc = tokio::process::Command::new(&shell_command[0]);
        subproc.args(&shell_command[1..]);

        // Run subprocess.
        let mut ctx = CommandResultContext::new(binding_name.clone());
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
                tracing::info!("Output:\n{}", output);
                ctx.update(Arc::new(output), exit == 0);
                (entry, ctx)
            }
            Err(e) => {
                ctx.update(
                    Arc::new(format!("Failed to create subprocess: {}", e)),
                    false,
                );
                (entry, ctx)
            }
        }
    }

    /// Check for an update and handle completed updates.
    async fn check_for_update(&mut self, force_init: bool) {
        // Start update in background.
        let now = chrono::Local::now().to_utc();
        if force_init
            || now - self.updater_state.last_update
                > chrono::Duration::seconds(2)
        {
            let config = self.config.clone();
            let updater = self.updater_state.updater.clone();
            if self.updater_state.future.is_none() {
                self.updater_state.future = Some(
                    async move {
                        let updater = updater.lock().await;
                        let entries = updater.collect_all(config.as_ref());
                        Ok(entries)
                    }
                    .boxed(),
                );
            }
        }

        // Check for new update.
        if let Some(entries_fut) = &mut self.updater_state.future {
            if let Some(entries_res) = entries_fut.now_or_never() {
                match entries_res {
                    Ok(new_entries) => {
                        let mut entries = self.entries.write().await;
                        *entries = new_entries;
                        if self.interaction_state.selection > entries.len() {
                            if entries.len() == 0 {
                                self.terminal_state.window = 0;
                                self.interaction_state.selection = 0;
                            } else {
                                self.interaction_state.selection =
                                    entries.len() - 1;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to update: {e:?}");
                    }
                }
                self.updater_state.future = None;
            }
        }

        // Check for loaded entries.
        while let Some(res) = self.command_futures.try_join_next() {
            if let Ok((entry, context)) = res {
                self.views.get(&entry).add_result(context);
            }
        }
    }
}

/// Widget to render the reader.
struct ReaderWidget<'a> {
    reader: &'a mut Reader,
    entries: &'a Vec<slipfeed::Entry>,
}

impl<'a> ReaderWidget<'a> {
    fn new(reader: &'a mut Reader, entries: &'a Vec<slipfeed::Entry>) -> Self {
        Self { reader, entries }
    }
}

impl<'a> Widget for ReaderWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // Update components.
        self.reader.terminal_state.size = (area.width, area.height);

        // Do not render below minimum size.
        if !self.reader.check_size_or_render(buf) {
            return ();
        }

        // Render the entry list:

        // Compute layout.
        let title_layout;
        let list_layout;
        let entry_layout;
        if area.width > MIN_HORIZONTAL_WIDTH {
            let vert_layouts = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Min(1),
                    Constraint::Percentage(100),
                ])
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
                .constraints(vec![
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
        if self.reader.interaction_state.selection
            < self.reader.terminal_state.window + SCROLL_WINDOW
        {
            self.reader.terminal_state.window =
                (self.reader.interaction_state.selection as isize
                    - SCROLL_WINDOW as isize)
                    .max(0) as usize;
        }
        if self.reader.interaction_state.selection
            > self.reader.terminal_state.window + list_layout.height as usize
                - SCROLL_WINDOW
        {
            self.reader.terminal_state.window =
                (self.reader.interaction_state.selection as isize
                    + SCROLL_WINDOW as isize
                    - list_layout.height as isize)
                    .max(0) as usize;
        }

        // Show slipstream header.
        Text::styled(
            format!(
                "{:<width$}",
                format!(
                    "slipstream {}/{}",
                    self.reader.interaction_state.selection + 1,
                    self.entries.len()
                ),
                width = &(title_layout.width as usize),
            ),
            Style::new().bg(Color::Blue).fg(Color::Black),
        )
        .render(title_layout, buf);

        let updater_lock = self.reader.updater_state.updater.try_lock().ok();
        let updater = match updater_lock.as_ref() {
            Some(x) => Some(&**x),
            None => None,
        };

        // Show titles.
        let formatted_entries = self
            .entries
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                *i >= self.reader.terminal_state.window
                    && *i
                        < self.reader.terminal_state.window
                            + list_layout.height as usize
            })
            .map(|(i, e)| {
                let feed: String = 'feed: {
                    for feed_id in e.feeds().iter() {
                        if let Some(updater) = updater {
                            if let Some(feed_name) = updater.feed_name(*feed_id)
                            {
                                break 'feed feed_name.clone();
                            }
                        }
                    }
                    "???".to_owned()
                };
                let style = if i == self.reader.interaction_state.selection {
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
                    if !self.reader.views.get(e).has_been_read {
                        Style::new().fg(Color::Yellow)
                    } else {
                        if self.reader.views.get(e).important {
                            Style::new().bg(Color::Red).fg(Color::Black)
                        } else {
                            Style::new()
                        }
                    }
                };
                return ratatui::text::Text::styled(
                    format!(
                        "[{:<10}] {}",
                        &feed[..10.min(feed.len())],
                        e.title()
                    ),
                    style,
                );
            });
        ratatui::widgets::List::new(formatted_entries).render(list_layout, buf);

        // Render the selection:

        self.reader.terminal_state.command_width = entry_layout.width - 6;
        if self.reader.interaction_state.selection < self.entries.len() {
            let entry = &self.entries[self.reader.interaction_state.selection];
            self.reader.views.get(entry).set_read();
            EntryViewWidget::new(
                self.reader.views.get(entry),
                entry,
                &self.reader.interaction_state.focus,
            )
            .render(entry_layout, buf);
        }
    }
}
