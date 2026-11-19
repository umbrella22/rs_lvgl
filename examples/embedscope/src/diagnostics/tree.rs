//! The settings page exports a fixed set of public names, not application text.
use escope_lvgl::TreeSource;
use escope_protocol::{Status, UiNode};
use rs_lvgl::sys;
use std::ptr::NonNull;

pub(super) const BRIGHTNESS: &str = "settings.brightness";
pub(super) const PREVIEW: &str = "settings.preview";
const BRIGHTNESS_INDEX: usize = 6;
const PREVIEW_INDEX: usize = 7;
const NODES: [(&str, &str); 8] = [
    ("screen", ""),
    ("column", "settings"),
    ("label", "settings.title"),
    ("label", "settings.count"),
    ("button", "settings.increment"),
    ("label", "settings.brightness-caption"),
    ("slider", "action:settings.brightness"),
    ("lv_obj", PREVIEW),
];

pub(super) struct SettingsTree {
    screen: NonNull<sys::lv_obj_t>,
    revision: u32,
}

impl SettingsTree {
    /// # Safety
    /// `screen` is this example's live screen, until after the tree is dropped.
    /// Call only at the UI loop's safe points, outside native callbacks.
    pub(super) unsafe fn new(screen: NonNull<sys::lv_obj_t>) -> Self {
        Self {
            screen,
            revision: 1,
        }
    }

    pub(super) fn refresh_layout(&mut self) {
        // SAFETY: the example has completed reconciliation on the UI thread.
        unsafe { sys::lv_obj_update_layout(self.screen.as_ptr()) };
    }

    pub(super) fn changed(&mut self) -> Result<(), Status> {
        self.revision = self.revision.checked_add(1).ok_or(Status::Capacity)?;
        Ok(())
    }

    fn object(&self, index: usize) -> Result<NonNull<sys::lv_obj_t>, Status> {
        let child = |parent, index| {
            // SAFETY: the parent resolves from the active screen in this synchronous read.
            NonNull::new(unsafe { sys::lv_obj_get_child(parent, index) }).ok_or(Status::Stale)
        };
        if index == 0 {
            return Ok(self.screen);
        }
        // Ui owns an intermediate host; the app's column is its only child.
        // The other exported roles are fixed children of that column. This is
        // the example's projection, not a general-purpose Rust View key lookup.
        let host = child(self.screen.as_ptr(), 0)?;
        let column = child(host.as_ptr(), 0)?;
        if index == 1 {
            return Ok(column);
        }
        child(column.as_ptr(), (index - 2) as i32)
    }

    pub(super) fn hits_brightness(&mut self, x: u32, y: u32) -> Result<bool, Status> {
        let node = self.read_node(BRIGHTNESS_INDEX as u32)?;
        let dx = i64::from(x) - i64::from(node.x);
        let dy = i64::from(y) - i64::from(node.y);
        Ok(dx >= 0 && dx < i64::from(node.width) && dy >= 0 && dy < i64::from(node.height))
    }

    pub(super) fn preview_center(&mut self) -> Result<(usize, usize), Status> {
        let node = self.read_node(PREVIEW_INDEX as u32)?;
        let center = |origin, size| {
            usize::try_from(i64::from(origin) + i64::from(size) / 2).map_err(|_| Status::Internal)
        };
        Ok((center(node.x, node.width)?, center(node.y, node.height)?))
    }
}

impl TreeSource for SettingsTree {
    fn revision(&self) -> u32 {
        self.revision
    }

    fn read_node(&mut self, cursor: u32) -> Result<UiNode<'_>, Status> {
        let index = cursor as usize;
        let &(kind, name) = NODES.get(index).ok_or(Status::InvalidArgument)?;
        let obj = self.object(index)?;
        // SAFETY: the object was resolved on the LVGL thread during this read.
        let (area, flags) = unsafe {
            let mut area = std::mem::zeroed();
            sys::lv_obj_get_coords(obj.as_ptr(), &mut area);
            (area, sys::lv_obj_get_state(obj.as_ptr()))
        };
        let extent = |min, max| {
            u32::try_from(i64::from(max) - i64::from(min) + 1).map_err(|_| Status::Internal)
        };
        Ok(UiNode {
            id: cursor + 1,
            parent: cursor.min(2),
            kind,
            name,
            x: area.x1,
            y: area.y1,
            width: extent(area.x1, area.x2)?,
            height: extent(area.y1, area.y2)?,
            flags,
            revision: self.revision,
            next_cursor: (index + 1 < NODES.len()).then_some(cursor + 1),
        })
    }
}
