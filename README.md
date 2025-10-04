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
is used to define feeds, relationships, and filters.

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

While the `slipstream-feed` and `slipstream-cli` APIs may not be stable, they
are essentially complete as-is.

### Slipstream 3.0

- `slipstream` (general)
  - [ ] Support hooks
- `slipsteam` (serve)
  - [ ] Track updated `updated_at` in database
- `slipstream` (read)
  - [ ] Indicate pending updates
- `slipstream` (api)
  - [ ] Allow syncing tags to a slipstream server
  - [ ] Add support for a shared secret in an http header

### Beyond

- `slipstream` (general)
  - [ ] Add more filters (regex/pomsky, allowlists, etc.)
- `slipstream` (read)
  - [ ] Improve help menu

## Contributing

Simple bug fixes and suggestions are welcome. At this time, more in-depth
contributions will likely be rejected unless discussed ahead-of-time.
