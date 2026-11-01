//! Resolve keyboard targets and unbind them before native deletion.

use super::{Counters, MountedNode};
use crate::widgets::Props;
use alloc::vec::Vec;
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

/// Drop keyboard bindings that refer to objects inside `subtree`.
///
/// Must run while the subtree's native objects are still alive: a keyboard
/// outside the subtree bound to a textarea inside it is unbound here, because
/// LVGL's keyboard would otherwise keep a dangling textarea pointer.
pub(crate) fn release_keyboards<M: Clone + 'static>(
    subtree: &MountedNode<M>,
    counters: &mut Counters,
) {
    if counters.keyboards.is_empty() {
        return;
    }
    let mut objects = Vec::new();
    collect_objects(subtree, &mut objects);
    counters.keyboards.retain(|(keyboard, textarea)| {
        if objects.contains(keyboard) {
            // The keyboard is deleted with the subtree.
            return false;
        }
        if objects.contains(textarea) {
            // SAFETY: the keyboard is live outside the subtree and the
            // textarea is still alive; null unbinds the keyboard.
            unsafe { set_keyboard_textarea(*keyboard, core::ptr::null_mut()) };
            return false;
        }
        true
    });
}

fn collect_objects<M: Clone + 'static>(
    node: &MountedNode<M>,
    objects: &mut Vec<NonNull<lv_obj_t>>,
) {
    objects.push(node.obj);
    #[cfg(feature = "widget-ime-pinyin")]
    if let Some(input) = node.parts.input {
        objects.push(input);
    }
    for child in &node.children {
        collect_objects(child, objects);
    }
}

/// Resolve keyboard targets after a render. Textareas register their
/// application-scoped `keyboard_id`; keyboards bind to the matching object.
///
/// Only keyboards whose target changed since the last render are touched, so
/// an unchanged tree does not re-focus the textarea on every render.
pub(crate) fn bind_keyboards<M: Clone + 'static>(
    roots: &[Option<&MountedNode<M>>],
    counters: &mut Counters,
) {
    let mut targets: Vec<(u64, NonNull<lv_obj_t>)> = Vec::new();
    let mut keyboards: Vec<(NonNull<lv_obj_t>, u64)> = Vec::new();
    for root in roots.iter().flatten() {
        collect_keyboards(root, &mut targets, &mut keyboards);
    }
    let mut bound = Vec::with_capacity(keyboards.len());
    for (keyboard, target) in keyboards {
        let textarea = targets
            .iter()
            .find(|(id, _)| *id == target)
            .map(|(_, object)| *object);
        let previous = counters
            .keyboards
            .iter()
            .find(|(existing, _)| *existing == keyboard)
            .map(|(_, textarea)| *textarea);
        if previous != textarea {
            let raw = textarea
                .map(NonNull::as_ptr)
                .unwrap_or(core::ptr::null_mut());
            // SAFETY: the keyboard is live; the previously bound textarea is
            // live because deletions unbind it first (`release_keyboards`);
            // the new textarea, if any, is live in the mounted tree.
            unsafe { set_keyboard_textarea(keyboard, raw) };
        }
        if let Some(textarea) = textarea {
            bound.push((keyboard, textarea));
        }
    }
    counters.keyboards = bound;
}

/// Bind a native keyboard to a textarea, or unbind it with null.
///
/// # Safety
///
/// `keyboard` must be a live keyboard and `textarea` null or a live textarea.
#[cfg(feature = "widget-keyboard")]
unsafe fn set_keyboard_textarea(keyboard: NonNull<lv_obj_t>, textarea: *mut lv_obj_t) {
    // SAFETY: guaranteed by the caller.
    unsafe { rs_lvgl_sys::lv_keyboard_set_textarea(keyboard.as_ptr(), textarea) };
}

/// Without keyboards no binding is ever recorded, so this is unreachable.
///
/// # Safety
///
/// No requirements; the function does nothing.
#[cfg(not(feature = "widget-keyboard"))]
unsafe fn set_keyboard_textarea(_keyboard: NonNull<lv_obj_t>, _textarea: *mut lv_obj_t) {}

fn collect_keyboards<M: Clone + 'static>(
    node: &MountedNode<M>,
    targets: &mut Vec<(u64, NonNull<lv_obj_t>)>,
    keyboards: &mut Vec<(NonNull<lv_obj_t>, u64)>,
) {
    #[cfg(feature = "widget-ime-pinyin")]
    if let Props::PinyinKeyboard { target, .. } = node.props {
        keyboards.push((node.parts.input.expect("IME keyboard"), target));
    }
    if let Some(id) = node.keyboard_id {
        // `node::validate` rejects duplicate ids tree-wide.
        targets.push((id, node.obj));
    }
    #[cfg(feature = "widget-keyboard")]
    if let Props::Keyboard { target } = &node.props {
        keyboards.push((node.obj, *target));
    }
    for child in &node.children {
        collect_keyboards(child, targets, keyboards);
    }
}
