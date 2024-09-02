use std::sync::Arc;

// This started based on the direct composition example:
// https://github.com/microsoft/windows-rs/tree/ef06753b0df2aaa16894416191bcde328b9d6ffb/crates/samples/windows/dcomp

// API
//  - Drawable -> Returns RAII handle with interface to drawable.
//  - Should be thread safe (all of it)
//  - Need a wrapper with an interior Arc.


mod windows;
use windows::{OverlayImpl, IDVisual, ImageTexture};

use parking_lot::Mutex;
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

struct VisualToken {
    overlay: Arc<Mutex<OverlayImpl>>,
    visual: IDVisual,
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

impl Overlay {
    pub fn new() -> std::result::Result<Overlay, Error> {
        let window = Arc::new(Mutex::new(OverlayImpl::new()?));
        {
            let mut wlock = window.lock();
            wlock.create_window()?;
            wlock.create_device_resources()?;
        }
        Ok(Self { overlay: window })
    }

    pub fn draw_geometry(&self, geometry: &DrawGeometry, stroke: &Stroke) -> std::result::Result<VisualToken, Error> {
        {
            let mut wlock = self.overlay.lock();
            let visual = wlock.draw_geometry(geometry, stroke)?;
            Ok(VisualToken {
                visual,
                overlay: self.overlay.clone(),
            })
        }
    }

    pub fn draw_text(&self, text: &str, layout: &Rect, color: &Color) -> std::result::Result<VisualToken, Error> {
        {
            let mut wlock = self.overlay.lock();
            let visual = wlock.draw_text(text, layout, color)?;
            Ok(VisualToken {
                visual,
                overlay: self.overlay.clone(),
            })
        }
    }
    pub fn load_texture<P: AsRef<std::path::Path>>(&self, path: P) -> std::result::Result<ImageTexture, Error> {
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
    pub fn draw_texture(&self, position: &Point, texture: &ImageTexture, texture_region: &Rect, color: &Color, alpha: f32) -> std::result::Result<VisualToken, Error> {
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


#[derive(Copy, Clone, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
impl Color {
    pub const TRANSPARENT: Color = Color{r: 0, g: 0, b: 0, a: 0};
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

#[derive(Copy, Clone, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    const ORIGIN: Point = Point{x: 0.0, y: 0.0};
    pub fn new(x: f32, y: f32) -> Self {
        Self{x, y        }
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
pub struct Rect{
    pub min: Point,
    pub max: Point,
}
impl Rect {
    pub fn from(x: f32, y: f32) -> Self {
        Self {
            min: Point{x, y},
            max: Point::ORIGIN
        }
    }
    pub fn sized(self, w: f32, h: f32) -> Self{
        Self {
            min: self.min,
            max: self.min + Point::new(w,h)
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
struct DrawGeometry {
    pub elements: Vec<GeometryElement>,
}
impl DrawGeometry {
    pub fn new() -> Self {
        Self {
            elements: vec![]
        }
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

    pub fn rectangle(self, min: &Point, max: &Point) -> Self {
        self.hollow(min.x, min.y).line(min.x, max.y).line(max.x, max.y).line(max.x, min.y).closed()
    }
}

pub fn main() -> std::result::Result<(), Error> {
    windows::setup()?;
    let window = Overlay::new()?;


    let twindow = window.clone();
    let msg_loop_thread = std::thread::spawn(move || {


        let color = Color {
            r: 255,
            g: 0,
            b: 255,
            a: 128,
        };
        let alpha = 0.5;
        let image2 = twindow.load_texture(std::path::PathBuf::from("PNG_transparency_demonstration_1.png")).expect("failed to load image");
        let t2 = twindow.draw_texture(&Point::new(500.0, 500.0), &image2, &Rect::from(0.0, 0.0).sized(200.0, 200.0), &color, alpha).expect("texture draw failed");

        let image = twindow.load_texture(std::path::PathBuf::from("image.jpg")).expect("failed to load image");
        let t = twindow.draw_texture(&Point::new(1000.0, 500.0), &image, &Rect::from(0.0, 0.0).sized(200.0, 200.0), &Color::TRANSPARENT, alpha).expect("texture draw failed");



        std::thread::sleep(std::time::Duration::from_millis(1000));

        let z =   {
            let geometry = DrawGeometry::new().hollow(200.0, 10.0)
                .line(100.0, 100.0)
                .closed();
            let color = Color {
                r: 255,
                g: 0,
                b: 255,
                a: 255,
            };
            let stroke = Stroke { color, width: 30.0 };

            let v = twindow
                .draw_geometry(&geometry, &stroke)
                .expect("create image failed");
            std::thread::sleep(std::time::Duration::from_millis(500));
            v
        };
        if true {
            let color = Color {
                r: 255,
                g: 0,
                b: 255,
                a: 255,
            };
            let v = twindow.draw_text("hello", &Rect::from(200.0, 200.0).sized(600.0, 300.0), &color).expect("create image failed");
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
        println!("blocking now");
        std::thread::sleep(std::time::Duration::from_millis(1000000));
    });
    Ok(windows::run_msg_loop()?)

    // Ok(())
}
