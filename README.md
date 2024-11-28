# Slipstream

Feed aggregator, filterer, and combinator.

## Crates

- `slipfeed` - Feed aggregator, filterer, and combinator.
- `slipknot` - Simple CLI `slipfeed` server.
- `slipstore` - `slipfeed` persistent storage.
- `slipstream` - UI for `slipfeed`, backed by `slipstore`.

## Getting Started

`slipknot` is a command-line application for serving feeds from existing feeds a
la Yahoo Pipes. A simple configuration file (e.g.
[slipknot.toml](examples/slipknot.toml)) is used to define feeds, relationships,
and filters.

## Roadmap

- `slipfeed`
  - [ ] Improve generic `Entry` model and parsing
  - [ ] Improve update scheduler
  - [ ] Check cycles and use loops instead of recursion for feed relationships
  - [ ] Cache feeds and keep available during updates
  - [ ] Move update durations to be feed-specific
  - [ ] Add synchronous interfaces
- `slipknot`
  - [ ] Add more filters (regex/pomsky)
  - [ ] Add terminal interface for viewing feeds and a stream/ticker
  - [ ] Add caching and make updates nonblocking
  - [ ] Add feed import/export to/from opml
- `slipstore`
  - [ ] Allow storing to sqlite database
  - [ ] Track reads
- `slipstream`
  - [ ] Design web interface
  - [ ] Add authentication/private feeds
  - [ ] Allow filter definitions via lua

## Contributing

Simple bug fixes and suggestions are welcome. At this time, more in-depth
contributions will likely be rejected unless communicated and planned
ahead-of-time.
