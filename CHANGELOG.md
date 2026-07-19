# Changelog

## [Unreleased]

### Added

- Support for If-Modified-Since in `slipstream serve`.
- Support for "live" mode in slipstream.
- Support for "new" mode in slipstream.
- Flags (icon) support for matched feeds.
- Transforms have been added in addition to filters:
  - `author` replaces the author given a templated string.
  - `tag-derivations` adds tag based on a match.
  - `tag-aliases` replaces tag based on a match.
- Improve mouse support

### Changed

- Changed the `include-tags` and `exclude-tags` filters to utilize fuzzy
  matching, where strict matching is now utilized in `include-tags-strict` and
  `exclude-tags-strict`.
- All include & exclude tags have been reworked and include a basic test-suite.
- Reworked feed fetch order
  - Removed "worlds" from aggregate feeds

### Deprecated

- N/A

### Removed

- N/A

### Fixed

- Improved heuristic for entry deduplication
- Add better support for rfc2822 timestamps
- Improved markdown parsing & output

### Security

- N/A

## slipstream-cli 2.6.0

### Added

- OPML and TOML imports and exports.
- `read` mode improvements:
  - Mouse click and scroll support.
  - Built-in commands.
  - Custom commands.
  - Custom keybinds.
  - Custom styling based on tags.
  - Pagination support.
- `serve` improvements:
  - Web mode:
    - Support for tags.
    - Support for embedded HTML.
- Persistent storage utilizing a sqlite database.

## slipstream-feeds 0.5.0

### Added

- Respect tags/categories in fetched entries.
- Support for markdown conversion.
- Mastodon feed support.

### Changed

- Tag formatting:
  - Tags are now specified by the number-sign/hash/octothorp/hash, e.g.,
    `#cool-tag`.
  - Tags are now lowercase.
- Only a set number of feeds are changed at a time based on the `workers`
  configuration option.

### Fixed

- Feed order is now respected for comment URLs.

## slipstream-cli 2.0.0

### Added

- Added slipstream `read` mode.
- Added behavior to utilize the `If-Modified-Since` header.

## slipstream-feeds 0.4.0

### Added

- New `EntrySet` class for returning updates.

### Changed

- Using traits to handle multiple types of feeds.

## slipstream-cli 0.3.0

### Added

- Added web views to `slipstream`.
- Added tag filters to `slipstream`.

### Changed

- Merged `slipknot` into `slipstream`.

### Removed

- `slipknot` has been removed.

### Fixed

- The `[all]` configuration section is now respected.
