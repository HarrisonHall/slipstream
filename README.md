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
- `slipstream-feeds` (`slipfeed`) - Feed fetcher, filterer, and aggregator
  library.

## Getting Started

### slipstream

`slipstream` is a command-line application for serving filtered/aggregated feeds
from existing feeds a la Yahoo Pipes. A simple configuration file (e.g.
[slipstream.toml](https://github.com/HarrisonHall/slipstream/blob/main/examples/config/slipstream.toml))
is used to define feeds, filters, and aggregations.

The original goal of `slipstream serve` was to support a single, self-hostable
service that can aggregate feeds across devices. No need to share `opml` files
across desktops, phones, and laptops-- all feeds can be accessible from a single
new aggregate atom feed. Slipstream supports basic filters for
allowlisting/denylisting entries from feeds based on substrings and tags.
Everything `slipstream serve` supports, `slipstream read` also supports.

#### Installation

`cargo install slipstream-cli`

#### Serve

Running `slipstream --config <your-config.toml> serve --port <your-port>` will
start a web server that exposes the following endpoints:

- `/config` for viewing the config (toml).
- `/all` (or `/`) for viewing all entries (html).
- `/feed/<feed_name>` for viewing a specific feed (html).
- `/tag/<tag_name>` for viewing a feed for entries with a specific tag (html).
- `/all/feed` for viewing all entries (atom).
- `/feed/<feed_name>/feed` for viewing a specific feed (atom).
- `/tag/<tag_name>/feed` for viewing a feed for entries with a specific tag
  (atom).

An example can be found at my personal website
[feeds.hachha.dev](https://feeds.hachha.dev/).

![web screenshot](https://github.com/HarrisonHall/slipstream/blob/main/examples/media/web.png)

### Read

Running `slipstream --config <your-config.toml> read` will launch a local tui.
The slipstream reader supports the following features:

- Custom commands
- Custom keybindings
- Custom colors
- Responsive layout (horizontal and vertical)
- Hooks

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
  - [ ] Improve help menu
- `slipstream` (api)
  - [ ] Allow syncing tags to a slipstream server

### Beyond

- `slipstream` (general)
  - [ ] Add more filters (regex/pomsky, allowlists, etc.)

## Contributing

Simple bug fixes and suggestions are welcome. At this time, more in-depth
contributions will likely be rejected unless discussed ahead-of-time.
