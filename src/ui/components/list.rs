use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::ui::theme::{Theme, ARROW};

/// A selectable list component
pub struct SelectableList<T> {
    /// Items in the list
    pub items: Vec<T>,
    /// Currently selected index
    pub selected: usize,
    /// List state for ratatui
    state: ListState,
}

impl<T> SelectableList<T> {
    pub fn new(items: Vec<T>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            selected: 0,
            state,
        }
    }

    /// Move selection up
    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.items.len() - 1
        } else {
            self.selected - 1
        };
        self.state.select(Some(self.selected));
    }

    /// Move selection down
    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.items.len();
        self.state.select(Some(self.selected));
    }

    /// Get the currently selected item
    pub fn get_selected(&self) -> Option<&T> {
        self.items.get(self.selected)
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the number of items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Render the list with a custom item renderer
    pub fn render<F>(&mut self, frame: &mut Frame, area: Rect, title: &str, theme: &Theme, render_item: F)
    where
        F: Fn(&T, bool) -> Vec<Span<'static>>,
    {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = i == self.selected;
                let spans = render_item(item, is_selected);

                // Add selection arrow
                let mut content_spans = if is_selected {
                    vec![Span::styled(format!("{} ", ARROW), theme.selected())]
                } else {
                    vec![Span::raw("  ")]
                };
                content_spans.extend(spans);

                ListItem::new(Line::from(content_spans))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.border())
                    .title(title),
            )
            .highlight_style(theme.selected());

        frame.render_stateful_widget(list, area, &mut self.state);
    }
}

impl<T> Default for SelectableList<T> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
