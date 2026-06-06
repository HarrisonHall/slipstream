//! Menu widget.

use ratatui::widgets::{BorderType, Wrap};

use super::*;

/// Widget to render the reader.
pub(super) struct MenuWidget<'a> {
    reader: &'a mut Reader,
}

impl<'a> MenuWidget<'a> {
    pub(super) fn new(reader: &'a mut Reader) -> Self {
        Self { reader }
    }
}

impl<'a> Widget for MenuWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // Compute layout.
        let layouts = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Min(1),
                Constraint::Min(1),
                Constraint::Percentage(30),
                Constraint::Percentage(70),
                Constraint::Min(1),
            ])
            .split(area);
        let title_layout = layouts[0];
        let stats_layout = layouts[1];
        let keyboard_layout = layouts[2];
        let log_layout = layouts[3];
        let help_layout = layouts[4];

        // Show slipstream menu header.
        Text::styled(
            format!(
                "{:<width$}",
                format!(
                    "menu {}/{}",
                    self.reader.interaction_state.selection + 1,
                    self.reader.entries.len()
                ),
                width = &(title_layout.width as usize),
            ),
            Style::new().bg(Color::Red).fg(Color::Black),
        )
        .render(title_layout, buf);

        // Show status.
        Line::from("Status")
            .bg(Color::Green)
            .fg(Color::Black)
            .render(stats_layout, buf);

        // Show keyboard layout.
        let keyboard_text: String = self
            .reader
            .config
            .read
            .bindings
            .iter()
            .map(|(binding, commandish)| {
                format!("<{}>: {}", binding.binding(), commandish)
            })
            .collect::<Vec<String>>()
            .join(", ");
        Paragraph::new(keyboard_text)
            .fg(Color::White)
            .wrap(Wrap::default())
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title_top("Bindings"),
            )
            .render(keyboard_layout, buf);

        // Show logs.
        use ansi_to_tui::IntoText;
        Paragraph::new(
            get_logger()
                .peek((log_layout.height as usize).saturating_sub(2))
                .join("")
                .into_text()
                .unwrap_or_else(|_| Text::raw("")),
        )
        .wrap(Wrap { trim: false })
        .block(
            Block::bordered()
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title_top("Logs"),
        )
        .render(log_layout, buf);

        // Show help.
        Line::from("Help: Press <q> to quit, <esc> to return to slipstream.")
            .bg(Color::Red)
            .fg(Color::Black)
            .render(help_layout, buf);
    }
}
