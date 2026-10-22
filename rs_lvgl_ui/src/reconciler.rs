use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::NonNull;

use rs_lvgl::{Obj, Widget};
use rs_lvgl_sys::lv_obj_t;

use crate::custom::{CustomObject, CustomWidgetKind};
use crate::events::{EventQueue, HandlerSlot};
use crate::node::{GridCell, HandlerSet, Key, Node, Part, Selector, State, WidgetKind};
use crate::theme::Style;
use crate::widgets::{self, KindTag, Parts, Props};
use crate::UiError;

#[derive(Default)]
pub(crate) struct Counters {
    pub(crate) created: u64,
    pub(crate) deleted: u64,
    pub(crate) moved: u64,
    /// Keyboard → textarea bindings applied to native keyboards.
    ///
    /// LVGL's keyboard does not observe its textarea's deletion, so every
    /// subtree deletion unbinds keyboards whose textarea disappears before
    /// the native objects are freed.
    pub(crate) keyboards: Vec<(NonNull<lv_obj_t>, NonNull<lv_obj_t>)>,
}

/// The lifecycle state of one mounted custom widget.
pub(crate) struct MountedCustom {
    entry: CustomWidgetKind,
    object: CustomObject,
}

impl MountedCustom {
    fn update(&mut self, elapsed_ms: u32) -> Result<(), UiError> {
        self.entry.update(&mut self.object, elapsed_ms)
    }

    fn delete(mut self) {
        self.entry.delete(&mut self.object);
    }
}

pub(crate) struct MountedNode<M: Clone + 'static> {
    pub(crate) obj: NonNull<lv_obj_t>,
    pub(crate) key: Key,
    tag: KindTag,
    props: Props,
    style: Style,
    transitions: Vec<crate::animation::NativeTransition>,
    animations: Vec<crate::Animation>,
    slot: Option<NonNull<HandlerSlot<M>>>,
    parts: Parts,
    grid_cell: Option<GridCell>,
    part_styles: Vec<(Selector, Style)>,
    keyboard_id: Option<u64>,
    clickable: Option<bool>,
    scrollable: Option<bool>,
    pub(crate) interaction: crate::scroll::Interaction,
    #[cfg(feature = "input-keypad")]
    pub(crate) focusable: Option<bool>,
    #[cfg(feature = "input-keypad")]
    pub(crate) focus_id: u64,
    #[cfg(feature = "input-keypad")]
    pub(crate) modal: bool,
    #[cfg(feature = "input-gridnav")]
    pub(crate) grid_navigation: Option<crate::GridNavigation>,
    #[cfg(feature = "input-gridnav")]
    pub(crate) grid_state: Option<crate::gridnav::GridState>,
    #[cfg(feature = "input-gridnav")]
    pub(crate) grid_child_focusable: Option<bool>,
    grid_columns: Option<Box<[i32]>>,
    grid_rows: Option<Box<[i32]>>,
    retained: widgets::Retained,
    /// Page roots of a tabview or tileview, one per child.
    pages: Option<Vec<NonNull<lv_obj_t>>>,
    custom: Option<MountedCustom>,
    pub(crate) children: Vec<MountedNode<M>>,
    /// Number of app-level nodes in this subtree, excluding implicit labels.
    pub(crate) count: usize,
    /// Number of lifecycle custom widgets in this subtree; `tick` skips
    /// subtrees without any.
    pub(crate) customs: usize,
}

impl<M: Clone + 'static> MountedNode<M> {
    #[cfg(feature = "input-keypad")]
    pub(crate) fn focus_object(&self) -> NonNull<lv_obj_t> {
        #[cfg(feature = "widget-ime-pinyin")]
        if let Some(input) = self.parts.input {
            return input;
        }
        self.obj
    }
    fn recount(&mut self) {
        self.count = 1 + self.children.iter().map(|child| child.count).sum::<usize>();
        let own = self
            .custom
            .as_ref()
            .map(|custom| usize::from(custom.entry.is_lifecycle()))
            .unwrap_or(0);
        self.customs = own
            + self
                .children
                .iter()
                .map(|child| child.customs)
                .sum::<usize>();
    }

    /// Call every custom widget's delete hook exactly once, while the native
    /// objects are still alive.
    ///
    /// This must run before `lv_obj_delete` on any object that owns this
    /// subtree: LVGL's deletion would otherwise invalidate the handles passed
    /// to the hooks. Taking the `MountedCustom` out makes a second call a no-op.
    pub(crate) fn run_delete_hooks(&mut self) {
        if let Some(custom) = self.custom.take() {
            custom.delete();
        }
        for child in &mut self.children {
            child.run_delete_hooks();
        }
    }

    /// Advance every custom widget in this subtree once, pre-order.
    pub(crate) fn tick(&mut self, elapsed_ms: u32) -> Result<(), UiError> {
        if self.customs == 0 {
            return Ok(());
        }
        if let Some(custom) = &mut self.custom {
            custom.update(elapsed_ms)?;
        }
        for child in &mut self.children {
            child.tick(elapsed_ms)?;
        }
        Ok(())
    }
}

impl<M: Clone + 'static> Drop for MountedNode<M> {
    fn drop(&mut self) {
        // Safety net for early-return paths that drop a partially reconciled
        // subtree (for example an error in a later sibling). Native objects are
        // still alive whenever a `MountedCustom` reaches this point; every
        // explicit deletion path takes the hook first.
        if let Some(custom) = self.custom.take() {
            custom.delete();
        }
        // Children drop recursively through this same implementation.
    }
}

/// Delete a native object whose mount failed after it was created.
fn abort_create(custom: Option<MountedCustom>, obj: NonNull<lv_obj_t>) {
    if let Some(custom) = custom {
        custom.delete();
    }
    // SAFETY: the object was created by the failing `create` call, the custom
    // delete hook (if any) already ran, and no mounted handle refers to it.
    unsafe { rs_lvgl_sys::lv_obj_delete(obj.as_ptr()) };
}

/// Apply explicit clickable/scrollable flags that changed.
fn apply_flags(
    obj: NonNull<lv_obj_t>,
    clickable: Option<bool>,
    scrollable: Option<bool>,
    old_clickable: Option<bool>,
    old_scrollable: Option<bool>,
) {
    // LVGL 9.6 deprecates the generic flag functions in favour of dedicated
    // setters.
    let setters: [(
        Option<bool>,
        Option<bool>,
        unsafe extern "C" fn(*mut lv_obj_t, bool),
    ); 2] = [
        (clickable, old_clickable, rs_lvgl_sys::lv_obj_set_clickable),
        (
            scrollable,
            old_scrollable,
            rs_lvgl_sys::lv_obj_set_scrollable,
        ),
    ];
    for (value, old, set) in setters {
        let Some(value) = value else {
            continue;
        };
        if old == Some(value) {
            continue;
        }
        // SAFETY: `obj` is a live object owned by the mounted tree.
        unsafe { set(obj.as_ptr(), value) };
    }
}

/// Write a controlled checked state that differs from the native object.
fn apply_checked(obj: NonNull<lv_obj_t>, tag: KindTag, checked: Option<bool>) {
    let Some(checked) = checked else {
        return;
    };
    if tag.owns_checked() {
        return;
    }
    let state = rs_lvgl_sys::lv_state_t_LV_STATE_CHECKED;
    // SAFETY: `obj` is a live object owned by the mounted tree. Changing a
    // state sends no value event.
    unsafe {
        if rs_lvgl_sys::lv_obj_has_state(obj.as_ptr(), state) != checked {
            if checked {
                rs_lvgl_sys::lv_obj_add_state(obj.as_ptr(), state);
            } else {
                rs_lvgl_sys::lv_obj_remove_state(obj.as_ptr(), state);
            }
        }
    }
}

/// Install `handlers` on a mounted object, allocating the slot lazily.
fn sync_handlers<M: Clone + 'static>(
    slot: &mut Option<NonNull<HandlerSlot<M>>>,
    obj: NonNull<lv_obj_t>,
    tag: KindTag,
    handlers: HandlerSet<M>,
    queue: &Rc<EventQueue<M>>,
) -> Result<(), UiError> {
    match slot {
        Some(existing) => {
            // SAFETY: the slot is owned by the live native object and is
            // released only when that object is deleted.
            unsafe { existing.as_ref() }.set_handlers(obj, handlers)
        }
        None if handlers.is_empty() => Ok(()),
        None => {
            // SAFETY: `obj` is live and has no slot yet. On error the slot is
            // owned by the object's delete callback or already released.
            let attached = unsafe {
                HandlerSlot::attach(
                    obj,
                    queue.clone(),
                    widgets::value_source(tag),
                    widgets::event_inner_target(tag, obj),
                    handlers,
                )
            }?;
            *slot = Some(attached);
            Ok(())
        }
    }
}

pub(crate) fn create<M: Clone + 'static>(
    parent: NonNull<lv_obj_t>,
    node: Node<M>,
    queue: &Rc<EventQueue<M>>,
    counters: &mut Counters,
) -> Result<MountedNode<M>, UiError> {
    let Node {
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
    } = node;

    let tag = widgets::tag(&kind);
    let (props, entry) = widgets::into_props(kind);

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
    if tag != KindTag::Custom {
        widgets::apply_style(obj, &style, tag);
        for (selector, part_style) in &part_styles {
            widgets::apply_style_for_selector(obj, part_style, tag, *selector);
        }
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
            if let Some(slot) = slot_ref {
                slot.suppress();
            }
            let pages = widgets::tabview_pages(obj, tabs, *active);
            if let Some(slot) = slot_ref {
                slot.clear_suppress();
            }
            Some(pages)
        }
        #[cfg(feature = "widget-tileview")]
        Props::Tileview { tiles, active } => {
            if let Some(slot) = slot_ref {
                slot.suppress();
            }
            let pages = widgets::tileview_pages(obj, tiles, *active);
            if let Some(slot) = slot_ref {
                slot.clear_suppress();
            }
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
        match create(parent, child, queue, counters) {
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

/// Re-apply style and part styles when either changed.
fn update_styles<M: Clone + 'static>(
    mounted: &mut MountedNode<M>,
    style: Style,
    part_styles: Vec<(Selector, Style)>,
) {
    let tag = mounted.tag;
    if tag == KindTag::Custom {
        mounted.style = style;
        mounted.part_styles = part_styles;
        return;
    }
    let part_styles_changed = part_styles != mounted.part_styles;
    let styles_changed = style != mounted.style || part_styles_changed;
    if styles_changed {
        crate::animation::clear_transitions(mounted.obj, &mut mounted.transitions);
        widgets::apply_style(mounted.obj, &style, tag);
    }
    if part_styles_changed {
        for part in [
            Part::Scrollbar,
            Part::Indicator,
            Part::Knob,
            Part::Selected,
            Part::Items,
            Part::Cursor,
        ] {
            widgets::clear_selector_style(mounted.obj, Selector::new(part, State::DEFAULT));
        }
        // State styles live on their own selectors, including the main part.
        for (selector, _) in &mounted.part_styles {
            if selector.state != State::DEFAULT {
                widgets::clear_selector_style(mounted.obj, *selector);
            }
        }
        for (selector, part_style) in &part_styles {
            widgets::apply_style_for_selector(mounted.obj, part_style, tag, *selector);
        }
        mounted.part_styles = part_styles;
    }
    if styles_changed {
        mounted.transitions =
            crate::animation::install_transitions(mounted.obj, &style, &mounted.part_styles);
    }
    mounted.style = style;
}

pub(crate) fn update<M: Clone + 'static>(
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
        key: _,
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
    } = node;
    // A mounted lifecycle widget keeps its instance; the new descriptor's
    // factory is dropped here.
    let (props, _entry) = widgets::into_props(kind);

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
        update_styles(mounted, style, part_styles);
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
            if let Some(slot) = slot {
                slot.suppress();
            }
            match &props {
                #[cfg(feature = "widget-tabview")]
                Props::Tabview { active, .. } => widgets::set_tabview_active(obj, *active),
                #[cfg(feature = "widget-tileview")]
                Props::Tileview { active, .. } => widgets::set_tileview_active(obj, pages, *active),
                _ => {}
            }
            if let Some(slot) = slot {
                slot.clear_suppress();
            }
        }
        for (page, child) in mounted.children.iter_mut().zip(children) {
            if let Err(error) = update(page, child, queue, counters) {
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
    update_styles(mounted, style, part_styles);
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

    let replacement = create(parent, node, queue, counters)?;
    // Preserve the identity position before the old object disappears.
    // SAFETY: both objects are children of `parent`.
    unsafe { rs_lvgl_sys::lv_obj_move_to_index(replacement.obj.as_ptr(), index) };

    let old = core::mem::replace(mounted, replacement);
    delete_subtree(old, counters);
    Ok(())
}

/// Ordered form of an explicit key used for child matching.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum MatchKey {
    Named(&'static str),
    Id(u64),
}

impl MatchKey {
    fn of(key: Key) -> Option<Self> {
        match key {
            Key::Auto => None,
            Key::Named(name) => Some(Self::Named(name)),
            Key::Id(id) => Some(Self::Id(id)),
        }
    }
}

fn reconcile_children<M: Clone + 'static>(
    parent: NonNull<lv_obj_t>,
    old_children: Vec<MountedNode<M>>,
    new_children: Vec<Node<M>>,
    queue: &Rc<EventQueue<M>>,
    counters: &mut Counters,
    first_index: i32,
) -> Result<Vec<MountedNode<M>>, UiError> {
    // Explicit keys are looked up in O(log n); auto-keyed children match the
    // next unused auto-keyed child in order.
    let mut keyed: BTreeMap<MatchKey, usize> = BTreeMap::new();
    for (index, child) in old_children.iter().enumerate() {
        if let Some(key) = MatchKey::of(child.key) {
            keyed.insert(key, index);
        }
    }
    let mut available: Vec<Option<MountedNode<M>>> = old_children.into_iter().map(Some).collect();
    let mut result = Vec::with_capacity(new_children.len());
    let mut auto_cursor = 0usize;

    for child in new_children {
        let matched = match MatchKey::of(child.key) {
            None => {
                let mut found = None;
                for (index, candidate) in available.iter().enumerate().skip(auto_cursor) {
                    if candidate
                        .as_ref()
                        .map(|candidate| candidate.key == Key::Auto)
                        .unwrap_or(false)
                    {
                        found = Some(index);
                        break;
                    }
                }
                if let Some(index) = found {
                    auto_cursor = index + 1;
                }
                found
            }
            Some(key) => keyed.remove(&key),
        };

        let next = match matched {
            Some(index) => {
                let mut existing = available[index]
                    .take()
                    .expect("matched mounted child is present");
                match update(&mut existing, child, queue, counters) {
                    Ok(()) => Ok(existing),
                    Err(error) => {
                        delete_subtree(existing, counters);
                        Err(error)
                    }
                }
            }
            None => create(parent, child, queue, counters),
        };
        match next {
            Ok(node) => result.push(node),
            Err(error) => {
                // These nodes have been moved out of the parent's Rust tree.
                // Delete their native objects before releasing retained buffers;
                // the caller will then delete the now-empty parent on failure.
                for node in result.into_iter().chain(available.into_iter().flatten()) {
                    delete_subtree(node, counters);
                }
                return Err(error);
            }
        }
    }

    for child in available.into_iter().flatten() {
        delete_subtree(child, counters);
    }

    reorder(&result, first_index, counters);
    Ok(result)
}

pub(crate) fn delete_subtree<M: Clone + 'static>(
    mut mounted: MountedNode<M>,
    counters: &mut Counters,
) {
    release_keyboards(&mounted, counters);
    // Custom delete hooks run before LVGL invalidates the native objects.
    mounted.run_delete_hooks();
    let count = mounted.count;
    // SAFETY: the object is live and owned by the mounted tree. Deleting it
    // also deletes its children and fires their delete callbacks exactly once.
    unsafe { rs_lvgl_sys::lv_obj_delete(mounted.obj.as_ptr()) };
    counters.deleted += count as u64;
}

/// Move only the children that are not part of a longest increasing
/// subsequence of their current positions. This preserves object identity and
/// touches the minimum number of objects for the new order.
fn reorder<M: Clone + 'static>(
    children: &[MountedNode<M>],
    first_index: i32,
    counters: &mut Counters,
) {
    if children.len() < 2 {
        return;
    }
    let mut current = Vec::with_capacity(children.len());
    for child in children {
        // SAFETY: every mounted object is live and parented.
        let index = unsafe { rs_lvgl_sys::lv_obj_get_index(child.obj.as_ptr()) };
        current.push(index - first_index);
    }
    let keep = lis_keep(&current);
    for (index, child) in children.iter().enumerate() {
        if !keep[index] {
            // SAFETY: the object is live and `index` is a valid position among
            // the explicit children of the parent.
            unsafe {
                rs_lvgl_sys::lv_obj_move_to_index(child.obj.as_ptr(), index as i32 + first_index)
            };
            counters.moved += 1;
        }
    }
}

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

/// Indices kept in place: a longest strictly increasing subsequence of
/// `values`, computed in O(n log n) with patience sorting.
fn lis_keep(values: &[i32]) -> Vec<bool> {
    let mut keep = vec![false; values.len()];
    if values.is_empty() {
        return keep;
    }
    // `tails[k]` is the index of the smallest tail of an increasing
    // subsequence of length `k + 1`.
    let mut tails: Vec<usize> = Vec::with_capacity(values.len());
    let mut previous = vec![usize::MAX; values.len()];
    for (i, value) in values.iter().enumerate() {
        let position = tails.partition_point(|&tail| values[tail] < *value);
        if position > 0 {
            previous[i] = tails[position - 1];
        }
        if position == tails.len() {
            tails.push(i);
        } else {
            tails[position] = i;
        }
    }
    let mut index = *tails.last().expect("non-empty input has a tail");
    loop {
        keep[index] = true;
        if previous[index] == usize::MAX {
            break;
        }
        index = previous[index];
    }
    keep
}

#[cfg(test)]
mod tests {
    use super::lis_keep;

    #[test]
    fn lis_marks_a_permutation_minimal_move_set() {
        // Move the first item to the end: only one element leaves the LIS.
        assert_eq!(
            lis_keep(&[1, 2, 3, 4, 0]),
            vec![true, true, true, true, false]
        );
        // Move the last item to the front: only one element leaves the LIS.
        assert_eq!(
            lis_keep(&[4, 0, 1, 2, 3]),
            vec![false, true, true, true, true]
        );
        // Swap the two ends: two elements leave the LIS.
        let keep = lis_keep(&[4, 1, 2, 3, 0]);
        assert_eq!(keep.iter().filter(|kept| !**kept).count(), 2);
    }
}
