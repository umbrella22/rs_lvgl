use super::super::{HEIGHT, WIDTH};

/// The actual full memory display, assembled from LVGL's RGB888 flushes.
pub struct Frame {
    pub pixels: Vec<u8>,
    pub id: u64,
    dirty: bool,
}

impl Frame {
    pub fn new() -> Self {
        Self {
            pixels: vec![0; WIDTH as usize * HEIGHT as usize * 3],
            id: 0,
            dirty: false,
        }
    }

    pub fn flush<const N: usize>(&mut self, refresh: &rs_lvgl::DisplayRefresh<N>) {
        let a = &refresh.area;
        let row = (a.x2 - a.x1 + 1) as usize;
        for y in a.y1..=a.y2 {
            for x in a.x1..=a.x2 {
                let color = refresh.colors[(y - a.y1) as usize * row + (x - a.x1) as usize];
                let at = (y as usize * WIDTH as usize + x as usize) * 3;
                self.pixels[at..at + 3].copy_from_slice(&[color.r(), color.g(), color.b()]);
            }
        }
        self.dirty = true;
    }

    pub fn finish(&mut self) {
        if std::mem::take(&mut self.dirty) {
            self.id = self.id.checked_add(1).expect("frame sequence exhausted");
        }
    }
}
