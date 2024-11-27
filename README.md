# Slipstream

Feed aggregator, filterer, and combinator.

## Crates

- `slipfeed` - Feed aggregator, filterer, and combinator.
- `slipknot` - Simple CLI `slipfeed` server.
- `slipstore` - `slipfeed` persistent storage.
- `slipstream` - UI for `slipfeed`, backed by `slipstore`.

## Getting Started

`slipknot` is a command-line application for serving feeds from existing feeds a
la Yahoo Pipes.

## TODO

- `slipfeed`
  - [ ] Improve update scheduler
  - [ ] Check cycles and use loops instead of recursion
  - [ ] Cache feeds and keep available during updates
  - [ ] Move update durations to be feed-specific
- `slipknot`
  - [ ] Add more filters (regex/pomsky)
  - [ ] Add terminal interface for viewing feeds
  - [ ] Add terminal interface for viewing stream/ticker
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
