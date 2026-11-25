//! Development aid: with `DND_DM_SHOT=<file.bmp>` set, the app saves a
//! picture of its own window a moment after starting and then closes. Used
//! to check the interface where the screen cannot be captured.

use eframe::egui;

#[derive(Default)]
pub struct Shot {
    path: Option<String>,
    frames: u32,
}

impl Shot {
    pub fn from_env() -> Self {
        Self { path: std::env::var("DND_DM_SHOT").ok().filter(|p| !p.is_empty()), frames: 0 }
    }

    pub fn tick(&mut self, ctx: &egui::Context) {
        let Some(path) = self.path.clone() else { return };
        self.frames += 1;
        ctx.request_repaint();
        if self.frames == 45 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = image {
            let (w, h) = (img.size[0], img.size[1]);
            let row = (w * 3 + 3) / 4 * 4;
            let mut out = Vec::with_capacity(54 + row * h);
            out.extend_from_slice(b"BM");
            out.extend_from_slice(&((54 + row * h) as u32).to_le_bytes());
            out.extend_from_slice(&[0; 4]);
            out.extend_from_slice(&54u32.to_le_bytes());
            out.extend_from_slice(&40u32.to_le_bytes());
            out.extend_from_slice(&(w as i32).to_le_bytes());
            out.extend_from_slice(&(h as i32).to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&24u16.to_le_bytes());
            out.extend_from_slice(&[0; 24]);
            for y in (0..h).rev() {
                for x in 0..w {
                    let c = img.pixels[y * w + x];
                    out.extend_from_slice(&[c.b(), c.g(), c.r()]);
                }
                out.extend(std::iter::repeat(0).take(row - w * 3));
            }
            let _ = std::fs::write(&path, out);
            self.path = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
