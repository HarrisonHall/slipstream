# Slipstream

Feed aggregator, filterer, and combinator.

- TODO
  - [ ] Major features
    - [x] Create basic feeds interface (`slipfeed`)
    - [x] Allow deploying RSS feed (`slipknot`)
    - [ ] Add web interface (`slipstream`)
  - [ ] Minor features
    - [ ] Sync interface
    - [ ] Private feeds
    - [ ] Cycle checking
    - [ ] Durations for individual feeds

## Crates

- `slipfeed` - Feed aggregator, filterer, and combinator.
- `slipknot` - Simple CLI `slipfeed` server.
- `slipstore` - `slipfeed` persistent storage.
- `slipstream` - UI for `slipfeed`, backed by `slipstore`.
