use super::style::FIELD_VALUE_DELIMITER;
use super::util::Field;
use crate::tui::action::Direction;
use crate::tui::util::bytes_to_human_readable_units;

#[derive(Debug)]
/// [super::Pane::ImageInfo] pane's state.
pub struct ImageInfoPane {
    pub active_field: ImageInfoActiveField,
    pub repository: String,
    pub tag: String,
    pub size: u64,
    pub architecture: String,
    pub os: String,
}

impl ImageInfoPane {
    pub fn new(repository: String, tag: String, size: u64, architecture: String, os: String) -> Self {
        ImageInfoPane {
            active_field: ImageInfoActiveField::default(),
            repository,
            tag,
            size,
            architecture,
            os,
        }
    }

    pub fn get_fields(&self) -> [Field; 5] {
        let (image_size, unit) = bytes_to_human_readable_units(self.size);
        [
            ("Image", (&self.repository).into()),
            ("Tag", (&self.tag).into()),
            (
                "Image Size",
                format!("{}{:.1} {}", FIELD_VALUE_DELIMITER, image_size, unit.human_readable()).into(),
            ),
            ("Architecture", (&self.architecture).into()),
            ("OS", (&self.os).into()),
        ]
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
/// Currently selected field in the [Pane::ImageInfo] pane.
///
/// # Safety
///
/// Variants in this enum should appear in the order they are rendered onto the terminal in [super::Pane::ImageInfo].
pub enum ImageInfoActiveField {
    #[default]
    Repository,
    Tag,
    Size,
    Architecture,
    Os,
}

impl ImageInfoActiveField {
    const FIELD_ORDER: [ImageInfoActiveField; 5] = [
        ImageInfoActiveField::Repository,
        ImageInfoActiveField::Tag,
        ImageInfoActiveField::Size,
        ImageInfoActiveField::Architecture,
        ImageInfoActiveField::Os,
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
