use arboard::Clipboard;

use super::action::AppAction;
use super::view::{ActivePane, ImageInfoPane, Pane};
use crate::parser::Image;

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
}

impl AppState {
    pub fn new(image: Image) -> Self {
        let image_info_pane = Pane::ImageInfo(ImageInfoPane::new(
            image.repository,
            image.tag,
            image.size,
            image.architecture,
            image.os,
        ));

        let clipboard = Clipboard::new().ok();
        AppState {
            panes: [image_info_pane, Pane::LayerSelector, Pane::LayerInspector],
            active_pane: ActivePane::default(),
            clipboard,
        }
    }
}

impl Store for AppState {
    type Action = AppAction;

    fn handle(&mut self, action: Self::Action) -> anyhow::Result<()> {
        match action {
            AppAction::Empty => tracing::trace!("Received an empty event"),
            AppAction::TogglePane(direction) => self.active_pane.toggle(direction),
            AppAction::Move(direction) => self.panes[self.active_pane.pane_idx()].move_within_pane(direction),
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
