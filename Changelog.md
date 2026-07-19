# Changelog

## [Unreleased]

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

[unreleased]: https://gitlab.com/eldoheiri/wltile/-/compare/v1.3.0...main
[v1.3.0]: https://gitlab.com/eldoheiri/wltile/-/compare/v1.2.1...v1.3.0
[v1.2.1]: https://gitlab.com/eldoheiri/wltile/-/compare/v1.2.0...v1.2.1
[v1.2.0]: https://gitlab.com/eldoheiri/wltile/-/compare/v1.1.2...v1.2.0
[v1.1.2]: https://gitlab.com/eldoheiri/wltile/-/compare/v1.1.1...v1.1.2
[v1.1.1]: https://gitlab.com/eldoheiri/wltile/-/compare/v1.1.0...v1.1.1
[v1.1.0]: https://gitlab.com/eldoheiri/wltile/-/compare/v1.0.0...v1.1.0
[v1.0.0]: https://gitlab.com/eldoheiri/wltile/-/compare/v0.3.4...v1.0.0
[v0.3.4]: https://gitlab.com/eldoheiri/wltile/-/compare/v0.3.3...v0.3.4
[v0.3.3]: https://gitlab.com/eldoheiri/wltile/-/compare/v0.3.2...v0.3.3
[v0.3.2]: https://gitlab.com/eldoheiri/wltile/-/compare/v0.3.1...v0.3.2
[v0.3.1]: https://gitlab.com/eldoheiri/wltile/-/compare/v0.3.0...v0.3.1
[v0.3.0]: https://gitlab.com/eldoheiri/wltile/-/compare/v0.2.0...v0.3.0
[v0.2.0]: https://gitlab.com/eldoheiri/wltile/-/compare/v0.1.1...v0.2.0
[v0.1.1]: https://gitlab.com/eldoheiri/wltile/-/compare/v0.1.1-alpha.1...v0.1.1
[v0.1.1-alpha.2]: https://gitlab.com/eldoheiri/wltile/-/commits/v0.1.1-alpha.2?ref_type=tags
