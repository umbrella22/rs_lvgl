//! Textarea editing settings, separate from authoritative model text.

use alloc::string::String;
use core::num::NonZeroU32;

#[derive(Debug, PartialEq, Eq)]
/// Opaque textarea input settings assembled by the view modifiers.
/// Default permits all characters and pointer cursor placement, without a length limit.
pub struct TextareaInputOptions {
    pub(crate) accepted: Option<String>,
    pub(crate) maximum: Option<NonZeroU32>,
    pub(crate) cursor: Option<u32>,
    pub(crate) click: bool,
}

impl Default for TextareaInputOptions {
    fn default() -> Self {
        Self {
            accepted: None,
            maximum: None,
            cursor: None,
            click: true,
        }
    }
}

impl TextareaInputOptions {
    pub(crate) fn validate(&self, text: &str) -> Result<(), crate::UiError> {
        if self
            .accepted
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.as_bytes().contains(&0))
            || self
                .cursor
                .is_some_and(|p| p > i32::MAX as u32 || p as usize > text.chars().count())
        {
            return Err(crate::UiError::InvalidProperty);
        }
        Ok(())
    }
}
