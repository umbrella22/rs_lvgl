//! Patch compatible nodes or replace a changed native subtree.

use super::children::reconcile_children;
use super::interaction::{apply_checked, apply_flags, sync_handlers};
use super::mount::create_inner;
use super::styles::update_styles;
use super::{delete_subtree, Counters, MountedNode};
use crate::events::EventQueue;
use crate::node::{Node, WidgetKind};
use crate::widgets::{self, Props};
use crate::UiError;
use alloc::rc::Rc;
use core::ptr::NonNull;

pub(crate) fn update<M: Clone + 'static>(
    mounted: &mut MountedNode<M>,
    node: Node<M>,
    queue: &Rc<EventQueue<M>>,
    counters: &mut Counters,
    child_index: usize,
) -> Result<(), UiError> {
    queue.diagnostics.node(
        node.key,
        child_index,
        widgets::tag(&node.kind),
        crate::diagnostics::DiagnosticStage::Update,
        crate::Property::Value,
        || update_inner(mounted, node, queue, counters),
    )
}

fn update_inner<M: Clone + 'static>(
    mounted: &mut MountedNode<M>,
    node: Node<M>,
    queue: &Rc<EventQueue<M>>,
    counters: &mut Counters,
) -> Result<(), UiError> {
    let tag = widgets::tag(&node.kind);

    // Identity is the key: a different key, kind, custom variant, or a
    // structural property LVGL cannot patch (tab count, tiles, message box
    // header and buttons) replaces the native subtree instead.
    let replace = node.key != mounted.key
        || tag != mounted.tag
        || match (&mounted.custom, &node.kind) {
            (Some(custom), WidgetKind::Custom(entry)) => !custom.entry.same_variant(entry),
            (None, WidgetKind::Custom(_)) => true,
            _ => false,
        }
        || widgets::needs_recreate(&mounted.props, &node.kind);
    if replace {
        return recreate(mounted, node, queue, counters);
    }

    let Node {
        static_text,
        key: _,
        kind,
        font_resource,
        #[cfg(feature = "widget-image")]
        image_resource,
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
    // A mounted lifecycle widget keeps its instance; the new descriptor's
    // factory is dropped here.
    let (mut props, _entry) = widgets::into_props(kind);
    widgets::retain_static_text(&mut props, static_text);

    sync_handlers(&mut mounted.slot, mounted.obj, tag, handlers, queue)?;
    apply_flags(
        mounted.obj,
        clickable,
        scrollable,
        mounted.clickable,
        mounted.scrollable,
    );
    apply_checked(mounted.obj, tag, checked);
    interaction.apply(mounted.obj, mounted.interaction);
    mounted.interaction = interaction;
    mounted.clickable = clickable.or(mounted.clickable);
    mounted.scrollable = scrollable.or(mounted.scrollable);
    #[cfg(feature = "input-keypad")]
    {
        mounted.focusable = focusable;
        mounted.modal = modal;
    }
    #[cfg(feature = "input-gridnav")]
    {
        mounted.grid_navigation = grid_navigation;
    }
    let slot = mounted.slot.map(|slot| unsafe {
        // SAFETY: the slot is owned by the live native object and is released
        // only when that object is deleted.
        &*slot.as_ptr()
    });

    if tag.is_pager() {
        update_styles(mounted, style, part_styles, font_resource);
        crate::animation::sync(mounted.obj, &mounted.animations, &animations)?;
        mounted.animations = animations;
        if mounted.grid_cell != grid_cell {
            if let Some(cell) = grid_cell {
                widgets::apply_grid_cell(mounted.obj, cell);
            }
            mounted.grid_cell = grid_cell;
        }
        // Active pages are controlled like other interactive values: they
        // are compared with the native widget, which the user may have
        // changed.
        let obj = mounted.obj;
        let pages = mounted.pages.as_deref().unwrap_or(&[]);
        let write_active = match (&mounted.props, &props) {
            #[cfg(feature = "widget-tabview")]
            (Props::Tabview { tabs: old_tabs, .. }, Props::Tabview { tabs, active }) => {
                widgets::set_tabview_titles(obj, old_tabs, tabs);
                // SAFETY: the tabview is live.
                let native = unsafe { rs_lvgl_sys::lv_tabview_get_tab_active(obj.as_ptr()) };
                native != *active
            }
            #[cfg(feature = "widget-tileview")]
            (Props::Tileview { .. }, Props::Tileview { active, .. }) => {
                !widgets::tileview_is_active(obj, pages, *active)
            }
            _ => false,
        };
        if write_active {
            widgets::suppress(slot, || match &props {
                #[cfg(feature = "widget-tabview")]
                Props::Tabview { active, .. } => widgets::set_tabview_active(obj, *active),
                #[cfg(feature = "widget-tileview")]
                Props::Tileview { active, .. } => widgets::set_tileview_active(obj, pages, *active),
                _ => {}
            });
        }
        for (index, (page, child)) in mounted.children.iter_mut().zip(children).enumerate() {
            if let Err(error) = update(page, child, queue, counters, index) {
                mounted.recount();
                return Err(error);
            }
        }
        mounted.props = props;
        mounted.keyboard_id = keyboard_id;
        mounted.recount();
        return Ok(());
    }

    widgets::apply_props(
        mounted.obj,
        mounted.parts,
        &props,
        Some(&mounted.props),
        slot,
        &mut mounted.retained,
    )?;
    #[cfg(feature = "widget-image")]
    if let Some(image) = image_resource {
        mounted.retained = widgets::Retained::LoadedImage(image);
    }

    update_styles(mounted, style, part_styles, font_resource);
    crate::animation::sync(mounted.obj, &mounted.animations, &animations)?;
    mounted.animations = animations;
    let grid_changed = match (&mounted.props, &props) {
        (
            Props::Grid {
                columns: old_columns,
                rows: old_rows,
            },
            Props::Grid { columns, rows },
        ) => old_columns != columns || old_rows != rows,
        _ => false,
    };
    if grid_changed {
        if let Props::Grid { columns, rows } = &props {
            let (columns, rows) = widgets::grid_descriptors(columns, rows);
            widgets::set_grid_descriptors(mounted.obj, &columns, &rows);
            mounted.grid_columns = Some(columns);
            mounted.grid_rows = Some(rows);
        }
    }
    if mounted.grid_cell != grid_cell {
        if let Some(cell) = grid_cell {
            widgets::apply_grid_cell(mounted.obj, cell);
        }
        mounted.grid_cell = grid_cell;
    }

    let first_index = mounted.parts.first_child_index();
    let old_children = core::mem::take(&mut mounted.children);
    let new_children = reconcile_children(
        mounted.parts.children_parent(mounted.obj),
        old_children,
        children,
        queue,
        counters,
        first_index,
    );
    // Failed reconciliation already deleted the moved-out children. Recount
    // the remaining parent before propagating an error to its deletion path.
    mounted.recount();
    let new_children = new_children?;

    mounted.props = props;
    mounted.keyboard_id = keyboard_id;
    mounted.children = new_children;
    mounted.recount();
    Ok(())
}

/// Replace the native object with a freshly created subtree at the same index.
fn recreate<M: Clone + 'static>(
    mounted: &mut MountedNode<M>,
    node: Node<M>,
    queue: &Rc<EventQueue<M>>,
    counters: &mut Counters,
) -> Result<(), UiError> {
    let parent = unsafe {
        // SAFETY: the mounted object is a live child of a valid parent.
        rs_lvgl_sys::lv_obj_get_parent(mounted.obj.as_ptr())
    };
    let Some(parent) = NonNull::new(parent) else {
        return Err(UiError::InvalidProperty);
    };
    let index = unsafe {
        // SAFETY: the mounted object is live.
        rs_lvgl_sys::lv_obj_get_index(mounted.obj.as_ptr())
    };

    let replacement = create_inner(parent, node, queue, counters)?;
    // Preserve the identity position before the old object disappears.
    // SAFETY: both objects are children of `parent`.
    unsafe { rs_lvgl_sys::lv_obj_move_to_index(replacement.obj.as_ptr(), index) };

    let old = core::mem::replace(mounted, replacement);
    delete_subtree(old, counters);
    Ok(())
}
