# Overview
`wltile` is a cli tool that positions outputs/displays for wlroots based compositors.

# Install
## Cargo
```sh
cargo install wltile`
```

### Arch Linux
Available on AUR https://aur.archlinux.org/packages/wltile

```sh
paru wltile
```

# Usage
```
wltile <COMMAND>

Commands:
  list      Lists all connected outputs
  show      Shows the current layout. If an output is provided as argument, it shows detailed info
            for the specified output
  position  Position outputs
  set       Sets properties of the output to a desired value
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## List
```sh
$ wltile list
eDP-1:
	Make: Lenovo Group Limited 0x414B
	Size: 2880 x 1800 scale: 2
	Refresh Rate: 120 kHz
	Position: (3440, 540)
DP-2:
	Make: Dell Inc. DELL S3423DWC
	Size: 3440 x 1440 scale: 1
	Refresh Rate: 60 kHz
	Position: (0, 0)
```

## Show
```
wltile show [OUTPUT]

Arguments:
  [OUTPUT]  Could be a serial number, name or partial match with the screen make

Options:
  -h, --help  Print help
```

Example: Print display's name on each display
```sh
wltile show
```

Example: Show details about a specific display/output
```sh
$ wltile show DP-2
Make: Lenovo Group Limited
Model: 0x414B
Size: 2880 x 1800
Scale: 2
Transform: 0
Physical Size: 300 x 190 mm
Refresh Rate: 120 kHz
Position: (0, 0)
Modes:
        > 0. 2880 x 1800 @ 120 kHz (*)
          1. 2880 x 1800 @ 60 kHz
          2. 1920 x 1200 @ 120 kHz
          3. 1920 x 1080 @ 120 kHz
          4. 1600 x 1200 @ 120 kHz
          5. 1680 x 1050 @ 120 kHz
          6. 1440 x 900 @ 120 kHz
          7. 1280 x 1024 @ 120 kHz
          8. 1280 x 800 @ 120 kHz
          9. 1280 x 720 @ 120 kHz
          10. 1024 x 768 @ 120 kHz
          11. 800 x 600 @ 120 kHz
          12. 640 x 480 @ 120 kHz
```
Modes format is as following: `<ordinal> <resolution> @ <refresh rate> <prefered mode>`.
The `<ordinal>` is used later to refer to the mode in other operations (for
example to set the mode to a desired one).

Modes are always shown ordered from the highest resolution and referesh rate,
to the lowest. This makes the ordinals stable between runs.

## Position
```
wltile position <TARGET_OUTPUT> <RELATION> <REFERENCE_OUTPUT> [ALIGNMENT]

Arguments:
  <TARGET_OUTPUT>     Output to be positioned
  <RELATION>          How is it positioned to the reference output [possible values: left-of, right-of, top-of, bottom-of]
  <REFERENCE_OUTPUT>  Reference Output
  [ALIGNMENT]         Alignment [default: align-bottom] [possible values: align-bottom, align-top, align-right, align-left]

Options:
  -h, --help  Print help
```

Example:

```sh
$ wltile position DP-2 left-of eDP-1 align-bottom
```

## Set
```
wltile set <TARGET_OUTPUT> <PROPERTY> <VALUE>

Arguments:
  <TARGET_OUTPUT>
  <PROPERTY>       [possible values: mode, scale, rotation]
  <VALUE>

Options:
  -h, --help  Print help
```

Example:
```sh
$ wltile show DP-2 # Show the supported modes
Make: Lenovo Group Limited
Model: 0x414B
Size: 2880 x 1800
Scale: 2
Transform: 0
Physical Size: 300 x 190 mm
Refresh Rate: 120 kHz
Position: (0, 0)
Modes:
        > 0. 2880 x 1800 @ 120 kHz (*)
          1. 2880 x 1800 @ 60 kHz
          2. 1920 x 1200 @ 120 kHz
          3. 1920 x 1080 @ 120 kHz
          4. 1600 x 1200 @ 120 kHz
          5. 1680 x 1050 @ 120 kHz
          6. 1440 x 900 @ 120 kHz
          7. 1280 x 1024 @ 120 kHz
          8. 1280 x 800 @ 120 kHz
          9. 1280 x 720 @ 120 kHz
          10. 1024 x 768 @ 120 kHz
          11. 800 x 600 @ 120 kHz
          12. 640 x 480 @ 120 kHz

# set the desired mode using shown ordinal
$ wltile set DP-2 mode 3

$ wltile set DP-2 scale 1.5

$ wltile set DP-2 rotation 270

```
