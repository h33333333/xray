use std::borrow::Cow;
use std::ops::Deref;

use super::util::{Field, FieldKey};
use crate::tui::action::Direction;
use crate::tui::util::{Unit, ValueWithStringRepresentation};
use crate::{render_order_enum, sort_fields_by_render_order};

render_order_enum!(ImageInfoField, Repository, Tag, Size, Architecture, Os);
sort_fields_by_render_order!(ImageInfoField);

impl FieldKey for ImageInfoField {
    fn name(&self) -> &'static str {
        match self {
            ImageInfoField::Repository => "Image",
            ImageInfoField::Tag => "Tag",
            ImageInfoField::Size => "Image Size",
            ImageInfoField::Architecture => "Architecture",
            ImageInfoField::Os => "OS",
        }
    }
}

/// [super::Pane::ImageInfo] pane's state.
#[derive(Debug)]
pub struct ImageInfoPane {
    pub active_field: ImageInfoField,
    pub image_name: Cow<'static, str>,
    pub tag: Cow<'static, str>,
    pub size: ValueWithStringRepresentation<u64>,
    pub architecture: String,
    pub os: String,
}

impl ImageInfoPane {
    pub fn new(
        image_name: Cow<'static, str>,
        tag: Cow<'static, str>,
        size: u64,
        architecture: String,
        os: String,
    ) -> Self {
        ImageInfoPane {
            active_field: ImageInfoField::default(),
            image_name,
            tag,
            size: ValueWithStringRepresentation::new(size),
            architecture,
            os,
        }
    }

    pub fn get_fields(&self) -> [Field<'_, ImageInfoField>; 5] {
        let (image_size, unit) =
            Unit::bytes_to_human_readable_units(*self.size.deref());
        let mut fields = [
            (ImageInfoField::Repository, self.image_name.as_ref().into()),
            (ImageInfoField::Tag, self.tag.as_ref().into()),
            (
                ImageInfoField::Size,
                format!("{:.1} {}", image_size, unit.human_readable()).into(),
            ),
            (ImageInfoField::Architecture, (&self.architecture).into()),
            (ImageInfoField::Os, (&self.os).into()),
        ];
        // Ensure that fields are always sorted in the order determined by `ImageInfoField`.
        // This is not necessary, but ensures that there is only a single source of truth for the order of fields inside the pane.
        ImageInfoField::sort_fields_by_order(&mut fields);
        fields
    }

    pub fn toggle_active_field(&mut self, direction: Direction) {
        self.active_field.toggle(direction);
    }
}
