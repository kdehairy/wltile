#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Output {
    pub name: String,
    pub active: bool,
    #[serde(default)]
    pub scale: f64,
    pub rect: Rect,
    pub current_mode: Option<Mode>,
}

#[derive(Debug, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Deserialize)]
pub struct Mode {
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
}

/// Parses the JSON emitted by `swaymsg -t get_outputs`.
pub fn parse_outputs(json: &[u8]) -> Vec<Output> {
    serde_json::from_slice(json)
        .unwrap_or_else(|e| panic!("failed to parse swaymsg output: {e}\n{}", String::from_utf8_lossy(json)))
}
