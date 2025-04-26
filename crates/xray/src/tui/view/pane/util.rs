use std::borrow::Cow;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::style::FIELD_VALUE_DELIMITER;

/// A trait that represents a value that can be converted to a human-readable field name.
pub trait FieldKey {
    /// Returns a human-readable name for this field.
    fn name(&self) -> &'static str;
}

type FieldValue<'a> = Cow<'a, str>;

/// A single field within a pane with its key and [value](FieldValue).
pub type Field<'a, K> = (K, FieldValue<'a>);

/// Creates [Lines](Line) from the passed iterator over [Fields](Field) for some pane.
///
/// # Note
///
/// As this function is generic, it's not possible to render fields of multiple panes at the same time,
/// but this is not needed in any of the current use cases, so it's fine by me.
pub fn fields_into_lines<'a, K: FieldKey>(
    fields: impl IntoIterator<Item = Field<'a, K>>,
    field_key_style: Style,
    field_value_style: Style,
    get_style_for_field: impl Fn(K) -> Style,
) -> Vec<Line<'a>> {
    fields
        .into_iter()
        .map(|(field_key, field_value)| {
            Line::from(vec![
                Span::styled(field_key.name(), field_key_style),
                Span::styled(FIELD_VALUE_DELIMITER, field_key_style),
                Span::styled(field_value, field_value_style),
            ])
            .style(get_style_for_field(field_key))
        })
        .collect::<Vec<_>>()
}
