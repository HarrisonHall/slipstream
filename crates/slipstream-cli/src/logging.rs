//! Logging.

use super::*;

use std::collections::VecDeque;
use std::io::Write;
use std::sync::LazyLock;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::{Level, level_filters::LevelFilter};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;

static LOGGER: LazyLock<Logger> = LazyLock::new(|| Logger::new());

pub fn get_logger() -> Logger {
    LOGGER.clone()
}

/// Make stderr writer that can be toggled into a buffer.
#[derive(Clone)]
pub struct Logger {
    buffer: Arc<RwLock<VecDeque<Box<[u8]>>>>,
    cursor: Arc<RwLock<usize>>,
    max_size: usize,
    writing: Arc<AtomicBool>,
}

impl Logger {
    /// Create a new logger.
    fn new() -> Self {
        Self {
            buffer: Arc::new(RwLock::new(VecDeque::new())),
            cursor: Arc::new(RwLock::new(0)),
            max_size: 255,
            writing: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Toggle whether or not the writer is outputing to the terminal.
    pub fn set_writing(&mut self, writing: bool) -> Result<()> {
        self.writing.store(writing, Ordering::Release);
        if writing {
            self.flush()?;
        }
        Ok(())
    }

    /// Peek the back of the flush buffer.
    pub fn peek(&self, len: usize) -> Vec<String> {
        let mut logs = Vec::with_capacity(len);
        if let Ok(buffer) = self.buffer.read() {
            let mut cursor = buffer.len().wrapping_sub(1);
            while cursor < buffer.len() && logs.len() < len {
                logs.push(
                    String::from_utf8_lossy(&buffer[cursor]).into_owned(),
                );
                cursor = cursor.wrapping_sub(1);
            }
            logs.reverse();
        }
        logs
    }
}

impl std::io::Write for Logger {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let (Ok(mut buffer), Ok(mut cursor)) =
            (self.buffer.write(), self.cursor.write())
        {
            buffer.push_back(Box::from(buf));
            while buffer.len() > self.max_size {
                buffer.pop_front();
                if *cursor > 0 {
                    *cursor -= 1;
                }
            }
        }
        if self.writing.load(Ordering::Acquire) {
            self.flush()?;
        }
        return Ok(buf.len());
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut se = std::io::stderr();
        if let (Ok(mut buffer), Ok(mut cursor)) =
            (self.buffer.write(), self.cursor.write())
        {
            while *cursor < buffer.len() {
                se.write(&buffer[*cursor])?;
                *cursor += 1;
            }
            // Clear buffer and cursor.
            *cursor = 0;
            buffer.clear();
        }
        Ok(())
    }
}

/// Setup logging.
pub fn setup_logging(cli: &Cli, config: &Config) -> Result<()> {
    let level = match cli.verbose {
        true => Level::TRACE,
        false => match cli.debug {
            true => Level::DEBUG,
            false => Level::INFO,
        },
    };
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(LevelFilter::OFF)
        .with_target("slipstream", level)
        .with_target("slipfeed", level);

    // CLI layer (to stderr).
    let cli_logger = match cli.command {
        CommandMode::Read { .. } => {
            if cli.debug || cli.verbose {
                Some(
                    tracing_subscriber::fmt::layer()
                        .compact()
                        // .event_format(CliFormatter)
                        .with_writer(get_logger)
                        .with_filter(filter.clone()),
                )
            } else {
                None
            }
        }
        _ => Some(
            tracing_subscriber::fmt::layer()
                .compact()
                // .event_format(CliFormatter)
                .with_writer(get_logger)
                .with_filter(filter.clone()),
        ),
    };

    // File layer.
    let file_logger = match config.log.as_ref() {
        Some(log_file) => {
            let filename = shellexpand::full(log_file)
                .expect(&format!("Unable to expand log file {}", log_file))
                .into_owned();
            let path = std::path::PathBuf::from_str(&filename)
                .expect(&format!("Log file at invalid path {}", filename));
            if let Some(parent_dir) = path.parent() {
                std::fs::create_dir_all(parent_dir).expect(&format!(
                    "Unable to initialize path for {}",
                    filename
                ));
            }
            let file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&filename)
                .expect(&format!("Failed to create log file {}", log_file));
            Some(
                tracing_subscriber::fmt::layer()
                    .compact()
                    .with_writer(file)
                    // .event_format(FileFormatter)
                    .with_filter(filter),
            )
        }
        None => None,
    };

    // Log file formatting.
    let subscriber = tracing_subscriber::Registry::default()
        .with(cli_logger)
        .with(file_logger);

    // Set this logger as global.
    if let Err(_) = tracing::subscriber::set_global_default(subscriber) {
        bail!("Unable to initialize logging.");
    }

    Ok(())
}
