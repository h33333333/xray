use std::borrow::Cow;

use super::util::{Field, FieldKey};
use crate::parser::{Layer, Sha256Digest};
use crate::tui::util::encode_hex;
use crate::{render_order_enum, sort_fields_by_render_order};

const MISSING_COMMENT_PLACEHOLDER: &str = "<missing>";

render_order_enum!(LayerInfoField, Digest, Command, Comment);
sort_fields_by_render_order!(LayerInfoField);

impl FieldKey for LayerInfoField {
    fn name(&self) -> &'static str {
        match self {
            LayerInfoField::Digest => "Digest",
            LayerInfoField::Command => "Command",
            LayerInfoField::Comment => "Comment",
        }
    }
}

#[derive(Debug, Default)]
/// [super::Pane::LayerInfo] pane's state.
pub struct LayerInfoPane {
    pub active_field: LayerInfoField,
}

impl LayerInfoPane {
    pub fn get_fields<'a>(digest: &'a Sha256Digest, layer: &'a Layer) -> [Field<'a, LayerInfoField>; 3] {
        let comment: Cow<'a, str> = if let Some(comment) = layer.comment.as_ref() {
            comment.into()
        } else {
            MISSING_COMMENT_PLACEHOLDER.into()
        };
        let mut fields = [
            (LayerInfoField::Digest, encode_hex(digest).into()),
            (LayerInfoField::Command, (&layer.created_by).into()),
            (LayerInfoField::Comment, comment),
        ];
        // Ensure that fields are always sorted in the order determined by `LayerInfoField`.
        // This is not necessary, but ensures that there is only a single source of truth for the order of fields inside the pane.
        LayerInfoField::sort_fields_by_order(&mut fields);
        fields
    }
}
