/// The legacy system, using dxgi on linux, glfw + x11 + three_d on linux. The new system uses egui/eframe for a full
/// screen overlay.
pub mod legacy;

#[derive(Debug, Copy, Clone)]
pub struct Visual(usize);

use eframe::egui::{self, Color32, ViewportCommand};

pub fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1920.0, 1080.0]) // This doesn't seem to be pixel coordinates?
            .with_resizable(false)
            // .with_fullscreen(true)
            .with_maximized(true)
            .with_transparent(true)
            .with_mouse_passthrough(true) // This doesn't actually work, but setting the viewportcommand later does.
            .with_always_on_top()
            .with_decorations(false)
            .with_titlebar_shown(false)
            .with_override_redirect(true)
            .with_window_type(egui::X11WindowType::Utility),
        ..Default::default()
    };
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::default())
        }),
    )
}

#[derive(Default)]
pub struct MyApp {}

impl eframe::App for MyApp {
    // fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let window = _frame.window_handle().unwrap();

        // println!("available_width: {}", ui.available_width());
        // println!("available_height: {}", ui.available_height());
        // println!("available_size: {:?}", ui.available_size());
        // println!("pixels_per_point: {}", ui.pixels_per_point());
        // pixels per point is 0.953, which aligns with:
        // >>> 1832 / 1920
        // 0.9541666666666667
        // Which is why our window size doesn't cover the entire monitor :<

        // ctx.send_viewport_cmd(ViewportCommand::Transparent(true));
        // ctx.send_viewport_cmd(ViewportCommand::Decorations(false));
        // Mouse pass through doesn't work...
        // https://docs.rs/winit/latest/winit/window/struct.Window.html#method.set_cursor_hittest should be possible.
        let ctx = ui.ctx();
        ctx.set_pixels_per_point(1.0);
        ctx.send_viewport_cmd(ViewportCommand::InnerSize((1920.0, 1080.0).into()));
        ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(true));
        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
        // ctx.send_viewport_cmd(ViewportCommand::Maximized(true));
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
                /*
                egui::ScrollArea::both().show(ui, |ui| {
                    // ui.image(egui::include_image!("../../examples/crosshair_image.png"))
                    //     .on_hover_text_at_pointer("WebP");

                    /*ui.image(egui::include_image!("cat.webp"))
                        .on_hover_text_at_pointer("WebP");
                    ui.image(egui::include_image!("ferris.gif"))
                        .on_hover_text_at_pointer("Gif");
                    ui.image(egui::include_image!("ferris.svg"))
                        .on_hover_text_at_pointer("Svg");
                    let url = "https://picsum.photos/seed/1.759706314/1024";
                    ui.add(egui::Image::new(url).corner_radius(10))
                        .on_hover_text_at_pointer(url);
                        */
                });*/
            });
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}
