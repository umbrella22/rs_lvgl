//! Structured metadata. Values typed by applications never enter this stream.
use crate::widgets::KindTag;
use crate::{Key, UiError};
#[cfg(feature = "diagnostics")]
use alloc::rc::Rc;
#[cfg(feature = "diagnostics")]
use core::cell::{Cell, RefCell};

/// A property identifier; no application-supplied property value is retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Property {
    Value,
    Children,
    Style,
    Animation,
    KeyboardId,
    GridNavigation,
    Text,
    TextOverflow,
    ThumbStyle,
    ProgressStyle,
    Range,
    Source,
    Scale,
    Placeholder,
    Password,
    Multiline,
    VisibleRows,
    RollerMode,
    Digits,
    Step,
    SpinnerAnimation,
    Brightness,
    ColumnWidths,
    MessageBoxTitle,
    MessageBoxButtons,
    Today,
    Highlighted,
    PressedImage,
    Repeat,
    Font,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticStage {
    Validation,
    Create,
    Update,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticTree {
    View,
    Overlay,
}

/// Enter/Exit events form a path stack. Child indices disambiguate Auto keys;
/// Tree selects view or overlay. Error is emitted once at the deepest failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticEvent {
    Tree(DiagnosticTree),
    Enter {
        key: Key,
        child_index: usize,
        kind: KindTag,
        stage: DiagnosticStage,
    },
    Exit,
    Error {
        error: UiError,
        property: Property,
        stage: DiagnosticStage,
    },
    IgnoredProperty(Property),
}

#[cfg(feature = "diagnostics")]
pub type DiagnosticSink = Rc<dyn Fn(DiagnosticEvent)>;

#[derive(Default)]
pub(crate) struct Diagnostics {
    #[cfg(feature = "diagnostics")]
    sink: RefCell<Option<DiagnosticSink>>,
    #[cfg(feature = "diagnostics")]
    errors: Cell<u64>,
}

impl Diagnostics {
    #[cfg(feature = "diagnostics")]
    pub(crate) fn set(&self, sink: Option<DiagnosticSink>) {
        *self.sink.borrow_mut() = sink;
    }

    pub(crate) fn emit(&self, event: DiagnosticEvent) {
        #[cfg(feature = "diagnostics")]
        {
            if matches!(event, DiagnosticEvent::Error { .. }) {
                self.errors.set(self.errors.get().wrapping_add(1));
            }
            let sink = self.sink.borrow().clone();
            if let Some(sink) = sink {
                // SAFETY: both stack values remain live throughout the
                // synchronous call. The C ABI prevents a sink panic from
                // unwinding past partially mounted native/retained state.
                unsafe {
                    invoke_sink(
                        (&sink as *const DiagnosticSink).cast(),
                        (&event as *const DiagnosticEvent).cast(),
                    );
                }
            }
        }
        #[cfg(not(feature = "diagnostics"))]
        let _ = event;
    }

    fn serial(&self) -> u64 {
        #[cfg(feature = "diagnostics")]
        {
            self.errors.get()
        }
        #[cfg(not(feature = "diagnostics"))]
        {
            0
        }
    }

    pub(crate) fn report(&self, error: UiError, property: Property, stage: DiagnosticStage) {
        self.emit(DiagnosticEvent::Error {
            error,
            property,
            stage,
        });
    }

    pub(crate) fn node<T>(
        &self,
        key: Key,
        child_index: usize,
        kind: KindTag,
        stage: DiagnosticStage,
        property: Property,
        run: impl FnOnce() -> Result<T, UiError>,
    ) -> Result<T, UiError> {
        let before = self.serial();
        self.emit(DiagnosticEvent::Enter {
            key,
            child_index,
            kind,
            stage,
        });
        let result = run();
        if let Err(error) = &result {
            if self.serial() == before {
                self.report(*error, property, stage);
            }
        }
        self.emit(DiagnosticEvent::Exit);
        result
    }
}

/// # Safety
/// Both opaque pointers must refer to live DiagnosticSink/DiagnosticEvent values
/// for this synchronous call on the UI thread. Neither pointer is retained.
#[cfg(feature = "diagnostics")]
unsafe extern "C" fn invoke_sink(sink: *const core::ffi::c_void, event: *const core::ffi::c_void) {
    // SAFETY: emit supplies pointers to its live stack values of these types.
    let (sink, event) = unsafe {
        (
            &*sink.cast::<DiagnosticSink>(),
            *event.cast::<DiagnosticEvent>(),
        )
    };
    sink(event);
}
