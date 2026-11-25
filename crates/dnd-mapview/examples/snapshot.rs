//! Draws a saved campaign's map to a picture without opening a window, to
//! check how it looks (a tiny software rasteriser for egui's triangles).
//!
//! ```text
//! cargo run --release -p dnd-mapview --example snapshot -- content saves/my_campaign out.bmp dm
//! cargo run --release -p dnd-mapview --example snapshot -- content saves/my_campaign out.bmp party 12.5 8 64
//! ```
//! Arguments: content folder, campaign folder, output `.bmp`, viewer (`dm`,
//! `party` or a character id), then optionally centre x, centre y, zoom and the id of another map to show.

use dnd_core::view::Viewer;
use dnd_core::World;
use dnd_mapview::{Anim, Camera, DrawOpts};
use egui::{pos2, vec2, Rect};
use std::path::PathBuf;

const W: usize = 1500;
const H: usize = 1000;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize, d: &str| a.get(i).cloned().unwrap_or_else(|| d.to_string());
    let mut world = World::new(vec![PathBuf::from(arg(0, "content"))]);
    world.campaign = dnd_core::save::load_campaign(&PathBuf::from(arg(1, "saves/demo"))).expect("campaign");
    if let Some(id) = a.get(7) {
        let rules = world.library.rules.clone();
        world.campaign.switch_map(id, |s| rules.size_squares(s));
    }
    world.update_explored();
    let who = arg(3, "dm");
    let viewer = match who.as_str() {
        "dm" => Viewer::Dm,
        "party" => Viewer::Party,
        id => Viewer::Character(id.to_string()),
    };
    let view = world.map_view(&viewer);
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(W as f32, H as f32));
    let mut cam = Camera::default();
    cam.fit(rect, view.width, view.height);
    if let (Some(x), Some(y), Some(z)) = (a.get(4).and_then(|s| s.parse().ok()), a.get(5).and_then(|s| s.parse().ok()), a.get(6).and_then(|s| s.parse().ok())) {
        cam.center = vec2(x, y);
        cam.zoom = z;
    }
    let mut anim = Anim::default();

    let ctx = egui::Context::default();
    let mut font: Option<(usize, usize, Vec<f32>)> = None;
    let mut shapes = Vec::new();
    let mut ppp = 1.0;
    for frame in 0..2 {
        let input = egui::RawInput { screen_rect: Some(rect), time: Some(3.7 + frame as f64), ..Default::default() };
        let out = ctx.run(input, |ctx| {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());
                let opts = DrawOpts { grid: true, solid_fog: who != "dm", ..Default::default() };
                dnd_mapview::draw(&painter, resp.rect, &cam, &view, &mut anim, 3.7, &opts);
            });
        });
        for (_, delta) in out.textures_delta.set {
            if let (egui::ImageData::Font(f), None) = (&delta.image, delta.pos) {
                font = Some((f.size[0], f.size[1], f.pixels.clone()));
            }
        }
        shapes = out.shapes;
        ppp = out.pixels_per_point;
    }
    let (fw, fh, fpx) = font.expect("font atlas");
    let coverage = |u: f32, v: f32| -> f32 {
        let x = ((u * fw as f32) as usize).min(fw - 1);
        let y = ((v * fh as f32) as usize).min(fh - 1);
        fpx[y * fw + x]
    };

    let mut fb = vec![[0f32; 3]; W * H];
    for prim in ctx.tessellate(shapes, ppp) {
        let egui::epaint::Primitive::Mesh(mesh) = prim.primitive else { continue };
        let clip = prim.clip_rect;
        for tri in mesh.indices.chunks_exact(3) {
            let v = [&mesh.vertices[tri[0] as usize], &mesh.vertices[tri[1] as usize], &mesh.vertices[tri[2] as usize]];
            let (p0, p1, p2) = (v[0].pos, v[1].pos, v[2].pos);
            let area = (p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y);
            if area.abs() < 1e-6 {
                continue;
            }
            let x_lo = p0.x.min(p1.x).min(p2.x).max(clip.min.x).max(0.0).floor() as usize;
            let x_hi = (p0.x.max(p1.x).max(p2.x).min(clip.max.x).min(W as f32 - 1.0).ceil() as usize).min(W - 1);
            let y_lo = p0.y.min(p1.y).min(p2.y).max(clip.min.y).max(0.0).floor() as usize;
            let y_hi = (p0.y.max(p1.y).max(p2.y).min(clip.max.y).min(H as f32 - 1.0).ceil() as usize).min(H - 1);
            for y in y_lo..=y_hi {
                for x in x_lo..=x_hi {
                    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                    let w0 = ((p1.x - px) * (p2.y - py) - (p2.x - px) * (p1.y - py)) / area;
                    let w1 = ((p2.x - px) * (p0.y - py) - (p0.x - px) * (p2.y - py)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let mix = |f: fn(&egui::Color32) -> u8| (w0 * f(&v[0].color) as f32 + w1 * f(&v[1].color) as f32 + w2 * f(&v[2].color) as f32) / 255.0;
                    let cov = coverage(w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x, w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y);
                    let (r, g, b, al) = (mix(|c| c.r()) * cov, mix(|c| c.g()) * cov, mix(|c| c.b()) * cov, mix(|c| c.a()) * cov);
                    let d = &mut fb[y * W + x];
                    *d = [r + d[0] * (1.0 - al), g + d[1] * (1.0 - al), b + d[2] * (1.0 - al)];
                }
            }
        }
    }

    // 24-bit BMP, bottom row first
    let row = (W * 3 + 3) / 4 * 4;
    let mut out = Vec::with_capacity(54 + row * H);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((54 + row * H) as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(W as i32).to_le_bytes());
    out.extend_from_slice(&(H as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for y in (0..H).rev() {
        for x in 0..W {
            let p = fb[y * W + x];
            let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u8;
            out.extend_from_slice(&[c(p[2]), c(p[1]), c(p[0])]);
        }
        out.extend(std::iter::repeat(0).take(row - W * 3));
    }
    let path = arg(2, "map.bmp");
    std::fs::write(&path, out).expect("write");
    println!("wrote {path}");
}
