# Overview
`wayout` is a cli tool that positions outputs/displays for wlroot based compositors.

# Usage Examples
## Listing existing displays and their positions
```sh
$ wayout list
DP-2:
        Make: Dell Inc. DELL S3423DWC
        Size: 3440 x 1440
        Position: (0, 0)
eDP-1:
        Make: Samsung Display Corp. 0x419F
        Size: 2880 x 1800
        Position: (3440, 540)
```

## Positioning DP-2 left of eDP-1 and align them to the bottom
```sh
$ wayout position DP-2 left-of eDP-1 align-bottom
```

# Limitations
This tool is still under developement.

That said, my aim is to do a first release that:
- lists all displays along with relevant info for positioning them.
- position two displays relative to each other either left or right.
- while positioning the displays, align them to bottom or top.

Currently it does all of that, but not properly handling failures.
