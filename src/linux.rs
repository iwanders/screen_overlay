#![allow(unused_variables, unused_imports, unreachable_code)]
use crate::{
    CapStyle, CircleDirection, Color, DashStyle, DrawGeometry, Error, GeometryElement, LineJoin,
    LineStyle, OverlayConfig, Point, Rect, Stroke, TextAlignment, TextProperties,
};

/*
    We can probably draw on https://github.com/ftorkler/x11-overlay for a lot of the logic.
*/

use x11_dl::xlib::{self, Xlib, _XDisplay, TrueColor};
use x11_dl::xfixes;

use std::sync::Arc;

#[derive(Clone)]
pub struct ImageTexture {}
impl std::fmt::Debug for ImageTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "ImageTexture {:?}", &self)
    }
}

#[derive(Clone)]
pub struct PreparedFont {}
impl std::fmt::Debug for PreparedFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "PreparedFont {:?}", &self)
    }
}

pub type IDVisual = usize;

pub struct OverlayImpl {
    instance: Xlib,
    display: *mut _XDisplay,
    window: Option<u64>,
}
unsafe impl Send for OverlayImpl{}

impl OverlayImpl {
    pub fn new() -> Result<Self, Error> {
        let instance = xlib::Xlib::open()?;
        let display = unsafe { (instance.XOpenDisplay)(std::ptr::null()) };
        if display.is_null() {
            return Err("failed to retrieve display ptr".into());
        }
        Ok(Self {instance, display, window:None})
    }

    pub fn create_window(&mut self, config: &OverlayConfig) -> Result<(), Error> {
        unsafe {
            let screen = (self.instance.XDefaultScreen)(self.display);
            let root_window = (self.instance.XDefaultRootWindow)(self.display);
            println!("Screen: {screen:?}");
            println!("root_window: {root_window:?}");


            let mut attributes : xlib::XWindowAttributes = std::mem::MaybeUninit::zeroed().assume_init();
            let status = (self.instance.XGetWindowAttributes)(self.display, root_window, &mut attributes);
            if status != 1 {
                return Err("failed to retrieve root window attributes".into());
            }
            println!("attributes: {attributes:?}");
            let root_width = attributes.width;
            let root_height = attributes.height;

            let mut visual_info = std::mem::MaybeUninit::<xlib::XVisualInfo>::uninit();

            let status = (self.instance.XMatchVisualInfo)(self.display as _, screen as i32, 32, xlib::TrueColor, visual_info.as_mut_ptr());
            // https://tronche.com/gui/x/xlib/utilities/XMatchVisualInfo.html:
            // If a visual is found, XMatchVisualInfo() returns nonzero and the information on the visual to vinfo_return.
            // yet that seems not to be the case, it clearly returns 0 on errors.
            if status == 0 {
                return Err("failed to retrieve visual info".into());
            }
            let visual_info = visual_info.assume_init();
            println!("visual_info: {visual_info:?}");


            let mut attributes: xlib::XSetWindowAttributes = std::mem::MaybeUninit::zeroed().assume_init();
            attributes.colormap = (self.instance.XCreateColormap)(self.display, root_window, visual_info.visual, xlib::AllocNone);
            attributes.border_pixel = (self.instance.XBlackPixel)(self.display, screen);
            attributes.background_pixel = (self.instance.XBlackPixel)(self.display, screen);
            attributes.override_redirect = true as i32;
            let attr_mask = xlib::CWColormap | xlib::CWBorderPixel | xlib::CWBackPixel | xlib::CWOverrideRedirect;
            let x = 0;
            let y = 0;
            let window = (self.instance.XCreateWindow)(self.display, root_window, 0, 0, root_width as _, root_height as _, 0, visual_info.depth, xlib::InputOutput as _, visual_info.visual, attr_mask, &mut attributes);
            if window == 0 {
                return Err("failed to create window".into());
            }
            println!("window: {window:?}");
            let xlib_fixes = xfixes::Xlib::open()?;
            let region = (xlib_fixes.XFixesCreateRegion)(self.display, std::ptr::null_mut(), 0);
            println!("region: {region:?}");
            #[allow(non_upper_case_globals)]
            const ShapeInput: i32 = 2;
            (xlib_fixes.XFixesSetWindowShapeRegion)(self.display, window, ShapeInput, 0, 0, region);
            (xlib_fixes.XFixesDestroyRegion)(self.display, region);
            (self.instance.XMapWindow)(self.display, window);
            self.window = Some(window);
        }
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
        Ok(3)
    }

    pub fn prepare_font(&mut self, properties: &TextProperties) -> Result<PreparedFont, Error> {
        Ok(PreparedFont {})
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        layout: &Rect,
        color: &Color,
        font: &PreparedFont,
    ) -> Result<IDVisual, Error> {
        println!("would print {text}");
        Ok(3)
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
        Ok(3)
    }

    pub fn remove_visual(&mut self, visual: &IDVisual) -> Result<(), Error> {
        Ok(())
    }
}
// Is this legal?
// unsafe impl Send for OverlayImpl {}
pub fn run_msg_loop() -> Result<(), Error> {
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Ok(())
}

pub fn setup() -> Result<(), Error> {
    Ok(())
}
