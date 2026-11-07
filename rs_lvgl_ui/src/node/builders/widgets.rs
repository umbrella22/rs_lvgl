//! Widget-specific modifiers and invalid-property recording.

use crate::node::{CalendarDate, ImageSource, Node, RollerMode, WidgetKind};
use alloc::string::String;

impl<M, K> Node<M, K> {
    /// Restrict inserted Unicode scalar values. Omission allows all characters.
    /// An explicit empty set or embedded NUL rejects before native mutation.
    /// Authoritative model text is preserved even outside this input filter.
    #[cfg(feature = "widget-textarea")]
    pub fn accepted_chars(mut self, chars: impl Into<String>) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Text>,
    {
        self.edit_text_options(|options| options.accepted = Some(chars.into()));
        self
    }

    /// Maximum input length in Unicode scalar values; `None` removes the limit.
    /// This limits editing, without truncating the application's model text.
    #[cfg(feature = "widget-textarea")]
    pub fn input_max_length(mut self, maximum: Option<core::num::NonZeroU32>) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Text>,
    {
        self.edit_text_options(|options| options.maximum = maximum);
        self
    }

    /// Explicit character cursor, counting Unicode scalars (not bytes/graphemes).
    /// The position must fit native i32 and lie in `0..=model.chars().count()`.
    /// `None` leaves native placement to editing and model text assignment.
    #[cfg(feature = "widget-textarea")]
    pub fn cursor_position(mut self, position: Option<u32>) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Text>,
    {
        self.edit_text_options(|options| options.cursor = position);
        self
    }

    /// Allow pointer clicks to place the cursor. Omission restores `true`.
    #[cfg(feature = "widget-textarea")]
    pub fn cursor_click_position(mut self, enabled: bool) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Text>,
    {
        self.edit_text_options(|options| options.click = enabled);
        self
    }

    #[cfg(feature = "widget-textarea")]
    fn edit_text_options(&mut self, edit: impl FnOnce(&mut crate::TextareaInputOptions)) {
        if let WidgetKind::Textarea { input, .. } = &mut self.kind {
            edit(input.get_or_insert_with(Default::default));
        } else {
            self.record_invalid_property(crate::Property::Text);
        }
    }

    /// Wrap row/column children onto additional tracks. Omission restores false.
    /// This does not change track alignment or child identity.
    pub fn flex_wrap(mut self, enabled: bool) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Flex>,
    {
        if matches!(self.kind, WidgetKind::Row | WidgetKind::Column) {
            self.flex_flow.0 = enabled;
        } else {
            self.record_invalid_property(crate::Property::Style);
        }
        self
    }

    /// Reverse native Flex child traversal. With wrapping, this also changes
    /// which children enter each track. Omission restores false.
    pub fn flex_reverse(mut self, enabled: bool) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Flex>,
    {
        if matches!(self.kind, WidgetKind::Row | WidgetKind::Column) {
            self.flex_flow.1 = enabled;
        } else {
            self.record_invalid_property(crate::Property::Style);
        }
        self
    }

    /// Set how a label displays text that exceeds its available space.
    pub fn text_overflow(mut self, overflow: crate::TextOverflow) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Label>,
    {
        if let WidgetKind::Label { text_overflow, .. } = &mut self.kind {
            *text_overflow = overflow;
        } else {
            self.record_invalid_property(crate::Property::TextOverflow);
        }
        self
    }

    #[cfg(feature = "widget-textarea")]
    pub fn placeholder(mut self, text: impl Into<String>) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Text>,
    {
        if let WidgetKind::Textarea { placeholder, .. } = &mut self.kind {
            *placeholder = text.into();
        } else {
            self.record_invalid_property(crate::Property::Placeholder);
        }
        self
    }

    #[cfg(feature = "widget-textarea")]
    pub fn password(mut self, enabled: bool) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Text>,
    {
        if let WidgetKind::Textarea { password, .. } = &mut self.kind {
            *password = enabled;
        } else {
            self.record_invalid_property(crate::Property::Password);
        }
        self
    }

    #[cfg(feature = "widget-textarea")]
    pub fn multiline(mut self) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Text>,
    {
        if let WidgetKind::Textarea { one_line, .. } = &mut self.kind {
            *one_line = false;
        } else {
            self.record_invalid_property(crate::Property::Multiline);
        }
        self
    }

    /// Register this textarea as a keyboard target.
    ///
    /// IDs are application-scoped; the reconciler resolves them after each
    /// render. Only textareas may carry a keyboard id.
    pub fn keyboard_id(mut self, id: u64) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Text>,
    {
        self.keyboard_id = Some(id);
        self
    }

    /// Number of rows a roller shows. Ignored for other kinds.
    #[cfg(feature = "widget-roller")]
    pub fn visible_rows(mut self, rows: u32) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Roller>,
    {
        if let WidgetKind::Roller { visible_rows, .. } = &mut self.kind {
            *visible_rows = rows;
        } else {
            self.record_invalid_property(crate::Property::VisibleRows);
        }
        self
    }

    /// Scrolling behavior of a roller. Ignored for other kinds.
    #[cfg(feature = "widget-roller")]
    pub fn roller_mode(mut self, roller_mode: RollerMode) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Roller>,
    {
        if let WidgetKind::Roller { mode, .. } = &mut self.kind {
            *mode = roller_mode;
        } else {
            self.record_invalid_property(crate::Property::RollerMode);
        }
        self
    }

    /// Digit count and decimal separator position of a spinbox.
    ///
    /// `decimal_pos` is the number of integer digits before the separator;
    /// `0` hides it. Ignored for other kinds.
    #[cfg(feature = "widget-spinbox")]
    pub fn digits(mut self, count: u32, decimal_position: u32) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Spinbox>,
    {
        if let WidgetKind::Spinbox {
            digit_count,
            decimal_pos,
            ..
        } = &mut self.kind
        {
            *digit_count = count;
            *decimal_pos = decimal_position;
        } else {
            self.record_invalid_property(crate::Property::Digits);
        }
        self
    }

    /// Increment applied by LVGL's spinbox key handling. Ignored for other
    /// kinds.
    #[cfg(feature = "widget-spinbox")]
    pub fn step(mut self, value: u32) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Spinbox>,
    {
        if let WidgetKind::Spinbox { step, .. } = &mut self.kind {
            *step = value;
        } else {
            self.record_invalid_property(crate::Property::Step);
        }
        self
    }

    /// Spinner animation parameters. Ignored for other kinds.
    #[cfg(feature = "widget-spinner")]
    pub fn spinner_animation(mut self, duration: u32, angle: u32) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Spinner>,
    {
        if let WidgetKind::Spinner {
            duration_ms,
            arc_angle,
        } = &mut self.kind
        {
            *duration_ms = duration;
            *arc_angle = angle;
        } else {
            self.record_invalid_property(crate::Property::SpinnerAnimation);
        }
        self
    }

    /// LED brightness. LVGL clamps it to `80..=255`; `80` looks off.
    /// Ignored for other kinds.
    #[cfg(feature = "widget-led")]
    pub fn brightness(mut self, value: u8) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Led>,
    {
        if let WidgetKind::Led { brightness, .. } = &mut self.kind {
            *brightness = value;
        } else {
            self.record_invalid_property(crate::Property::Brightness);
        }
        self
    }

    /// Table column widths in pixels. Columns without an entry keep their
    /// current width (LVGL's default unless set before). Ignored for other
    /// kinds.
    #[cfg(feature = "widget-table")]
    pub fn column_widths(mut self, widths: impl IntoIterator<Item = i32>) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Table>,
    {
        if let WidgetKind::Table { column_widths, .. } = &mut self.kind {
            *column_widths = widths.into_iter().collect();
        } else {
            self.record_invalid_property(crate::Property::ColumnWidths);
        }
        self
    }

    /// Message box title. Ignored for other kinds.
    #[cfg(feature = "widget-msgbox")]
    pub fn message_box_title(mut self, text: impl Into<String>) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::MessageBox>,
    {
        if let WidgetKind::MessageBox { title, .. } = &mut self.kind {
            *title = Some(text.into());
        } else {
            self.record_invalid_property(crate::Property::MessageBoxTitle);
        }
        self
    }

    /// Message box footer buttons. A click reports the button index through
    /// [`Node::on_value_changed`]. Ignored for other kinds.
    #[cfg(feature = "widget-msgbox")]
    pub fn message_box_buttons(mut self, texts: impl IntoIterator<Item = impl Into<String>>) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::MessageBox>,
    {
        if let WidgetKind::MessageBox { buttons, .. } = &mut self.kind {
            *buttons = texts.into_iter().map(Into::into).collect();
        } else {
            self.record_invalid_property(crate::Property::MessageBoxButtons);
        }
        self
    }

    /// The date marked as today. Ignored for other kinds.
    #[cfg(feature = "widget-calendar")]
    pub fn today(mut self, date: CalendarDate) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Calendar>,
    {
        if let WidgetKind::Calendar { today, .. } = &mut self.kind {
            *today = Some(date);
        } else {
            self.record_invalid_property(crate::Property::Today);
        }
        self
    }

    /// Dates drawn highlighted. Ignored for other kinds.
    #[cfg(feature = "widget-calendar")]
    pub fn highlighted(mut self, dates: impl IntoIterator<Item = CalendarDate>) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::Calendar>,
    {
        if let WidgetKind::Calendar { highlighted, .. } = &mut self.kind {
            *highlighted = dates.into_iter().collect();
        } else {
            self.record_invalid_property(crate::Property::Highlighted);
        }
        self
    }

    /// Image button picture while pressed. Ignored for other kinds.
    #[cfg(feature = "widget-imagebutton")]
    pub fn pressed_image(mut self, source: ImageSource) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::ImageButton>,
    {
        if let WidgetKind::ImageButton { pressed, .. } = &mut self.kind {
            *pressed = Some(source);
        } else {
            self.record_invalid_property(crate::Property::PressedImage);
        }
        self
    }

    /// Animated image repeat count. Ignored for other kinds.
    #[cfg(feature = "widget-animimage")]
    pub fn repeat(mut self, count: u32) -> Self
    where
        K: crate::typed::Supports<crate::typed::capability::AnimatedImage>,
    {
        if let WidgetKind::AnimatedImage { repeat, .. } = &mut self.kind {
            *repeat = count;
        } else {
            self.record_invalid_property(crate::Property::Repeat);
        }
        self
    }
}
