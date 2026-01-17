use screen_overlay::egui::{Color32, pos2};
use screen_overlay::{Drawable, Overlay, OverlayConfig, OverlayHandle, PositionedElements};
pub fn main() -> std::result::Result<(), eframe::Error> {
    let config = OverlayConfig::new().with_size([1920.0, 1080.0]);
    let overlay = Overlay::new(config.clone());
    let overlay = OverlayHandle::new(overlay);

    let cpos = pos2(1000.0, 500.0); // crosshair pos, but then short.
    let len = egui::vec2(15.0, 0.0);
    let color = Color32::ORANGE;
    let _crosshair = overlay.add_drawable(Drawable::CentralElement(
        PositionedElements::new()
            .fixed_pos(egui::pos2(0.0, 0.0))
            .default_size(config.size)
            // .debug_color()
            .paint(vec![
                egui::Shape::Circle(egui::epaint::CircleShape {
                    center: cpos,
                    radius: 10.0,
                    fill: Color32::TRANSPARENT,
                    stroke: egui::Stroke::new(1.5, color),
                }),
                egui::Shape::LineSegment {
                    points: [cpos - len, cpos + len],
                    stroke: egui::Stroke::new(2.0, color),
                },
                egui::Shape::LineSegment {
                    points: [cpos - len.rot90(), cpos + len.rot90()],
                    stroke: egui::Stroke::new(2.0, color),
                },
            ]),
    ));
    eframe::run_native(
        "Image Viewer",
        overlay.native_options(),
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(overlay))
        }),
    )
}
