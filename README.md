# Slipstream

Feed fetcher, filterer, and aggregator.

## Crates

- `slipfeed` - Feed fetcher, filterer, and aggregator library.
- `slipknot` - Simple CLI `slipfeed` server utilizing a simple
  [config](examples/config/slipknot.toml).
- `slipstream` - Web UI for `slipfeed`, backed by persistent storage for
  entries.

## Getting Started

`slipknot` is a command-line application for serving filtered/aggregated feeds
from existing feeds a la Yahoo Pipes. A simple configuration file (e.g.
[slipknot.toml](examples/config/slipknot.toml)) is used to define feeds,
relationships, and filters. While still a work-in-progress, running something
like `slipknot --config <your-config.toml> --port <your-port>` will start a web
server that exposes the following endpoints:

- `/config` for viewing the config.
- `/all` for viewing all entries.
- `/feed/<feed_name>` for viewing a specific feed.
- `/tag/<tag_name>` for viewing a feed for entries with a specific tag.

## Roadmap

While the `slipfeed` and `slipknot` APIs may not be stable, they are essentially
complete as-is. `slipstream` development has not yet started. While `slipstream`
features overlap greatly with `slipknot`, the implementation will likely be
completely independent.

- `slipfeed`
  - [ ] Add other built-in feed implementations (e.g. activitypub)
- `slipknot`
  - [ ] Add more filters (regex/pomsky, allowlists, etc.)
  - [ ] OPML conversion support
- `slipstream`
  - [ ] Design web interface
  - [ ] Use sqlite for storing entries and feed definitions
  - [ ] Track reads
  - [ ] Add authentication/private feeds
  - [ ] Allow filter definitions via gleam/lua
  - [ ] Support atom exports

## Contributing

Simple bug fixes and suggestions are welcome. At this time, more in-depth
contributions will likely be rejected unless communicated and planned
ahead-of-time.
