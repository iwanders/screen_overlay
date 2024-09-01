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

    pub fn create_image(&self) -> std::result::Result<(), Error> {
        {
            let mut wlock = self.overlay.lock();
            Ok(wlock.create_image()?)
        }
    }

    pub fn draw_line(&self) -> std::result::Result<VisualToken, Error> {
        {
            let mut wlock = self.overlay.lock();
            let visual = wlock.draw_line()?;
            Ok(VisualToken {
                visual,
                overlay: self.overlay.clone(),
            })
        }
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
    pub fn draw_texture(&self, position: &Point, texture: &ImageTexture, texture_region: &Rect) -> std::result::Result<VisualToken, Error> {
        {
            let mut wlock = self.overlay.lock();
            let visual = wlock.draw_texture(position, texture, texture_region)?;
            Ok(VisualToken {
                visual,
                overlay: self.overlay.clone(),
            })
        }
    }
}


#[derive(Copy, Clone, Debug)]
struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Copy, Clone, Debug)]
struct Stroke {
    pub color: Color,
    pub width: f32,
}

#[derive(Copy, Clone, Debug)]
struct Point {
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
struct Rect{
    min: Point,
    max: Point,
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
    fn appended(self, e: GeometryElement) -> Self {
        let mut elements = self.elements;
        elements.push(e);
        Self { elements }
    }
    pub fn hollow(x: f32, y: f32) -> Self {
        Self {
            elements: vec![GeometryElement::Start {
                start: Point { x, y },
                filled: false,
            }],
        }
    }
    pub fn closed(self) -> Self {
        self.appended(GeometryElement::End { closed: true })
    }
    pub fn line(self, x: f32, y: f32) -> Self {
        self.appended(GeometryElement::Line(Point { x, y }))
    }
}

pub fn main() -> std::result::Result<(), Error> {
    windows::setup()?;
    let window = Overlay::new()?;


    let twindow = window.clone();
    let msg_loop_thread = std::thread::spawn(move || {

        let image2 = twindow.load_texture(std::path::PathBuf::from("PNG_transparency_demonstration_1.png")).expect("failed to load image");
        let t2 = twindow.draw_texture(&Point::new(500.0, 500.0), &image2, &Rect::from(0.0, 0.0).sized(200.0, 200.0)).expect("texture draw failed");

        let image = twindow.load_texture(std::path::PathBuf::from("image.jpg")).expect("failed to load image");
        let t = twindow.draw_texture(&Point::new(1000.0, 500.0), &image, &Rect::from(0.0, 0.0).sized(200.0, 200.0)).expect("texture draw failed");


        std::thread::sleep(std::time::Duration::from_millis(1000));
        twindow.create_image().expect("create image failed");
        std::thread::sleep(std::time::Duration::from_millis(1000));
        if true {
            let v = twindow.draw_line().expect("create image failed");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        let z =   {
            let geometry = DrawGeometry::hollow(200.0, 10.0)
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
        if false {
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
