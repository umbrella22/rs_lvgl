//! Explicit focus activation and native input binding.

use super::guard::enter;
use super::Ui;
use crate::UiError;
use core::ptr::NonNull;

impl<M: Clone + 'static> Ui<M> {
    /// for missing or ineligible nodes; ambiguous keys and Auto are invalid.
    /// This does not change sibling reconciliation identity rules.
    /// Enable focus with `enable_focus` or bind an input device first.
    #[cfg(feature = "input-keypad")]
    pub fn focus(&mut self, key: crate::Key) -> Result<bool, UiError> {
        let _guard = enter(&self.busy)?;
        if key == crate::Key::Auto {
            return Err(UiError::InvalidProperty);
        }
        self.focus
            .as_ref()
            .map(|group| group.focus(&[self.mounted.as_ref(), self.overlay.as_ref()], key))
            .unwrap_or(Ok(false))
    }

    /// Bind a live typed keypad to this UI's group.
    #[cfg(feature = "input-keypad")]
    pub fn bind_keypad(&mut self, keypad: &rs_lvgl::input_device::Keypad) -> Result<(), UiError> {
        let raw = keypad.raw().ok_or(UiError::InvalidProperty)?;
        // SAFETY: the keypad exposes a live descriptor on the same UI thread.
        unsafe { self.bind_input(raw) }
    }

    /// Activate this UI's focus group, even without a bound device.
    /// Merely enabling the Cargo feature does not change existing screens'
    /// focus or emit new focus messages. Binding an input also activates it.
    #[cfg(feature = "input-keypad")]
    pub fn enable_focus(&mut self) -> Result<(), UiError> {
        let _guard = enter(&self.busy)?;
        if self.focus.is_none() {
            self.focus = Some(crate::focus::FocusGroup::new()?);
        }
        self.sync_focus()
    }

    /// Bind a platform keypad or encoder (including LVGL's SDL devices).
    ///
    /// # Safety
    /// `input` must be a live LVGL input device on this UI thread. Do not
    /// mutate/delete it reentrantly during callbacks. Native group deletion
    /// automatically detaches the device, so either may be deleted first.
    #[cfg(feature = "input-keypad")]
    pub unsafe fn bind_input(
        &mut self,
        input: NonNull<rs_lvgl_sys::lv_indev_t>,
    ) -> Result<(), UiError> {
        let _guard = enter(&self.busy)?;
        // SAFETY: the caller guarantees input validity; screen is live under
        // Ui's lifecycle contract. Reject foreign displays before mutation.
        unsafe {
            let kind = rs_lvgl_sys::lv_indev_get_type(input.as_ptr());
            if !matches!(
                kind,
                rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_KEYPAD
                    | rs_lvgl_sys::lv_indev_type_t_LV_INDEV_TYPE_ENCODER
            ) || rs_lvgl_sys::lv_indev_get_display(input.as_ptr())
                != rs_lvgl_sys::lv_obj_get_display(self.screen.as_ptr())
            {
                return Err(UiError::InvalidProperty);
            }
            if self.focus.is_none() {
                self.focus = Some(crate::focus::FocusGroup::new()?);
            }
            rs_lvgl_sys::lv_indev_set_group(
                input.as_ptr(),
                self.focus.as_ref().expect("group was just ensured").raw(),
            );
        }
        self.sync_focus()
    }

    /// Select native encoder editing or navigation mode.
    #[cfg(feature = "input-keypad")]
    pub fn set_editing(&mut self, editing: bool) -> Result<(), UiError> {
        let _guard = enter(&self.busy)?;
        let group = self.focus.as_ref().ok_or(UiError::InvalidProperty)?;
        // SAFETY: the per-UI group is live on this thread.
        unsafe { rs_lvgl_sys::lv_group_set_editing(group.raw(), editing) };
        Ok(())
    }

    #[cfg(feature = "input-keypad")]
    pub(super) fn sync_focus(&mut self) -> Result<(), UiError> {
        if let Some(group) = &mut self.focus {
            #[cfg(feature = "input-gridnav")]
            for tree in [&mut self.mounted, &mut self.overlay].into_iter().flatten() {
                crate::gridnav::sync(tree);
            }
            if let Err(error) = group.sync(&[self.mounted.as_ref(), self.overlay.as_ref()]) {
                self.delete_overlay();
                self.delete_root();
                self.focus = None;
                return Err(error);
            }
        }
        Ok(())
    }
}
