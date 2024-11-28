//! Logging.

use super::*;

use std::fmt;

use colored::{ColoredString, Colorize};
use tracing::{level_filters::LevelFilter, Event, Level, Subscriber};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{
    fmt::{
        format::{self, FormatEvent, FormatFields},
        FmtContext, FormattedFields,
    },
    layer::SubscriberExt,
};

struct CliFormatter {}

impl CliFormatter {
    /// Get string for a level.
    fn get_level_string(&self, level: Level) -> &'static str {
        match level {
            Level::TRACE => "TRC",
            Level::DEBUG => "DBG",
            Level::INFO => "INF",
            Level::WARN => "WRN",
            Level::ERROR => "ERR",
        }
    }

    /// Get string for a level, ANSI colored.
    fn get_level_string_colored(&self, level: Level) -> ColoredString {
        let level = match level {
            Level::TRACE => self.get_level_string(level).cyan(),
            Level::DEBUG => self.get_level_string(level).magenta(),
            Level::INFO => self.get_level_string(level).blue(),
            Level::WARN => self.get_level_string(level).yellow(),
            Level::ERROR => self.get_level_string_colored(level).red(),
        };
        level.bold()
    }
}

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
        // TODO: Move to real filter!
        if !(metadata.target().contains(env!("CARGO_CRATE_NAME"))
            || metadata.target().contains("slipfeed"))
        {
            return Ok(());
        }
        write!(
            &mut writer,
            "{} :: ",
            self.get_level_string_colored(*metadata.level()),
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

/// Setup logging.
/// TODO: Add a log file.
pub fn setup_logging(cli: &Cli) {
    // let filter = tracing_subscriber::filter::Targets::new()
    //     .with_default(LevelFilter::OFF)
    //     .with_target("slipknot", Level::TRACE);
    // let x = tracing_subscriber::registry()
    //     .with(tracing_subscriber::fmt::layer())
    //     .with(filter);
    // tracing::subscriber::set_global_default(x).ok();
    // tracing_subscriber::registry().with()
    tracing_subscriber::fmt()
        .event_format(CliFormatter {})
        .with_max_level(if cli.debug {
            tracing::Level::TRACE
        } else {
            tracing::Level::INFO
        })
        // .with_target(false)
        // .with_writer()
        .init();
}
