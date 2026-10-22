//! Opt-in native arrow navigation among direct children.
use crate::reconciler::MountedNode;

/// Native spatial navigation for the direct children of Obj/Row/Column/Grid.
/// The container occupies one group position. Tab leaves it; arrows select a
/// clickable, eligible child. Descendants of those children are not group members.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GridNavigation {
    /// Wrap at an edge instead of moving to the next/previous group member.
    pub rollover: bool,
    /// Scroll the selected child before moving to another child.
    pub scroll_first: bool,
    pub axis: NavigationAxis,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NavigationAxis {
    #[default]
    Both,
    Horizontal,
    Vertical,
}
impl GridNavigation {
    fn raw(self) -> u32 {
        let mut flags = match self.axis {
            NavigationAxis::Both => rs_lvgl_sys::lv_gridnav_ctrl_t_LV_GRIDNAV_CTRL_NONE,
            NavigationAxis::Horizontal => {
                rs_lvgl_sys::lv_gridnav_ctrl_t_LV_GRIDNAV_CTRL_HORIZONTAL_MOVE_ONLY
            }
            NavigationAxis::Vertical => {
                rs_lvgl_sys::lv_gridnav_ctrl_t_LV_GRIDNAV_CTRL_VERTICAL_MOVE_ONLY
            }
        };
        if self.rollover {
            flags |= rs_lvgl_sys::lv_gridnav_ctrl_t_LV_GRIDNAV_CTRL_ROLLOVER;
        }
        if self.scroll_first {
            flags |= rs_lvgl_sys::lv_gridnav_ctrl_t_LV_GRIDNAV_CTRL_SCROLL_FIRST;
        }
        flags
    }
}
pub(crate) struct GridState {
    config: GridNavigation,
    arrow: bool,
    selected: Option<u64>,
    installed: bool,
}

/// Save stable identity and detach native child pointers before reconciliation.
pub(crate) fn capture<M: Clone + 'static>(node: &mut MountedNode<M>) {
    if let Some(state) = &mut node.grid_state {
        // SAFETY: called before any objects in the mounted tree are mutated.
        unsafe {
            if let Some(child) = node.children.iter().find(|child| {
                rs_lvgl_sys::lv_obj_has_state(
                    child.obj.as_ptr(),
                    rs_lvgl_sys::lv_state_t_LV_STATE_FOCUSED,
                )
            }) {
                state.selected = Some(child.focus_id);
            }
            if state.installed {
                rs_lvgl_sys::lv_gridnav_remove(node.obj.as_ptr());
                state.installed = false;
            }
        }
    }
    for child in &mut node.children {
        capture(child);
    }
}

pub(crate) fn sync<M: Clone + 'static>(node: &mut MountedNode<M>) {
    // SAFETY: reconciliation has finished; the node and its immediate children
    // remain live throughout this synchronous flag/callback update.
    unsafe {
        let obj = node.obj.as_ptr();
        if let Some(config) = node.grid_navigation {
            for child in &mut node.children {
                let child_obj = child.obj.as_ptr();
                if child.grid_child_focusable.is_none() {
                    child.grid_child_focusable =
                        Some(rs_lvgl_sys::lv_obj_is_click_focusable(child_obj));
                }
                let selectable = child
                    .focusable
                    .unwrap_or_else(|| rs_lvgl_sys::lv_obj_is_group_def(child_obj))
                    && crate::focus::eligible(child_obj);
                rs_lvgl_sys::lv_obj_set_click_focusable(child_obj, selectable);
            }
            let state = node.grid_state.get_or_insert_with(|| GridState {
                config,
                arrow: rs_lvgl_sys::lv_obj_is_scroll_with_arrow(obj),
                selected: None,
                installed: false,
            });
            if state.installed {
                if let Some(child) = node.children.iter().find(|child| {
                    rs_lvgl_sys::lv_obj_has_state(
                        child.obj.as_ptr(),
                        rs_lvgl_sys::lv_state_t_LV_STATE_FOCUSED,
                    )
                }) {
                    state.selected = Some(child.focus_id);
                }
            }
            if !state.installed || state.config != config {
                rs_lvgl_sys::lv_gridnav_add(obj, config.raw());
                state.config = config;
                state.installed = true;
            }
            let selectable = |child: &&MountedNode<M>| {
                rs_lvgl_sys::lv_obj_is_clickable(child.obj.as_ptr())
                    && rs_lvgl_sys::lv_obj_is_click_focusable(child.obj.as_ptr())
                    && crate::focus::eligible(child.obj.as_ptr())
            };
            let selected = node
                .children
                .iter()
                .filter(selectable)
                .find(|child| Some(child.focus_id) == state.selected)
                .or_else(|| node.children.iter().find(selectable));
            state.selected = selected.map(|child| child.focus_id);
            for child in &node.children {
                if Some(child.focus_id) != state.selected {
                    rs_lvgl_sys::lv_obj_remove_state(
                        child.obj.as_ptr(),
                        rs_lvgl_sys::lv_state_t_LV_STATE_FOCUSED
                            | rs_lvgl_sys::lv_state_t_LV_STATE_FOCUS_KEY,
                    );
                }
            }
            if let Some(child) = selected {
                rs_lvgl_sys::lv_gridnav_set_focused(obj, child.obj.as_ptr(), false);
                // An unfocused container must not paint an active child focus.
                if !rs_lvgl_sys::lv_obj_has_state(obj, rs_lvgl_sys::lv_state_t_LV_STATE_FOCUSED) {
                    rs_lvgl_sys::lv_obj_remove_state(
                        child.obj.as_ptr(),
                        rs_lvgl_sys::lv_state_t_LV_STATE_FOCUSED
                            | rs_lvgl_sys::lv_state_t_LV_STATE_FOCUS_KEY,
                    );
                }
            } else {
                // Native press forwarding expects a non-null selected child.
                // An empty/ineligible grid never has a callback or group member.
                rs_lvgl_sys::lv_gridnav_remove(obj);
                state.installed = false;
                for child in &node.children {
                    rs_lvgl_sys::lv_obj_remove_state(
                        child.obj.as_ptr(),
                        rs_lvgl_sys::lv_state_t_LV_STATE_FOCUSED
                            | rs_lvgl_sys::lv_state_t_LV_STATE_FOCUS_KEY,
                    );
                }
            }
        } else if let Some(state) = node.grid_state.take() {
            if state.installed {
                rs_lvgl_sys::lv_gridnav_remove(obj);
            }
            rs_lvgl_sys::lv_obj_set_scroll_with_arrow(obj, state.arrow);
            for child in &mut node.children {
                if let Some(original) = child.grid_child_focusable.take() {
                    rs_lvgl_sys::lv_obj_set_click_focusable(child.obj.as_ptr(), original);
                    rs_lvgl_sys::lv_obj_remove_state(
                        child.obj.as_ptr(),
                        rs_lvgl_sys::lv_state_t_LV_STATE_FOCUSED
                            | rs_lvgl_sys::lv_state_t_LV_STATE_FOCUS_KEY,
                    );
                }
            }
        }
    }
    for child in &mut node.children {
        sync(child);
    }
}
