use anyhow::Context;
use arboard::Clipboard;
use indexmap::IndexMap;

use super::action::AppAction;
use super::util::copy_to_clipboard;
use super::view::{ActivePane, ImageInfoPane, LayerInfoPane, LayerSelectorPane, Pane};
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
    pub panes: [Pane; 4],
    /// The currently selected pane.
    pub active_pane: ActivePane,
    /// A [Clipboard] that is used for handling of [AppAction::Copy].
    ///
    /// Can be missing if there was an error while creating it.
    pub clipboard: Option<Clipboard>,
    /// All layers in the currently viewed image.
    pub layers: IndexMap<Sha256Digest, Layer>,
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
            layer.changeset.clone().unwrap_or(LayerChangeSet::new_empty_dir()),
        ));
        let layer_info_pane = Pane::LayerInfo(LayerInfoPane::default());

        let clipboard = Clipboard::new().ok();

        let mut panes = [
            image_info_pane,
            layer_info_pane,
            layer_selector_pane,
            Pane::LayerInspector,
        ];

        // Ensure that panes are always sorted by the render order, determined
        // by the order of enum's variants declaration.
        panes.sort_by_key(|a| Into::<usize>::into(Into::<ActivePane>::into(a)));

        Ok(AppState {
            panes,
            active_pane: ActivePane::default(),
            clipboard,
            layers: image.layers,
        })
    }

    /// Returns a reference to the currently selected [Layer] and its [Sha256Digest].
    pub fn get_selected_layer(&self) -> anyhow::Result<(&Sha256Digest, &Layer)> {
        let layer_selector_pane_idx: usize = ActivePane::LayerSelector.into();
        let layer_selector_pane = &self.panes[layer_selector_pane_idx];
        let selected_layer_idx = if let Pane::LayerSelector(pane) = layer_selector_pane {
            pane.selected_layer().1
        } else {
            anyhow::bail!("layer selector pane is no longer at the expected position in the UI");
        };
        self.layers
            .get_index(selected_layer_idx)
            .context("selected layer has an invalid index")
    }

    /// Returns a reference to the [LayerChangeSet] of the currently selected layers.
    pub fn get_selected_layers_changeset(&self) -> anyhow::Result<&LayerChangeSet> {
        let layer_selector_pane_idx: usize = ActivePane::LayerSelector.into();
        let layer_selector_pane = &self.panes[layer_selector_pane_idx];
        if let Pane::LayerSelector(pane) = layer_selector_pane {
            Ok(pane.selected_layers_changeset())
        } else {
            anyhow::bail!("layer selector pane is no longer at the expected position in the UI");
        }
    }
}

impl Store for AppState {
    type Action = AppAction;

    fn handle(&mut self, action: Self::Action) -> anyhow::Result<()> {
        match action {
            AppAction::Empty => tracing::trace!("Received an empty event"),
            AppAction::TogglePane(direction) => self.active_pane.toggle(direction),
            AppAction::Move(direction) => self.panes[Into::<usize>::into(self.active_pane)]
                .move_within_pane(direction, &self.layers)
                .context("error while handling the 'move' action")?,
            AppAction::Copy => {
                if self.clipboard.is_none() {
                    tracing::trace!("Can't copy: no clipboard is available");
                    return Ok(());
                }

                // HACK: take the clipboard here to avoid fighting the borrow checker in the next block
                let mut clipboard = self.clipboard.take();
                if let Some(text_to_copy) = self.panes[Into::<usize>::into(self.active_pane)].get_selected_field(self) {
                    copy_to_clipboard(clipboard.as_mut(), text_to_copy);
                }
                // Return the clipboard where it belongs
                self.clipboard = clipboard;
            }
        };

        Ok(())
    }
}
