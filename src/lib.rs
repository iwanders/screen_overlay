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

pub const DEBUG_COLOR: Color32 = egui::Color32::from_rgba_unmultiplied_const(10, 10, 10, 128);

use egui::{Pos2, Stroke, Vec2, pos2};
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

pub struct PositionedElements {
    fixed_pos: Pos2,
    default_size: Vec2,
    fill: Option<Color32>,
    contents: Vec<Box<dyn Fn(&mut egui::Ui) + std::marker::Send + std::marker::Sync>>,
}

impl PositionedElements {
    /// Create a new positioned element that's mostly empty.
    pub fn new() -> Self {
        Self {
            fixed_pos: Pos2::ZERO,
            default_size: Vec2::NAN, // makes it default to the style's default.
            fill: None,
            contents: Default::default(),
        }
    }
    /// This specifies where the element is positioned.
    pub fn fixed_pos(mut self, fixed_pos: impl Into<Pos2>) -> Self {
        self.fixed_pos = fixed_pos.into();
        self
    }

    /// This specifies the default size, without this it inherits a default size.
    ///
    /// Be careful with elements that will grow in size. See [egui::Area::default_size].
    /// Be also aware that anything outside of the area will be clipped. Specifically important with drawings that
    /// don't flow.
    pub fn default_size(mut self, default_size: impl Into<Vec2>) -> Self {
        self.default_size = default_size.into();
        self
    }

    /// The fill color of this area.
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Set the fill color to the debug color.
    pub fn debug_color(self) -> Self {
        self.fill(DEBUG_COLOR)
    }

    /// Add a a lambda to this element.
    pub fn add(
        mut self,
        ui_gen: impl Fn(&mut egui::Ui) + std::marker::Send + std::marker::Sync + 'static,
    ) -> Self {
        self.contents.push(Box::new(ui_gen));
        self
    }

    /// Add a boxed lambda to this element.
    pub fn add_dyn(
        mut self,
        ui_gen: Box<dyn Fn(&mut egui::Ui) + std::marker::Send + std::marker::Sync>,
    ) -> Self {
        self.contents.push(ui_gen);
        self
    }

    /// Paint shapes to the screen, note that these are clipped using the size.
    pub fn paint<I: IntoIterator<Item = egui::Shape>>(self, items: I) -> Self {
        let shapes: Vec<egui::Shape> = items.into_iter().collect();
        self.add(move |ui| {
            // Note painted contents are clipped to the default size space.
            let (mut _response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap(), egui::Sense::empty());

            // May need to_screen * pos2(0.0, 0.0)... do we?
            // let to_screen = egui::emath::RectTransform::from_to(
            //     egui::Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
            //     response.rect,
            // );

            painter.extend(shapes.iter().cloned());
        })
    }
}

// Area + label suffers from https://github.com/emilk/egui/issues/5138
// Should we use a scroll area?
// Or even just a fixed window?
// https://docs.rs/egui/0.33.3/egui/index.html#auto-sizing-panels-and-windows

pub enum Drawable {
    /// Draw goes first, it can do anything on the ui, including adding things like panels.
    Draw(Box<dyn Fn(&mut egui::Ui) + std::marker::Send + std::marker::Sync>),
    /// Then the central panel gets created and the central elements are drawn.
    CentralElement(PositionedElements),
}

impl From<PositionedElements> for Drawable {
    fn from(elements: PositionedElements) -> Self {
        Drawable::CentralElement(elements)
    }
}
impl Drawable {
    fn draw(&self, ui: &mut egui::Ui) {
        match self {
            Drawable::Draw(drawable) => {
                // Trivial situation, just call it and move on.
                (drawable)(ui)
            }
            Drawable::CentralElement(elements) => {
                // Here, we create a new area, that we give the approprpiate position and size, then we make a new
                // central panel in that area to ensure it gets the appropriate size

                // Generate the id and consume it.
                let id = ui.next_auto_id();
                ui.skip_ahead_auto_ids(1);
                egui::Area::new(id)
                    .fixed_pos(elements.fixed_pos)
                    .default_size(elements.default_size)
                    .movable(false) // it's passthrough, but lets do this anyway.
                    .show(ui.ctx(), |ui| {
                        egui::CentralPanel::default() // This fills up the space in the default size.
                            .frame(
                                egui::Frame::default() // style the area
                                    .fill(elements.fill.unwrap_or(Color32::TRANSPARENT)),
                            )
                            .show_inside(ui, |ui| {
                                // Finally, iterate over the callbacks and populate them.
                                for e in elements.contents.iter() {
                                    (e)(ui);
                                }
                            });
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
            Drawable::CentralElement(elements) => f
                .debug_struct("Drawable::Elements")
                .field("fixed_pos", &elements.fixed_pos)
                .field("default_size", &elements.default_size)
                .field("fill", &elements.fill)
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

#[derive(Debug)]
pub struct Overlay {
    style: egui::Style,
    counter: std::sync::atomic::AtomicUsize,
    elements: RwLock<std::collections::HashMap<VisualId, Drawable>>,
}

impl Overlay {
    pub fn new() -> Self {
        let mut style = egui::Style::default();

        // To find the right keys, https://www.egui.rs/ and click backend.

        // Top panel style.
        style.visuals.panel_fill = Color32::TRANSPARENT; // panel background.
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::NONE; // Top panel divider line.

        // Color behind a progress bar.
        style.visuals.extreme_bg_color = Color32::from_rgba_unmultiplied(10, 10, 10, 128);

        Self {
            style,
            counter: 0.into(),
            elements: Default::default(),
        }
    }
    fn draw(&self, ui: &mut egui::Ui) {
        // Apply the overlay style.
        (*ui.style_mut()) = self.style.clone();

        let z = self.elements.read();
        // First, draw the raw drawables, that may add panels, and do whatever they want.
        for (_k, v) in z.iter() {
            if matches!(v, Drawable::Draw(_)) {
                v.draw(ui)
            };
        }
        // Then, create the central panel for the remaining elements
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::TRANSPARENT))
            .show_inside(ui, |ui| {
                for (_k, v) in z.iter() {
                    if matches!(v, Drawable::CentralElement(_)) {
                        v.draw(ui)
                    };
                }
            });
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
    pub fn new(overlay: Overlay) -> OverlayHandle {
        OverlayHandle(overlay.into())
    }
    pub fn add_drawable(&self, drawable: Drawable) -> VisualHandle {
        let id = self.0.add_element(drawable);
        VisualHandle {
            visual: id,
            overlay: self.0.clone(),
        }
    }
    pub fn draw(&self, ui: &mut egui::Ui) {
        self.0.draw(ui)
    }
}

pub fn main_test() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let config = OverlayConfig {
        width: 1920 * 2,
        height: 1080,
    };
    let options = fullscreen_overlay_native_options(&config);

    println!("DEBUG_COLOR: {DEBUG_COLOR:?}");

    let overlay = Overlay::new();
    let overlay = OverlayHandle::new(overlay);
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
        let our_counter_draw2 = our_counter.clone();

        let token2 = overlay.add_drawable(
            PositionedElements::new()
                .fixed_pos(egui::pos2(300.0, 200.0))
                .default_size(egui::vec2(150.0, 200.0))
                .add(move |ui| {
                    let value = our_counter_draw.load(Ordering::Relaxed);
                    ui.label(format!("normal text {}", value));
                })
                .add(|ui| {
                    ui.label("hahaha");
                })
                .into(),
        );

        let token_progressbar = overlay.add_drawable(Drawable::CentralElement(
            PositionedElements::new()
                .fixed_pos(egui::pos2(500.0, 250.0))
                .default_size(egui::vec2(850.0, 100.0))
                .add(move |ui| {
                    let value = our_counter_draw2.load(Ordering::Relaxed);
                    let ratio = (value % 10) as f32 / 10.0;

                    ui.add(egui::widgets::ProgressBar::new(ratio));
                }),
        ));

        let drawable = overlay.add_drawable(Drawable::CentralElement(
            PositionedElements::new()
                .fixed_pos(egui::pos2(500.0, 350.0))
                .default_size(egui::vec2(850.0, 100.0))
                .debug_color()
                .paint(vec![
                    egui::Shape::line(
                        vec![pos2(500.0, 350.0), pos2(850.0, 450.0)],
                        egui::Stroke::new(5.0, Color32::RED),
                    ),
                    egui::Shape::Circle(egui::epaint::CircleShape {
                        center: pos2(550.0, 400.0),
                        radius: 5.0,
                        fill: Color32::GREEN,
                        stroke: egui::Stroke::new(2.0, Color32::ORANGE),
                    }),
                ]),
        ));

        let cpos = pos2(1000.0, 500.0);
        let len = egui::vec2(15.0, 0.0);
        let crosshair = overlay.add_drawable(Drawable::CentralElement(
            PositionedElements::new()
                .fixed_pos(egui::pos2(0.0, 0.0))
                .default_size(egui::vec2(config.width as f32, config.height as f32))
                .paint(vec![
                    egui::Shape::Circle(egui::epaint::CircleShape {
                        center: cpos,
                        radius: 10.0,
                        fill: Color32::TRANSPARENT,
                        stroke: egui::Stroke::new(2.0, Color32::ORANGE),
                    }),
                    egui::Shape::LineSegment {
                        points: [cpos - len, cpos + len],
                        stroke: egui::Stroke::new(2.0, Color32::ORANGE),
                    },
                    egui::Shape::LineSegment {
                        points: [cpos - len.rot90(), cpos + len.rot90()],
                        stroke: egui::Stroke::new(2.0, Color32::ORANGE),
                    },
                ]),
        ));

        for i in 0..10000 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            // Do nothing, just wait
            our_counter.fetch_add(1, Ordering::Relaxed);
            // if i > 10 {
            //     break;
            // }
        }
    });
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(TestOverlayApp { config, overlay }))
        }),
    )
}

struct TestOverlayApp {
    config: OverlayConfig,
    overlay: OverlayHandle,
}

impl eframe::App for TestOverlayApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();
        fullscreen_overlay_configure(ctx, &self.config);
        ctx.global_style_mut(|style| {
            // Make the background of the progress bar semi-transparent
            style.visuals.extreme_bg_color = egui::Color32::from_rgba_unmultiplied(10, 10, 10, 128);
        });
        // ctx.send_viewport_cmd(ViewportCommand::Maximized(true));

        self.overlay.draw(ui);
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}
