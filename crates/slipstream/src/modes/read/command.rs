///! Shell commands io storage.
use super::*;

const RUNNING_TEXT: &'static str = "Running...";
const FAILED_TEXT: &'static str = "Failed to execute command.";
const BAD_OUTPUT_TEXT: &'static str = "Unable to parse command output.";

/// Results from a shell command.
pub enum CommandResult {
    /// The command is running.
    Running,
    /// The command has finished.
    Finished { output: Arc<String>, success: bool },
}

/// Context of a completed shell command.
pub struct CommandResultContext {
    pub binding_name: Arc<String>,
    pub result: CommandResult,
    pub vertical_scroll: usize,
}

impl CommandResultContext {
    pub fn new(binding_name: Arc<String>) -> Self {
        Self {
            binding_name,
            result: CommandResult::Running,
            vertical_scroll: 0,
        }
    }

    pub fn update(&mut self, output: Arc<String>, success: bool) {
        self.result = CommandResult::Finished { output, success };
    }

    pub fn widget<'a>(&'a self) -> ratatui::widgets::Paragraph<'a> {
        match &self.result {
            CommandResult::Running => {
                let t = Text::raw(RUNNING_TEXT);
                Paragraph::new(t).left_aligned()
            }
            CommandResult::Finished { output, success } => {
                if *success {
                    use ansi_to_tui::IntoText;
                    let t = output
                        .into_text()
                        .unwrap_or_else(|_| Text::raw(FAILED_TEXT));
                    Paragraph::new(t)
                        .left_aligned()
                        .wrap(ratatui::widgets::Wrap { trim: true })
                        .scroll((self.vertical_scroll as u16, 0))
                } else {
                    let t = Text::raw(BAD_OUTPUT_TEXT);
                    Paragraph::new(t).left_aligned()
                }
            }
        }
    }
}
