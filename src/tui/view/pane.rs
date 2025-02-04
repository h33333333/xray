use std::cmp::Ordering;

use arboard::Clipboard;
use indexmap::IndexMap;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::block::Title;
use ratatui::widgets::{Block, BorderType, Paragraph, Widget};

use crate::parser::{Layer, Sha256Digest};
use crate::tui::action::Direction;
use crate::tui::store::AppState;
use crate::tui::util::bytes_to_human_readable_units;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug)]
pub struct ImageInfoPane {
    active_field: ImageInfoActiveField,
    repository: String,
    tag: String,
    size: u64,
    architecture: String,
    os: String,
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
}

#[derive(Debug)]
pub struct LayerSelectorPane {
    /// The currently selected layer.
    selected_layer_digest: Sha256Digest,
    /// We store both its [LayerSelectorPane::selected_layer_digest] and the index in order to optimize the lookup.
    ///
    /// The index **must** be a valid index that points to an entry in [AppState::layers].
    selected_layer_idx: usize,
}

impl LayerSelectorPane {
    pub fn new(digest: Sha256Digest, idx: usize) -> Self {
        LayerSelectorPane {
            selected_layer_digest: digest,
            selected_layer_idx: idx,
        }
    }
}

/// All panes that exist in the app.
///
/// Each variant can also hold all the state that a particular pane needs, as
/// these variants are created once during the app initialization and are then reused.
pub enum Pane {
    /// Contains all image-related information from [crate::parser::Image].
    ImageInfo(ImageInfoPane),
    LayerSelector(LayerSelectorPane),
    LayerInspector,
}

impl Pane {
    /// Returns a [Widget] that can be used to render the current pane onto the terminal.
    pub fn render<'a>(&'a self, state: &AppState) -> impl Widget + 'a {
        let pane_is_active = state.active_pane.is_pane_active(self);

        let (border_type, border_style) = if pane_is_active {
            (BorderType::Thick, Style::new().white())
        } else {
            (BorderType::Plain, Style::new().gray())
        };

        let block = Block::bordered()
            .border_type(border_type)
            .border_style(border_style)
            .title(self.title(pane_is_active))
            .title_alignment(ratatui::layout::Alignment::Center);

        match self {
            Pane::ImageInfo(ImageInfoPane {
                active_field,
                repository,
                tag,
                size,
                architecture,
                os,
            }) => {
                let color = if pane_is_active { Color::White } else { Color::Gray };

                let field_title_style = || Style::default().bold().fg(color);
                let field_value_style = || Style::default().italic().fg(color);
                let selected_field_style = || Style::default().underlined();

                let mut lines = vec![];

                lines.push(
                    Line::from(vec![
                        Span::styled("Image", field_title_style()),
                        Span::styled(": ", field_value_style()),
                        Span::styled(repository, field_value_style()),
                    ])
                    .style(
                        if matches!(active_field, ImageInfoActiveField::Repository) && pane_is_active {
                            selected_field_style()
                        } else {
                            Style::default()
                        },
                    ),
                );

                lines.push(
                    Line::from(vec![
                        Span::styled("Tag", field_title_style()),
                        Span::styled(": ", field_value_style()),
                        Span::styled(tag, field_value_style()),
                    ])
                    .style(
                        if matches!(active_field, ImageInfoActiveField::Tag) && pane_is_active {
                            selected_field_style()
                        } else {
                            Style::default()
                        },
                    ),
                );

                let (image_size, unit) = bytes_to_human_readable_units(*size);
                lines.push(
                    Line::from(vec![
                        Span::styled("Image size", field_title_style()),
                        Span::styled(
                            format!(": {:.1} {}", image_size, unit.human_readable()),
                            field_value_style(),
                        ),
                    ])
                    .style(
                        if matches!(active_field, ImageInfoActiveField::Size) && pane_is_active {
                            selected_field_style()
                        } else {
                            Style::default()
                        },
                    ),
                );

                lines.push(
                    Line::from(vec![
                        Span::styled("Architecture", field_title_style()),
                        Span::styled(": ", field_value_style()),
                        Span::styled(architecture, field_value_style()),
                    ])
                    .style(
                        if matches!(active_field, ImageInfoActiveField::Architecture) && pane_is_active {
                            selected_field_style()
                        } else {
                            Style::default()
                        },
                    ),
                );

                lines.push(
                    Line::from(vec![
                        Span::styled("OS", field_title_style()),
                        Span::styled(": ", field_value_style()),
                        Span::styled(os, field_value_style()),
                    ])
                    .style(
                        if matches!(active_field, ImageInfoActiveField::Os) && pane_is_active {
                            selected_field_style()
                        } else {
                            Style::default()
                        },
                    ),
                );

                Paragraph::new(Text::from(lines)).block(block)
            }
            Pane::LayerSelector(LayerSelectorPane { selected_layer_idx, .. }) => {
                let color = if pane_is_active { Color::White } else { Color::Gray };

                let field_value_style = || Style::default().fg(color);
                let layer_colored_block_indicator_style = |layer_idx: usize| {
                    let style = Style::default();
                    match layer_idx.cmp(selected_layer_idx) {
                        Ordering::Equal => style.bg(Color::LightGreen),
                        Ordering::Less => style.bg(Color::LightMagenta),
                        Ordering::Greater => style,
                    }
                };

                let lines = state
                    .layers
                    .iter()
                    .enumerate()
                    .map(|(idx, (_, layer))| {
                        let (layer_size, unit) = bytes_to_human_readable_units(layer.size);
                        Line::from(vec![
                            // A colored block that acts as an indicator of the currently selected layer.
                            // It's also used to display the layers that are currently used to show aggregated changes.
                            Span::styled("  ", layer_colored_block_indicator_style(idx)),
                            Span::styled(
                                format!(" {:>5.1} {:<2} {}", layer_size, unit.human_readable(), layer.created_by),
                                field_value_style(),
                            ),
                        ])
                    })
                    .collect::<Vec<_>>();

                Paragraph::new(Text::from(lines)).block(block)
            }
            Pane::LayerInspector => Paragraph::new("Layer inspector").block(block),
        }
    }

    /// Moves to the next entry in the specified [Direction] inside the [Pane].
    // FIXME: passing `layers` here is ugly. Can I do something about it?
    pub fn move_within_pane(&mut self, direction: Direction, layers: &IndexMap<Sha256Digest, Layer>) {
        match self {
            Pane::ImageInfo(ImageInfoPane { active_field, .. }) => active_field.toggle(direction),
            Pane::LayerSelector(LayerSelectorPane {
                selected_layer_digest,
                selected_layer_idx,
            }) => {
                // FIXME: move this logic somewhere else
                let current_layer_idx = *selected_layer_idx;
                let next_layer_idx = match direction {
                    Direction::Forward => (current_layer_idx + 1) % layers.len(),
                    Direction::Backward => (current_layer_idx + layers.len() - 1) % layers.len(),
                };

                let (digest, _) = layers
                    .get_index(next_layer_idx)
                    .expect("the logic above ensures that idx is valid");

                *selected_layer_digest = *digest;
                *selected_layer_idx = next_layer_idx;
            }
            _ => {}
        }
    }

    /// Copies the currently selected value to the [Clipboard].
    pub fn copy(&self, clipboard: &mut Clipboard) {
        let text_to_copy = match self {
            Pane::ImageInfo(ImageInfoPane {
                active_field,
                repository,
                tag,
                size,
                architecture,
                os,
            }) => match active_field {
                ImageInfoActiveField::Repository => repository,
                ImageInfoActiveField::Tag => tag,
                // FIXME: this is kinda ugly, can I do better somehow?
                ImageInfoActiveField::Size => &format!("{}", size),
                ImageInfoActiveField::Architecture => architecture,
                ImageInfoActiveField::Os => os,
            },
            _ => return,
        };
        if let Err(e) = clipboard.set_text(text_to_copy) {
            tracing::debug!("Failed to copy text to the clipboard: {}", e);
        };
    }

    /// Returns the pane's title.
    fn title(&self, is_active: bool) -> impl Into<Title<'static>> {
        let title = match self {
            Pane::ImageInfo(..) => "Image information",
            Pane::LayerSelector(..) => "Layers",
            Pane::LayerInspector => "Layer changes",
        };

        if is_active {
            title.bold().white()
        } else {
            title.not_bold().gray()
        }
    }
}

/// A helper enum that tracks the currently active pane and contains the relevant pane-related logic.
///
/// This logic was extracted from [Pane] to avoid having a copy of the currently active [Pane] in [AppState] and instead
/// use a simple and small enum that doesn't hold any pane-related state.
#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum ActivePane {
    #[default]
    ImageInfo,
    LayerSelector,
    LayerInspector,
}

impl ActivePane {
    /// Returns an array of all panes in their cycling order.
    const PANE_ORDER: [ActivePane; 3] = [
        ActivePane::ImageInfo,
        ActivePane::LayerSelector,
        ActivePane::LayerInspector,
    ];

    /// Changes the current pane to the next one.
    pub fn toggle(&mut self, direction: Direction) {
        let current_index = Self::PANE_ORDER.iter().position(|pane| pane == self).unwrap();

        let next_index = match direction {
            Direction::Forward => (current_index + 1) % Self::PANE_ORDER.len(),
            Direction::Backward => (current_index + Self::PANE_ORDER.len() - 1) % Self::PANE_ORDER.len(),
        };

        *self = Self::PANE_ORDER[next_index];
    }

    /// Checks if the provided [Pane] is the currently active one.
    pub fn is_pane_active(&self, pane: &Pane) -> bool {
        match self {
            ActivePane::ImageInfo if matches!(pane, Pane::ImageInfo(..)) => true,
            ActivePane::LayerSelector if matches!(pane, Pane::LayerSelector(..)) => true,
            ActivePane::LayerInspector if matches!(pane, Pane::LayerInspector) => true,
            _ => false,
        }
    }

    pub fn pane_idx(&self) -> usize {
        *self as usize
    }
}
