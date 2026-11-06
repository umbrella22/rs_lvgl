//! Mount native objects and clean up incomplete subtrees.

use super::interaction::{apply_checked, apply_flags, sync_handlers};
use super::styles::apply_resource_font;
use super::{delete_subtree, Counters, MountedCustom, MountedNode};
use crate::events::EventQueue;
use crate::node::Node;
use crate::widgets::{self, KindTag, Parts, Props};
use crate::UiError;
use alloc::{rc::Rc, vec::Vec};
use core::ptr::NonNull;
use rs_lvgl::{Obj, Widget};
use rs_lvgl_sys::lv_obj_t;

/// Delete a native object whose mount failed after it was created.
fn abort_create(custom: Option<MountedCustom>, obj: NonNull<lv_obj_t>) {
    if let Some(custom) = custom {
        custom.delete();
    }
    // SAFETY: the object was created by the failing `create` call, the custom
    // delete hook (if any) already ran, and no mounted handle refers to it.
    unsafe { rs_lvgl_sys::lv_obj_delete(obj.as_ptr()) };
}
pub(crate) fn create<M: Clone + 'static>(
    parent: NonNull<lv_obj_t>,
    node: Node<M>,
    queue: &Rc<EventQueue<M>>,
    counters: &mut Counters,
    child_index: usize,
) -> Result<MountedNode<M>, UiError> {
    queue.diagnostics.node(
        node.key,
        child_index,
        widgets::tag(&node.kind),
        crate::diagnostics::DiagnosticStage::Create,
        crate::Property::Value,
        || create_inner(parent, node, queue, counters),
    )
}

pub(super) fn create_inner<M: Clone + 'static>(
    parent: NonNull<lv_obj_t>,
    node: Node<M>,
    queue: &Rc<EventQueue<M>>,
    counters: &mut Counters,
) -> Result<MountedNode<M>, UiError> {
    let Node {
        static_text,
        font_resource,
        #[cfg(feature = "widget-image")]
        image_resource,
        key,
        kind,
        style,
        part_styles,
        keyboard_id,
        grid_cell,
        children,
        handlers,
        clickable,
        scrollable,
        checked,
        interaction,
        animations,
        #[cfg(feature = "input-keypad")]
        focusable,
        #[cfg(feature = "input-keypad")]
        modal,
        #[cfg(feature = "input-gridnav")]
        grid_navigation,
        ..
    } = node;

    let tag = widgets::tag(&kind);
    let (mut props, entry) = widgets::into_props(kind);
    widgets::retain_static_text(&mut props, static_text);

    let mut custom = None;
    let (obj, parts) = match entry {
        Some(mut entry) => {
            // SAFETY: `parent` is a live object for the duration of this call;
            // `Obj` is a non-owning handle with no destructor.
            let mut parent_obj = unsafe { Obj::from_raw(parent) }.ok_or(UiError::Allocation)?;
            let object = entry.create(&mut parent_obj)?;
            let obj = object.raw();
            custom = Some(MountedCustom { entry, object });
            (obj, Parts::default())
        }
        None => widgets::create(tag, parent)?,
    };
    let lifecycle = custom
        .as_ref()
        .map(|custom| custom.entry.is_lifecycle())
        .unwrap_or(false);

    // Handlers only get a native slot when the node has any; objects without
    // handlers carry no event callbacks at all.
    let mut slot = None;
    if let Err(error) = sync_handlers(&mut slot, obj, tag, handlers, queue) {
        abort_create(custom, obj);
        return Err(error);
    }
    apply_flags(obj, clickable, scrollable, None, None);
    interaction.apply(obj, crate::scroll::Interaction::default());
    apply_checked(obj, tag, checked);

    let slot_ref = slot.map(|slot| unsafe {
        // SAFETY: the slot was just allocated and is owned by the native
        // object's delete callback; it remains valid for this call.
        &*slot.as_ptr()
    });
    let mut retained = widgets::Retained::None;
    if let Err(error) = widgets::apply_props(obj, parts, &props, None, slot_ref, &mut retained) {
        abort_create(custom, obj);
        return Err(error);
    }
    #[cfg(feature = "widget-image")]
    if let Some(image) = image_resource {
        retained = widgets::Retained::LoadedImage(image);
    }
    if tag != KindTag::Custom {
        widgets::apply_style(obj, &style, tag);
        for (selector, native_part_style) in &part_styles {
            widgets::apply_style_for_selector(obj, native_part_style, tag, *selector);
        }
    }
    if font_resource.is_some() {
        apply_resource_font(obj, &style, font_resource.as_ref());
    }
    let transitions = if tag != KindTag::Custom {
        crate::animation::install_transitions(obj, &style, &part_styles)
    } else {
        Vec::new()
    };
    if let Err(error) = crate::animation::sync(obj, &[], &animations) {
        abort_create(custom, obj);
        return Err(error);
    }
    let (grid_columns, grid_rows) = match &props {
        Props::Grid { columns, rows } => {
            let (columns, rows) = widgets::grid_descriptors(columns, rows);
            widgets::set_grid_descriptors(obj, &columns, &rows);
            (Some(columns), Some(rows))
        }
        _ => (None, None),
    };
    if let Some(cell) = grid_cell {
        widgets::apply_grid_cell(obj, cell);
    }
    let pages = match &props {
        #[cfg(feature = "widget-tabview")]
        Props::Tabview { tabs, active } => {
            let pages = widgets::suppress(slot_ref, || widgets::tabview_pages(obj, tabs, *active));
            Some(pages)
        }
        #[cfg(feature = "widget-tileview")]
        Props::Tileview { tiles, active } => {
            let pages =
                widgets::suppress(slot_ref, || widgets::tileview_pages(obj, tiles, *active));
            Some(pages)
        }
        _ => None,
    };
    let pages = match pages.transpose() {
        Ok(pages) => pages,
        Err(error) => {
            abort_create(custom, obj);
            return Err(error);
        }
    };

    counters.created += 1;
    let mut mounted = MountedNode {
        obj,
        key,
        tag,
        props,
        style,
        font_resource,
        transitions,
        animations,
        slot,
        parts,
        grid_cell,
        part_styles,
        keyboard_id,
        clickable,
        scrollable,
        interaction,
        grid_columns,
        #[cfg(feature = "input-keypad")]
        focusable,
        #[cfg(feature = "input-keypad")]
        modal,
        #[cfg(feature = "input-gridnav")]
        grid_navigation,
        grid_rows,
        #[cfg(feature = "input-keypad")]
        focus_id: counters.created,
        #[cfg(feature = "input-gridnav")]
        grid_state: None,
        #[cfg(feature = "input-gridnav")]
        grid_child_focusable: None,
        retained,
        pages,
        custom,
        children: Vec::new(),
        count: 1,
        customs: usize::from(lifecycle),
    };

    let children_parent = parts.children_parent(obj);
    for (index, child) in children.into_iter().enumerate() {
        let parent = mounted
            .pages
            .as_ref()
            .map(|pages| pages[index])
            .unwrap_or(children_parent);
        match create(parent, child, queue, counters, index) {
            Ok(child) => {
                mounted.count += child.count;
                mounted.customs += child.customs;
                mounted.children.push(child);
            }
            Err(error) => {
                // Run custom delete hooks while the native objects are still
                // alive; LVGL deletes the whole subtree next.
                delete_subtree(mounted, counters);
                return Err(error);
            }
        }
    }

    Ok(mounted)
}
