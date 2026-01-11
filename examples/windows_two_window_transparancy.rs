use egui::{self, Color32};
use screen_overlay::{
    Drawable, Overlay, OverlayConfig, PositionedElements, fullscreen_overlay_configure,
    fullscreen_overlay_native_options, main_test,
};

const OVERLAY_IN_DEFERED: bool = false;
struct TestOverlayApp {
    config: OverlayConfig,
}

fn normal_window_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("normal")
            .with_inner_size([200.0, 100.0])
            .with_position([300.0, 300.0]),
        ..Default::default()
    }
}
fn overlay_window_options(config: &OverlayConfig) -> eframe::NativeOptions {
    fullscreen_overlay_native_options(&config)
}

impl eframe::App for TestOverlayApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let circle: Drawable = PositionedElements::new()
            .fixed_pos(egui::pos2(0.0, 0.0))
            .default_size(self.config.size)
            // .debug_color()
            .paint(vec![egui::Shape::Circle(egui::epaint::CircleShape {
                center: egui::Pos2 { x: 50.0, y: 50.0 },
                radius: 10.0,
                fill: Color32::TRANSPARENT,
                stroke: egui::Stroke::new(1.5, Color32::RED),
            })])
            .into();
        let config = self.config;

        if OVERLAY_IN_DEFERED {
            ui.ctx().show_viewport_deferred(
                egui::ViewportId::from_hash_of("deferred_viewport"),
                overlay_window_options(&self.config).viewport,
                move |ui, _class| {
                    fullscreen_overlay_configure(ui, &config);
                    circle.draw(ui);
                },
            );
            ui.label("hello!");
        } else {
            ui.ctx().show_viewport_deferred(
                egui::ViewportId::from_hash_of("deferred_viewport"),
                normal_window_options().viewport,
                move |ui, _class| {
                    ui.label("hello!");
                },
            );
            fullscreen_overlay_configure(ui, &config);
            circle.draw(ui);
        }
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}
pub fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let config = OverlayConfig::new().with_size([100.0, 100.0]);
    eframe::run_native(
        "Image Viewer",
        if OVERLAY_IN_DEFERED {
            normal_window_options()
        } else {
            overlay_window_options(&config)
        },
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(TestOverlayApp { config }))
        }),
    )
}
