use super::*;

/// Fetch feed.
pub async fn fetch_cli(
    config: Config,
    url: impl AsRef<str>,
    feed: impl AsRef<str>,
    format: FetchOutputFormat,
) -> Result<()> {
    // Create slipfeed updater with new custom feed.
    let mut updater =
        slipfeed::Updater::new(slipfeed::Duration::from_days(999), 1024);
    let mut attr = slipfeed::FeedAttributes::new();
    attr.display_name = feed.as_ref().to_string().into();
    let synd = slipfeed::StandardSyndication::new(url.as_ref());
    updater.add_feed(synd, attr);

    // Fetch results.
    let results = updater.update().await;

    // Create export object.
    let export = EntriesExport {
        feed: feed.as_ref().to_string(),
        url: url.as_ref().to_string(),
        entries: results
            .as_slice()
            .iter()
            .map(|e| ExportEntry::from_entry_markdown(e, &config))
            .collect(),
    };

    // Create export format.
    let templater = HtmlServer::templater()?;
    let output = match format {
        FetchOutputFormat::Toml => toml::to_string(&export)?,
        FetchOutputFormat::Json => serde_json::to_string_pretty(&export)?,
        FetchOutputFormat::Markdown => {
            let template = templater.get_template("template.md")?;
            template.render(&export)?
        }
        FetchOutputFormat::Custom(path) => {
            let template = std::fs::read_to_string(path)?;
            templater.render_str(&template, &export)?
        }
    };

    // Display results.
    // FUTURE: Check std::io::stdout().is_terminal() and either invoke pager or
    // auto-format with ANSI.
    // NOTE: We use writeln as println can panic and this is intended to occasionally
    // be used with piping.
    {
        use std::io::Write;

        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        writeln!(handle, "{}", &output).ok();
    }

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct EntriesExport {
    feed: String,
    url: String,
    entries: Vec<ExportEntry>,
}
