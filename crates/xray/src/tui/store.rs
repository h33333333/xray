use anyhow::Context;
use arboard::Clipboard;
use indexmap::IndexMap;

use super::action::AppAction;
use super::util::copy_to_clipboard;
use super::view::{ActivePane, ImageInfoPane, LayerInfoPane, LayerInspectorPane, LayerSelectorPane, Pane, SideEffect};
use crate::parser::{Image, Layer, LayerChangeSet, Sha256Digest};

/// A Flux store that can handle a [Store::Action].
pub trait Store {
    /// A Flux action that this store supports and can handle.
    type Action;

    /// Handles the [Store::Action].
    fn handle(&mut self, action: Self::Action) -> anyhow::Result<()>;
}

pub struct AppState {
    /// All the [Panes](Pane) sorted by their render order.
    ///
    /// Check docs of [ActivePane] to understand how panes are ordered.
    pub panes: [Option<Pane>; 4],
    /// The currently selected pane.
    pub active_pane: ActivePane,
    /// A [Clipboard] that is used for handling of [AppAction::Copy].
    ///
    /// Can be missing if there was an error while creating it.
    pub clipboard: Option<Clipboard>,
    /// All layers in the currently viewed image.
    pub layers: IndexMap<Sha256Digest, Layer>,
    /// Whether the help popup is currently shown in the UI.
    pub show_help_popup: bool,
    /// Whether the UI is currently in the "insert" mode (i.e. allows free text input).
    pub is_in_insert_mode: bool,
}

impl AppState {
    /// Creates a new instance of the [AppState] using data from the provided [Image].
    pub fn new(image: Image) -> anyhow::Result<Self> {
        // FIXME: move the pane instantiation somewhere else
        let image_info_pane = Pane::ImageInfo(ImageInfoPane::new(
            image.repository,
            image.tag,
            image.size,
            image.architecture,
            image.os,
        ));

        let (digest, layer) = image.layers.get_index(0).context("got an image with 0 layers")?;
        let layer_selector_pane = Pane::LayerSelector(LayerSelectorPane::new(
            *digest,
            0,
            layer.changeset.clone().unwrap_or(LayerChangeSet::new(*digest)),
        ));
        let layer_info_pane = Pane::LayerInfo(LayerInfoPane::default());
        let layer_inspector_pane = Pane::LayerInspector(LayerInspectorPane::default());

        let clipboard = Clipboard::new().ok();

        let mut panes = [
            Some(image_info_pane),
            Some(layer_info_pane),
            Some(layer_selector_pane),
            Some(layer_inspector_pane),
        ];

        // Ensure that panes are always sorted by the render order, determined
        // by the order of enum's variants declaration.
        panes.sort_by_key(|a| Into::<usize>::into(Into::<ActivePane>::into(a.as_ref().unwrap())));

        Ok(AppState {
            panes,
            active_pane: ActivePane::default(),
            clipboard,
            layers: image.layers,
            show_help_popup: false,
            is_in_insert_mode: false,
        })
    }

    /// Returns a reference to the currently selected [Layer] and its [Sha256Digest].
    pub fn get_selected_layer(&self) -> anyhow::Result<(&Sha256Digest, &Layer)> {
        let layer_selector_pane_idx: usize = ActivePane::LayerSelector.into();
        let layer_selector_pane = &self.panes[layer_selector_pane_idx];
        let selected_layer_idx = if let Some(Pane::LayerSelector(pane)) = layer_selector_pane {
            pane.selected_layer().1
        } else {
            anyhow::bail!("layer selector pane is no longer at the expected position in the UI");
        };
        self.layers
            .get_index(selected_layer_idx)
            .context("selected layer has an invalid index")
    }
    /// Returns a reference to the aggregated [LayerChangeSet] of the currently selected layer and its parents.
    pub fn get_aggregated_layers_changeset(&self) -> anyhow::Result<(&LayerChangeSet, usize)> {
        let layer_selector_pane_idx: usize = ActivePane::LayerSelector.into();
        let layer_selector_pane = &self.panes[layer_selector_pane_idx];
        if let Some(Pane::LayerSelector(pane)) = layer_selector_pane {
            Ok(pane.selected_layers_changeset())
        } else {
            anyhow::bail!("layer selector pane is no longer at the expected position in the UI");
        }
    }

    fn get_active_pane(&self) -> anyhow::Result<&Pane> {
        self.panes
            .get(Into::<usize>::into(self.active_pane))
            .and_then(|pane| pane.as_ref())
            .with_context(|| format!("bug: pane {:?} is no longer at its expected place", self.active_pane))
    }

    fn get_active_pane_mut(&mut self) -> anyhow::Result<&mut Pane> {
        self.panes
            .get_mut(Into::<usize>::into(self.active_pane))
            .and_then(|pane| pane.as_mut())
            .with_context(|| format!("bug: pane {:?} is no longer at its expected place", self.active_pane))
    }

    fn on_changeset_updated(&mut self) -> anyhow::Result<()> {
        let layer_inspector_pane_idx: usize = ActivePane::LayerInspector.into();
        let mut layer_inspector_pane = self.panes[layer_inspector_pane_idx].take();

        if let Some(Pane::LayerInspector(pane)) = layer_inspector_pane.as_mut() {
            // Reset state
            pane.reset();

            let (changeset, _) = self.get_aggregated_layers_changeset()?;
            // Filter the new changeset if filters are present
            pane.filter_current_changeset(changeset);
        } else {
            anyhow::bail!("layer inspector pane is no longer at the expected position in the UI");
        }

        // Return the pane back
        self.panes[layer_inspector_pane_idx].replace(layer_inspector_pane.expect("unreacheable"));

        Ok(())
    }

    fn apply_side_effect(&mut self, side_effect: SideEffect) -> anyhow::Result<()> {
        match side_effect {
            // Both side effects lead to the same actions and are handled by the same handler
            SideEffect::ChangesetUpdated | SideEffect::FiltersUpdated => self.on_changeset_updated()?,
        }
        Ok(())
    }
}

impl Store for AppState {
    type Action = AppAction;

    fn handle(&mut self, action: Self::Action) -> anyhow::Result<()> {
        match action {
            AppAction::Empty => tracing::trace!("Received an empty event"),
            AppAction::TogglePane(direction) if !self.show_help_popup => self.active_pane.toggle(direction),
            action @ (AppAction::Interact | AppAction::Move(..)) if !self.show_help_popup => {
                let active_pane_idx = Into::<usize>::into(self.active_pane);
                // HACK: take the pane here in order to be able to provide a reference to the state when handling the action.
                let mut active_pane = self.panes[active_pane_idx]
                    .take()
                    .with_context(|| format!("bug: forgot to return the {} pane?", active_pane_idx))?;
                let side_effect: Option<SideEffect> = match action {
                    AppAction::Interact => {
                        active_pane
                            .interact_within_pane(self)
                            .context("error while handling the 'interact' action")?;
                        None
                    }
                    AppAction::Move(direction) => active_pane
                        .move_within_pane(direction, self)
                        .context("error while handling the 'move' action")?,
                    _ => unreachable!("Checked above"),
                };
                // Return the pane back
                self.panes[active_pane_idx].replace(active_pane);

                // Apply a side effect if any
                if let Some(side_effect) = side_effect {
                    self.apply_side_effect(side_effect)
                        .context("error while applying a side effect")?
                };
            }
            AppAction::Copy if !self.show_help_popup => {
                if self.clipboard.is_none() {
                    tracing::trace!("Can't copy: no clipboard is available");
                    return Ok(());
                }

                // HACK: take the clipboard here to avoid fighting the borrow checker in the next block
                let mut clipboard = self.clipboard.take();
                if let Some(text_to_copy) = self.get_active_pane()?.get_selected_field(self) {
                    copy_to_clipboard(clipboard.as_mut(), text_to_copy);
                }
                // Return the clipboard where it belongs
                self.clipboard = clipboard;
            }
            AppAction::ToggleHelpPane => {
                self.show_help_popup = !self.show_help_popup;
            }
            AppAction::SelectPane(index) if !self.show_help_popup => self
                .active_pane
                .select(index)
                .context("failed to select a pane by index")?,
            AppAction::ToggleInputMode => {
                let (is_in_insert_mode, side_effect) = self.get_active_pane_mut()?.toggle_input_mode();
                if let Some(side_effect) = side_effect {
                    self.apply_side_effect(side_effect)?;
                }
                self.is_in_insert_mode = is_in_insert_mode;
            }
            AppAction::InputCharacter(input) => {
                self.get_active_pane_mut()?.on_input_character(input);
            }
            AppAction::InputDeleteCharacter => {
                self.get_active_pane_mut()?.on_backspace();
            }
            // Do nothing in cases when the help popup is active and the user tries to do something besides closing the popup.
            _ => {}
        };

        Ok(())
    }
}
