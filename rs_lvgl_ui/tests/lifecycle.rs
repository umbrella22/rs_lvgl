mod common;

use common::{init, lock, screen};
use rs_lvgl_ui::{App, Key, Node, Theme, Ui};

struct LifecycleApp {
    items: Vec<u64>,
}

impl App for LifecycleApp {
    type Msg = ();

    fn update(&mut self, _: ()) {}

    fn view(&self, _: &Theme) -> Node<()> {
        Node::column().children(
            self.items
                .iter()
                .map(|id| Node::button(format!("item {id}")).key(Key::Id(*id))),
        )
    }
}

#[test]
fn thousand_mount_update_unmount_cycles_do_not_leak_objects() {
    let _guard = lock();
    let display = init();
    let mut ui = Ui::new(&display, common::theme()).unwrap();
    let app = LifecycleApp {
        items: (0..10).collect(),
    };

    for _ in 0..1_000 {
        ui.mount(&app).unwrap();
        ui.update(&app).unwrap();
        ui.unmount();
        assert_eq!(
            unsafe { rs_lvgl_sys::lv_obj_get_child_count(screen().as_ptr()) },
            0,
            "unmount must delete the root container"
        );
    }

    let stats = ui.stats();
    assert_eq!(stats.mounted_nodes, 0);
    assert_eq!(stats.created_nodes, stats.deleted_nodes);
    assert_eq!(stats.created_nodes, 11_000);
}
