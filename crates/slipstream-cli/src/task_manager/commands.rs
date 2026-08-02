use super::*;

/// Context required to run a custom command.
pub struct CustomCommandContext {
    /// ID of entry in db.
    pub entry_id: EntryDbId,
    /// (width, height).
    pub terminal_size: (u16, u16),
}

impl Default for CustomCommandContext {
    fn default() -> Self {
        Self {
            entry_id: 0,
            terminal_size: (80, 60),
        }
    }
}

/// Result from a custom command.
#[derive(Debug, Clone)]
pub struct CustomCommandResult {
    /// Entry db id.
    pub entry_id: EntryDbId,
    /// Custom command name.
    pub command: CustomCommand,
    /// stdout output from command.
    pub output: String,
    /// Exit code from command.
    pub exit_code: i32,
}

impl CustomCommandResult {
    pub(super) fn empty() -> Self {
        Self {
            entry_id: 0,
            command: CustomCommand {
                name: "".to_string().into(),
                command: Arc::new(Vec::new()),
                save: false,
            },
            output: "".into(),
            exit_code: 0,
        }
    }
}

/// Run a custom shell command.
/// This replaces select substrings of the shell command with values from the
/// entry.
pub async fn run_custom_command(
    task_manager_handle: TaskManagerHandle,
    custom_command: CustomCommand,
    entry_id: EntryDbId,
) -> CustomCommandResult {
    // Get entry.
    let entry = match task_manager_handle.get_entry(entry_id).await {
        Ok(entry) => entry,
        Err(e) => {
            tracing::error!("Could not run command: {e}");
            return CustomCommandResult {
                entry_id,
                output: format!("{e}"),
                exit_code: 1,
                command: custom_command,
            };
        }
    };

    let ctx = CustomCommandContext {
        entry_id: entry.db_id,
        // terminal_size: (width, 60),
        terminal_size: (80, 60),
    };

    // Build command.
    let mut shell_command: Vec<String> = (*custom_command.command).clone();

    for argument in shell_command.iter_mut() {
        // Add links.
        *argument = argument.replace("{{link.url}}", &entry.source().url);
        let mut link_count: usize = 0;
        if !entry.source().url.is_empty() {
            link_count += 1;
            *argument = argument.replace(
                &format!("{{{{link.url{}}}}}", link_count),
                &entry.source().url,
            );
        }
        if !entry.comments().url.is_empty() {
            link_count += 1;
            *argument = argument.replace(
                &format!("{{{{link.url{}}}}}", link_count),
                &entry.comments().url,
            );
        }
        for i in 0..entry.other_links().len() {
            link_count += 1;
            *argument = argument.replace(
                &format!("{{{{link.url{}}}}}", link_count),
                &entry.other_links()[i].url,
            );
        }

        // Add link name.
        if argument.contains("{{link.name}}")
            || argument.contains("{{link.name_}}")
        {
            let link_name = entry
                .title()
                .clone()
                .replace(&['(', ')', ',', '\"', '.', ';', ':', '\''][..], "")
                .replace(" ", "_")
                .to_lowercase();
            *argument = argument.replace("{{link.name}}", &link_name);
            *argument = argument.replace("{{link.name_}}", &link_name);
        }
        if argument.contains("{{link.name-}}") {
            let link_name = entry
                .title()
                .clone()
                .replace(&['(', ')', ',', '\"', '.', ';', ':', '\''][..], "")
                .replace(" ", "-")
                .to_lowercase();
            *argument = argument.replace("{{link.name-}}", &link_name);
        }

        // Add feed information.
        *argument =
            argument.replace("{{feed}}", &entry.entry.primary_feed().name);

        // Add terminal settings.
        *argument = argument
            .replace("{{terminal.width}}", &format!("{}", ctx.terminal_size.0));
    }

    // Log final command.
    tracing::trace!("Command: {:?}", &shell_command);

    // Build subprocess.
    let mut subproc = tokio::process::Command::new(&shell_command[0]);
    subproc.args(&shell_command[1..]);

    // Run subprocess.
    let result = match subproc.output().await {
        Ok(output) => {
            let exit_code: i32 = output.status.code().unwrap_or(1);
            let output: String = match exit_code {
                0 => String::from_utf8(output.stdout)
                    .unwrap_or_else(|_| String::new()),
                _ => String::from_utf8(output.stderr).unwrap_or_else(|_| {
                    format!("Failed to execute command: {:?}", shell_command)
                }),
            };
            tracing::info!("Command:\n{:?}", &custom_command.command);
            tracing::info!("Output:\n{}", output);
            CustomCommandResult {
                output,
                exit_code,
                entry_id: entry.db_id,
                command: custom_command,
            }
        }
        Err(e) => CustomCommandResult {
            output: format!("Failed to run command: {}", e),
            exit_code: 1,
            entry_id: entry.db_id,
            command: custom_command,
        },
    };

    // Store result.
    // task_manager_handle.save_command(result.clone()).await;

    result
}
