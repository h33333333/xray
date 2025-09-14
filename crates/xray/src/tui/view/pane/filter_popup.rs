use std::borrow::Cow;
use std::path::Path;

use ratatui::layout::Constraint;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Padding, Paragraph, Wrap};
use regex::Regex;

use crate::parser::NodeFilters;
use crate::render_order_enum;
use crate::tui::action::Direction;
use crate::tui::util::Unit;

const POPUP_PADDING: Padding = Padding {
    left: 2,
    right: 2,
    top: 0,
    bottom: 0,
};

render_order_enum!(FilterInput, PathFilter, SizeFilter);
render_order_enum!(PathFilterKind, Regular, Regex);

/// A filter popup used for inputting filters for the layer inspector pane.
#[derive(Default, Debug)]
pub struct FilterPopup {
    /// Currently active filter input
    pub active_filter_input: FilterInput,
    /// Path-based filter supplied by the user
    pub path_filter: String,
    /// Kind of the path filter
    pub path_filter_kind: PathFilterKind,
    /// Node size filter supplied by the user
    pub node_size_filter: u64,
    /// Units used for the node size filter
    pub size_filter_units: Unit,
}

impl FilterPopup {
    /// Returns a widget that can be rendered inside the layer inspector pane and its vertical and horizontal size constraints.
    pub fn render_with_layout_constraints(
        &self,
    ) -> (Paragraph<'_>, Constraint, Constraint) {
        let block = Block::bordered()
            .border_type(BorderType::Thick)
            .padding(POPUP_PADDING)
            .title(Line::from(self.title()).centered())
            .title_bottom(Line::from(self.keybindings()).centered());

        let text = match self.active_filter_input {
            FilterInput::PathFilter => Text::from(self.path_filter.as_str()),
            FilterInput::SizeFilter => Text::from(format!(
                "{} {}",
                self.node_size_filter,
                self.size_filter_units.human_readable()
            )),
        };

        (
            Paragraph::new(text).wrap(Wrap { trim: false }).block(block),
            Constraint::Length(3),
            Constraint::Percentage(70),
        )
    }

    /// Appends a single character to the currently active filter.
    pub fn append_to_filter(&mut self, input: char) {
        match self.active_filter_input {
            FilterInput::PathFilter => self.path_filter.push(input),
            FilterInput::SizeFilter => {
                if let Some(digit) = input.to_digit(10) {
                    self.node_size_filter =
                        self.node_size_filter * 10 + digit as u64
                }
            }
        }
    }

    /// Removes a single character/symbol from the active filter.
    pub fn pop_from_filter(&mut self) {
        match self.active_filter_input {
            FilterInput::PathFilter => {
                self.path_filter.pop();
            }
            FilterInput::SizeFilter => self.node_size_filter /= 10,
        };
    }

    /// Changes settings of the currently active filter (doesn't change the filter itself).
    pub fn toggle_active_input(&mut self) {
        match self.active_filter_input {
            FilterInput::PathFilter => {
                self.path_filter_kind.toggle(Direction::Forward)
            }
            FilterInput::SizeFilter => {
                self.size_filter_units.toggle(Direction::Forward)
            }
        }
    }

    /// Returns a [NodeFilters] instance created using this popup's data.
    pub fn filters(&self) -> NodeFilters<'_, '_> {
        let mut filter = NodeFilters::default()
            .with_size_filter(self.size_filter_in_units());

        match self.path_filter_kind {
            PathFilterKind::Regular => {
                filter = filter.with_path_filter(Path::new(&self.path_filter))
            }
            PathFilterKind::Regex => {
                if let Some(regex) = self.path_regex() {
                    filter = filter.with_regex(regex);
                }
            }
        }

        filter
    }

    /// Resets this popup's state to the default (empty) state.
    pub fn reset(&mut self) {
        self.path_filter.clear();
        self.node_size_filter = 0;
    }

    /// Returns the currently inputted node size filter converted to bytes from the selected size units.
    fn size_filter_in_units(&self) -> u64 {
        self.size_filter_units.scale_to_units(self.node_size_filter)
    }

    /// Returns a [Regex] created from the inputted path filter or [Option::None] if any error was encountered.
    fn path_regex(&self) -> Option<Cow<'static, Regex>> {
        Regex::new(&self.path_filter).ok().map(Cow::Owned)
    }

    /// Returns title for the currently active filter.
    fn title(&self) -> &'static str {
        match self.active_filter_input {
            FilterInput::PathFilter => match self.path_filter_kind {
                PathFilterKind::Regular => "  Path Filter  ",
                PathFilterKind::Regex => "  Path Filter (RegEx)  ",
            },
            FilterInput::SizeFilter => "  Node Size Filter  ",
        }
    }

    /// Returns a [Vec] of keybindings to be rendered at the bottom of the popup.
    fn keybindings(&self) -> Vec<Span<'_>> {
        let mut keybindings = vec![
            // Padding
            Span::from("  "),
        ];

        let mut add_new_keybinding =
            |seq: &'static str, description: &'static str| {
                if keybindings.len() > 1 {
                    // Separate keybindings
                    keybindings
                        .push(Span::styled(", ", Style::new().fg(Color::Gray)));
                }

                // Add the key sequence
                keybindings.push(Span::styled(
                    seq,
                    Style::new().bold().fg(Color::White),
                ));
                // Separator
                keybindings
                    .push(Span::styled(" - ", Style::new().fg(Color::Gray)));
                // Add the description
                keybindings.push(Span::styled(
                    description,
                    Style::new().fg(Color::Gray),
                ));
            };

        add_new_keybinding("tab", "toggle filter");

        match self.active_filter_input {
            FilterInput::PathFilter => {
                add_new_keybinding("ctrl-l", "toggle filter kind")
            }
            FilterInput::SizeFilter => {
                add_new_keybinding("ctrl-l", "toggle size units")
            }
        }

        add_new_keybinding("enter", "apply");

        // Padding
        keybindings.push(Span::from("  "));

        keybindings
    }
}
