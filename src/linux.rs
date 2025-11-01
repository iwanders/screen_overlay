#![allow(unused_variables, unused_imports, unreachable_code)]
use crate::{
    CapStyle, CircleDirection, Color, DashStyle, DrawGeometry, Error, GeometryElement, LineJoin,
    LineStyle, OverlayConfig, Point, Rect, Stroke, TextAlignment, TextProperties,
};

/*
    We can probably draw on https://github.com/ftorkler/x11-overlay for a lot of the logic.
*/

/*
 Okay so x11 does not support transparancy or alpha blending, it only supports binary alpha.
 Transparant images sort-of-look like they're right, but they behave as if the background colo
 is always a single color.

 We need to use xrender... but then the question is where do we do our actual image composition
 and rendering.
*/

use x11_dl::xlib::{self, Display, Pixmap, TrueColor, Visual, XErrorEvent, XImage, Xlib, GC};
use x11_dl::{xfixes, xft, xrender, xrender::Xrender};

use std::sync::Arc;

#[derive(Clone)]
pub struct ImageTexture {
    display: *mut Display,
    pm: Pixmap,
}
impl std::fmt::Debug for ImageTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "ImageTexture {:?}", &self)
    }
}
impl Drop for ImageTexture {
    fn drop(&mut self) {
        unsafe {
            let instance = xlib::Xlib::open().unwrap();
            (instance.XFreePixmap)(self.display, self.pm);
        }
    }
}

#[derive(Clone)]
pub struct PreparedFont {
    display: *mut Display,
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

/// We manually handle the panic, otherwise we get a bunch of frames that we don't care about because we captured the bt
/// from a function in which the unwind isn't possible. This gives us the relevant bt though.
fn panicker() -> ! {
    let ln = line!();
    let bt = std::backtrace::Backtrace::force_capture();
    let mut text = format!("{:}", bt);
    // Lets crop this a bit, because we really don't care about anything beyond std::sys::backtrace
    let loc = text.find("std::sys::backtrace");
    if let Some(z) = loc {
        text.truncate(z);
        // Also walk backwards to find the first newline now and cut from there.
        if let Some(v) = text.rfind(|z| z == '\n') {
            text.truncate(v);
        }
    };
    eprintln!("{}", text);
    std::process::exit(2);
}

extern "C" fn error_handler(display: *mut Display, event: *mut XErrorEvent) -> i32 {
    let instance = xlib::Xlib::open().unwrap();

    let code = unsafe { (*event).error_code as i32 };
    let mut buffer = [0u8; 256];
    unsafe {
        (instance.XGetErrorText)(
            display,
            code,
            buffer.as_mut_ptr() as *mut i8,
            buffer.len() as i32,
        )
    };
    let length = buffer.iter().position(|z| z == &0).unwrap();
    if length > 0 {
        let message =
            std::str::from_utf8(&buffer[..length as usize]).unwrap_or("<invalid message>");
        eprintln!("X Error: {}", message);
    } else {
        eprintln!("X Error: unknown error");
    }
    panicker();
    return 0;
}

// pub type IDVisual = usize;
#[derive(Clone, Debug)]
pub enum IDVisual {
    Text { xft_draw: *mut xft::XftDraw },
    Image { gc: GC, display: *mut Display },
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
                IDVisual::Image { gc, display } => {
                    println!("Clearing image");
                    let instance = xlib::Xlib::open().unwrap();
                    (instance.XFreeGC)(*display, *gc);
                }
                IDVisual::None => {}
            }
        }
    }
}

#[macro_export]
macro_rules! xflush {
    ($self:expr) => {
        unsafe {
            ($self.instance.XFlush)($self.display);
        }
    };
}

pub struct OverlayImpl {
    instance: Xlib,
    xrender: Xrender,
    display: *mut Display,
    default_visual: Option<*mut Visual>,
    screen: Option<i32>,
    window: Option<u64>,
    visual_info: Option<xlib::XVisualInfo>,
    gc: Option<GC>,
}
unsafe impl Send for OverlayImpl {}

impl OverlayImpl {
    pub fn new() -> Result<Self, Error> {
        let instance = xlib::Xlib::open()?;
        let xrender = xrender::Xrender::open()?;
        let display = unsafe { (instance.XOpenDisplay)(std::ptr::null()) };
        if display.is_null() {
            return Err("failed to retrieve display ptr".into());
        }

        unsafe {
            (instance.XSetErrorHandler)(Some(error_handler));
        }

        Ok(Self {
            instance,
            xrender,
            display,
            screen: None,
            window: None,
            visual_info: None,
            default_visual: None,
            gc: None,
        })
    }

    pub fn create_window(&mut self, config: &OverlayConfig) -> Result<(), Error> {
        unsafe {
            let screen = (self.instance.XDefaultScreen)(self.display);
            let root_window = (self.instance.XDefaultRootWindow)(self.display);
            let default_visual = (self.instance.XDefaultVisual)(self.display, screen);
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

            // Verifying that we have direct alpha blending and pict type direct;
            let fmt = (self.xrender.XRenderFindVisualFormat)(self.display, visual_info.visual);
            if (*fmt).type_ != xrender::PictTypeDirect || (*fmt).direct.alpha == 0 {
                return Err("could not fing rgba visual".into());
            }
            if visual_info.depth != 32 {
                return Err("visual depth is not 32 ".into());
            }

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
            attributes.event_mask = xlib::ExposureMask;

            let attr_mask = xlib::CWColormap
                | xlib::CWBorderPixel
                | xlib::CWBackPixel
                | xlib::CWOverrideRedirect
                | xlib::CWEventMask;

            let x = 0;
            let y = 0;

            let window = (self.instance.XCreateWindow)(
                self.display,
                root_window,
                x,
                y,
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
            // This sets the input region to zero.
            // println!("window: {window:?}");
            let xlib_fixes = xfixes::Xlib::open()?;
            let region = (xlib_fixes.XFixesCreateRegion)(self.display, std::ptr::null_mut(), 0);
            // println!("region: {region:?}");
            #[allow(non_upper_case_globals)]
            const ShapeInput: i32 = 2;
            (xlib_fixes.XFixesSetWindowShapeRegion)(self.display, window, ShapeInput, 0, 0, region);
            (xlib_fixes.XFixesDestroyRegion)(self.display, region);

            // Are these XChangeProperties necessary??
            let net_wm_state =
                (self.instance.XInternAtom)(self.display, "_NET_WM_STATE".as_ptr() as *const i8, 0);
            let wm_state_above = (self.instance.XInternAtom)(
                self.display,
                b"_NET_WM_STATE_ABOVE".as_ptr() as *const i8,
                0,
            );
            let wm_on_top = (self.instance.XInternAtom)(
                self.display,
                b"_NET_WM_STATE_STAYS_ON_TOP".as_ptr() as *const i8,
                0,
            );
            (self.instance.XChangeProperty)(
                self.display,
                window,
                net_wm_state,
                xlib::XA_ATOM,
                32,
                xlib::PropModeReplace,
                (&wm_state_above as *const u64) as *const u8,
                1,
            );
            (self.instance.XChangeProperty)(
                self.display,
                window,
                wm_on_top,
                xlib::XA_ATOM,
                32,
                xlib::PropModeReplace,
                (&wm_state_above as *const u64) as *const u8,
                1,
            );
            (self.instance.XMapWindow)(self.display, window);

            let gc = (self.instance.XCreateGC)(self.display, window, 0, std::ptr::null_mut());
            self.window = Some(window);
            self.visual_info = Some(visual_info);
            self.screen = Some(screen);
            self.default_visual = Some(default_visual);
            self.gc = Some(gc);
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
            let font_cstr = std::ffi::CString::new(font_name.as_os_str().as_encoded_bytes())?;
            let font = (xft.XftFontOpenName)(
                self.display,
                *self.screen.as_ref().ok_or("no screen")?,
                font_cstr.as_ptr(),
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
            let b: Vec<u8> = text.as_bytes().iter().copied().collect();
            (xft.XftDrawStringUtf8)(
                xft_draw,
                &xft_color,
                font.font,
                x,
                y,
                b.as_ptr(),
                b.len() as i32,
            );

            // XftDrawDestroy XftColorFree!
            Ok(IDVisual::Text { xft_draw })
        }
    }

    pub fn load_texture<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
    ) -> Result<ImageTexture, Error> {
        // This loads the image to a pixmap, that we can utilise later.
        let mut img = image::ImageReader::open(path)?.decode()?.to_rgba8();

        for (_x, _y, pixel) in img.enumerate_pixels_mut() {
            let p = pixel.0;
            let a = p[3];
            // (*pixel).0 = [p[2], p[1], p[0], a];
            (*pixel).0 = [a, p[0], p[1], p[2]];
        }

        let width = img.width();
        let height = img.height();

        let visual = *self.visual_info.as_ref().unwrap();

        // XDestroyImage() function calls frees both the image structure and the data pointed to by the image structure.
        // Need to transfer ownership of the bytes.
        let mut raw_container = img.into_raw();
        for i in 0..raw_container.len() / 4 {
            // raw_container[i * 4] = 0;
            // raw_container[i * 4 + 1] = 0;
            // raw_container[i * 4 + 2] = 0;
            // raw_container[i * 4 + 3] = 255;
        }

        let data = raw_container.as_ptr();

        let window = self.window.unwrap();
        let image = unsafe {
            (self.instance.XCreateImage)(
                self.display,
                visual.visual,
                //visual.depth as u32,
                32,
                xlib::ZPixmap,
                0, // offset
                data as *mut i8,
                width as u32,
                height as u32,
                32,               // bitmap pad
                (width * 4) as _, // bytes per line
            )
        };
        if image.is_null() {
            return Err("image creation failed".into());
        }

        let data = raw_container.as_ptr();
        let pm = unsafe {
            (self.instance.XCreatePixmap)(
                self.display,
                window, // only used for depth properties.
                width as u32,
                height as u32,
                32, // bitmap pad
            )
        };
        unsafe { (self.instance.XFlush)(self.display) };
        if pm == 0 {
            return Err("image creation failed".into());
        }
        let drawable = self.window.unwrap();

        let gc = self.gc.unwrap();

        unsafe { (self.instance.XFlush)(self.display) };
        let _ = unsafe {
            (self.instance.XPutImage)(
                self.display,
                pm,
                gc,
                image,
                0, // src
                0,
                0, // destination, drawable.
                0,
                width as u32,
                height as u32,
            )
        };

        unsafe { (self.instance.XFlush)(self.display) };

        // Image creation succeeded, leak the data because the X11 image owns it now and will clean it up on drop.
        raw_container.leak();

        Ok(ImageTexture {
            pm,
            display: self.display,
        })
    }

    pub fn draw_texture(
        &mut self,
        position: &Point,
        texture: &ImageTexture,
        texture_region: &Rect,
        color: &Color,
        alpha: f32,
    ) -> Result<IDVisual, Error> {
        let drawable = self.window.unwrap();
        let gc = self.gc.unwrap();

        println!("gc: {:?}", gc);

        // Do we first have to create an XRenderCreatePicture?
        //
        unsafe {
            let d = *self.window.as_ref().unwrap();
            println!("before find standard");
            let fmt =
                (self.xrender.XRenderFindStandardFormat)(self.display, xrender::PictStandardARGB32);
            let visual = self.visual_info.as_ref().unwrap();
            let fmtrgb =
                (self.xrender.XRenderFindStandardFormat)(self.display, xrender::PictStandardRGB24);
            let window_fmt = (self.xrender.XRenderFindVisualFormat)(
                self.display,
                (self.instance.XDefaultVisual)(self.display, self.screen.unwrap()),
            );
            println!("below standard format; XDefaultVisual {:?}", window_fmt);
            let window_fmt = (self.xrender.XRenderFindVisualFormat)(self.display, visual.visual);
            println!("below standard XRenderFindVisualFormat; fmt {:?}", fmt);
            println!("below standard format; window_fmt {:?}", window_fmt);
            println!("below standard format; fmtrgb {:?}", fmtrgb);
            let p = (self.xrender.XRenderCreatePicture)(
                self.display,
                texture.pm,
                window_fmt,
                0,
                std::ptr::null(),
            );

            let wp = (self.xrender.XRenderCreatePicture)(
                self.display,
                self.window.unwrap(),
                window_fmt,
                0,
                std::ptr::null(),
            );
            unsafe { (self.instance.XFlush)(self.display) };

            /*
            XRenderFillRectangle (Display		    *dpy,
                  int		    op,
                  Picture		    dst,
                  _Xconst XRenderColor  *color,
                  int		    x,
                  int		    y,
                  unsigned int	    width,
                  unsigned int	    height)*/
            let mut render_color: xrender::XRenderColor =
                std::mem::MaybeUninit::zeroed().assume_init();
            let r = 255; // This rectangle works.
            let g = 0;
            let b = 0;
            let alpha = 255;
            render_color.red = (r & 0xFF) * 257; // 8bit to 16bit
            render_color.green = (g & 0xFF) * 257;
            render_color.blue = (b & 0xFF) * 257;
            render_color.alpha = alpha * 257;
            (self.xrender.XRenderFillRectangle)(
                self.display,
                xrender::PictOpSrc, // is this just assign?
                p,                  // writing to wp here fails.
                &render_color,
                0,
                0,
                10,
                10,
            );
            render_color.blue = 255 * 257;
            render_color.alpha = 128 * 257;
            (self.xrender.XRenderFillRectangle)(
                self.display,
                xrender::PictOpSrc, // is this just assign?
                p,
                &render_color,
                30,
                0,
                20,
                20,
            );

            // Next up is rendering the image on the gc.
            //
            //    XRenderComposite (Display   *dpy,
            // int	    op,
            // Picture   src,
            // Picture   mask,
            // Picture   dst,
            // int	    src_x,
            // int	    src_y,
            // int	    mask_x,
            // int	    mask_y,
            // int	    dst_x,
            // int	    dst_y,
            // unsigned int	width,
            // unsigned int	height)

            unsafe {
                (self.xrender.XRenderComposite)(
                    self.display,
                    // xrender::PictOpOver as i32,
                    xrender::PictOpSrc as i32,
                    p,  // src
                    0,  // mask
                    wp, // dest
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    texture_region.width() as u32,
                    texture_region.height() as u32,
                );
            } /**/
            (self.xrender.XRenderFreePicture)(self.display, wp);
            unsafe { (self.instance.XFlush)(self.display) };
        }

        Ok(IDVisual::Image {
            gc,
            display: self.display,
        })
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
