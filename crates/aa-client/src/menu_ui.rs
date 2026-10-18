//! Typeface setup shared by the shell and the asset viewer.
//!
//! The front-end screens themselves are drawn from the TGI port
//! (`tgi_frontend.rs`, `aa-ui`); the hand-made menu kit that stood in for
//! them before round 2 is gone.

use eframe::egui;

/// Install the original typeface (Data/Fonts/TrueType/A&A.ttf) as the first
/// proportional font; egui's own fonts stay behind it for missing glyphs.
pub fn install_font(ctx: &egui::Context, root: &std::path::Path) {
    let path = root.join("assets/original/Data/Fonts/TrueType/A&A.ttf");
    let Ok(bytes) = std::fs::read(&path) else {
        log::warn!("Original font not found at {}", path.display());
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("aa".into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
    if let Some(list) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        list.insert(0, "aa".into());
    }
    ctx.set_fonts(fonts);
}
