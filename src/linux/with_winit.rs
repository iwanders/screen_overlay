#![allow(unused_variables, unused_imports, unreachable_code)]
use winit::{
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    platform::pump_events::EventLoopExtPumpEvents as _,
    window::WindowId,
};

use crate::{
    CapStyle, CircleDirection, Color, DashStyle, DrawGeometry, Error, GeometryElement, LineJoin,
    LineStyle, OverlayConfig, Point, Rect, Stroke, TextAlignment, TextProperties,
};

use winit::window::Window;

#[derive(Clone)]
pub struct ImageTexture {}
impl std::fmt::Debug for ImageTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "ImageTexture ")
    }
}
#[derive(Clone)]
pub struct PreparedFont {}
impl std::fmt::Debug for PreparedFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "PreparedFont  ")
    }
}
// pub type IDVisual = usize;
#[derive(Clone, Debug)]
pub struct IDVisual {}

pub struct OverlayImpl {}
unsafe impl Send for OverlayImpl {}
impl OverlayImpl {
    pub fn new() -> Result<Self, crate::Error> {
        Ok(Self {})
    }

    pub fn create_window(&mut self, config: &OverlayConfig) -> Result<(), Error> {
        Ok(())
    }

    pub fn create_device_resources(&mut self) -> Result<(), Error> {
        Ok(())
    }

    pub fn draw_geometry(
        &mut self,
        geometry: &DrawGeometry,
        stroke: &Stroke,
        line_style: &LineStyle,
    ) -> Result<IDVisual, Error> {
        Ok(IDVisual {})
    }

    pub fn prepare_font(&mut self, properties: &TextProperties) -> Result<PreparedFont, Error> {
        todo!()
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        layout: &Rect,
        color: &Color,
        font: &PreparedFont,
    ) -> Result<IDVisual, Error> {
        todo!();
    }

    pub fn load_texture<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
    ) -> Result<ImageTexture, Error> {
        Ok(ImageTexture {})
    }

    pub fn draw_texture(
        &mut self,
        position: &Point,
        texture: &ImageTexture,
        texture_region: &Rect,
        color: &Color,
        alpha: f32,
    ) -> Result<IDVisual, Error> {
        Ok(IDVisual {})
    }

    pub fn remove_visual(&mut self, visual: &IDVisual) -> Result<(), Error> {
        Ok(())
    }

    pub fn render(&mut self) {}
}

#[derive(Default)]
struct AppHandle {
    window: Option<Window>,
}

impl winit::application::ApplicationHandler for AppHandle {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}

pub struct ApplicationWrapper {
    app: AppHandle,
    event_loop: EventLoop<()>,
}

type OurApplicationType = ApplicationWrapper;
pub fn run_msg_loop(wrapper: OurApplicationType) -> Result<(), Error> {
    let ApplicationWrapper {
        mut app,
        event_loop,
    } = wrapper;

    event_loop.run_app(&mut app)?;
    Ok(())
}

pub fn setup() -> Result<OurApplicationType, Error> {
    standalone_test::main();

    let event_loop = EventLoop::new().unwrap();

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop.set_control_flow(ControlFlow::Poll);

    Ok(ApplicationWrapper {
        app: AppHandle { window: None },
        event_loop,
    })
}

// Based on discussion from here, since that did pretty much waht I wanted.
// https://github.com/emilk/egui/issues/4451
mod standalone_test {
    #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
    #![allow(rustdoc::missing_crate_level_docs)] // it's an example

    use eframe::egui::{self, Color32, ViewportCommand};

    pub fn main() -> eframe::Result {
        env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([120.0, 880.0])
                .with_transparent(true)
                .with_mouse_passthrough(true)
                .with_decorations(false)
                .with_always_on_top()
                .with_titlebar_shown(false),
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
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            // ctx.send_viewport_cmd(ViewportCommand::Transparent(true));
            // ctx.send_viewport_cmd(ViewportCommand::Decorations(false));
            // Mouse pass through doesn't work...
            // https://docs.rs/winit/latest/winit/window/struct.Window.html#method.set_cursor_hittest should be possible.
            ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(true));
            // always on top works.
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
            egui::CentralPanel::default()
                .frame(egui::Frame::default().fill(Color32::TRANSPARENT))
                .show(ctx, |ui| {
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.image(egui::include_image!("../../examples/crosshair_image.png"))
                            .on_hover_text_at_pointer("WebP");
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
                    });
                });
        }
        fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
            egui::Rgba::TRANSPARENT.to_array()
        }
    }
}
