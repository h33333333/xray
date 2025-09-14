use ratatui::layout::Constraint;
use ratatui::widgets::{Clear, Widget};

use crate::tui::view::popup_area;

/// A simple widget that allows rendering two widgets within the same area using a single method call to [Widget::render].
pub struct PaneWithPopup<W, P> {
    /// An optional pane that should be rendered.
    pane: Option<W>,
    /// An optional popup along with its vertical and horizontal area [Constraint].
    popup: Option<(P, Option<Constraint>, Option<Constraint>)>,
}

impl<W, P> PaneWithPopup<W, P> {
    pub fn new(
        pane: Option<W>,
        popup: Option<(P, Option<Constraint>, Option<Constraint>)>,
    ) -> Self {
        PaneWithPopup { pane, popup }
    }

    /// Sets a pane that should be rendered on [Widget::Render].
    pub fn set_pane(&mut self, pane: W) {
        self.pane = Some(pane);
    }

    /// Sets a popup that should be rendered on [Widget::Render].
    pub fn set_popup(
        &mut self,
        popup: (P, Option<Constraint>, Option<Constraint>),
    ) {
        self.popup = Some(popup);
    }
}

impl<W: Widget, P: Widget> Widget for PaneWithPopup<W, P> {
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        if let Some(pane) = self.pane {
            pane.render(area, buf);
        }
        if let Some((popup, vertical_constraint, horizontal_constraint)) =
            self.popup
        {
            let area =
                popup_area(area, vertical_constraint, horizontal_constraint);
            // Clear the area
            Clear.render(area, buf);
            popup.render(area, buf);
        }
    }
}
