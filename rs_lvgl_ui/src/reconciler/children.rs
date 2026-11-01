//! Key matching, stable-order reuse and minimal native moves.

use super::{create, delete_subtree, update, Counters, MountedNode};
use crate::events::EventQueue;
use crate::node::{Key, Node};
use crate::UiError;
use alloc::{collections::BTreeMap, rc::Rc, vec, vec::Vec};
use core::ptr::NonNull;
use rs_lvgl_sys::lv_obj_t;

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

pub(super) fn reconcile_children<M: Clone + 'static>(
    parent: NonNull<lv_obj_t>,
    mut old_children: Vec<MountedNode<M>>,
    new_children: Vec<Node<M>>,
    queue: &Rc<EventQueue<M>>,
    counters: &mut Counters,
    first_index: i32,
) -> Result<Vec<MountedNode<M>>, UiError> {
    // A stable key sequence in native order needs neither a match map nor a
    // reorder plan. Reuse the mounted vector, including for empty containers.
    // Check native positions too: raw/custom code may have moved the objects.
    let stable_order = old_children.len() == new_children.len()
        && old_children
            .iter()
            .zip(&new_children)
            .enumerate()
            .all(|(index, (old, new))| {
                old.key == new.key
                // SAFETY: mounted children are live native siblings.
                && unsafe { rs_lvgl_sys::lv_obj_get_index(old.obj.as_ptr()) }
                    == first_index + index as i32
            });
    if stable_order {
        for (index, child) in new_children.into_iter().enumerate() {
            if let Err(error) = update(&mut old_children[index], child, queue, counters, index) {
                // The parent has moved these children out. Every native object
                // must be retired before their retained descriptors are dropped.
                for old in old_children {
                    delete_subtree(old, counters);
                }
                return Err(error);
            }
        }
        return Ok(old_children);
    }

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

    for (index, child) in new_children.into_iter().enumerate() {
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
            Some(old_index) => {
                let mut existing = available[old_index]
                    .take()
                    .expect("matched mounted child is present");
                match update(&mut existing, child, queue, counters, index) {
                    Ok(()) => Ok(existing),
                    Err(error) => {
                        delete_subtree(existing, counters);
                        Err(error)
                    }
                }
            }
            None => create(parent, child, queue, counters, index),
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
