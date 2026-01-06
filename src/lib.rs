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
use std::sync::atomic::Ordering;
/*
 containers have Container.show(ui, |ui|{}), but they do consume Container.
 widgets have:  fn ui(self, ui: &mut Ui) -> Response;

 Should our generalised function just be ui(&self, ui: &mut Ui)?

 It is a bit convenient if we draw the centralpanel first in the overlay...
    Remember we can have multiple threads, independently of each other adding to the overlay... without being able to
    communicate to each other well... should we do a 'tree' in the naming?
*/

pub trait UiDrawable: std::marker::Send + std::marker::Sync {
    fn draw(&self, ui: &mut egui::Ui);
}

pub struct PositionedElements {
    area: egui::Area,
    contents: Vec<Box<dyn Fn(&mut egui::Ui) + std::marker::Send + std::marker::Sync>>,
}

pub enum Drawable {
    Draw(Box<dyn Fn(&mut egui::Ui) + std::marker::Send + std::marker::Sync>),
    PositionedElements(PositionedElements),
}
impl UiDrawable for Drawable {
    fn draw(&self, ui: &mut egui::Ui) {
        match self {
            Drawable::Draw(drawable) => (drawable)(ui),
            Drawable::PositionedElements(elements) => {
                let area = elements.area.clone();
                area.show(ui.ctx(), |ui| {
                    for e in elements.contents.iter() {
                        (e)(ui);
                    }
                });
            }
        }
    }
}

impl std::fmt::Debug for Drawable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Drawable::Draw(_drawable) => f
                .debug_struct("Drawable::Draw")
                .field("drawable", &"UiDrawable")
                .finish(),
            Drawable::PositionedElements(elements) => f
                .debug_struct("Drawable::Elements")
                .field("area", &elements.area)
                .field("contents.len()", &elements.contents.len())
                .finish(),
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct VisualId(usize);

#[must_use]
#[derive(Debug)]
pub struct VisualHandle {
    visual: VisualId,
    overlay: std::sync::Arc<Overlay>,
}
impl Drop for VisualHandle {
    fn drop(&mut self) {
        self.overlay.remove_element(self.visual);
    }
}

#[derive(Debug, Default)]
pub struct Overlay {
    counter: std::sync::atomic::AtomicUsize,
    elements: RwLock<std::collections::HashMap<VisualId, Drawable>>,
}

impl Overlay {
    fn draw(&self, ui: &mut egui::Ui) {
        let z = self.elements.read();
        for (_k, v) in z.iter() {
            v.draw(ui)
        }
    }
    pub fn add_element(&self, drawable: Drawable) -> VisualId {
        let index = VisualId(self.counter.fetch_add(1, Ordering::Relaxed));
        let mut v = self.elements.write();
        v.insert(index, drawable);
        index
    }
    fn remove_element(&self, visual: VisualId) {
        let mut v = self.elements.write();
        v.remove(&visual);
    }
}
#[derive(Debug, Clone)]
pub struct OverlayHandle(std::sync::Arc<Overlay>);

impl OverlayHandle {
    pub fn new() -> OverlayHandle {
        OverlayHandle(Overlay::default().into())
    }
    pub fn add_drawable(&self, drawable: Drawable) -> VisualHandle {
        let id = self.0.add_element(drawable);
        VisualHandle {
            visual: id,
            overlay: self.0.clone(),
        }
    }
}

fn test_clone(ui: &mut egui::Ui) {
    // let v = vec![egui::widgets::Label::new("foo").fixed_pos(100.0, 30.0)];
    use egui::Widget;
    // v[0].ui(ui); // no copy on Label
    // let a = v[0].clone(); // no clone on Label :/
    // Should probably just make a WidgetFactory?
}

pub fn main_test() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let config = OverlayConfig {
        width: 1920 * 2,
        height: 1080,
    };
    let options = fullscreen_overlay_native_options(&config);

    let overlay = OverlayHandle::new();
    let overlay_for_runner = overlay.clone();
    let handle = std::thread::spawn(move || {
        let overlay = overlay_for_runner;

        let token = overlay.add_drawable(Drawable::Draw(Box::new(|ui| {
            egui::Panel::top("my_panel").show_inside(ui, |ui| {
                ui.label("Hello World! From `TopBottomPanel`, that must be before `CentralPanel`!");
            });
        })));

        let our_counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let our_counter_draw = our_counter.clone();

        let token2 = overlay.add_drawable(Drawable::Draw(Box::new(move |ui| {
            let value = our_counter_draw.load(Ordering::Relaxed);
            let ratio = (value % 10) as f32 / 10.0;
            egui::CentralPanel::default()
                .frame(egui::Frame::default().fill(Color32::TRANSPARENT))
                .show_inside(ui, |ui| {
                    egui::Area::new(egui::Id::new("my_value"))
                        .fixed_pos(egui::pos2(300.0, 100.0))
                        .default_size(egui::vec2(500.0, 200.0))
                        .kind(egui::UiKind::GenericArea)
                        .show(ui.ctx(), |ui| ui.label(format!("{}", value)));
                    egui::Area::new(egui::Id::new("my_progressbar"))
                        .fixed_pos(egui::pos2(300.0, 200.0))
                        .default_size(egui::vec2(50.0, 200.0))
                        .show(ui.ctx(), |ui| {
                            ui.add(egui::widgets::ProgressBar::new(ratio))
                        });
                });
        })));

        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            // Do nothing, just wait
            our_counter.fetch_add(1, Ordering::Relaxed);
        }
    });
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(TestOverlayApp {
                config,
                counter: 0.0,
                overlay,
            }))
        }),
    )
}

struct TestOverlayApp {
    config: OverlayConfig,
    counter: f32,
    overlay: OverlayHandle,
}

impl eframe::App for TestOverlayApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();
        fullscreen_overlay_configure(ctx, &self.config);
        // ctx.send_viewport_cmd(ViewportCommand::Maximized(true));

        self.overlay.0.draw(ui);

        /*
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
            */
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}
