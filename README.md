<p align="center">
  <a href="https://github.com/harrisonhall/slipstream" align="center">
    <img alt="slipstream" src="https://github.com/HarrisonHall/slipstream/blob/main/crates/slipstream-cli/src/modes/serve/web/content/favicon.png" width="100" />
  </a>
</p>
<h1 align="center">slipstream</h1>

Feed fetcher, filterer, and aggregator.

## Crates

- `slipstream-cli` - Simple CLI `slipfeed` server and reader utilizing a simple
  [config](https://github.com/HarrisonHall/slipstream/blob/main/examples/config/slipstream.toml).
- `slipstream-feeds` (`slipfeed`) - Feed fetcher, filterer, transformer, and
  aggregator library.

## Getting Started

### slipstream

`slipstream` is a command-line application for serving filtered/aggregated feeds
from existing feeds a la Yahoo Pipes. A simple configuration file (e.g.
[slipstream.toml](https://github.com/HarrisonHall/slipstream/blob/main/examples/config/slipstream.toml))
is used to define feeds, filters, and aggregations.

The original goal of `slipstream serve` was to support a single, self-hostable
service that can aggregate feeds across devices. No need to share `opml` files
across desktops, phones, and laptops-- all feeds can be accessible from a single
new aggregate atom feed. Slipstream supports basic filters for allowlisting and
blocklisting entries from feeds based on substrings and tags. Everything
`slipstream serve` supports, `slipstream read` also supports.

- Fetch entries from various sources (rss, atom, mastodon)
  - Filter entries based on various criteria (allowlist/blocklist tags and
    substrings)
  - Apply & transform tags (aliases)
  - Make new aggregate feeds (composite feeds, matching tags)
- Serve feeds via HTML and Atom (`slipstream serve`)
- View the feeds locally with terminal reader (`slipstream read`)
  - Handle custom keybindings and compound commands
  - Display custom colors & flags based on tag matches
  - Execute shell commands and page the result (archival, fetching)
  - Command execution and result storage

#### Installation

`cargo install slipstream-cli`

#### Serve

Running `slipstream --config <your-config.toml> serve --port <your-port>` will
start a web server that exposes the following endpoints:

| Endpoint                 | Description               | Format |
| ------------------------ | ------------------------- | ------ |
| `/config`                | View the config           | `toml` |
| `/all`                   | View all entries          | `html` |
| `/all/feed`              | View all entries          | `atom` |
| `/feed/<feed_name>`      | View entries in feed      | `html` |
| `/feed/<feed_name>/feed` | View entries in feed      | `atom` |
| `/tag/<tag_name>`        | View entries matching tag | `html` |
| `/tag/<tag_name>/feed`   | View entries matching tag | `atom` |

An example can be found at my personal website
[feeds.hachha.dev](https://feeds.hachha.dev/).

![web screenshot](https://github.com/HarrisonHall/slipstream/blob/main/examples/media/web.png)

### Read

Running `slipstream --config <your-config.toml> read` will launch a local tui.

Check out the example
[config](https://github.com/HarrisonHall/slipstream/blob/main/examples/config/slipreader.toml)
to see additional configuration options.

![cli screenshot](https://github.com/HarrisonHall/slipstream/blob/main/examples/media/cli.png)

## Roadmap

While the `slipstream-feeds` and `slipstream-cli` APIs may not be stable, they
are essentially complete as-is.

### Slipstream 3.0

- `slipstream-feeds`
  - [ ] Custom HTML selector feeds
  - [ ] Release-date feeds with reminders
- `slipstream` (general)
  - [ ] Support hooks
- `slipstream` (read)
  - [ ] Indicate pending updates
- `slipstream` (api)
  - [ ] Allow syncing tags to a slipstream server

### Beyond

- `slipstream` (general)
  - [ ] Add more filters (regex/pomsky, allowlists, etc.)
- `slipstream` (read)
  - [ ] Better pagination and search

## Contributing

Simple bug fixes and suggestions are welcome. At this time, more in-depth
contributions will likely be rejected unless discussed ahead-of-time.
