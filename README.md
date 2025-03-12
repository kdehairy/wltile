# Overview
`wltile` is a cli tool that positions outputs/displays for wlroot based compositors.


# Usage
```
wltile <COMMAND>

Commands:
  list      Lists all connected outputs
  show      Shows detailed info for the specified output
  position  Position outputs
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
```sh
$ wltile show DP-2
Make: Lenovo Group Limited
Model: 0x414B
Size: 2880 x 1800
Scale: 2
Transform: 0
Physical Size: 300 x 190 mm
Refresh Rate: 120 kHz
Position: (3440, 540)
Modes:
        > 2880 x 1800 @ 120 kHz
          2880 x 1800 @ 60 kHz
          1920 x 1200 @ 120 kHz
          1920 x 1080 @ 120 kHz
          1600 x 1200 @ 120 kHz
          1680 x 1050 @ 120 kHz
          1280 x 1024 @ 120 kHz
          1440 x 900 @ 120 kHz
          1280 x 800 @ 120 kHz
          1280 x 720 @ 120 kHz
          1024 x 768 @ 120 kHz
          800 x 600 @ 120 kHz
          640 x 480 @ 120 kHz
```

## Position
```
wltile position <TARGET_OUTPUT> <RELATION> <REFERENCE_OUTPUT> [ALIGNMENT]

Arguments:
  <TARGET_OUTPUT>     Output to be positioned
  <RELATION>          How is it positioned to the reference output [possible values: left-of, right-of]
  <REFERENCE_OUTPUT>  Reference Output
  [ALIGNMENT]         Alignment [default: align-bottom] [possible values: align-bottom, align-top]

Options:
  -h, --help  Print help
```

Example:

```sh
$ wltile position DP-2 left-of eDP-1 align-bottom
```

# Roadmap
- Change output mode (resolution and refresh rate).
- Change output orientation.
- Interactive CLI mode
