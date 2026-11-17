# Changelog

## [Unreleased]

### Changed
- `wl_config::Position`'s fields are private, with `Position::new` as the only constructor: it
  rejects a relation and an alignment on the same axis, so every `Position` that exists can be
  applied. Read one back with `relation()`, `reference()` and `alignment()`. Breaking change for
  library users, who previously built `Position` as a struct literal.
- `wltile position` with `top-of`/`bottom-of` and no alignment now uses `align-left` instead of
  failing with "Impossible to align bottom on a vertical setup" — a mismatch invented by the
  `align-bottom` default nobody had typed. `left-of`/`right-of` keep defaulting to `align-bottom`.
- A `--config` path that does not exist is a startup error naming the path. The default
  `$XDG_CONFIG_HOME/wltile/config.toml` is still created empty.

### Fixed
- Panicking at startup when no compositor was reachable. The daemon waits instead, retrying once a
  second and doubling to 30s, and applies its config once a compositor appears.
- Panicking on an unreadable config file, invalid TOML, or a property of the wrong type. Logged, and
  that reload skipped.
- Panicking at startup when a log file could not be opened, or when `--config` named a path that is
  not a regular file.
- Any panic inside a reload ending the reload thread. Panics are caught and logged, and the next
  reload proceeds.
- Panicking instead of reading the cached output state after another thread had poisoned it.
- Config files accepted a position combining a relation and an alignment on the same axis
  (`position = "top-of eDP-1 align-top"`), which laid the outputs on top of each other. The pair
  is now rejected in `wltile-core`, so the daemon's config file and the CLI report the same error.
- The daemon's reload thread panicking on a position string it rejected, after which no SIGHUP or
  output change ever applied a config again. The failure is logged, naming the section, and that
  reload is skipped.

## [v1.3.3]
### Added
- Publishing the `wltile` AUR package automatically from CI on `vX.Y.Z` tags.

### Fixed
- Crashing in `show` when an overlay did not fit the shared memory pool, instead of reporting an
  error.
- Leaking the shared memory file descriptor when preparing the overlay buffer failed.
- `top-of`/`bottom-of` leaking the target output's width into its x coordinate.

## [v1.3.2]
### Fixed
- panicing when passing the reference head by make name

### Changed
- Publishing wltile-core and wltile separately

## [v1.3.1]
### Added
- Fail-fast on a broken compositor connection instead of hanging silently.

### Changed
- Event-driven compositor dispatch (no polling): lower reload latency and idle CPU.

### Fixed
- Applying geometry changes on stale cached configs.
- `set`, `position` and `list` hanging against a slow or starved compositor.
- Configuration applies spuriously failing under load because of outdated serial.
- no output-name overlay displayed in `show`.
- Overlay redraw writing into a neighbouring output's buffer.

## [v1.3.0]
### Fixed
- Crashing when listing displays with a disabled one amongst them.
- Race condition in daemon mode reload configs.

### Added
- Show disabled displays when list.
- Set a display enabled or disabled.

## [v1.2.1]
### Fixed
- Bug trying to create new config file even if exists in daemon mode.

## [v1.2.0]
### Added
- Daemon mode.
- Config file for automatic setup.

## [v1.1.2]
### Added
- pick outputs by serial number, name or make/brand.

 ### Fixed
- Can handle 3+ display setups correctly.

## [v1.1.1]
### Fixed
- Corrupting previously used drawing buffer before compositor releases it.

## [v1.1.0]
### Added
- Verbosity flag (-v[v[v]]).
- `show` command without arguments prints output name on each output.

### Changed
- Default logging verbosity in release to ERROR.

## [v1.0.0]
### Added
- Support Vertical alignment (top and bottom) in position command.

## [v0.3.4]
### Fixed
- Fix positioning for rotated outputs.

## [v0.3.3]
### Added
- Implement changing the orientation of an output.

## [v0.3.2]
### Fixed
- Bug detecting scale changes

## [v0.3.1]
### Added
- Logging level dectated by RUST_LOG env variable.
- Implement changing the "scale" property for an output.

## [v0.3.0]
### Added
- Add a command "Set <PROPERTY> <VALUE>" to change output properties.
- Implement changing the "mode" property for an output.

## [v0.2.0]
### Added
- Show physical size when listing outputs.
- Added a GPLv3 license file.
- Add a command "Show <OUTPUT>" to show more details for the specified output.

## [v0.1.1]
### Added
- Show the scaling factor when listing outputs
- Show the refresh rate when listing outputs

## [v0.1.1-alpha.2]
### Added
- lists all displays along with relevant info for positioning them.
- position two displays relative to each other either left or right.
- while positioning the displays, align them to bottom or top.

[unreleased]: https://github.com/kdehairy/wltile/compare/v1.3.3...main
[v1.3.3]: https://github.com/kdehairy/wltile/compare/v1.3.2...v1.3.3
[v1.3.2]: https://github.com/kdehairy/wltile/compare/v1.3.1...v1.3.2
[v1.3.1]: https://github.com/kdehairy/wltile/compare/v1.3.0...v1.3.1
[v1.3.0]: https://github.com/kdehairy/wltile/compare/v1.2.1...v1.3.0
[v1.2.1]: https://github.com/kdehairy/wltile/compare/v1.2.0...v1.2.1
[v1.2.0]: https://github.com/kdehairy/wltile/compare/v1.1.2...v1.2.0
[v1.1.2]: https://github.com/kdehairy/wltile/compare/v1.1.1...v1.1.2
[v1.1.1]: https://github.com/kdehairy/wltile/compare/v1.1.0...v1.1.1
[v1.1.0]: https://github.com/kdehairy/wltile/compare/v1.0.0...v1.1.0
[v1.0.0]: https://github.com/kdehairy/wltile/compare/v0.3.4...v1.0.0
[v0.3.4]: https://github.com/kdehairy/wltile/compare/v0.3.3...v0.3.4
[v0.3.3]: https://github.com/kdehairy/wltile/compare/v0.3.2...v0.3.3
[v0.3.2]: https://github.com/kdehairy/wltile/compare/v0.3.1...v0.3.2
[v0.3.1]: https://github.com/kdehairy/wltile/compare/v0.3.0...v0.3.1
[v0.3.0]: https://github.com/kdehairy/wltile/compare/v0.2.0...v0.3.0
[v0.2.0]: https://github.com/kdehairy/wltile/compare/v0.1.1...v0.2.0
[v0.1.1]: https://github.com/kdehairy/wltile/compare/v0.1.1-alpha.1...v0.1.1
[v0.1.1-alpha.2]: https://github.com/kdehairy/wltile/commits/v0.1.1-alpha.2
