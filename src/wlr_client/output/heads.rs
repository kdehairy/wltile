use std::collections::HashMap;
use std::fmt::Display;

use wayland_client::protocol::wl_output::Transform;

use crate::wlr_client::errors::ClientError;
use crate::wlr_client::output::configs::Configurations;
use crate::wlr_client::output::wlr_head::OutputHead;
use crate::wlr_client::output::wlr_mode::OutputMode;
use crate::wlr_client::point::Point;

#[derive(Default)]
pub struct Heads {
    heads: HashMap<String, Head>,
}

impl Heads {
    pub fn insert(&mut self, name: String, head: Head) {
        self.heads.insert(name, head);
    }

    pub fn heads(&self) -> Vec<&Head> {
        self.heads.values().collect()
    }

    pub fn find(&self, expr: &str) -> Option<&Head> {
        self.heads().into_iter().find(|&head| {
            head.serial_number() == expr
                || head.name() == expr
                || head.make().to_lowercase().contains(&expr.to_lowercase())
        })
    }
}

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

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn size(&self) -> Point {
        self.size
    }

    pub fn refresh(&self) -> i32 {
        self.refresh
    }

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

#[derive(PartialEq, Clone)]
pub struct Head {
    id: u32,
    name: String,
    description: String,
    physical_size: Point,
    current_mode: Mode,
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
    pub fn new(
        output_head: &OutputHead,
        output_mode: &OutputMode,
        configs: &Configurations,
    ) -> Result<Self, ClientError> {
        let mode_id = configs
            .to_mode_idx(output_mode.wl_id())
            .ok_or("No current mode id")?;
        let head_id = configs.to_head_idx(&output_head.id()).ok_or("No head id")?;
        let modes = configs.get_head_modes(head_id)?;
        Ok(Head {
            id: head_id,
            name: output_head.name().to_string(),
            description: output_head.description().to_string(),
            physical_size: output_head.physical_size(),
            current_mode: Mode {
                id: mode_id,
                size: output_mode.size(),
                refresh: output_mode.refresh(),
                prefered: output_mode.prefered(),
            },
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

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn mode(&self) -> &Mode {
        &self.current_mode
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn serial_number(&self) -> &str {
        &self.serial_number
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn make(&self) -> &str {
        &self.make
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn position(&self) -> Point {
        self.position
    }

    pub fn physical_size(&self) -> Point {
        self.physical_size
    }

    pub fn modes(&self) -> &Vec<Mode> {
        &self.modes
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }

    #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
    pub fn scaled_corrected_size(&self) -> Point {
        let hight = f64::from(self.mode().size().1) / self.scale();
        let hight = hight.round() as i32;

        let width = f64::from(self.mode().size().0) / self.scale();
        let width = width.round() as i32;

        if self.is_vertical() {
            Point(hight, width)
        } else {
            Point(width, hight)
        }
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
        write!(
            f,
            "{} => {} {} {} @ {}",
            self.name(),
            self.make(),
            self.model(),
            self.current_mode.size(),
            self.position()
        )
    }
}
