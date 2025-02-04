use anyhow::Context;
use arboard::Clipboard;
use indexmap::IndexMap;

use super::action::AppAction;
use super::view::{ActivePane, ImageInfoPane, LayerSelectorPane, Pane};
use crate::parser::{Image, Layer, Sha256Digest};

/// A Flux store that can handle a [Store::Action].
pub trait Store {
    /// A Flux action that this store supports and can handle.
    type Action;

    /// Handles the [Store::Action].
    fn handle(&mut self, action: Self::Action) -> anyhow::Result<()>;
}

pub struct AppState {
    /// By default, panes are placed as follows:
    ///     1. Upper left pane - image information pane.
    ///     2. Bottom left pane - layer selection pane.
    ///     3. Right pane - layer diff pane.
    pub panes: [Pane; 3],
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
        let image_info_pane = Pane::ImageInfo(ImageInfoPane::new(
            image.repository,
            image.tag,
            image.size,
            image.architecture,
            image.os,
        ));

        let (digest, _) = image.layers.get_index(0).context("got an image with 0 layers")?;
        let layer_selector_pane = Pane::LayerSelector(LayerSelectorPane::new(*digest, 0));

        let clipboard = Clipboard::new().ok();
        Ok(AppState {
            panes: [image_info_pane, layer_selector_pane, Pane::LayerInspector],
            active_pane: ActivePane::default(),
            clipboard,
            layers: image.layers,
        })
    }
}

impl Store for AppState {
    type Action = AppAction;

    fn handle(&mut self, action: Self::Action) -> anyhow::Result<()> {
        match action {
            AppAction::Empty => tracing::trace!("Received an empty event"),
            AppAction::TogglePane(direction) => self.active_pane.toggle(direction),
            AppAction::Move(direction) => {
                self.panes[self.active_pane.pane_idx()].move_within_pane(direction, &self.layers)
            }
            AppAction::Copy => {
                if self.clipboard.is_some() {
                    self.panes[self.active_pane.pane_idx()].copy(self.clipboard.as_mut().expect("checked before"));
                } else {
                    tracing::trace!("Can't copy: no clipboard is available");
                }
            }
        };

        Ok(())
    }
}
