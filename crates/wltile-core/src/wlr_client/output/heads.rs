use std::collections::HashMap;
use std::fmt::Display;

use wayland_client::protocol::wl_output::Transform;

use crate::wlr_client::errors::ClientError;
use crate::wlr_client::output::configs::Configurations;
use crate::wlr_client::output::rotation::Rotation;
use crate::wlr_client::output::wlr_head::OutputHead;
use crate::wlr_client::output::wlr_mode::OutputMode;
use crate::wlr_client::point::Point;

/// A snapshot of the connected outputs, as returned by
/// [`Client::heads`](crate::Client::heads).
///
/// It doesn't update; call [`Client::heads`](crate::Client::heads) again for
/// fresh data.
#[derive(Default)]
pub struct Heads {
    heads: HashMap<String, Head>,
}

impl Heads {
    pub(crate) fn insert(&mut self, name: String, head: Head) {
        self.heads.insert(name, head);
    }

    /// All outputs in the snapshot, in no particular order.
    #[must_use]
    pub fn heads(&self) -> Vec<&Head> {
        self.heads.values().collect()
    }

    /// Finds an output by its name (`eDP-1`), its serial number, or a
    /// case-insensitive substring of its make (`dell`).
    ///
    /// If more than one output matches, any one of them may be returned.
    #[must_use]
    pub fn find(&self, expr: &str) -> Option<&Head> {
        self.heads().into_iter().find(|&head| {
            head.serial_number() == expr
                || head.name() == expr
                || head.make().to_lowercase().contains(&expr.to_lowercase())
        })
    }
}

/// A video mode an output supports: a resolution and a refresh rate.
///
/// Modes are ordered by resolution (see [`Point`]), then by refresh rate.
#[derive(Eq, PartialEq, Clone)]
pub struct Mode {
    id: u32,
    size: Point,
    refresh: i32,
    prefered: bool,
}

impl Mode {
    pub(crate) fn new(id: u32, output_mode: &OutputMode) -> Self {
        Self {
            id,
            size: output_mode.size(),
            refresh: output_mode.refresh(),
            prefered: output_mode.prefered(),
        }
    }

    /// An identifier for this mode, assigned by the [`Client`](crate::Client)
    /// that produced it. Only meaningful within that client.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// The resolution in pixels, as `(width, height)`.
    #[must_use]
    pub fn size(&self) -> Point {
        self.size
    }

    /// The refresh rate in mHz (e.g. `60000` for 60 Hz).
    #[must_use]
    pub fn refresh(&self) -> i32 {
        self.refresh
    }

    /// Whether this is the output's preferred mode, usually its native
    /// resolution.
    #[must_use]
    pub fn prefered(&self) -> bool {
        self.prefered
    }
}

impl Ord for Mode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.size.cmp(&other.size) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.refresh.cmp(&other.refresh)
    }
}

impl PartialOrd for Mode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// One output (display) as the compositor reports it.
#[derive(PartialEq, Clone)]
pub struct Head {
    id: u32,
    name: String,
    description: String,
    physical_size: Point,
    current_mode: Option<Mode>,
    modes: Vec<Mode>,
    enabled: bool,
    position: Point,
    make: String,
    model: String,
    serial_number: String,
    scale: f64,
    transform: Transform,
}

impl Head {
    pub(crate) fn new(
        output_head: &OutputHead,
        output_mode: Option<&OutputMode>,
        configs: &Configurations,
    ) -> Result<Self, ClientError> {
        let head_id = configs.to_head_idx(&output_head.id()).ok_or("No head id")?;
        let modes = configs.get_head_modes(head_id)?;
        let current_mode = output_mode
            .map(|output_mode| -> Result<Mode, ClientError> {
                let mode_id = configs
                    .to_mode_idx(output_mode.wl_id())
                    .ok_or("No current mode id")?;
                Ok(Mode {
                    id: mode_id,
                    size: output_mode.size(),
                    refresh: output_mode.refresh(),
                    prefered: output_mode.prefered(),
                })
            })
            .transpose()?;
        Ok(Head {
            id: head_id,
            name: output_head.name().to_string(),
            description: output_head.description().to_string(),
            physical_size: output_head.physical_size(),
            current_mode,
            enabled: output_head.enabled(),
            position: output_head.position(),
            make: output_head.make().to_string(),
            model: output_head.model().to_string(),
            serial_number: output_head.serial_number().to_string(),
            scale: output_head.scale(),
            transform: output_head.transform(),
            modes,
        })
    }

    /// An identifier for this output, assigned by the
    /// [`Client`](crate::Client) that produced it. Only meaningful within that
    /// client.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// The current mode, or `None` if the output is disabled.
    #[must_use]
    pub fn mode(&self) -> Option<&Mode> {
        self.current_mode.as_ref()
    }

    /// The connector name, e.g. `eDP-1` or `DP-2`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The serial number, or an empty string if the output doesn't report one.
    #[must_use]
    pub fn serial_number(&self) -> &str {
        &self.serial_number
    }

    /// Whether the output is enabled.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// The manufacturer, e.g. `Dell Inc.`.
    #[must_use]
    pub fn make(&self) -> &str {
        &self.make
    }

    /// The model name, e.g. `DELL S3423DWC`.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The top-left corner of the output in the compositor's layout, in
    /// logical pixels.
    #[must_use]
    pub fn position(&self) -> Point {
        self.position
    }

    /// The physical size in millimeters, as `(width, height)`. `(0, 0)` if
    /// unknown.
    #[must_use]
    pub fn physical_size(&self) -> Point {
        self.physical_size
    }

    /// Every mode the output supports, in the order the compositor
    /// advertised them.
    #[must_use]
    pub fn modes(&self) -> &Vec<Mode> {
        &self.modes
    }

    /// The scale factor, e.g. `2.0` for a high-DPI display.
    #[must_use]
    pub fn scale(&self) -> f64 {
        self.scale
    }

    /// How the output's image is rotated and/or flipped.
    #[must_use]
    pub fn rotation(&self) -> Rotation {
        Rotation::from(self.transform)
    }

    pub(crate) fn transform(&self) -> Transform {
        self.transform
    }

    #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
    pub(crate) fn scaled_corrected_size(&self) -> Result<Point, ClientError> {
        let mode = self.mode().ok_or("no current mode; cannot compute size")?;

        let hight = f64::from(mode.size().1) / self.scale();
        let hight = hight.round() as i32;

        let width = f64::from(mode.size().0) / self.scale();
        let width = width.round() as i32;

        Ok(if self.is_vertical() {
            Point(hight, width)
        } else {
            Point(width, hight)
        })
    }

    fn is_vertical(&self) -> bool {
        match self.transform() {
            Transform::Normal | Transform::_180 | Transform::Flipped | Transform::Flipped180 => {
                false
            }
            Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => true,
            _ => panic!("Unexpected value for transform"),
        }
    }
}

impl Display for Head {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size = self
            .mode()
            .map_or_else(|| "N/A".to_string(), |mode| mode.size().to_string());
        write!(
            f,
            "{} => {} {} {} @ {}",
            self.name(),
            self.make(),
            self.model(),
            size,
            self.position()
        )
    }
}
