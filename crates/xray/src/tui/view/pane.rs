mod filter_popup;
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
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::block::Title;
use ratatui::widgets::{Block, BorderType, Paragraph, Widget, Wrap};
use style::{text_color, ACTIVE_FIELD_STYLE, ACTIVE_INSPECTOR_NODE_STYLE};
pub(super) use style::{
    ADDED_INSPECTOR_NODE_STYLE, DELETED_INSPECTOR_NODE_STYLE, FIELD_KEY_STYLE, FIELD_VALUE_STYLE,
    MODIFIED_INSPECTOR_NODE_STYLE,
};
use util::fields_into_lines;

use super::widgets::PaneWithPopup;
use super::{ActivePane, SideEffect};
use crate::parser::{Image, LayerChangeSet};
use crate::tui::action::Direction;
use crate::tui::store::AppState;
use crate::tui::util::encode_hex;

/// All panes that exist in the app.
///
/// Each variant also holds all the state that a particular pane needs, as
/// these variants are created once during the app initialization and are then reused.
pub enum Pane {
    /// Contains all image-related information from [crate::parser::Image].
    ImageInfo(ImageInfoPane),
    /// Displays infromation about the [LayerSelectorPane::selected_layer].
    LayerInfo(LayerInfoPane),
    /// Allows switching between [Layers](crate::parser::Layer) of the [crate::parser::Image].
    LayerSelector(LayerSelectorPane),
    /// Displays the aggregated changeset from the currently selected [Layers](crate::parser::Layer).
    LayerInspector(LayerInspectorPane),
}

impl Pane {
    /// Returns a [Widget] that can be used to render the current pane in the terminal.
    pub fn render<'a>(
        &'a self,
        state: &'a AppState,
        pane_rows: u16,
        pane_cols: u16,
    ) -> anyhow::Result<impl Widget + 'a> {
        let pane_is_active = state.active_pane == self.into() && !state.show_help_popup;

        let text_color = text_color(pane_is_active);
        // Don't override the field key's fg color unless the pane is not active.
        let field_key_style = if pane_is_active && FIELD_KEY_STYLE.fg.is_some() {
            FIELD_KEY_STYLE
        } else {
            FIELD_KEY_STYLE.fg(text_color)
        };
        let field_value_style = FIELD_VALUE_STYLE.fg(text_color);
        let active_field_style = ACTIVE_FIELD_STYLE.fg(text_color);

        // Two rows are taken by the block borders
        let remaining_rows = pane_rows - 2;
        // Two cols are taken by the block borders
        let remaining_cols = pane_cols - 2;

        let mut widget = PaneWithPopup::<Paragraph, Paragraph>::new(None, None);

        let block = self.get_styled_block(pane_is_active);
        let pane_widget = match self {
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

                Paragraph::new(Text::from(lines)).block(block)
            }
            Pane::LayerSelector(pane_state) => {
                let lines = pane_state.lines(state.layers.iter(), field_value_style, remaining_rows, remaining_cols);

                Paragraph::new(Text::from(lines)).block(block)
            }
            Pane::LayerInfo(pane_state) => {
                let (selected_layer_digest, selected_layer, _) = state
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

                // FIXME: add a vertical scroll
                // - this requires doing wrap manually, as there is no way to know how many lines will the wrapped text take
                Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true }).block(block)
            }
            Pane::LayerInspector(pane_state) => {
                let (layer_changeset, _) = state.get_aggregated_layers_changeset()?;
                let (_, _, current_layer_idx) = state.get_selected_layer()?;

                let lines = pane_state
                    .changeset_to_lines(
                        layer_changeset,
                        |node_is_active, node_updated_in, node_is_deleted, node_is_modified| {
                            if !pane_is_active {
                                return field_value_style;
                            }
                            if node_is_active {
                                ACTIVE_INSPECTOR_NODE_STYLE
                            } else if node_updated_in == current_layer_idx as u8 && current_layer_idx != 0 {
                                if node_is_deleted {
                                    DELETED_INSPECTOR_NODE_STYLE
                                } else if node_is_modified {
                                    MODIFIED_INSPECTOR_NODE_STYLE
                                } else {
                                    ADDED_INSPECTOR_NODE_STYLE
                                }
                            } else {
                                field_value_style
                            }
                        },
                        remaining_rows,
                    )
                    .context("layer inspector: failed to render a changeset")?;

                if let Some(filter_popup) = pane_state.filter_popup() {
                    let (popup, v_constraint, h_constraint) = filter_popup.render_with_layout_constraints();
                    widget.set_popup((popup, Some(v_constraint), Some(h_constraint)));
                }

                // FIXME: add a horizontal scroll
                Paragraph::new(Text::from(lines)).block(block)
            }
        };

        widget.set_pane(pane_widget);
        Ok(widget)
    }

    /// Moves (vertically) to the next entry in the specified [Direction] inside the [Pane].
    pub fn move_within_pane(&mut self, direction: Direction, state: &AppState) -> anyhow::Result<Option<SideEffect>> {
        match self {
            Pane::ImageInfo(pane_state) => {
                pane_state.toggle_active_field(direction);
                Ok(None)
            }
            Pane::LayerSelector(pane_state) => pane_state.move_within_pane(direction, state),
            Pane::LayerInfo(pane_state) => {
                pane_state.toggle_active_field(direction);
                Ok(None)
            }
            Pane::LayerInspector(pane_state) => pane_state.move_within_pane(direction, state).map(|_| None),
        }
    }

    /// Returns the currently selected value within a [Pane].
    pub fn get_selected_field<'a>(&'a self, state: &'a AppState) -> Option<Cow<'a, str>> {
        match self {
            Pane::ImageInfo(ImageInfoPane {
                active_field,
                image_name,
                tag,
                size,
                architecture,
                os,
            }) => Some(match active_field {
                ImageInfoField::Repository => image_name.as_ref().into(),
                ImageInfoField::Tag => tag.as_ref().into(),
                ImageInfoField::Size => size.as_ref().into(),
                ImageInfoField::Architecture => architecture.into(),
                ImageInfoField::Os => os.into(),
            }),
            Pane::LayerInfo(LayerInfoPane { active_field }) => {
                let Ok((selected_layer_digest, selected_layer, _)) = state.get_selected_layer() else {
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
        if let Pane::LayerInspector(pane_state) = self {
            // Only the inspector pane supports this action for now.
            pane_state
                .toggle_active_node(state)
                .context("layer inspector: failed to toggle the active node")?;
        }

        Ok(())
    }

    /// Toggles the input mode and allows the [Pane] to accept user's input without processing any special keys.
    ///
    /// What exactly user inputs depends on the [Pane] itself.
    pub fn toggle_input_mode(&mut self) -> (bool, Option<SideEffect>) {
        // Only the inspector pane supports this action for now.
        if let Pane::LayerInspector(pane_state) = self {
            let input_is_active = pane_state.toggle_filter_popup();
            // Apply the filter only when user exits the input screen to avoid using a lot of resources for nothing
            let side_effect = (!input_is_active).then_some(SideEffect::FiltersUpdated);
            return (input_is_active, side_effect);
        };

        (false, None)
    }

    /// Handles user's input when in "insert" mode.
    ///
    /// How user's input is handled depends on the [Pane] itself.
    ///
    /// # Safety
    ///
    /// Should be called only in insert mode.
    pub fn on_input_character(&mut self, input: char) {
        // Only the inspector pane supports this action for now.
        if let Pane::LayerInspector(pane_state) = self {
            pane_state.append_to_filter(input);
        };
    }

    /// Handles a backspace when in "insert" mode.
    ///
    /// How it's handled depends on the [Pane] itself.
    ///
    /// # Safety
    ///
    /// Should be called only in insert mode.
    pub fn on_backspace(&mut self) {
        // Only the inspector pane supports this action for now.
        if let Pane::LayerInspector(pane_state) = self {
            pane_state.pop_from_filter();
        };
    }

    /// Scroll horizontally within the [Pane].
    ///
    /// Vertical scroll is done in [Self::move_within_pane].
    pub fn scroll_within_pane(&mut self, direction: Direction, state: &AppState) -> anyhow::Result<()> {
        let pane_area = state
            .panes
            .get(Into::<usize>::into(Into::<ActivePane>::into(&*self)))
            .map(|(_, rect)| (rect.width, rect.height))
            .context("bug: ghost pane?")?;

        // Only the inspector pane supports this action for now.
        if let Pane::LayerSelector(pane_state) = self {
            pane_state
                .scroll(direction, pane_area, state)
                .context("error while scrolling the layer selector pane")?
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
            .title(Line::from(Into::<ActivePane>::into(self).to_formatted_index()))
    }

    /// Returns a styled [Title] for the pane.
    fn get_styled_title(&self, is_active: bool) -> impl Into<Title<'static>> {
        let title = match self {
            Pane::ImageInfo(..) => "Image Information",
            Pane::LayerSelector(..) => "Layers",
            Pane::LayerInfo(..) => "Layer Information",
            Pane::LayerInspector(..) => "Layer Changes",
        };

        let title = if is_active {
            title.bold().white()
        } else {
            title.not_bold().gray()
        };

        title.into_centered_line()
    }
}

/// Initializes all panes from the provided [Image].
pub fn init_panes(image: &mut Image) -> anyhow::Result<[(Option<Pane>, Rect); 4]> {
    let image_info_pane = Pane::ImageInfo(ImageInfoPane::new(
        std::mem::take(&mut image.image_name),
        std::mem::take(&mut image.tag),
        std::mem::take(&mut image.size),
        std::mem::take(&mut image.architecture),
        std::mem::take(&mut image.os),
    ));

    let (_, layer) = image.layers.get_index(0).context("got an image with 0 layers")?;
    let layer_selector_pane = Pane::LayerSelector(LayerSelectorPane::new(
        0,
        layer.changeset.clone().unwrap_or(LayerChangeSet::new(0)),
    ));
    let layer_info_pane = Pane::LayerInfo(LayerInfoPane::default());
    let layer_inspector_pane = Pane::LayerInspector(LayerInspectorPane::default());

    // Note that we assign zeroed rects here. This means that we won't be able to render anything before dispatching at least one
    // [AppAction::Empty] event with the correct terminal size.
    let mut panes = [
        (Some(image_info_pane), Rect::ZERO),
        (Some(layer_info_pane), Rect::ZERO),
        (Some(layer_selector_pane), Rect::ZERO),
        (Some(layer_inspector_pane), Rect::ZERO),
    ];

    // Ensure that panes are always sorted by the render order, determined
    // by the order of enum's variants declaration.
    panes.sort_by_key(|(a, _)| Into::<usize>::into(Into::<ActivePane>::into(a.as_ref().unwrap())));

    Ok(panes)
}
