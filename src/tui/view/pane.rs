mod image_info;
mod layer_info;
mod layer_selector;
mod style;
mod util;

use anyhow::Context;
use arboard::Clipboard;
pub use image_info::{ImageInfoActiveField, ImageInfoPane};
use indexmap::IndexMap;
pub use layer_info::LayerInfoActiveField;
pub use layer_selector::LayerSelectorPane;
use ratatui::style::{Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::block::Title;
use ratatui::widgets::{Block, BorderType, Paragraph, Widget, Wrap};
use style::{text_color, ACTIVE_FIELD_STYLE, FIELD_KEY_STYLE, FIELD_VALUE_STYLE};
use util::fields_into_lines;

use crate::parser::{Layer, Sha256Digest};
use crate::tui::action::Direction;
use crate::tui::store::AppState;

/// All panes that exist in the app.
///
/// Each variant can also hold all the state that a particular pane needs, as
/// these variants are created once during the app initialization and are then reused.
pub enum Pane {
    /// Contains all image-related information from [crate::parser::Image].
    ImageInfo(ImageInfoPane),
    /// Displays infromation about the [LayerSelectorPane::selected_layer].
    LayerInfo(LayerInfoActiveField),
    /// Allows switching between [Layers](Layer) of the [crate::parser::Image].
    LayerSelector(LayerSelectorPane),
    LayerInspector,
}

impl Pane {
    /// Returns a [Widget] that can be used to render the current pane onto the terminal.
    pub fn render<'a>(&'a self, state: &'a AppState) -> anyhow::Result<impl Widget + 'a> {
        let pane_is_active = state.active_pane.is_pane_active(self);

        let text_color = text_color(pane_is_active);
        let field_key_style = FIELD_KEY_STYLE.fg(text_color);
        let field_value_style = FIELD_VALUE_STYLE.fg(text_color);
        let active_field_style = ACTIVE_FIELD_STYLE.fg(text_color);

        let block = self.get_styled_block(pane_is_active);
        match self {
            Pane::ImageInfo(pane_state @ ImageInfoPane { active_field, .. }) => {
                let lines = fields_into_lines(
                    pane_state.get_fields(),
                    field_key_style,
                    field_value_style,
                    |field_idx| {
                        // FIXME: this implicitly requires the variants of the enum and fields to be layed out in a particular order.
                        // I don't like this dependency, can I remove it somehow or make it explicit?
                        if *active_field as usize == field_idx && pane_is_active {
                            active_field_style
                        } else {
                            Style::default()
                        }
                    },
                );

                Ok(Paragraph::new(Text::from(lines)).block(block))
            }
            Pane::LayerSelector(pane_state) => {
                let lines = pane_state.lines(state.layers.iter(), field_value_style);

                Ok(Paragraph::new(Text::from(lines)).block(block))
            }
            Pane::LayerInfo(active_field) => {
                let (selected_layer_digest, selected_layer) = state
                    .get_selected_layer()
                    .context("failed to get the currently selected layer")?;

                let lines = fields_into_lines(
                    layer_info::get_fields(selected_layer_digest, selected_layer),
                    field_key_style,
                    field_value_style,
                    |field_idx| {
                        // FIXME: this implicitly requires the variants of the enum and fields to be layed out in a particular order.
                        // I don't like this dependency, can I remove it somehow or make it explicit?
                        if *active_field as usize == field_idx && pane_is_active {
                            active_field_style
                        } else {
                            Style::default()
                        }
                    },
                );

                // FIXME: add a scrollbar in case the terminal's width is too small to fit everything
                Ok(Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true }).block(block))
            }
            Pane::LayerInspector => Ok(Paragraph::new("Layer inspector").block(block)),
        }
    }

    /// Moves to the next entry in the specified [Direction] inside the [Pane].
    // FIXME: passing `layers` here is ugly. Can I do something about it?
    pub fn move_within_pane(
        &mut self,
        direction: Direction,
        layers: &IndexMap<Sha256Digest, Layer>,
    ) -> anyhow::Result<()> {
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
                    .context("unnable find the next layer")?;

                *selected_layer_digest = *digest;
                *selected_layer_idx = next_layer_idx;
            }
            Pane::LayerInfo(active_field) => active_field.toggle(direction),
            _ => {}
        };

        Ok(())
    }

    /// Copies the currently selected value within a [Pane] to the [Clipboard].
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
            // Pane::LayerInfo(active_field) => match active_field {
            //     LayerInfoActiveField::Digest => &encode_hex(selected_layer_digest),
            //     LayerInfoActiveField::Command => &selected_layer.created_by,
            //     LayerInfoActiveField::Comment if matches!(selected_layer.comment, Some(_)) => {
            //         selected_layer.comment.as_ref().unwrap()
            //     }
            //     _ => return,
            // },
            // FIXME: make copying work for layer info
            _ => return,
        };
        if let Err(e) = clipboard.set_text(text_to_copy) {
            tracing::debug!("Failed to copy text to the clipboard: {}", e);
        }
    }

    /// Returns a styled [Block] for the pane.
    fn get_styled_block(&self, is_active: bool) -> Block<'_> {
        let (border_type, border_style) = if is_active {
            (BorderType::Thick, Style::new().white())
        } else {
            (BorderType::Plain, Style::new().gray())
        };

        Block::bordered()
            .border_type(border_type)
            .border_style(border_style)
            .title(self.get_styled_title(is_active))
            .title_alignment(ratatui::layout::Alignment::Center)
    }

    /// Returns a styled [Title] for the pane.
    fn get_styled_title(&self, is_active: bool) -> impl Into<Title<'static>> {
        let title = match self {
            Pane::ImageInfo(..) => "Image Information",
            Pane::LayerSelector(..) => "Layers",
            Pane::LayerInfo(..) => "Layer Information",
            Pane::LayerInspector => "Layer Changes",
        };

        if is_active {
            title.bold().white()
        } else {
            title.not_bold().gray()
        }
    }
}
