use std::io::IsTerminal;

use super::*;

/// Fetch feed.
pub async fn fetch_cli(
    // updater: UpdaterHandle,
    url: impl AsRef<str>,
    feed: impl AsRef<str>,
    format: FetchOutputFormat,
    // cancel_token: CancellationToken,
) -> Result<()> {
    // Create slipfeed updater with new custom feed.
    let mut updater =
        slipfeed::Updater::new(slipfeed::Duration::from_days(999), 1024);
    let mut attr = slipfeed::FeedAttributes::new();
    attr.display_name = feed.as_ref().to_string().into();
    // TODO: Separate feed timeout and oldest limit!
    attr.timeout = slipfeed::Duration::from_days(7);
    let synd = slipfeed::StandardSyndication::new(url.as_ref());
    updater.add_feed(synd, attr);

    // Fetch results.
    let results = updater.update().await;

    // Create export object.
    let export = EntriesExport {
        feed: feed.as_ref().to_string(),
        url: url.as_ref().to_string(),
        entries: results.as_slice().iter().map(|e| e.clone()).collect(),
    };

    // Create export format.
    let output = match format {
        // FetchOutputFormat::None => {
        //     todo!()
        // }
        FetchOutputFormat::Toml => toml::to_string(&export)?,
        FetchOutputFormat::Json => serde_json::to_string_pretty(&export)?,
    };

    // Display results.
    match std::io::stdout().is_terminal() {
        // FUTURE:
        // true => {
        //     // Get pager.
        //     let pager = match std::env::var("PAGER") {
        //         Ok(env_pager) => env_pager,
        //         Err(_) => "more".to_string(),
        //     };

        //     // If using terminal, page results.
        //     let mut command = std::process::Command::new(&pager)
        //         .stdin(std::process::Stdio::piped())
        //         .spawn()?;
        //     let stdin = command.stdin.as_mut().expect("stdin missing");
        //     stdin.write_all(output.as_bytes())?;
        //     command.wait()?;
        // }
        _ => {
            // Otherwise (if piping), just print to stdout.
            // for result in results.as_slice() {
            //     println!("Result: {result:?}");
            // }
            println!("{}", &output);
        }
    }

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct EntriesExport {
    feed: String,
    url: String,
    entries: Vec<slipfeed::Entry>,
}
