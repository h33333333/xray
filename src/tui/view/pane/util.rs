use std::borrow::Cow;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::style::FIELD_VALUE_DELIMITER;

type FieldKey = &'static str;
type FieldValue<'a> = Cow<'a, str>;
pub type Field<'a> = (FieldKey, FieldValue<'a>);

pub fn fields_into_lines<'a>(
    fields: impl IntoIterator<Item = Field<'a>>,
    field_key_style: Style,
    field_value_style: Style,
    get_style_for_field: impl Fn(usize) -> Style,
) -> Vec<Line<'a>> {
    fields
        .into_iter()
        .enumerate()
        .map(|(idx, (field_key, field_value))| {
            Line::from(vec![
                Span::styled(field_key, field_key_style),
                Span::styled(FIELD_VALUE_DELIMITER, field_value_style),
                Span::styled(field_value, field_value_style),
            ])
            .style(get_style_for_field(idx))
        })
        .collect::<Vec<_>>()
}
