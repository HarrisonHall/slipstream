//! Logging.

use super::*;

use std::collections::VecDeque;
use std::fmt;
use std::io::Write;
use std::sync::LazyLock;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

use colored::{ColoredString, Colorize};
use tracing::{Event, Level, Subscriber, level_filters::LevelFilter};
use tracing_subscriber::Layer;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{
    fmt::{
        FmtContext,
        format::{self, FormatEvent, FormatFields},
    },
    layer::SubscriberExt,
};

static LOGGER: LazyLock<Logger> = LazyLock::new(|| Logger::new());

pub fn get_logger() -> Logger {
    LOGGER.clone()
}

/// Get string for a level.
fn get_level_string(level: Level) -> &'static str {
    match level {
        Level::TRACE => "TRC",
        Level::DEBUG => "DBG",
        Level::INFO => "INF",
        Level::WARN => "WRN",
        Level::ERROR => "ERR",
    }
}

/// Get string for a level, ANSI colored.
fn get_level_string_colored(level: Level) -> ColoredString {
    let level = match level {
        Level::TRACE => get_level_string(level).cyan(),
        Level::DEBUG => get_level_string(level).magenta(),
        Level::INFO => get_level_string(level).blue(),
        Level::WARN => get_level_string(level).yellow(),
        Level::ERROR => get_level_string(level).red(),
    };
    level.bold()
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
    fn new() -> Self {
        Self {
            buffer: Arc::new(RwLock::new(VecDeque::new())),
            cursor: Arc::new(RwLock::new(0)),
            max_size: 255,
            writing: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn set_writing(&mut self, writing: bool) -> Result<()> {
        self.writing.store(writing, Ordering::Release);
        if writing {
            self.flush()?;
        }
        Ok(())
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
        if let (Ok(buffer), Ok(mut cursor)) =
            (self.buffer.write(), self.cursor.write())
        {
            while *cursor < buffer.len() {
                se.write(&buffer[*cursor])?;
                *cursor += 1;
            }
        }
        Ok(())
    }
}

/// Formatter for the cli.
struct CliFormatter;

impl<S, N> FormatEvent<S, N> for CliFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        write!(
            &mut writer,
            "{} :: ",
            get_level_string_colored(*metadata.level()),
            // metadata.target().bright_green().bold()
        )?;

        let now_string =
            chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        write!(&mut writer, "{} :: ", now_string.green())?;

        write!(&mut writer, "{} :: ", metadata.target().green().bold())?;

        // // Format all the spans in the event's span context.
        // if let Some(scope) = ctx.event_scope() {
        //     for span in scope.from_root() {
        //         write!(writer, "{}", span.name().bright_green().bold())?;

        //         // `FormattedFields` is a formatted representation of the span's
        //         // fields, which is stored in its extensions by the `fmt` layer's
        //         // `new_span` method. The fields will have been formatted
        //         // by the same field formatter that's provided to the event
        //         // formatter in the `FmtContext`.
        //         let ext = span.extensions();
        //         let fields = &ext
        //             .get::<FormattedFields<N>>()
        //             .expect("will never be `None`");

        //         // Skip formatting the fields if the span had no fields.
        //         if !fields.is_empty() {
        //             write!(writer, "{{{}}}", fields)?;
        //         }
        //     }
        //     write!(writer, " :: ")?;
        // }

        // Write fields on the event
        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)?;
        Ok(())
    }
}

/// Formatter for the log file.
struct FileFormatter;

impl<S, N> FormatEvent<S, N> for FileFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        write!(&mut writer, "{} :: ", get_level_string(*metadata.level()),)?;

        let now_string =
            chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        write!(&mut writer, "{} :: ", now_string)?;

        write!(&mut writer, "{} :: ", metadata.target())?;

        // Write fields on the event
        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)?;
        Ok(())
    }
}

/// Setup logging.
pub fn setup_logging(cli: &Cli, config: &Config) {
    let level = match cli.debug {
        true => Level::TRACE,
        false => Level::INFO,
    };
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(LevelFilter::OFF)
        .with_target("slipstream", level)
        .with_target("slipfeed", level);

    // CLI layer (to stderr).
    let cli = tracing_subscriber::fmt::layer()
        .event_format(CliFormatter)
        .with_writer(get_logger)
        .with_filter(filter.clone());

    // File layer.
    let file = match config.log.as_ref() {
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
                    .with_writer(file)
                    .event_format(FileFormatter)
                    .with_filter(filter),
            )
        }
        None => None,
    };

    // Log file formatting.
    let subscriber =
        tracing_subscriber::Registry::default().with(cli).with(file);
    tracing::subscriber::set_global_default(subscriber)
        .expect("Unable to initialize logging.");
}
