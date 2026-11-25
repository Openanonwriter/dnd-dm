//! Prints which characters egui's built-in fonts can draw (others show as boxes).
fn main() {
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |_| {});
    let font = egui::FontId::proportional(14.0);
    let arg: String = std::env::args().skip(1).collect();
    for c in arg.chars() {
        let ok = ctx.fonts(|f| f.has_glyph(&font, c));
        println!("{} U+{:04X} {}", if ok { "ok     " } else { "MISSING" }, c as u32, c);
    }
}
