use std::sync::Arc;

// This started based on the direct composition example:
// https://github.com/microsoft/windows-rs/tree/ef06753b0df2aaa16894416191bcde328b9d6ffb/crates/samples/windows/dcomp

// API
//  - Drawable -> Returns RAII handle with interface to drawable.
//  - Should be thread safe (all of it)
//  - Need a wrapper with an interior Arc.

mod windows;
use windows::{IDVisual, ImageTexture, OverlayImpl, PreparedFont};

pub use windows::setup as setup;
pub use windows::run_msg_loop as block_and_loop;

use parking_lot::Mutex;
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Clone)]
pub struct VisualToken {
    overlay: Arc<Mutex<OverlayImpl>>,
    visual: IDVisual,
}
impl std::fmt::Debug for VisualToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "VisualToken {:?}", &self.visual)
    }
}

impl Drop for VisualToken {
    fn drop(&mut self) {
        let mut wlock = self.overlay.lock();
        wlock
            .remove_visual(&self.visual)
            .expect("removing something thats already removed");
    }
}

#[derive(Clone)]
pub struct Overlay {
    overlay: Arc<Mutex<OverlayImpl>>,
}

#[derive(Clone, Debug)]
pub struct OverlayConfig {
    /// If true, the application shows in the task bar.
    pub task_bar: bool,

    /// If true, the overlay is on top of all other windows, regardless if the application is selected
    pub on_top: bool,

    /// The name to give the application in the task bar.
    pub name: String,
}
impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            task_bar: true,
            on_top: true,
            name: "Overlay".to_owned(),
        }
    }
}

impl Overlay {
    /// Create a new overlay
    pub fn new() -> std::result::Result<Overlay, Error> {
        Self::new_with_config(&Default::default())
    }

    pub fn new_with_config(config: &OverlayConfig) -> std::result::Result<Overlay, Error>{
        let window = Arc::new(Mutex::new(OverlayImpl::new()?));
        {
            let mut wlock = window.lock();
            wlock.create_window(config)?;
            wlock.create_device_resources()?;
        }
        Ok(Self { overlay: window })
    }

    /// Prepare a font for usage.
    ///
    /// Draw arbitrary geometry on the screen. You may need to offset by half a pixel to ensure you get pixel-perfect
    /// crisp lines.
    pub fn draw_geometry(
        &self,
        geometry: &DrawGeometry,
        stroke: &Stroke,
        line_style: &LineStyle,
    ) -> std::result::Result<VisualToken, Error> {
        {
            let mut wlock = self.overlay.lock();
            let visual = wlock.draw_geometry(geometry, stroke, line_style)?;
            Ok(VisualToken {
                visual,
                overlay: self.overlay.clone(),
            })
        }
    }

    /// Prepare a font for usage.
    ///
    /// Initialises the font according to the properties, this handle is passed to [`draw_text`].
    pub fn prepare_font(
        &self,
        properties: &TextProperties,
    ) -> std::result::Result<PreparedFont, Error> {
        let mut wlock = self.overlay.lock();
        Ok(wlock.prepare_font(properties)?)
    }

    /// Draw text on the screen.
    ///
    /// * `text` The text to write
    /// * `layout` The layout rectangle to stay in.
    /// * `color` The color with which to draw.
    /// * `font` The prepared font as returned by [`prepare_font`].
    pub fn draw_text(
        &self,
        text: &str,
        layout: &Rect,
        color: &Color,
        font: &PreparedFont,
    ) -> std::result::Result<VisualToken, Error> {
        {
            let mut wlock = self.overlay.lock();
            let visual = wlock.draw_text(text, layout, color, font)?;
            Ok(VisualToken {
                visual,
                overlay: self.overlay.clone(),
            })
        }
    }

    /// Load a texture from disk for later use.
    pub fn load_texture<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> std::result::Result<ImageTexture, Error> {
        {
            let mut wlock = self.overlay.lock();
            Ok(wlock.load_texture(path)?)
        }
    }

    /// Draw a texture's region at the specified position.
    ///
    /// * `alpha` 0.0 is transparent, 1.0 is opaque.
    /// * `position` Top Left position at whihc the region will be drawn.
    /// * `texture` The texture from which to draw the texture region.
    /// * `texture_region` The area of the texture to be drawn.
    /// * `color` The background color drawn before the texture.
    /// * `alpha` The alpha at which the texture is drawn over the background.
    pub fn draw_texture(
        &self,
        position: &Point,
        texture: &ImageTexture,
        texture_region: &Rect,
        color: &Color,
        alpha: f32,
    ) -> std::result::Result<VisualToken, Error> {
        {
            let mut wlock = self.overlay.lock();
            let visual = wlock.draw_texture(position, texture, texture_region, color, alpha)?;
            Ok(VisualToken {
                visual,
                overlay: self.overlay.clone(),
            })
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub enum TextAlignment {
    /// Align to the minimum possible value. (Left or Top)
    Min,
    #[default]
    /// Align to the center of the axis.
    Center,
    /// Align to the maximum possible value. (Right or Bottom)
    Max,
    /// Only applicable to horizontal; justified rendering.
    Justified,
}

/// Properties for the font and text.
#[derive(Clone, Debug)]
pub struct TextProperties {
    /// The font family name, on windows defaults to 'Arial'.
    pub font: String,
    /// The size of the font in device independent pixels.
    pub size: f32,
    /// Horizontal alignment specification.
    pub horizontal_align: TextAlignment,
    /// Vertical alignment specification.
    pub vertical_align: TextAlignment,
}
impl Default for TextProperties {
    fn default() -> Self {
        Self {
            // font: "Candara".to_owned(),
            font: "Arial".to_owned(),
            size: 16.0,
            horizontal_align: TextAlignment::default(),
            vertical_align: TextAlignment::default(),
        }
    }
}

/// Color representation with alpha.
#[derive(Copy, Clone, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// Alpha channel, 255 is opaque, 0 is transparent.
    pub a: u8,
}
impl Color {
    pub const TRANSPARENT: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub fn transparent(&self) -> Self {
        Color {
            r: self.r,
            g: self.g,
            b: self.b,
            a: 0,
        }
    }
    pub fn a_f32(&self) -> f32 {
        self.a as f32 / 255.0
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Stroke {
    pub color: Color,
    pub width: f32,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum CapStyle {
    #[default]
    Flat,
    Square,
    Round,
    Triangle,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Bevel,
    Round,
    MiterOrBevel,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum DashStyle {
    #[default]
    Solid,
    Dash,
    Dot,
    DashDot,
    DashDotDot,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct LineStyle {
    pub start_cap: CapStyle,
    pub end_cap: CapStyle,
    pub dash_cap: CapStyle,

    pub line_join: LineJoin,
    pub miter_limit: f32,

    pub dash_style: DashStyle,
    pub dash_offset: f32,
}


#[derive(Copy, Clone, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    const ORIGIN: Point = Point { x: 0.0, y: 0.0 };
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
impl std::ops::Add<Point> for Point {
    type Output = Point;
    fn add(self, other: Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}
impl Rect {
    pub fn from(x: f32, y: f32) -> Self {
        Self {
            min: Point { x, y },
            max: Point::ORIGIN,
        }
    }
    pub fn sized(self, w: f32, h: f32) -> Self {
        Self {
            min: self.min,
            max: self.min + Point::new(w, h),
        }
    }
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }
}

#[derive(Copy, Clone, Debug)]
pub enum CircleDirection {
    CounterClockWise,
    ClockWise,
}

#[derive(Copy, Clone, Debug)]
pub enum GeometryElement {
    Start {
        start: Point,
        filled: bool,
    },
    Line(Point),
    Arc {
        end_point: Point,
        radius: f32, // in degrees!?!?!
        angle: f32,
        direction: CircleDirection,
        // arc size??
    },
    End {
        closed: bool,
    },
}

#[derive(Clone, Debug)]
pub struct DrawGeometry {
    pub elements: Vec<GeometryElement>,
}
impl DrawGeometry {
    pub fn new() -> Self {
        Self { elements: vec![] }
    }

    fn appended(self, e: GeometryElement) -> Self {
        let mut elements = self.elements;
        elements.push(e);
        Self { elements }
    }

    pub fn hollow(self, x: f32, y: f32) -> Self {
        self.appended(GeometryElement::Start {
            start: Point { x, y },
            filled: false,
        })
    }

    pub fn closed(self) -> Self {
        self.appended(GeometryElement::End { closed: true })
    }

    pub fn line(self, x: f32, y: f32) -> Self {
        self.appended(GeometryElement::Line(Point { x, y }))
    }

    pub fn rectangle(self, rect: &Rect) -> Self {
        self.hollow(rect.min.x, rect.min.y)
            .line(rect.min.x, rect.max.y)
            .line(rect.max.x, rect.max.y)
            .line(rect.max.x, rect.min.y)
            .closed()
    }

    pub fn circle(self, position: &Point, radius: f32) -> Self {
        let start_of_circle = Point { x: position.x + radius, y: position.y };
        let half_circle = Point { x: position.x - radius, y: position.y };
        self.appended(GeometryElement::Start {
            start: start_of_circle,
            filled: false,
        }).appended(GeometryElement::Arc{
            end_point: half_circle,
            radius,
            angle: 0.0,
            direction: CircleDirection::CounterClockWise
        }).appended(GeometryElement::Arc{
            end_point: start_of_circle,
            radius,
            angle: 0.0,
            direction: CircleDirection::CounterClockWise
        }).closed()
    }
}
