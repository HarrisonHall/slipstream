//! Read mode.
//!
//! Fields: time (relative),

use super::*;

use std::{
    io,
    time::{Duration, Instant},
};

use futures::FutureExt;
use futures::future::BoxFuture;
use ratatui::{DefaultTerminal, Frame};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    widgets::{Block, Paragraph, Widget},
};
use ratatui::{
    prelude::{
        Color, Constraint, Direction, Layout, Line, Rect, Span, Style, Text,
    },
    style::Stylize,
};
use tokio::time::timeout;
use tracing::Instrument;

type Terminal = DefaultTerminal;

const QUIT: KeyEvent = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
const UPDATE: KeyEvent = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE);
const DOWN: KeyEvent = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
const UP: KeyEvent = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
const PAGE_DOWN: KeyEvent =
    KeyEvent::new(KeyCode::Char('j'), KeyModifiers::SHIFT);
const PAGE_UP: KeyEvent =
    KeyEvent::new(KeyCode::Char('k'), KeyModifiers::SHIFT);
const MENU: KeyEvent = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
const OPEN: KeyEvent = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
const LOAD: KeyEvent = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
const ENTRY_SCROLL_DOWN: KeyEvent =
    KeyEvent::new(KeyCode::Char('l'), KeyModifiers::SHIFT);
const ENTRY_SCROLL_UP: KeyEvent =
    KeyEvent::new(KeyCode::Char('h'), KeyModifiers::SHIFT);

const SCROLL_WINDOW: usize = 3;
const REFRESH_DELTA: f32 = 5.0;
const INPUT_DELTA: f32 = 0.01;

/// Perform the reader action.
pub async fn read(
    config: Arc<Config>,
    updater: Arc<Mutex<Updater>>,
) -> Result<()> {
    // Disable logging to stdout.
    get_logger().set_writing(false)?;

    // Show reader.
    let terminal = ratatui::init();
    let mut reader = Reader::new(config, updater, terminal)?;
    let result = reader.run().await;

    // Restore terminal.
    ratatui::restore();
    get_logger().set_writing(true)?;
    result
}

/// Terminal reader.
struct Reader {
    // Configuration
    config: Arc<Config>,
    updater: Arc<Mutex<Updater>>,
    // TUI.
    terminal: Terminal,
    // Jobs.
    last_update: chrono::DateTime<chrono::Utc>,
    updater_future: Option<BoxFuture<'static, Result<Vec<slipfeed::Entry>>>>,
    loading_futures: tokio::task::JoinSet<(slipfeed::Entry, LoadedEntry)>,
    // Reading state.
    selection: usize,
    entry_scroll: usize,
    window: usize,
    entries: RwLock<Vec<slipfeed::Entry>>,
    has_focus: bool,
    size: (u16, u16),
    show_menu: bool,
    loaded_entries: HashMap<slipfeed::Entry, LoadedEntry>,
}

impl Reader {
    /// Create a new reader.
    fn new(
        config: Arc<Config>,
        updater: Arc<Mutex<Updater>>,
        terminal: Terminal,
    ) -> Result<Self> {
        let size = terminal.size()?;
        // let focus = terminal.hide_cursor()?;
        Ok(Self {
            config,
            updater,
            terminal,
            last_update: chrono::Local::now().to_utc(),
            updater_future: None,
            loading_futures: tokio::task::JoinSet::new(),
            selection: 0,
            entry_scroll: 0,
            window: 0,
            entries: RwLock::new(Vec::new()),
            has_focus: true,
            size: (size.width, size.height),
            show_menu: false,
            loaded_entries: HashMap::new(),
        })
    }

    /// Run the reader.
    async fn run(&mut self) -> Result<()> {
        'reader: loop {
            // Draw entries.
            self.draw().await?;

            // Poll input.
            if self.handle_input().await.is_err() {
                break 'reader Ok(());
            }

            // Manage updater.
            self.handle_update(false).await;
        }
    }
}

// Draw logic.
impl Reader {
    /// Draw a frame of the TUI.
    async fn draw(&mut self) -> Result<()> {
        // Draw entries.
        let entries = self.entries.read().await;
        let updater_lock = self.updater.try_lock().ok();
        let updater = match updater_lock.as_ref() {
            Some(x) => Some(&**x),
            None => None,
        };
        self.terminal.draw(|f| {
            if let Err(e) = Reader::tui_render(
                f,
                self.selection,
                &mut self.window,
                updater,
                &entries,
                &self.loaded_entries,
                &mut self.entry_scroll,
                self.size,
                self.has_focus,
            ) {
                tracing::error!("Render error: {e}");
            }
        })?;
        let size = self.terminal.size()?;
        self.size = (size.width, size.height);

        Ok(())
    }

    fn tui_render(
        frame: &mut Frame,
        selection: usize,
        window: &mut usize,
        updater: Option<&Updater>,
        entries: &Vec<slipfeed::Entry>,
        loaded_entries: &HashMap<slipfeed::Entry, LoadedEntry>,
        entry_scroll: &mut usize,
        size: (u16, u16),
        has_focus: bool,
    ) -> Result<()> {
        // Check size.
        if !Reader::check_size(frame, size) {
            return Ok(());
        }

        // Compute layout.
        let title_layout;
        let list_layout;
        let entry_layout;
        if frame.area().width > 80 {
            let vert_layouts = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Min(1),
                    Constraint::Percentage(100),
                ])
                .split(frame.area());
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
                .split(frame.area());
            title_layout = layouts[0];
            list_layout = layouts[1];
            entry_layout = layouts[2];
        }

        // Update window based on layout.
        if selection < *window + SCROLL_WINDOW {
            *window =
                (selection as isize - SCROLL_WINDOW as isize).max(0) as usize;
        }
        if selection > *window + list_layout.height as usize - SCROLL_WINDOW {
            *window = (selection as isize + SCROLL_WINDOW as isize
                - list_layout.height as isize)
                .max(0) as usize;
        }

        // Show slipstream header.
        frame.render_widget(
            Text::styled(
                format!(
                    "{:<width$}",
                    format!("slipstream {}/{}", selection + 1, entries.len()),
                    width = &(title_layout.width as usize),
                ),
                Style::new().bg(Color::Green).fg(Color::Black),
            ),
            title_layout,
        );

        // Show titles.
        let formatted_entries = entries
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                *i >= *window && *i < *window + list_layout.height as usize
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
                let style = if i == selection {
                    if has_focus {
                        Style::new().bg(Color::Blue).fg(Color::Black)
                    } else {
                        Style::new().bg(Color::White).fg(Color::Black)
                    }
                } else {
                    Style::new()
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
        let entry_list = ratatui::widgets::List::new(formatted_entries)
            // .block(ratatui::widgets::Block::bordered().title("slipstream"))
        ;
        frame.render_widget(entry_list, list_layout);

        // Show selection.
        if selection < entries.len() {
            let entry = &entries[selection];
            Reader::entry_render(
                entry,
                match loaded_entries.get(entry) {
                    Some(l) => l,
                    None => &LoadedEntry::None,
                },
                entry_scroll,
                frame,
                &entry_layout,
            )?;
        }

        Ok(())
    }

    fn entry_render(
        entry: &slipfeed::Entry,
        loaded: &LoadedEntry,
        entry_scroll: &mut usize,
        frame: &mut Frame,
        rect: &Rect,
    ) -> Result<()> {
        // Render outline.
        let block = Block::bordered().title(entry.title().as_str());
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Min(2), Constraint::Percentage(100)])
            .split(block.inner(*rect));
        frame.render_widget(block, *rect);

        // Render metadata.
        let text = Text::from(vec![
            Line::from(Span::styled(
                format!("Link: {}", entry.source().url.as_str()),
                Style::default().fg(Color::Red),
            )),
            Line::from(Span::styled(
                format!("Author: {}", entry.author().as_str()),
                Style::default().fg(Color::LightGreen),
            )),
        ]);
        let p = Paragraph::new(text);
        frame.render_widget(p, layout[0]);

        // Render loaded entry.
        match loaded {
            LoadedEntry::None => frame.render_widget(
                Span::styled("Unloaded", Style::default()),
                layout[1],
            ),
            LoadedEntry::Loading => frame.render_widget(
                Span::styled("Loading", Style::default()),
                layout[1],
            ),
            LoadedEntry::Loaded { html: _, markdown } => {
                let t = tui_markdown::from_str(markdown.as_str());
                // let t = Text::raw(markdown.as_str());
                let p = Paragraph::new(t).scroll((*entry_scroll as u16, 0));
                frame.render_widget(p, layout[1]);
            }
            LoadedEntry::Failed => frame.render_widget(
                Span::styled("Failed", Style::default()),
                layout[1],
            ),
        };

        Ok(())
    }

    fn check_size(frame: &mut Frame, size: (u16, u16)) -> bool {
        if size.0 < 20 || size.1 < (2 * SCROLL_WINDOW as u16) + 5 {
            let rect = frame.area();
            frame.render_widget(
                ratatui::widgets::Paragraph::new("Too Small").block(
                    ratatui::widgets::Block::new()
                        .style(Style::new().bg(Color::White).fg(Color::Black))
                        .padding(ratatui::widgets::Padding::new(
                            0,
                            0,
                            rect.height / 2,
                            0,
                        )),
                ),
                rect,
            );
            return false;
        }
        return true;
    }
}

// Input logic.
impl Reader {
    /// Handle input. Quits on error.
    async fn handle_input(&mut self) -> Result<()> {
        for i in 0..5 {
            let poll_time: f32 =
                if i == 0 { REFRESH_DELTA } else { INPUT_DELTA };
            if event::poll(Duration::from_secs_f32(poll_time))? {
                // It's guaranteed that the `read()` won't block when the `poll()` function
                // returns `true`.
                match event::read()? {
                    Event::FocusGained => self.has_focus = true,
                    Event::FocusLost => self.has_focus = false,
                    Event::Key(key) => {
                        if key == UPDATE {
                            self.handle_update(true).await;
                        }
                        if key == DOWN {
                            if self.selection + 1
                                < self.entries.read().await.len()
                            {
                                self.selection += 1;
                            }
                            self.entry_scroll = 0;
                        }
                        if key == UP {
                            if (self.selection as isize) - 1 >= 0 {
                                self.selection -= 1;
                            }
                            self.entry_scroll = 0;
                        }
                        if key == PAGE_DOWN {
                            let entry_count =
                                self.entries.read().await.len() as isize;
                            let page_down = self.size.1 as isize
                                - (2 * SCROLL_WINDOW) as isize;
                            if self.selection as isize + page_down < entry_count
                            {
                                self.selection += page_down as usize;
                            } else {
                                self.selection =
                                    (entry_count - 1).max(0) as usize;
                            }
                            self.entry_scroll = 0;
                        }
                        if key == PAGE_UP {
                            let entry_count =
                                self.entries.read().await.len() as isize;
                            let page_up = self.size.1 as isize
                                - (2 * SCROLL_WINDOW) as isize;
                            if self.selection as isize - page_up >= 0 {
                                self.selection -=
                                    page_up.min(entry_count) as usize;
                            } else {
                                self.selection = 0;
                            }
                            self.entry_scroll = 0;
                        }
                        if key == ENTRY_SCROLL_DOWN {
                            self.entry_scroll =
                                (self.entry_scroll as isize + 1) as usize;
                        }
                        if key == ENTRY_SCROLL_UP {
                            self.entry_scroll = (self.entry_scroll as isize - 1)
                                .max(0)
                                as usize;
                        }
                        if key == QUIT {
                            bail!("Quit.");
                        }
                        if key == MENU {
                            self.show_menu = !self.show_menu;
                        }
                        if key == LOAD {
                            let entries = self.entries.read().await;
                            if self.selection < entries.len() {
                                if !self
                                    .loaded_entries
                                    .contains_key(&entries[self.selection])
                                {
                                    self.loading_futures.spawn(
                                        Reader::load_entry(
                                            entries[self.selection].clone(),
                                        ),
                                    );
                                }
                                self.loaded_entries.insert(
                                    entries[self.selection].clone(),
                                    LoadedEntry::Loading,
                                );
                            }
                        }
                        if key == OPEN {
                            let entries = self.entries.read().await;
                            if self.selection < entries.len() {
                                let link = entries[self.selection].source();
                                if link.url.len() > 0 {
                                    tracing::debug!("xdg-open {}", &link.url);
                                    let res =
                                        std::process::Command::new("xdg-open")
                                            .args([&link.url])
                                            .env_clear()
                                            .output();
                                    tracing::debug!("Res {:?}", res);
                                }
                            }
                        }
                    }
                    Event::Mouse(event) => tracing::debug!("Mouse {:?}", event),
                    Event::Resize(width, height) => {
                        self.size = (width, height);
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }
        Ok(())
    }
}

// Updating logic.
impl Reader {
    async fn handle_update(&mut self, force_init: bool) {
        // Start update in background.
        let now = chrono::Local::now().to_utc();
        if force_init || now - self.last_update > chrono::Duration::seconds(2) {
            self.feeds_update().await;
        }

        // Check for new update.
        if let Some(entries_fut) = &mut self.updater_future {
            if let Some(entries_res) = entries_fut.now_or_never() {
                match entries_res {
                    Ok(new_entries) => {
                        let mut entries = self.entries.write().await;
                        *entries = new_entries;
                        if self.selection > entries.len() {
                            if entries.len() == 0 {
                                self.window = 0;
                                self.selection = 0;
                            } else {
                                self.selection = entries.len() - 1;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to update: {e:?}");
                    }
                }
                self.updater_future = None;
            }
        }

        // Check for loaded entries.
        while let Some(res) = self.loading_futures.try_join_next() {
            if let Ok((entry, loaded)) = res {
                self.loaded_entries.insert(entry, loaded);
            }
        }
    }

    async fn feeds_update(&mut self) {
        let config = self.config.clone();
        let updater = self.updater.clone();
        if self.updater_future.is_none() {
            self.updater_future = Some(
                async move {
                    let updater = updater.lock().await;
                    let entries = updater.collect_all(config.as_ref());
                    Ok(entries)
                }
                .boxed(),
            );
        }
    }

    async fn load_entry(
        entry: slipfeed::Entry,
    ) -> (slipfeed::Entry, LoadedEntry) {
        let html = match reqwest::get(&entry.source().url).await {
            Ok(resp) => match resp.text().await {
                Ok(body) => body,
                Err(_) => return (entry, LoadedEntry::Failed),
            },

            Err(_) => return (entry, LoadedEntry::Failed),
        };
        // let markdown = match htmd::convert(&html) {
        //     Ok(md) => md,
        //     Err(_) => return (entry, LoadedEntry::Failed),
        // };
        // let markdown = html2md::rewrite_html(&html, false);
        let markdown = html2md::parse_html(&html, false);
        (entry, LoadedEntry::Loaded { html, markdown })
    }
}

enum LoadedEntry {
    None,
    Loading,
    Loaded { html: String, markdown: String },
    Failed,
}
