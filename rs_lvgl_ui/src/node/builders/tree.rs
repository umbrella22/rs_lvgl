//! Tree composition, styling and grid placement.

use crate::node::{GridAlign, GridCell, Key, Node, Part, Selector, State};
use crate::Style;

impl<M> Node<M> {
    /// Retain a loaded font for this node's main style and inherited children.
    /// This overrides a static main-style font; removing it restores that style.
    pub fn font_resource(mut self, font: crate::FontResource) -> Self {
        self.font_resource = Some(font);
        self
    }

    pub fn key(mut self, key: Key) -> Self {
        self.key = key;
        self
    }

    /// Merge the explicitly set fields over the node's current style.
    ///
    /// This composes with flat modifiers in call order: the last explicit
    /// write to each field wins, including zero and false. Unset fields retain
    /// earlier writes, the kind base or native inheritance.
    pub fn style(mut self, style: Style) -> Self {
        self.style = self.style.merged(style);
        self
    }

    /// Apply a style to one widget part (for example a slider indicator).
    ///
    /// Part styles are applied after the main style and override it on that
    /// part. Replacing or removing a part style restores the class defaults for
    /// that part when the view updates.
    pub fn part_style(mut self, part: Part, style: Style) -> Self {
        self.part_styles
            .push((Selector::new(part, State::DEFAULT), style));
        self
    }

    /// Apply a style to the main part while `state` is active (for example
    /// a pressed or checked list item).
    ///
    /// Object-wide properties (`hidden`, `disabled`, `gap`, `floating`) are
    /// ignored in state styles. Replacing or removing a state style restores
    /// the class defaults for that state when the view updates.
    pub fn state_style(self, state: State, style: Style) -> Self {
        self.part_state_style(Part::Main, state, style)
    }

    /// Apply a style to one widget part while `state` is active.
    pub fn part_state_style(mut self, part: Part, state: State, style: Style) -> Self {
        self.part_styles.push((Selector::new(part, state), style));
        self
    }

    /// Place this node in a grid parent.
    pub fn grid_cell(mut self, column: u16, row: u16, column_span: u16, row_span: u16) -> Self {
        self.grid_cell = Some(GridCell::new(column, row, column_span, row_span));
        self
    }

    /// Override the alignment inside this node's grid cell.
    pub fn grid_align(mut self, column: GridAlign, row: GridAlign) -> Self {
        if let Some(cell) = &mut self.grid_cell {
            cell.column_align = column;
            cell.row_align = row;
        }
        self
    }

    pub fn child(mut self, child: Node<M>) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Node<M>>) -> Self {
        self.children.extend(children);
        self
    }
}
