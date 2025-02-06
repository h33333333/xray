use std::borrow::Cow;

use super::util::Field;
use crate::parser::{Layer, Sha256Digest};
use crate::tui::action::Direction;
use crate::tui::util::encode_hex;

pub fn get_fields<'a>(digest: &'a Sha256Digest, layer: &'a Layer) -> [Field<'a>; 3] {
    let comment: Cow<'a, str> = if let Some(comment) = layer.comment.as_ref() {
        comment.into()
    } else {
        "<missing>".into()
    };
    [
        ("Digest", encode_hex(digest).into()),
        ("Command", (&layer.created_by).into()),
        ("Comment", comment),
    ]
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
/// Currently selected field in the [Pane::LayerInfo] pane.
pub enum LayerInfoActiveField {
    #[default]
    Digest,
    Command,
    Comment,
}

impl LayerInfoActiveField {
    const FIELD_ORDER: [LayerInfoActiveField; 3] = [
        LayerInfoActiveField::Digest,
        LayerInfoActiveField::Command,
        LayerInfoActiveField::Comment,
    ];

    pub fn toggle(&mut self, direction: Direction) {
        let current_idx = Self::FIELD_ORDER.iter().position(|field| field == self).unwrap();

        let next_idx = match direction {
            Direction::Forward => (current_idx + 1) % Self::FIELD_ORDER.len(),
            Direction::Backward => (current_idx + Self::FIELD_ORDER.len() - 1) % Self::FIELD_ORDER.len(),
        };

        *self = Self::FIELD_ORDER[next_idx];
    }
}
