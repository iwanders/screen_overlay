/// The legacy system, using dxgi on linux, glfw + x11 + three_d on linux. The new system uses egui/eframe for a full
/// screen overlay.
pub mod legacy;

use eframe::egui::{self, Color32, ViewportCommand};

#[derive(Debug, Clone, Copy)]
pub struct OverlayConfig {
    pub width: u32,
    pub height: u32,
}

#[cfg(target_os = "linux")]
fn fullscreen_overlay_native_options(config: &OverlayConfig) -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([config.width as f32, config.height as f32]) // Must be repeated in viewport configure.
            // .with_resizable(false)
            // .with_fullscreen(true)
            // .with_maximized(true)
            .with_transparent(true) // This one must be here
            .with_mouse_passthrough(true) // Must be repeated in viewport configure.
            .with_always_on_top() // Must be repeated in viewport configure.
            // .with_decorations(false)
            // .with_titlebar_shown(false)
            .with_override_redirect(true) // Must be here to bypass the window manager, but not the compositor.
            .with_window_type(egui::X11WindowType::Utility),
        ..Default::default()
    }
}
#[cfg(target_os = "linux")]
fn fullscreen_overlay_configure(ctx: &egui::Context, config: &OverlayConfig) {
    ctx.set_pixels_per_point(1.0); // Can we do this, or does this affect the other window?
    ctx.send_viewport_cmd(ViewportCommand::InnerSize(
        (config.width as f32, config.height as f32).into(),
    ));
    ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(true));
    ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
}

use parking_lot::RwLock;

pub trait PositionedWidget: std::fmt::Debug {
    fn area(&self) -> egui::Area;
    fn widget(&self) -> Vec<Box<dyn egui::Widget>>;
}

/*
 containers have Container.show(ui, |ui|{}), but they do consume Container.
 widgets have:  fn ui(self, ui: &mut Ui) -> Response;

 Should our generalised function just be ui(&self, ui: &mut Ui)?

 It is a bit convenient if we draw the centralpanel first in the overlay...
    Remember we can have multiple threads, independently of each other adding to the overlay... without being able to
    communicate to each other well... should we do a 'tree' in the naming?
*/

#[derive(Copy, Clone, Debug)]
pub struct VisualId(usize);
#[derive(Debug)]
pub struct Overlay {
    // elements?
    elements: std::collections::HashMap<VisualId, Box<dyn PositionedWidget>>,
}

impl Overlay {
    fn draw(&self, ui: &mut egui::Ui) {}
}

fn test_clone(ui: &mut egui::Ui) {
    let v = vec![egui::widgets::Label::new("foo").fixed_pos(100.0, 30.0)];
    use egui::Widget;
    // v[0].ui(ui); // no copy on Label
    // let a = v[0].clone(); // no clone on Label :/
    // Should probably just make a WidgetFactory?
}

#[derive(Debug, Clone)]
pub struct OverlayHandle(std::sync::Arc<Overlay>);

pub fn main_test() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let config = OverlayConfig {
        width: 1920 * 2,
        height: 1080,
    };
    let options = fullscreen_overlay_native_options(&config);
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(TestOverlayApp {
                config,
                counter: 0.0,
            }))
        }),
    )
}

struct TestOverlayApp {
    config: OverlayConfig,
    counter: f32,
    // overlay: OverlayHandle,
}

impl eframe::App for TestOverlayApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();
        fullscreen_overlay_configure(ctx, &self.config);
        // ctx.send_viewport_cmd(ViewportCommand::Maximized(true));
        //

        let pos = egui::pos2(1920.0 / 2.0 - 100.0, 1080.0 / 2.0 - 100.0);
        self.counter = self.counter.rem_euclid(1.0) + 0.005;

        egui::Panel::top("my_panel").show_inside(ui, |ui| {
            ui.label("Hello World! From `TopBottomPanel`, that must be before `CentralPanel`!");
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::TRANSPARENT))
            .show_inside(ui, |ui| {
                egui::Area::new(egui::Id::new("my_areaz"))
                    .fixed_pos(egui::pos2(300.0, 100.0))
                    .default_size(egui::vec2(500.0, 200.0))
                    .kind(egui::UiKind::GenericArea)
                    .show(ui.ctx(), |ui| ui.label("Normal text"));
                egui::Area::new(egui::Id::new("my_area"))
                    .fixed_pos(egui::pos2(320.0, 320.0))
                    .default_size(egui::vec2(500.0, 200.0))
                    .show(ui.ctx(), |ui| {
                        ui.image(egui::include_image!(
                            // "../../PNG_transparency_demonstration_1.png"
                            "../examples/crosshair_image.png"
                        ))
                    });
                egui::Area::new(egui::Id::new("my_progressbar"))
                    .fixed_pos(pos)
                    .default_size(egui::vec2(50.0, 200.0))
                    .show(ui.ctx(), |ui| {
                        ui.add(egui::widgets::ProgressBar::new(self.counter))
                    });
            });
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}
