//! The text face both apps use: Adwaita Sans (the Inter family, SIL Open
//! Font License; the font and its licence are in `assets/fonts`). egui's own
//! fonts stay behind it for symbols and emoji.

const FACE: &[u8] = include_bytes!("../assets/fonts/AdwaitaSans-Regular.ttf");

/// Make it the first proportional font of this egui context.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("face".into(), std::sync::Arc::new(egui::FontData::from_static(FACE)));
    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "face".into());
    ctx.set_fonts(fonts);
}
