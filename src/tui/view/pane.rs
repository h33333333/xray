use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, Widget};

use crate::tui::store::AppState;

#[derive(Default, PartialEq, Eq)]
pub enum Pane {
    #[default]
    ImageInfo,
    LayerSelector,
    LayerInspector,
}

impl Pane {
    pub fn toggle(&mut self) {
        let next_pane = match self {
            Pane::ImageInfo => Pane::LayerSelector,
            Pane::LayerSelector => Pane::LayerInspector,
            Pane::LayerInspector => Pane::ImageInfo,
        };

        *self = next_pane;
    }

    pub fn render(&self, state: &AppState) -> impl Widget {
        let border_style = if state.active_pane == *self {
            Style::new().red()
        } else {
            Style::new().white()
        };

        Block::bordered().border_style(border_style).title(self.title())
    }

    fn title(&self) -> &'static str {
        match self {
            Pane::ImageInfo => "Image information",
            Pane::LayerSelector => "Layers",
            Pane::LayerInspector => "Layer changes",
        }
    }
}
