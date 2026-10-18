extern crate egui;
fn aspect_fit(available: egui::Rect, pixels: [u32; 2]) -> egui::Rect {
    let source = egui::vec2(pixels[0] as f32, pixels[1] as f32);
    let scale = (available.width() / source.x).min(available.height() / source.y);
    egui::Rect::from_center_size(available.center(), source * scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn window_resize_preserves_the_actual_native_target_aspect() {
        let wide = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(1600.0, 900.0));
        let fit = aspect_fit(wide, [800, 600]);
        assert_eq!(fit.size(), egui::vec2(1200.0, 900.0));
        assert_eq!(fit.center(), wide.center());
        let tall = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 1000.0));
        assert_eq!(aspect_fit(tall, [1600, 900]).size(), egui::vec2(600.0, 337.5));
    }
}
