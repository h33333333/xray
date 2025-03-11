mod image_info;
mod layer_info;
mod layer_inspector;
mod layer_selector;
mod style;
mod util;

use std::borrow::Cow;

use anyhow::Context;
use image_info::ImageInfoField;
pub use image_info::ImageInfoPane;
use layer_info::LayerInfoField;
pub use layer_info::LayerInfoPane;
pub use layer_inspector::LayerInspectorPane;
pub use layer_selector::LayerSelectorPane;
use ratatui::style::{Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::block::Title;
use ratatui::widgets::{Block, BorderType, Paragraph, Widget, Wrap};
use style::{text_color, ACTIVE_FIELD_STYLE, ACTIVE_INSPECTOR_NODE_STYLE};
pub(super) use style::{FIELD_KEY_STYLE, FIELD_VALUE_STYLE};
use util::fields_into_lines;

use crate::parser::LayerChangeSet;
use super::SideEffect;
use crate::tui::action::Direction;
use crate::tui::store::AppState;
use crate::tui::util::encode_hex;

/// All panes that exist in the app.
///
/// Each variant can also hold all the state that a particular pane needs, as
/// these variants are created once during the app initialization and are then reused.
pub enum Pane {
    /// Contains all image-related information from [crate::parser::Image].
    ImageInfo(ImageInfoPane),
    /// Displays infromation about the [LayerSelectorPane::selected_layer].
    LayerInfo(LayerInfoPane),
    /// Allows switching between [Layers](Layer) of the [crate::parser::Image].
    LayerSelector(LayerSelectorPane),
    LayerInspector(LayerInspectorPane),
}

impl Pane {
    /// Returns a [Widget] that can be used to render the current pane in the terminal.
    pub fn render<'a>(&'a self, state: &'a AppState, pane_rows: u16) -> anyhow::Result<impl Widget + 'a> {
        let pane_is_active = state.active_pane == self.into() && !state.show_help_popup;

        let text_color = text_color(pane_is_active);
        let field_key_style = FIELD_KEY_STYLE.fg(text_color);
        let field_value_style = FIELD_VALUE_STYLE.fg(text_color);
        let active_field_style = ACTIVE_FIELD_STYLE.fg(text_color);

        let block = self.get_styled_block(pane_is_active);
        match self {
            Pane::ImageInfo(pane_state) => {
                let lines = fields_into_lines(
                    pane_state.get_fields(),
                    field_key_style,
                    field_value_style,
                    |field_key| {
                        if pane_state.active_field == field_key && pane_is_active {
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
            Pane::LayerInfo(pane_state) => {
                let (selected_layer_digest, selected_layer) = state
                    .get_selected_layer()
                    .context("failed to get the currently selected layer")?;

                let lines = fields_into_lines(
                    LayerInfoPane::get_fields(selected_layer_digest, selected_layer),
                    field_key_style,
                    field_value_style,
                    |field_key| {
                        if pane_state.active_field == field_key && pane_is_active {
                            active_field_style
                        } else {
                            Style::default()
                        }
                    },
                );

                // FIXME: add a scrollbar in case the terminal's width is too small to fit everything
                Ok(Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true }).block(block))
            }
            Pane::LayerInspector(pane_state) => {
                let (layer_changeset, changeset_size) = state.get_selected_layers_changeset()?;

                // Two rows are taken by the block borders
                let remaining_rows = pane_rows - 2;
                let lines = pane_state
                    .changeset_to_lines(
                        layer_changeset,
                        changeset_size,
                        |node_is_active| {
                            if node_is_active && pane_is_active {
                                // Underlining doesn't look that good in the file tree, so just use a standard BoW outline
                                ACTIVE_INSPECTOR_NODE_STYLE
                            } else {
                                field_value_style
                            }
                        },
                        remaining_rows,
                    )
                    .context("layer inspector: failed to render a changeset")?;

                Ok(Paragraph::new(Text::from(lines)).block(block))
            }
        }
    }

    /// Moves to the next entry in the specified [Direction] inside the [Pane].
    pub fn move_within_pane(&mut self, direction: Direction, state: &AppState) -> anyhow::Result<Option<SideEffect>> {
        match self {
            Pane::ImageInfo(ImageInfoPane { active_field, .. }) => active_field.toggle(direction),
            Pane::LayerSelector(LayerSelectorPane {
                selected_layer_digest,
                selected_layer_idx,
                selected_layers_changeset,
            }) => {
                // FIXME: move this logic somewhere else
                let current_layer_idx = *selected_layer_idx;
                let next_layer_idx = match direction {
                    Direction::Forward => (current_layer_idx + 1) % state.layers.len(),
                    Direction::Backward => (current_layer_idx + state.layers.len() - 1) % state.layers.len(),
                };

                let (digest, _) = state
                    .layers
                    .get_index(next_layer_idx)
                    .context("unnable to find the next layer")?;

                let all_current_layers = state
                    .layers
                    .get_range(..next_layer_idx + 1)
                    .context("bug: the next layer idx points to an invalid index")?;

                let mut aggregated_layers = all_current_layers
                    .get_index(0)
                    .and_then(|(_, layer)| layer.changeset.clone())
                    .context("not a single layer in the all currently selected layers")?;

                for (_, layer) in all_current_layers.get_range(1..).into_iter().flatten() {
                    if let Some(changeset) = layer.changeset.as_ref() {
                        aggregated_layers = aggregated_layers.merge(changeset.clone())
                    }
                }

                let aggregated_layers_changeset_size = aggregated_layers.iter().count();
                *selected_layer_digest = *digest;
                *selected_layer_idx = next_layer_idx;
                *selected_layers_changeset = (aggregated_layers, aggregated_layers_changeset_size);

                // We need to reset the layer inspector pane, as its state is now invalid
                return Ok(Some(SideEffect::ResetLayerInspector));
            }
            Pane::LayerInfo(pane_state) => pane_state.active_field.toggle(direction),
            Pane::LayerInspector(pane_state) => pane_state.move_within_pane(direction, state)?,
        };

        Ok(None)
    }

    /// Returns the currently selected value within a [Pane].
    pub fn get_selected_field<'a>(&'a self, state: &'a AppState) -> Option<Cow<'a, str>> {
        match self {
            Pane::ImageInfo(ImageInfoPane {
                active_field,
                repository,
                tag,
                size,
                architecture,
                os,
            }) => Some(match active_field {
                ImageInfoField::Repository => repository.into(),
                ImageInfoField::Tag => tag.into(),
                // FIXME: this is kinda ugly, can I do better somehow?
                ImageInfoField::Size => format!("{}", size).into(),
                ImageInfoField::Architecture => architecture.into(),
                ImageInfoField::Os => os.into(),
            }),
            Pane::LayerInfo(LayerInfoPane { active_field }) => {
                let Ok((selected_layer_digest, selected_layer)) = state.get_selected_layer() else {
                    // Add a log here for debugging purposes in case this happens somehow
                    tracing::debug!("Failed to get the currently selected layer when getting the selected field from the LayerInfo pane");
                    return None;
                };

                Some(match active_field {
                    LayerInfoField::Digest => encode_hex(selected_layer_digest).into(),
                    LayerInfoField::Command => selected_layer.created_by.as_str().into(),
                    LayerInfoField::Comment => selected_layer.comment.as_ref()?.into(),
                })
            }
            _ => None,
        }
    }

    /// Interacts with the currently active element inside the [Pane].
    ///
    /// The actual action depends on the currently active [Pane] and its state.
    pub fn interact_within_pane(&mut self, state: &AppState) -> anyhow::Result<()> {
        // Only the inspector pane supports this action for now.
        if let Pane::LayerInspector(pane_state) = self {
            pane_state
                .toggle_active_node(state)
                .context("layer inspector: failed to toggle the active node")?;
        };

        Ok(())
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
            Pane::LayerInspector(..) => "Layer Changes",
        };

        if is_active {
            title.bold().white()
        } else {
            title.not_bold().gray()
        }
    }
}
