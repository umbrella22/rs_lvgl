//! Per-UI native groups, modal membership and stable focus restoration.
use crate::{reconciler::MountedNode, Key, UiError};
use alloc::vec::Vec;
use core::ptr::NonNull;
use rs_lvgl_sys::{lv_group_t, lv_obj_t};

#[derive(Clone, Copy)]
struct FocusIdentity {
    id: u64,
    editing: bool,
}
struct ModalFrame {
    id: u64,
    restore: Option<FocusIdentity>,
}
pub(crate) struct FocusGroup {
    raw: NonNull<lv_group_t>,
    before: Option<FocusIdentity>,
    modals: Vec<ModalFrame>,
}
impl FocusGroup {
    pub(crate) fn new() -> Result<Self, UiError> {
        // SAFETY: creation runs on the UI thread after LVGL initialization.
        let raw =
            NonNull::new(unsafe { rs_lvgl_sys::lv_group_create() }).ok_or(UiError::Allocation)?;
        Ok(Self {
            raw,
            before: None,
            modals: Vec::new(),
        })
    }
    pub(crate) fn raw(&self) -> *mut lv_group_t {
        self.raw.as_ptr()
    }
    /// Capture before reconciliation can delete the focused native object.
    pub(crate) fn capture<M: Clone + 'static>(&mut self, trees: &[Option<&MountedNode<M>>]) {
        // SAFETY: the group and all mounted objects are still live.
        let focused = unsafe { rs_lvgl_sys::lv_group_get_focused(self.raw()) };
        self.before = None;
        for tree in trees.iter().flatten() {
            visit(tree, &mut |node| {
                if node.focus_object().as_ptr() == focused {
                    self.before = Some(FocusIdentity {
                        id: node.focus_id,
                        editing: unsafe { rs_lvgl_sys::lv_group_get_editing(self.raw()) },
                    });
                }
            });
        }
    }
    pub(crate) fn sync<M: Clone + 'static>(
        &mut self,
        trees: &[Option<&MountedNode<M>>],
    ) -> Result<(), UiError> {
        let before = self.before.take();
        let mut modal = None;
        for tree in trees.iter().flatten() {
            last_modal(tree, &mut modal);
        }
        let modal_id = modal.map(|node| node.focus_id);
        let previous = self.modals.last().map(|scope| scope.id);
        let mut restore = None;
        if previous != modal_id {
            if let Some(id) = modal_id {
                if let Some(index) = self.modals.iter().position(|frame| frame.id == id) {
                    restore = self.modals.get(index + 1).and_then(|frame| frame.restore);
                    self.modals.truncate(index + 1);
                } else {
                    self.modals.push(ModalFrame {
                        id,
                        restore: before,
                    });
                }
            } else {
                restore = self.modals.first().and_then(|frame| frame.restore);
                self.modals.clear();
            }
        }
        let mut desired = Vec::new();
        if let Some(modal) = modal {
            collect(modal, &mut desired);
        } else {
            for tree in trees.iter().flatten() {
                collect(tree, &mut desired);
            }
        }
        let mut restore_obj = None;
        let mut previous_obj = None;
        for tree in trees.iter().flatten() {
            visit(tree, &mut |node| {
                let obj = node.focus_object().as_ptr();
                // SAFETY: objects are live. Explicit focus manages membership
                // even if the platform's default group auto-added a new object.
                unsafe {
                    if !desired.contains(&obj) && !rs_lvgl_sys::lv_obj_get_group(obj).is_null() {
                        rs_lvgl_sys::lv_group_remove_obj(obj);
                    }
                }
                if before.is_some_and(|identity| identity.id == node.focus_id)
                    && desired.contains(&obj)
                {
                    previous_obj = Some(obj);
                }
                if restore.is_some_and(|identity| identity.id == node.focus_id)
                    && desired.contains(&obj)
                {
                    restore_obj = Some(obj);
                }
            });
        }
        // SAFETY: all native pointers come from live trees or this live group.
        // LVGL removes objects from groups as part of native deletion.
        unsafe {
            let focused = rs_lvgl_sys::lv_group_get_focused(self.raw());
            let editing = rs_lvgl_sys::lv_group_get_editing(self.raw());
            let current: Vec<_> = (0..rs_lvgl_sys::lv_group_get_obj_count(self.raw()))
                .map(|i| rs_lvgl_sys::lv_group_get_obj_by_index(self.raw(), i))
                .collect();
            for obj in current {
                if !desired.contains(&obj) {
                    rs_lvgl_sys::lv_group_remove_obj(obj);
                }
            }
            for obj in &desired {
                if rs_lvgl_sys::lv_obj_get_group(*obj) != self.raw() {
                    rs_lvgl_sys::lv_group_add_obj(self.raw(), *obj);
                }
            }
            if rs_lvgl_sys::lv_group_get_obj_count(self.raw()) as usize != desired.len() {
                return Err(UiError::Allocation);
            }
            for (i, obj) in desired.iter().enumerate() {
                let current = rs_lvgl_sys::lv_group_get_obj_by_index(self.raw(), i as u32);
                if current != *obj {
                    rs_lvgl_sys::lv_group_swap_obj(current, *obj);
                }
            }
            let target = restore_obj
                .or(previous_obj)
                .or_else(|| desired.contains(&focused).then_some(focused));
            if let Some(target) = target {
                if rs_lvgl_sys::lv_group_get_focused(self.raw()) != target {
                    rs_lvgl_sys::lv_group_focus_obj(target);
                }
                let editing = if restore_obj.is_some() {
                    restore.unwrap().editing
                } else if previous_obj.is_some() {
                    before.unwrap().editing
                } else {
                    editing
                };
                if rs_lvgl_sys::lv_group_get_editing(self.raw()) != editing {
                    rs_lvgl_sys::lv_group_set_editing(self.raw(), editing);
                }
            }
        }
        Ok(())
    }
    pub(crate) fn focus<M: Clone + 'static>(
        &self,
        trees: &[Option<&MountedNode<M>>],
        key: Key,
    ) -> Result<bool, UiError> {
        if key == Key::Auto {
            return Err(UiError::InvalidProperty);
        }
        let mut matches = Vec::new();
        for tree in trees.iter().flatten() {
            find(tree, key, None, &mut matches);
        }
        if matches.len() > 1 {
            return Err(UiError::InvalidProperty);
        }
        let Some(&(obj, grid)) = matches.first() else {
            return Ok(false);
        };
        // SAFETY: unique live match. Only eligible objects in this group's
        // active modal scope may receive focus, including native grid children.
        unsafe {
            if !eligible(obj) {
                return Ok(false);
            }
            #[cfg(feature = "input-gridnav")]
            if let Some(grid) = grid {
                if rs_lvgl_sys::lv_obj_get_group(grid) != self.raw()
                    || !rs_lvgl_sys::lv_obj_is_click_focusable(obj)
                    || !rs_lvgl_sys::lv_obj_is_clickable(obj)
                {
                    return Ok(false);
                }
                rs_lvgl_sys::lv_group_focus_obj(grid);
                rs_lvgl_sys::lv_gridnav_set_focused(grid, obj, false);
                return Ok(true);
            }
            #[cfg(not(feature = "input-gridnav"))]
            let _ = grid;
            if rs_lvgl_sys::lv_obj_get_group(obj) != self.raw() {
                return Ok(false);
            }
            rs_lvgl_sys::lv_group_focus_obj(obj);
            Ok(rs_lvgl_sys::lv_group_get_focused(self.raw()) == obj)
        }
    }
}
fn visit<M: Clone + 'static>(node: &MountedNode<M>, visitor: &mut impl FnMut(&MountedNode<M>)) {
    visitor(node);
    for child in &node.children {
        visit(child, visitor);
    }
}
fn last_modal<'a, M: Clone + 'static>(
    node: &'a MountedNode<M>,
    out: &mut Option<&'a MountedNode<M>>,
) {
    // SAFETY: the mounted node and its ancestors are live.
    if !unsafe { eligible(node.obj.as_ptr()) } {
        return;
    }
    if node.modal {
        *out = Some(node);
    }
    for child in &node.children {
        last_modal(child, out);
    }
}
fn collect<M: Clone + 'static>(node: &MountedNode<M>, out: &mut Vec<*mut lv_obj_t>) {
    let obj = node.focus_object().as_ptr();
    // SAFETY: traversal borrows a live mounted tree, excluding raw descendants.
    unsafe {
        #[cfg(feature = "input-gridnav")]
        if node.grid_navigation.is_some() {
            if node.focusable != Some(false)
                && eligible(obj)
                && node.children.iter().any(|child| {
                    rs_lvgl_sys::lv_obj_is_clickable(child.obj.as_ptr())
                        && rs_lvgl_sys::lv_obj_is_click_focusable(child.obj.as_ptr())
                        && eligible(child.obj.as_ptr())
                })
            {
                out.push(obj);
            }
            return;
        }
        if node
            .focusable
            .unwrap_or_else(|| rs_lvgl_sys::lv_obj_is_group_def(obj))
            && eligible(obj)
        {
            out.push(obj);
        }
    }
    for child in &node.children {
        collect(child, out);
    }
}
fn find<M: Clone + 'static>(
    node: &MountedNode<M>,
    key: Key,
    grid: Option<*mut lv_obj_t>,
    out: &mut Vec<(*mut lv_obj_t, Option<*mut lv_obj_t>)>,
) {
    if node.key == key {
        out.push((node.focus_object().as_ptr(), grid));
    }
    #[cfg(feature = "input-gridnav")]
    let child_grid = node.grid_navigation.map(|_| node.obj.as_ptr());
    #[cfg(not(feature = "input-gridnav"))]
    let child_grid = None;
    for child in &node.children {
        find(child, key, child_grid, out);
    }
}
/// # Safety
/// The object and every ancestor remain live for this synchronous walk.
pub(crate) unsafe fn eligible(mut obj: *mut lv_obj_t) -> bool {
    while !obj.is_null() {
        // SAFETY: each parent returned by LVGL is live with its child.
        unsafe {
            if rs_lvgl_sys::lv_obj_is_hidden(obj)
                || rs_lvgl_sys::lv_obj_has_state(obj, rs_lvgl_sys::lv_state_t_LV_STATE_DISABLED)
            {
                return false;
            }
            obj = rs_lvgl_sys::lv_obj_get_parent(obj);
        }
    }
    true
}
impl Drop for FocusGroup {
    fn drop(&mut self) {
        // SAFETY: Ui deletes its trees first; native deletion detaches devices.
        unsafe { rs_lvgl_sys::lv_group_delete(self.raw()) };
    }
}
