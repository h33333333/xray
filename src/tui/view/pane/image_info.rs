use super::util::{Field, FieldKey};
use crate::tui::util::bytes_to_human_readable_units;
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

#[derive(Debug)]
/// [super::Pane::ImageInfo] pane's state.
pub struct ImageInfoPane {
    pub active_field: ImageInfoField,
    pub repository: String,
    pub tag: String,
    pub size: u64,
    pub architecture: String,
    pub os: String,
}

impl ImageInfoPane {
    pub fn new(repository: String, tag: String, size: u64, architecture: String, os: String) -> Self {
        ImageInfoPane {
            active_field: ImageInfoField::default(),
            repository,
            tag,
            size,
            architecture,
            os,
        }
    }

    pub fn get_fields(&self) -> [Field<'_, ImageInfoField>; 5] {
        let (image_size, unit) = bytes_to_human_readable_units(self.size);
        let mut fields = [
            (ImageInfoField::Repository, (&self.repository).into()),
            (ImageInfoField::Tag, (&self.tag).into()),
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
}
