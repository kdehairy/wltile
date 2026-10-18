[![crates.io](https://img.shields.io/crates/v/wltile-core.svg)](https://crates.io/crates/wltile-core)
[![docs.rs](https://img.shields.io/docsrs/wltile-core)](https://docs.rs/wltile-core)
[![CI](https://github.com/kdehairy/wltile/actions/workflows/ci.yml/badge.svg)](https://github.com/kdehairy/wltile/actions/workflows/ci.yml)

# Overview
`wltile-core` is a library for managing outputs/displays on wlroots based compositors, using the
`zwlr-output-management-v1` Wayland protocol. It is the engine behind the
[`wltile`](https://crates.io/crates/wltile) CLI, split out so other tools can reuse it.

It can:
- list connected outputs and inspect their make, model, modes, scale, rotation and position,
- position outputs relative to one another,
- set properties like mode, scale, rotation and active/disabled state,
- notify you when the output configuration changes (displays connected or removed),
- run the `wltile` daemon loop, which re-applies a config file on every output change.

The API is still young (`0.x`) and may change between minor versions.

# Requirements
A Wayland session on a compositor that exposes `zwlr_output_manager_v1` at version 4 (e.g. sway).

# Install
```sh
cargo add wltile-core
```

# Usage
Everything goes through a `Client`, which connects to the compositor and keeps an up-to-date
cache of its outputs on a background thread.

## Inspect outputs
```rust
use wltile_core::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;
    for head in client.heads()?.heads() {
        let pos = head.position();
        println!("{} ({} {}) at ({}, {})", head.name(), head.make(), head.model(), pos.0, pos.1);
    }
    Ok(())
}
```

`Heads::find` looks an output up by its name (`eDP-1`), its serial number, or a
case-insensitive substring of its make (`dell`).

## Position outputs
Describe the layout you want as a `Config` of `Target`s, then apply it:

```rust
use wltile_core::Client;
use wltile_core::wl_config::{Alignment, Config, Position, Relation, Target};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;

    let mut config = Config::default();
    config.add_target(Target {
        position: Some(Position {
            relation: Relation::LeftOf,
            reference: "eDP-1".into(),
            alignment: Alignment::AlignBottom,
        }),
        ..Target::new("dell".into())
    });

    wltile_core::position(&config, &client)?;
    Ok(())
}
```

`position` keeps the whole layout anchored at the origin, shifting outputs that aren't part of
the config by the same offset.

## Set properties
```rust
use wltile_core::Client;
use wltile_core::wl_config::{Config, Target};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;

    let mut config = Config::default();
    config.add_target(Target {
        scale: Some(2.0),
        rotation: Some(90),
        ..Target::new("eDP-1".into())
    });

    wltile_core::set_property(&config, &client)?;
    Ok(())
}
```

- `mode` is an index into the output's modes, sorted from largest/fastest to smallest.
- `rotation` is in degrees and must be a multiple of 90.
- `active` enables (`true`) or disables (`false`) the output.

## React to output changes
```rust
use wltile_core::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;
    for () in client.subscribe().iter() {
        println!("outputs changed: {} connected", client.heads()?.heads().len());
    }
    Ok(())
}
```

## Run the daemon
`daemon::daemon_main` runs the same loop as `wltile daemon`: it applies a TOML config file
whenever outputs change or the process receives `SIGHUP`, and returns on `SIGTERM`/`SIGINT`.
See the [`wltile` README](https://github.com/kdehairy/wltile#readme) for the config file format.

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    wltile_core::daemon::daemon_main("config.toml".into())
}
```

## Show output names
`wltile_core::show(&mut client)` draws each output's name on that output and blocks until a key
is pressed. It needs a compositor with XDG shell support and a keyboard seat.

# License
GPL-3.0-or-later
