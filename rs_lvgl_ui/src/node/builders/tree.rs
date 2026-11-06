//! Tree composition, styling and grid placement.

use crate::node::{GridAlign, GridCell, Key, NativePart, Node, Selector, State, WidgetKind};
use crate::Style;

impl<M, K> Node<M, K> {
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

    /// Style the draggable thumb of a slider or switch.
    pub fn thumb_style(self, style: Style) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Thumb>,
    {
        self.thumb_state_style(State::DEFAULT, style)
    }

    /// Style a slider/switch thumb while the given state is active.
    pub fn thumb_state_style(self, state: State, style: Style) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Thumb>,
    {
        let supported = match self.kind {
            #[cfg(feature = "widget-slider")]
            WidgetKind::Slider { .. } => true,
            #[cfg(feature = "widget-switch")]
            WidgetKind::Switch { .. } => true,
            _ => false,
        };
        self.semantic_part_style(
            supported,
            NativePart::Knob,
            state,
            style,
            crate::Property::ThumbStyle,
        )
    }

    /// Style the filled progress region of a slider or progress bar.
    pub fn progress_style(self, style: Style) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Progress>,
    {
        self.progress_state_style(State::DEFAULT, style)
    }

    /// Style a slider/progress bar's filled region while the given state is active.
    pub fn progress_state_style(self, state: State, style: Style) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Progress>,
    {
        let supported = match self.kind {
            #[cfg(feature = "widget-slider")]
            WidgetKind::Slider { .. } => true,
            #[cfg(feature = "widget-bar")]
            WidgetKind::ProgressBar { .. } => true,
            _ => false,
        };
        self.semantic_part_style(
            supported,
            NativePart::Indicator,
            state,
            style,
            crate::Property::ProgressStyle,
        )
    }

    fn semantic_part_style(
        mut self,
        supported: bool,
        part: NativePart,
        state: State,
        style: Style,
        property: crate::Property,
    ) -> Self {
        if supported {
            self.part_styles.push((Selector::new(part, state), style));
        } else {
            self.record_invalid_property(property);
        }
        self
    }

    /// Apply a style to one widget part (for example a slider indicator).
    ///
    /// Part styles are applied after the main style and override it on that
    /// part. A later patch for the same part/state replaces the entire earlier
    /// patch, including fields it leaves unset. Replacing or removing a part style restores the class defaults for
    /// that part when the view updates.
    pub fn native_part_style(mut self, part: NativePart, style: Style) -> Self {
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
        self.native_part_state_style(NativePart::Main, state, style)
    }

    /// Apply a style to one widget part while `state` is active.
    pub fn native_part_state_style(mut self, part: NativePart, state: State, style: Style) -> Self {
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

    pub fn child(mut self, child: impl Into<crate::View<M>>) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Children>,
    {
        self.children.push(child.into());
        self
    }

    /// Append a child only when `condition` is true.
    ///
    /// The closure is evaluated once in the true branch and may borrow local
    /// state. It creates a descriptor, not a native widget. Stable sibling keys
    /// preserve native identity when optional children enter or leave the tree.
    pub fn child_if<C: Into<crate::View<M>>>(
        self,
        condition: bool,
        child: impl FnOnce() -> C,
    ) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Children>,
    {
        if condition {
            self.child(child())
        } else {
            self
        }
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Node<M>>) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Children>,
    {
        self.children.extend(children);
        self
    }
}
