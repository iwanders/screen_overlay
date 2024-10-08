#![allow(unused_variables, unused_imports, unreachable_code)]
use crate::{
    CapStyle, CircleDirection, Color, DashStyle, DrawGeometry, Error, GeometryElement, LineJoin,
    LineStyle, OverlayConfig, Point, Rect, Stroke, TextAlignment, TextProperties,
};

/*
    We can probably draw on https://github.com/ftorkler/x11-overlay for a lot of the logic.
*/

use x11_dl::xlib::{self, TrueColor, Xlib, _XDisplay};
use x11_dl::{xfixes, xft, xrender};

use std::sync::Arc;

#[derive(Clone)]
pub struct ImageTexture {}
impl std::fmt::Debug for ImageTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "ImageTexture {:?}", &self)
    }
}

#[derive(Clone)]
pub struct PreparedFont {
    display: *mut _XDisplay,
    font: *mut xft::XftFont,
}
impl std::fmt::Debug for PreparedFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "PreparedFont {:?}", &self)
    }
}
impl Drop for PreparedFont {
    fn drop(&mut self) {
        unsafe {
            let xft = xft::Xft::open();
            if xft.is_err() {
                return; // how can we handle this? return of drop is void.
            }
            let font = (xft.unwrap().XftFontClose)(self.display, self.font);
        }
    }
}

// pub type IDVisual = usize;
#[derive(Clone, Debug)]
pub enum IDVisual {
    Text { xft_draw: *mut xft::XftDraw },
    None,
}
impl Drop for IDVisual {
    fn drop(&mut self) {
        unsafe {
            match self {
                IDVisual::Text { xft_draw } => {
                    let xft = xft::Xft::open();
                    if xft.is_err() {
                        return; // how can we handle this? return of drop is void.
                    }
                    let font = (xft.unwrap().XftDrawDestroy)(*xft_draw);
                }
                IDVisual::None => {}
            }
        }
    }
}

pub struct OverlayImpl {
    instance: Xlib,
    display: *mut _XDisplay,
    screen: Option<i32>,
    window: Option<u64>,
    visual_info: Option<xlib::XVisualInfo>,
}
unsafe impl Send for OverlayImpl {}

impl OverlayImpl {
    pub fn new() -> Result<Self, Error> {
        let instance = xlib::Xlib::open()?;
        let display = unsafe { (instance.XOpenDisplay)(std::ptr::null()) };
        if display.is_null() {
            return Err("failed to retrieve display ptr".into());
        }
        Ok(Self {
            instance,
            display,
            screen: None,
            window: None,
            visual_info: None,
        })
    }

    pub fn create_window(&mut self, config: &OverlayConfig) -> Result<(), Error> {
        unsafe {
            let screen = (self.instance.XDefaultScreen)(self.display);
            let root_window = (self.instance.XDefaultRootWindow)(self.display);
            // println!("Screen: {screen:?}");
            // println!("root_window: {root_window:?}");

            let mut attributes: xlib::XWindowAttributes =
                std::mem::MaybeUninit::zeroed().assume_init();
            let status =
                (self.instance.XGetWindowAttributes)(self.display, root_window, &mut attributes);
            if status != 1 {
                return Err("failed to retrieve root window attributes".into());
            }
            // println!("attributes: {attributes:?}");
            let root_width = attributes.width;
            let root_height = attributes.height;

            let mut visual_info = std::mem::MaybeUninit::<xlib::XVisualInfo>::uninit();

            let status = (self.instance.XMatchVisualInfo)(
                self.display as _,
                screen as i32,
                32,
                xlib::TrueColor,
                visual_info.as_mut_ptr(),
            );
            // https://tronche.com/gui/x/xlib/utilities/XMatchVisualInfo.html:
            // If a visual is found, XMatchVisualInfo() returns nonzero and the information on the visual to vinfo_return.
            // yet that seems not to be the case, it clearly returns 0 on errors.
            if status == 0 {
                return Err("failed to retrieve visual info".into());
            }
            let visual_info = visual_info.assume_init();
            // println!("visual_info: {visual_info:?}");

            let mut attributes: xlib::XSetWindowAttributes =
                std::mem::MaybeUninit::zeroed().assume_init();
            attributes.colormap = (self.instance.XCreateColormap)(
                self.display,
                root_window,
                visual_info.visual,
                xlib::AllocNone,
            );
            attributes.border_pixel = (self.instance.XBlackPixel)(self.display, screen);
            attributes.background_pixel = (self.instance.XBlackPixel)(self.display, screen);
            attributes.override_redirect = true as i32;
            let attr_mask = xlib::CWColormap
                | xlib::CWBorderPixel
                | xlib::CWBackPixel
                | xlib::CWOverrideRedirect;
            let x = 0;
            let y = 0;
            let window = (self.instance.XCreateWindow)(
                self.display,
                root_window,
                0,
                0,
                root_width as _,
                root_height as _,
                0,
                visual_info.depth,
                xlib::InputOutput as _,
                visual_info.visual,
                attr_mask,
                &mut attributes,
            );
            if window == 0 {
                return Err("failed to create window".into());
            }
            // println!("window: {window:?}");
            let xlib_fixes = xfixes::Xlib::open()?;
            let region = (xlib_fixes.XFixesCreateRegion)(self.display, std::ptr::null_mut(), 0);
            // println!("region: {region:?}");
            #[allow(non_upper_case_globals)]
            const ShapeInput: i32 = 2;
            (xlib_fixes.XFixesSetWindowShapeRegion)(self.display, window, ShapeInput, 0, 0, region);
            (xlib_fixes.XFixesDestroyRegion)(self.display, region);
            (self.instance.XMapWindow)(self.display, window);
            self.window = Some(window);
            self.visual_info = Some(visual_info);
            self.screen = Some(screen);
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
        Ok(IDVisual::None)
    }

    pub fn prepare_font(&mut self, properties: &TextProperties) -> Result<PreparedFont, Error> {
        unsafe {
            let xft = xft::Xft::open()?;
            let font_descriptor =
                format!("{}:pixelsize={}", properties.font, properties.size as i32);
            let font_name = std::ffi::OsString::from(&font_descriptor);
            let font_str = std::mem::transmute::<*const u8, _>(
                font_name.as_os_str().as_encoded_bytes().as_ptr(),
            );
            let font = (xft.XftFontOpenName)(
                self.display,
                *self.screen.as_ref().ok_or("no screen")?,
                font_str,
            );
            // println!("font prop: {font:?}");
            Ok(PreparedFont {
                display: self.display,
                font,
            })
        }
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        layout: &Rect,
        color: &Color,
        font: &PreparedFont,
    ) -> Result<IDVisual, Error> {
        // println!("would print {text}");
        unsafe {
            let xft = xft::Xft::open()?;
            let screen = *self
                .screen
                .as_ref()
                .ok_or("draw_text called without screen created")?;
            let window = *self
                .window
                .as_ref()
                .ok_or("draw_text called without window created")?;

            // This is a bit of a hack.
            (self.instance.XClearWindow)(self.display, window);

            let visual_info = *self
                .visual_info
                .as_ref()
                .ok_or("draw_text called without window created")?;
            let colormap = (self.instance.XDefaultColormap)(self.display, screen);
            let xft_draw = (xft.XftDrawCreate)(self.display, window, visual_info.visual, colormap);
            let mut xft_color: xft::XftColor = std::mem::MaybeUninit::zeroed().assume_init();

            let mut render_color: xrender::XRenderColor =
                std::mem::MaybeUninit::zeroed().assume_init();
            render_color.red = ((color.r_f32() * color.a_f32()) * 255.0) as u16 * 255;
            render_color.green = ((color.g_f32() * color.a_f32()) * 255.0) as u16 * 255;
            render_color.blue = ((color.b_f32() * color.a_f32()) * 255.0) as u16 * 255;
            render_color.alpha = color.a as u16 * 255;
            let status = (xft.XftColorAllocValue)(
                self.display,
                visual_info.visual,
                colormap,
                &render_color,
                &mut xft_color,
            );
            if status == 0 {
                return Err("could not allocate color".into());
            }

            // XftDrawStringUtf8(xftDraw, &xftColor, xftFont, x, y + xftFont->ascent, (const FcChar8*)text.c_str(), text.size());
            let x = layout.min.x as i32;
            let y = layout.min.y as i32 + (*(font.font)).ascent;
            // println!("x: {x}, y: {y}");
            let b: Vec<u8> = text
                .as_bytes()
                .iter()
                .chain([0u8].iter())
                .copied()
                .collect();
            (xft.XftDrawStringUtf8)(
                xft_draw,
                &xft_color,
                font.font,
                x,
                y,
                b.as_ptr(),
                b.len() as i32,
            );

            (self.instance.XFlush)(self.display);
            // XftDrawDestroy XftColorFree!
            Ok(IDVisual::Text { xft_draw })
        }
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
        Ok(IDVisual::None)
    }

    pub fn remove_visual(&mut self, visual: &IDVisual) -> Result<(), Error> {
        Ok(())
    }
}

pub fn run_msg_loop() -> Result<(), Error> {
    unsafe {
        let instance = xlib::Xlib::open()?;
        let display = (instance.XOpenDisplay)(std::ptr::null());
        let mut event: xlib::XEvent = std::mem::MaybeUninit::zeroed().assume_init();
        (instance.XNextEvent)(display, &mut event);
    }
    Ok(())
}

pub fn setup() -> Result<(), Error> {
    Ok(())
}
