//! The new system uses egui/eframe for a full screen overlay.
//!
//! Any interaction with the [`Overlay`] is thread safe.
//! Adding elements to the [`OverlayHandle`] returns a [`VisualHandle`], if that is dropped the visual element will be
//! removed from the overlay.
//! Interacting directly with the [`Overlay`] is a bit more low level and returns [`VisualId`] objects that need to be
//! manually removed.
//!
//! In general you want to interact with [`OverlayHandle`]  as that can clone the pointer for passing
//! it to the deferred viewport function call. You create the overlay, put it in a handle, you pass the handle around
//! to the places that need access to the overlay to add [`Drawable`]'s and from your [`eframe::App::ui`] method the
//! [`OverlayHandle::show_viewport_deferred`] method is called to draw the overlay.
//!
//! The [`OverlayHandle`] does implement [`eframe::App`], which is useful in simple use cases where the overlay is the
//! only element drawn.
//!
//! On Windows, there's a known [issue](https://github.com/emilk/egui/issues/3632#issuecomment-3733528750) around the
//! non-primary windows transparency support.
//!

/// Legacy module using dxgi and raw x11/glfw/three_d.
#[cfg(feature = "legacy")]
pub mod legacy;

use egui::{Color32, Pos2, Stroke, Vec2, ViewportCommand, pos2};
use parking_lot::RwLock;
use std::sync::atomic::Ordering;

pub use egui;

use serde::{Deserialize, Serialize};

/// Configuration for the overlay.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct OverlayConfig {
    /// The size to use for the overlay.
    ///
    /// Elements are shrunk to fit into this.
    pub size: Vec2,

    /// The position at which to draw the overlay.
    ///
    /// This denotes the top left corner and is the relative anchor for all other positioning.
    pub position: Pos2,

    /// The fill color for the central panel that holds PositionedElements.
    ///
    /// This can be helpful to set to [crate::DEBUG_COLOR] to understand positioning and size.
    pub central_panel_fill: Color32,

    /// The id assigned to this viewport.
    ///
    /// If this is unique, the old overlay created may persist indefinitely?
    pub viewport_id: String,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            position: Default::default(),
            size: [100.0, 100.0].into(),
            central_panel_fill: Color32::TRANSPARENT,
            viewport_id: "overlay".to_owned(),
        }
    }
}

impl OverlayConfig {
    /// Create a new overlay config at the top left corner of the screen. Default size is 100x100, modify this.
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns this overlay config with the size modified to be the argument.
    pub fn with_size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }
    /// Returns this overlay config with the position modified to be the argument.
    pub fn with_position(mut self, position: impl Into<Pos2>) -> Self {
        self.position = position.into();
        self
    }
    /// Returns this overlay config with the central panel color modified to be the argument.
    pub fn with_central_panel_fill(mut self, color: impl Into<Color32>) -> Self {
        self.central_panel_fill = color.into();
        self
    }
}

/// Returns the NativeOptions to create the window on linux.
#[cfg(target_os = "linux")]
pub fn fullscreen_overlay_native_options(config: &OverlayConfig) -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(config.size) // Must be repeated in viewport configure.
            .with_position(config.position)
            // .with_resizable(false)
            // .with_fullscreen(true)
            // .with_maximized(true)
            .with_transparent(true) // This one must be here
            .with_mouse_passthrough(true) // Must be repeated in viewport configure.
            .with_always_on_top() // Must be repeated in viewport configure.
            // .with_decorations(false)
            // .with_titlebar_shown(false)
            .with_clamp_size_to_monitor_size(false) // Necessary to ensure we can have windows wider than the monitor.
            .with_override_redirect(true) // Must be here to bypass the window manager, but not the compositor.
            .with_window_type(egui::X11WindowType::Utility),
        ..Default::default()
    }
}
/// Sends the appropriate viewport commands to configure the overlay on linux.
#[cfg(target_os = "linux")]
pub fn fullscreen_overlay_configure(ctx: &egui::Context, config: &OverlayConfig) {
    let _ = config;
    ctx.set_pixels_per_point(1.0); // Can we do this, or does this affect the other window?
    // These size & position are necessary for larger windows that appear to be clamped on initial creation.
    ctx.send_viewport_cmd(ViewportCommand::InnerSize(config.size));
    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(config.position));
    ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(true));
    ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
}

/// Returns the NativeOptions to create the window on windows.
#[cfg(target_os = "windows")]
pub fn fullscreen_overlay_native_options(config: &OverlayConfig) -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([config.width as f32, config.height as f32]) // Must be repeated in viewport configure.
            .with_resizable(false)
            .with_fullscreen(true)
            .with_maximized(true)
            .with_transparent(true)
            .with_taskbar(false)
            // .with_mouse_passthrough(true) // This doesn't actually work, but setting the viewportcommand later does.
            .with_always_on_top()
            .with_decorations(false)
            //.with_position(egui::pos2(-1920.0, 0.0)) // for left monitor.
            .with_titlebar_shown(false), // Necessary to ensure we can have windows wider than the monitor.
        // multisampling: 1,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    }
}
/// Sends the appropriate viewport commands to configure the overlay on windows.
#[cfg(target_os = "windows")]
pub fn fullscreen_overlay_configure(ctx: &egui::Context, config: &OverlayConfig) {
    ctx.send_viewport_cmd(ViewportCommand::InnerSize(config.size));
    ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(true));
    ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
    ctx.send_viewport_cmd(ViewportCommand::Transparent(true));
}

/// An debug color that's more elegant than egui::DEBUG_COLLOR, this is dark grey and 50% transparent.
pub const DEBUG_COLOR: Color32 = egui::Color32::from_rgba_unmultiplied_const(10, 10, 10, 128);

/// Thread safe function that can draw a ui.
pub type DrawUiFunction = dyn Fn(&mut egui::Ui) + std::marker::Send + std::marker::Sync;
/// This is a helper for drawing positioned ui elements.
///
/// It holds a position and a size, and the provided elements are drawn into that area.
/// The area can be visualised for easy debugging by setting a fill color.
pub struct PositionedElements {
    fixed_pos: Pos2,
    default_size: Vec2,
    fill: Option<Color32>,
    contents: Vec<Box<DrawUiFunction>>,
}
impl Default for PositionedElements {
    fn default() -> Self {
        Self {
            fixed_pos: Pos2::ZERO,
            default_size: Vec2::NAN, // makes it default to the style's default.
            fill: None,
            contents: Default::default(),
        }
    }
}

impl PositionedElements {
    /// Create a new positioned element in the top left corner with the default area size.
    pub fn new() -> Self {
        Default::default()
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

    /// Set the fill color to the debug color, which is a transparent dark gray.
    pub fn debug_color(self) -> Self {
        self.fill(DEBUG_COLOR)
    }

    /// Add a a lambda to this element.
    pub fn add_closure(
        mut self,
        ui_gen: impl Fn(&mut egui::Ui) + std::marker::Send + std::marker::Sync + 'static,
    ) -> Self {
        self.contents.push(Box::new(ui_gen));
        self
    }

    /// Add a boxed lambda to this element.
    pub fn add_dyn(mut self, ui_gen: Box<DrawUiFunction>) -> Self {
        self.contents.push(ui_gen);
        self
    }

    /// Paint shapes to the screen, note that these are clipped to the area. Coordinates are in screen space points.
    pub fn paint<I: IntoIterator<Item = egui::Shape>>(self, items: I) -> Self {
        let shapes: Vec<egui::Shape> = items.into_iter().collect();
        self.add_closure(move |ui| {
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

/// A drawable that can be added to the overlay. Usually created from [`PositionedElements`].
///
/// First all [`Drawable::Draw`] entities are drawn, then the central panel is created and [`Drawable::CentralElement`]
/// drawables are drawn.
pub enum Drawable {
    /// These drawables can do anything, including allocating and populating containers like panels.
    Draw(Box<dyn Fn(&mut egui::Ui) + std::marker::Send + std::marker::Sync>),
    /// After the [`Drawable::Draw`] elements, a central panel gets created and in that the [`PositionedElements`]'s are drawn.
    CentralElement(PositionedElements),
}

impl From<PositionedElements> for Drawable {
    fn from(elements: PositionedElements) -> Self {
        Drawable::CentralElement(elements)
    }
}

impl Drawable {
    /// Draw the drawable on the UI.
    pub fn draw(&self, ui: &mut egui::Ui) {
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

/// Id for a particular visual held by the overlay.
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
pub struct VisualId(usize);

/// RAII visual handle that keeps a [`Drawable`] alive in the overlay.
///
/// If this goes out of scope [`Overlay::remove_element`] is called for it.
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

/// The overlay, this keeps a thread-safe map of visuals to draw each frame.
#[derive(Debug)]
pub struct Overlay {
    style: egui::Style,
    counter: std::sync::atomic::AtomicUsize,
    elements: RwLock<std::collections::HashMap<VisualId, Drawable>>,
    config: OverlayConfig,
}

impl Overlay {
    /// Create a new overlay using the provided configuration.
    pub fn new(config: OverlayConfig) -> Self {
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
            config,
        }
    }

    /// Draw the overlay onto a ui.
    pub fn draw(&self, ui: &mut egui::Ui) {
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
            .frame(egui::Frame::default().fill(self.config.central_panel_fill))
            .show_inside(ui, |ui| {
                for (_k, v) in z.iter() {
                    if matches!(v, Drawable::CentralElement(_)) {
                        v.draw(ui)
                    };
                }
            });
    }

    /// Configure the viewport to draw the overlay.
    pub fn configure(&self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        fullscreen_overlay_configure(ctx, &self.config);
    }

    /// Add a drawable element to the overlay.
    pub fn add_element(&self, drawable: Drawable) -> VisualId {
        let index = VisualId(self.counter.fetch_add(1, Ordering::Relaxed));
        let mut v = self.elements.write();
        v.insert(index, drawable);
        index
    }

    /// Add an element by its id from the overlay.
    pub fn remove_element(&self, visual: VisualId) {
        let mut v = self.elements.write();
        v.remove(&visual);
    }

    /// Return the native options for this overlay.
    pub fn native_options(&self) -> eframe::NativeOptions {
        fullscreen_overlay_native_options(&self.config)
    }

    /// Return the viewport options for this overlay.
    pub fn viewport_builder(&self) -> egui::ViewportBuilder {
        self.native_options().viewport
    }
}

/// The handle holds a pointer to an [`Overlay`] as well as some convenience functions.
#[derive(Debug, Clone)]
pub struct OverlayHandle(std::sync::Arc<Overlay>);

impl std::cmp::PartialEq for OverlayHandle {
    fn eq(&self, other: &OverlayHandle) -> bool {
        std::sync::Arc::as_ptr(&self.0) == std::sync::Arc::as_ptr(&other.0)
    }
}

impl OverlayHandle {
    /// Create a new overlay handle.
    pub fn new(overlay: Overlay) -> OverlayHandle {
        OverlayHandle(overlay.into())
    }

    /// Create a handle from a weak pointer.
    pub fn from_weak(weak: std::sync::Weak<Overlay>) -> Option<Self> {
        weak.upgrade().map(Self::from_ptr)
    }

    /// Create a weak pointer from this handle's strong pointer.
    pub fn to_weak(&self) -> std::sync::Weak<Overlay> {
        std::sync::Arc::<Overlay>::downgrade(&self.0)
    }

    /// Create a handle from a strong pointer.
    pub fn from_ptr(overlay: std::sync::Arc<Overlay>) -> Self {
        OverlayHandle(overlay)
    }

    /// Add a drawable to the overlay and return a RAII [`VisualHandle`].
    pub fn add_drawable(&self, drawable: Drawable) -> VisualHandle {
        let id = self.0.add_element(drawable);
        VisualHandle {
            visual: id,
            overlay: self.0.clone(),
        }
    }

    /// Passthrough to [`Overlay::configure`].
    pub fn configure(&self, ui: &mut egui::Ui) {
        self.0.configure(ui);
    }

    /// Passthrough to [`Overlay::draw`].
    pub fn draw(&self, ui: &mut egui::Ui) {
        self.0.draw(ui)
    }

    /// Passthrough to [`Overlay::viewport_builder`].
    pub fn viewport_builder(&self) -> egui::ViewportBuilder {
        self.0.viewport_builder()
    }
    /// Passthrough to [`Overlay::native_options`].
    pub fn native_options(&self) -> eframe::NativeOptions {
        self.0.native_options()
    }

    /// Shows the overlay in a deferred viewport, this is the function to call from the [`eframe::App::ui`] method.
    ///
    /// This is the main entry point you likely want to call to draw the overlay.
    ///
    /// **NOTE**: If you use this to integrate with
    /// an existing application, it is paramount to ensure it clears with a transparent color, use this in the
    /// [`eframe::App`] implementation:
    /// ```ignore
    /// fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
    ///    egui::Rgba::TRANSPARENT.to_array()
    /// }
    /// ```
    /// If this is missing, the overlay will always have a dark gray semi-transparent background regardless of the
    /// fill color specified.
    pub fn show_viewport_deferred(&self, ui: &mut egui::Ui) {
        let overlay_copy = self.clone();
        ui.ctx().show_viewport_deferred(
            egui::ViewportId::from_hash_of(&self.0.config.viewport_id),
            self.viewport_builder(),
            move |ui, _class| {
                overlay_copy.configure(ui);
                overlay_copy.draw(ui);
            },
        );
    }
}

impl eframe::App for OverlayHandle {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.configure(ui);
        self.draw(ui);
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}

/// This is a development main function... mostly such that it's in the same file for development convenience.
#[allow(unused_variables)]
pub fn main_test() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let config = OverlayConfig::new()
        .with_size([1920.0 * 0.5, 1080.0 * 0.5])
        .with_position([100.0, 100.0]);

    println!("DEBUG_COLOR: {DEBUG_COLOR:?}");

    let overlay = Overlay::new(config.clone());
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
        let our_counter_draw3 = our_counter.clone();

        let token2 = overlay.add_drawable(
            PositionedElements::new()
                .fixed_pos(egui::pos2(300.0, 200.0))
                .default_size(egui::vec2(150.0, 200.0))
                .debug_color()
                .add_closure(move |ui| {
                    let value = our_counter_draw.load(Ordering::Relaxed);
                    ui.label(format!("normal text {}", value));
                })
                .add_closure(|ui| {
                    ui.label("ha-haaa it works!");
                })
                .into(),
        );

        let big_text = overlay.add_drawable(
            PositionedElements::new()
                .fixed_pos(egui::pos2(100.0, 300.0))
                .default_size(egui::vec2(150.0, 200.0))
                // .debug_color()
                .add_closure(move |ui| {
                    let value = our_counter_draw3.load(Ordering::Relaxed);
                    let text =
                        egui::widget_text::RichText::new(format!("Big text {}", value)).size(30.0);
                    ui.label(text);
                })
                .add_closure(|ui| {
                    ui.label("ha-haaa it works! 🎉");
                })
                .into(),
        );

        let token_progressbar = overlay.add_drawable(Drawable::CentralElement(
            PositionedElements::new()
                .fixed_pos(egui::pos2(500.0, 250.0))
                .default_size(egui::vec2(850.0, 100.0))
                .add_closure(move |ui| {
                    let value = our_counter_draw2.load(Ordering::Relaxed);
                    let ratio = (value % 10) as f32 / 10.0;

                    ui.add(egui::widgets::ProgressBar::new(ratio));
                }),
        ));

        let drawable = overlay.add_drawable(Drawable::CentralElement(
            PositionedElements::new()
                .fixed_pos(egui::pos2(500.0, 350.0))
                .default_size(egui::vec2(850.0, 100.0))
                // .debug_color()
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

        let cpos = pos2(1000.0, 500.0); // crosshair pos, but then short.
        let len = egui::vec2(15.0, 0.0);
        let crosshair = overlay.add_drawable(Drawable::CentralElement(
            PositionedElements::new()
                .fixed_pos(egui::pos2(0.0, 0.0))
                .default_size(config.size)
                // .debug_color()
                .paint(vec![
                    egui::Shape::Circle(egui::epaint::CircleShape {
                        center: cpos,
                        radius: 10.0,
                        fill: Color32::TRANSPARENT,
                        stroke: egui::Stroke::new(1.5, Color32::ORANGE),
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

        // This image scales to fit the size.
        let image_on_screen1 = overlay.add_drawable(Drawable::CentralElement(
            PositionedElements::new()
                .fixed_pos(egui::pos2(900.0, 300.0))
                .default_size(egui::vec2(235.0 * 1.1, 140.0 * 1.1)) // Should scale up and be a bit blurry.
                .add_closure(|ui| {
                    // Adding the image like this scales to this size.
                    ui.image(egui::include_image!("../examples/crosshair_image.png"));
                }),
        ));
        let image_on_screen2 = overlay.add_drawable(Drawable::CentralElement(
            PositionedElements::new()
                .fixed_pos(egui::pos2(1300.0, 300.0))
                .default_size(egui::vec2(535.0, 240.0)) // Adding the image like this scales to this size.
                // .debug_color() // toggle this to see that this doesn't cover the entire area.
                .add_closure(|ui| {
                    // THis way the image should be crisp.
                    ui.add(
                        egui::Image::new(egui::include_image!("../examples/crosshair_image.png"))
                            .fit_to_original_size(1.0), // prevents scaling to fit the size.
                    );
                }),
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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("normal")
            .with_inner_size([200.0, 100.0]),
        ..Default::default()
    };
    // let options = fullscreen_overlay_native_options(&config);
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(TestOverlayApp { overlay }))
        }),
    )
}

// used by main test.
struct TestOverlayApp {
    overlay: OverlayHandle,
}

impl eframe::App for TestOverlayApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.overlay.show_viewport_deferred(ui);

        // println!("things");
        // self.overlay.draw(ui);
        ui.heading("My egui Application");
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }
}
