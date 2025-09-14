//! Menu widget.

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
                Constraint::Percentage(75),
                Constraint::Percentage(25),
                Constraint::Min(1),
            ])
            .split(area);
        let title_layout = layouts[0];
        let stats_layout = layouts[1];
        let _feeds_layout = layouts[2];
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
        Line::from("Stats: TODO")
            .bg(Color::Green)
            .fg(Color::Black)
            .render(stats_layout, buf);

        // Show logs.
        use ansi_to_tui::IntoText;
        Paragraph::new(
            get_logger()
                .peek((log_layout.height as usize).saturating_sub(2))
                .join("")
                .into_text()
                .unwrap_or_else(|_| Text::raw("")),
        )
        .block(Block::bordered().title("Logs"))
        .render(log_layout, buf);

        // Show help.
        Line::from("Help: TODO")
            .bg(Color::Red)
            .fg(Color::Black)
            .render(help_layout, buf);
    }
}
