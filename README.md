# Slipstream

Feed fetcher, filterer, and aggregator.

## Crates

- `slipfeed` - Feed fetcher, filterer, and aggregator.
- `slipknot` - Simple CLI `slipfeed` server.
- `slipstore` - `slipfeed` persistent storage.
- `slipstream` - UI for `slipfeed`, backed by `slipstore`.

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

While `slipknot` is usable, these crates are far from complete.

- `slipfeed`
  - [ ] Improve generic `Entry` model and parsing
  - [ ] Move update durations to be feed-specific
- `slipknot`
  - [ ] Add more filters (regex/pomsky, allowlists, etc.)
  - [ ] Add caching and make updates nonblocking
  - [ ] Add feed import/export to/from opml
  - [ ] Add optional log file
- `slipstore`
  - [ ] Allow storing entries in a sqlite database
  - [ ] Track reads
- `slipstream`
  - [ ] Design web interface
  - [ ] Add authentication/private feeds
  - [ ] Allow filter definitions via lua

## Contributing

Simple bug fixes and suggestions are welcome. At this time, more in-depth
contributions will likely be rejected unless communicated and planned
ahead-of-time.
