use std::borrow::Cow;
use std::path::Path;

use ratatui::layout::Constraint;
use ratatui::style::{Color, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, BorderType, Padding, Paragraph, Wrap};
use regex::Regex;

use crate::parser::TreeFilter;
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

render_order_enum!(PathFilterKind, Regular, Regexp);

#[derive(Default, Debug)]
pub struct FilterPopup {
    /// Currently active filter input
    pub active_input: FilterInput,
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
    pub fn render_with_layout_constraints(&self) -> (Paragraph<'_>, Constraint, Constraint) {
        let block = Block::bordered()
            .border_type(BorderType::Thick)
            .padding(POPUP_PADDING)
            .title(Line::from(self.title()).centered())
            .title_bottom(Line::from(self.keybindings()).centered().fg(Color::Gray));

        let text = match self.active_input {
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

    pub fn append_to_filter(&mut self, input: char) {
        match self.active_input {
            FilterInput::PathFilter => self.path_filter.push(input),
            FilterInput::SizeFilter => {
                if let Some(digit) = input.to_digit(10) {
                    self.node_size_filter = self.node_size_filter * 10 + digit as u64
                }
            }
        }
    }

    pub fn pop_from_filter(&mut self) {
        match self.active_input {
            FilterInput::PathFilter => {
                self.path_filter.pop();
            }
            FilterInput::SizeFilter => self.node_size_filter /= 10,
        };
    }

    pub fn toggle_active_input(&mut self) {
        match self.active_input {
            FilterInput::PathFilter => self.path_filter_kind.toggle(Direction::Forward),
            FilterInput::SizeFilter => self.size_filter_units.toggle(Direction::Forward),
        }
    }

    pub fn filters(&self) -> TreeFilter<'_, '_> {
        let mut filter = TreeFilter::default().with_size_filter(self.size_filter_in_units());

        if let Some(regex) = self.path_regex() {
            filter = filter.with_regex(regex);
        } else {
            filter = filter.with_path_filter(Path::new(&self.path_filter));
        }

        filter
    }

    fn size_filter_in_units(&self) -> u64 {
        self.size_filter_units.scale_to_units(self.node_size_filter)
    }

    fn path_regex(&self) -> Option<Cow<'static, Regex>> {
        Regex::new(&self.path_filter).ok().map(Cow::Owned)
    }

    fn title(&self) -> &'static str {
        match self.active_input {
            FilterInput::PathFilter => match self.path_filter_kind {
                PathFilterKind::Regular => "  Path Filter  ",
                PathFilterKind::Regexp => "  Path Filter (RegExp)  ",
            },
            FilterInput::SizeFilter => "  Node Size Filter  ",
        }
    }

    fn keybindings(&self) -> &'static str {
        // FIXME: this is ugly
        match self.active_input {
            FilterInput::PathFilter => "  [s-]tab - toggle filter, ctrl-l - toggle filter kind  ",
            FilterInput::SizeFilter => "  [s-]tab - toggle filter, ctrl-l - toggle size units  ",
        }
    }
}
