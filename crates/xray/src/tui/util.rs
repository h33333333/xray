#![allow(unused_doc_comments)]

use std::fmt::Write as _;

use arboard::Clipboard;
use ratatui::layout::{Constraint, Layout, Rect};

use crate::render_order_enum;

type CommandBarArea = Rect;
type PaneAreas = [Rect; 4];

#[derive(Debug)]
pub struct ValueWithStringRepresentation<T> {
    value: T,
    string_representation: String,
}

impl<T: ToString> ValueWithStringRepresentation<T> {
    pub fn new(value: T) -> Self {
        let s = value.to_string();

        ValueWithStringRepresentation {
            value,
            string_representation: s,
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn string_representation(&self) -> &str {
        &self.string_representation
    }
}

/// Represents the unit of a value.
render_order_enum!(Unit, Bytes, Kilobytes, Megabytes, Gigabytes);

impl Unit {
    const KILOBYTE: u64 = 1000;
    const MEGABYTE: u64 = Self::KILOBYTE * 1000;
    const GIGABYTE: u64 = Self::MEGABYTE * 1000;

    pub fn human_readable(&self) -> &'static str {
        match self {
            Unit::Bytes => "B",
            Unit::Kilobytes => "kB",
            Unit::Megabytes => "MB",
            Unit::Gigabytes => "GB",
        }
    }

    pub fn scale_to_units(&self, value: u64) -> u64 {
        match self {
            Unit::Bytes => value,
            Unit::Kilobytes => value * Self::KILOBYTE,
            Unit::Megabytes => value * Self::MEGABYTE,
            Unit::Gigabytes => value * Self::GIGABYTE,
        }
    }
}

/// Converts the passed number of bytes to a human-readable representation using any suitable [Unit].
pub(crate) fn bytes_to_human_readable_units(bytes: impl Into<u64>) -> (f64, Unit) {
    let bytes = bytes.into();
    match bytes {
        0..Unit::KILOBYTE => (bytes as f64, Unit::Bytes),
        Unit::KILOBYTE..Unit::MEGABYTE => ((bytes as f64) / (Unit::KILOBYTE as f64), Unit::Kilobytes),
        Unit::MEGABYTE..Unit::GIGABYTE => ((bytes as f64) / (Unit::MEGABYTE as f64), Unit::Megabytes),
        Unit::GIGABYTE.. => ((bytes as f64) / (Unit::GIGABYTE as f64), Unit::Gigabytes),
    }
}

pub(crate) fn encode_hex(digest: impl AsRef<[u8]>) -> String {
    let mut s = String::with_capacity(digest.as_ref().len() * 2);
    for &b in digest.as_ref().iter() {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

/// Copies the provided [data] into [Clipboard] if it's present.
pub(crate) fn copy_to_clipboard(clipboard: Option<&mut Clipboard>, data: std::borrow::Cow<'_, str>) {
    if let Some(clipboard) = clipboard {
        if let Err(e) = clipboard.set_text(data) {
            tracing::debug!("Failed to copy text to the clipboard: {}", e);
        };
    }
}

/// Splits the passed [Rect] into two equal columns, also splitting the first column into three vertical sections.
///
/// Returns an array that contains upper left, middle left, lower left, and right [Rect].
pub(crate) fn split_layout(initial_area: Rect) -> (PaneAreas, CommandBarArea) {
    let [main, command_bar] = Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)]).areas(initial_area);
    let [left, right] = Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(70)]).areas(main);
    let [upper_left, middle_left, lower_left] =
        Layout::vertical([Constraint::Min(8), Constraint::Min(10), Constraint::Percentage(100)]).areas(left);

    ([upper_left, middle_left, lower_left, right], command_bar)
}
