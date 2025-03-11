# Overview
`wltile` is a cli tool that positions outputs/displays for wlroot based compositors.


# Usage
```
wltile <COMMAND>

Commands:
  list      Lists all connected outputs
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

# Limitations
This tool is still under developement.

That said, my aim is to do a first release that:
- lists all displays along with relevant info for positioning them.
- position two displays relative to each other either left or right.
- while positioning the displays, align them to bottom or top.

Currently it does all of that, but not properly handling failures.
