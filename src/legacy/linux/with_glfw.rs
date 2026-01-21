#![allow(unused_variables, unused_imports, unreachable_code)]
use crate::legacy::{
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

use glfw::PWindow;
use std::sync::Arc;
use x11_dl::xlib::{self, Display, GC, Pixmap, TrueColor, Visual, XErrorEvent, XImage, Xlib};
use x11_dl::{xfixes, xft, xrender, xrender::Xrender};
extern crate glfw;
use glfw::{Action, Context, Key};

use parking_lot::RwLock;
use std::collections::HashMap;
use three_d::Context as Context3D;
use three_d::{CpuTexture, Viewport};

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
struct DrawId(usize);
struct DrawTexture {
    rectangle: three_d::Rectangle,
    material: three_d::ColorMaterial,
}

impl std::fmt::Debug for DrawTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "DrawTexture")
    }
}

type ComponentState = Arc<RwLock<DrawComponents>>;
#[derive(Debug)]
struct DrawComponents {
    textures: HashMap<DrawId, DrawTexture>,
    id_counter: usize,
}
impl DrawComponents {
    fn new() -> Arc<RwLock<Self>> {
        RwLock::new(Self {
            textures: Default::default(),
            id_counter: 0,
        })
        .into()
    }
    fn add_draw_texture(&mut self, draw_texture: DrawTexture) -> DrawId {
        let id = DrawId(self.id_counter);
        self.id_counter += 1;
        self.textures.insert(id, draw_texture);
        id
    }
    fn remove_id(&mut self, id: DrawId) {
        let _ = self.textures.remove(&id);
    }

    fn sprites(&self) -> Vec<three_d::Gm<&three_d::Rectangle, &three_d::ColorMaterial>> {
        self.textures
            .iter()
            .map(|(_, dt)| three_d::Gm {
                geometry: &dt.rectangle,
                material: &dt.material,
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct ImageTexture {
    texture: CpuTexture,
}
impl std::fmt::Debug for ImageTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "ImageTexture {:?}", &self)
    }
}
impl Drop for ImageTexture {
    fn drop(&mut self) {}
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
    DrawId {
        id: DrawId,
        components: ComponentState,
    },
    None,
}
impl Drop for IDVisual {
    fn drop(&mut self) {
        unsafe {
            match self {
                IDVisual::DrawId { id, components } => {
                    let mut l = components.write();
                    l.remove_id(*id);
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
    // Libs
    instance: Xlib,
    xrender: Xrender,
    glx: x11_dl::glx::Glx,
    glfw: glfw::Glfw,
    // X11 stuff
    display: *mut Display,
    screen: Option<i32>,
    // winit window and size
    window: Option<PWindow>,
    window_size: Option<Rect>,
    // The three_d context
    context: Option<Context3D>,
    viewport: Option<Viewport>,
    // drawables
    components: ComponentState,
}
unsafe impl Send for OverlayImpl {}

impl OverlayImpl {
    pub fn new() -> Result<Self, Error> {
        let instance = xlib::Xlib::open()?;
        let xrender = xrender::Xrender::open()?;
        let glx = x11_dl::glx::Glx::open()?;
        let display = unsafe { (instance.XOpenDisplay)(std::ptr::null()) };
        if display.is_null() {
            return Err("failed to retrieve display ptr".into());
        }

        use glfw::fail_on_errors;

        let glfw = glfw::init(fail_on_errors!()).unwrap();

        unsafe {
            (instance.XSetErrorHandler)(Some(error_handler));
        }

        Ok(Self {
            instance,
            xrender,
            glfw,
            glx,
            display,
            components: DrawComponents::new(),
            screen: None,
            window: None,
            window_size: None,
            context: None,
            viewport: None,
        })
    }

    pub fn create_window(&mut self, config: &OverlayConfig) -> Result<(), Error> {
        unsafe {
            let screen = (self.instance.XDefaultScreen)(self.display);
            let root_window = (self.instance.XDefaultRootWindow)(self.display);

            let mut attributes: xlib::XWindowAttributes =
                std::mem::MaybeUninit::zeroed().assume_init();
            let status =
                (self.instance.XGetWindowAttributes)(self.display, root_window, &mut attributes);
            if status != 1 {
                return Err("failed to retrieve root window attributes".into());
            }
            // println!("attributes: {attributes:?}");
            // This doesn't do too much, we fix it later after doing the x11 setup.
            let root_width = attributes.width;
            let root_height = attributes.height;
            let x = 0;
            let y = 0;
            let window_size = Rect::from(0.0, 0.0).sized(root_width as f32, root_height as f32);

            let glfw = &mut self.glfw;
            glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
            glfw.window_hint(glfw::WindowHint::OpenGlProfile(
                glfw::OpenGlProfileHint::Core,
            ));
            glfw.window_hint(glfw::WindowHint::Decorated(false));
            glfw.window_hint(glfw::WindowHint::FocusOnShow(false));
            glfw.window_hint(glfw::WindowHint::AlphaBits(Some(8)));
            glfw.window_hint(glfw::WindowHint::DepthBits(Some(24)));
            glfw.window_hint(glfw::WindowHint::RedBits(Some(8)));
            glfw.window_hint(glfw::WindowHint::GreenBits(Some(8)));
            glfw.window_hint(glfw::WindowHint::BlueBits(Some(8)));
            glfw.window_hint(glfw::WindowHint::TransparentFramebuffer(true));

            #[cfg(target_os = "macos")]
            glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
            // Create a windowed mode window and its OpenGL context
            let (mut window, events) = glfw
                .create_window(
                    root_width as u32,
                    root_height as u32,
                    "Hello this is window",
                    glfw::WindowMode::Windowed,
                )
                .expect("Failed to create GLFW window.");
            let xwindow = window.get_x11_window() as u64;
            let glxwindow = window.get_glx_context();
            // int XChangeWindowAttributes(Display *display, Window w, unsigned long valuemask, XSetWindowAttributes *attributes);
            // set override direct
            //

            let mut z: xlib::XWindowAttributes = std::mem::MaybeUninit::zeroed().assume_init();

            let mut attributes: xlib::XSetWindowAttributes =
                std::mem::MaybeUninit::zeroed().assume_init();
            /*
            let w = (self.instance.XGetWindowAttributes)(self.display, window, &mut z);
            attributes.colormap = (self.instance.XCreateColormap)(
                self.display,
                root_window,
                z.visual,
                xlib::AllocNone,
            );
            attributes.border_pixel = (self.instance.XBlackPixel)(self.display, screen);
            attributes.background_pixel = (self.instance.XBlackPixel)(self.display, screen);
            */
            attributes.override_redirect = true as i32;
            //attributes.event_mask = xlib::ExposureMask;

            (self.instance.XUnmapWindow)(self.display, xwindow);
            let attr_mask = xlib::CWOverrideRedirect;
            (self.instance.XChangeWindowAttributes)(
                self.display,
                xwindow,
                attr_mask,
                &mut attributes,
            );

            window.make_current();
            window.set_key_polling(false);
            window.set_framebuffer_size_polling(true);
            let mut realwindow = window;
            let window = xwindow;
            println!("viewport is {:?}", gl::Viewport::is_loaded());
            gl::load_with(|s| glfw.get_proc_address_raw(s).unwrap() as *const std::ffi::c_void);
            println!("viewport is {:?}", gl::Viewport::is_loaded());

            (self.instance.XMapWindow)(self.display, xwindow);
            (self.instance.XRaiseWindow)(self.display, xwindow);
            unsafe { (self.instance.XFlush)(self.display) };

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

            let wm_hints = (self.instance.XInternAtom)(
                self.display,
                b"_MOTIF_WM_HINTS".as_ptr() as *const i8,
                0,
            );

            realwindow.set_pos(x, y);
            realwindow.set_size(root_width as i32, root_height as i32);
            (self.instance.XMapWindow)(self.display, xwindow);

            // clear the screen once
            //
            unsafe {
                // ------
                gl::Viewport(0, 0, root_width, root_height);

                // gl::Enable(gl::BLEND);
                // gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                // gl::ClearColor(0.0, 0.0, 0.3, 0.2);
                // gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                realwindow.swap_buffers();
            }
            let context = three_d::context::Context::from_loader_function(|s| {
                glfw.get_proc_address_raw(s).unwrap() as *const std::ffi::c_void
            });
            let context = three_d::core::Context::from_gl_context(context.into()).unwrap();
            let viewport = three_d::Viewport {
                x: 0,
                y: 0,
                width: root_width as u32,
                height: root_height as u32,
            };
            context.set_viewport(viewport);
            context.set_blend(three_d::Blend::TRANSPARENCY);
            // use three_d::{BlendEquationType, BlendMultiplierType};
            // context.set_blend(three_d::Blend::Enabled {
            //     source_rgb_multiplier: BlendMultiplierType::SrcAlpha,
            //     source_alpha_multiplier: BlendMultiplierType::SrcAlpha,
            //     destination_rgb_multiplier: BlendMultiplierType::OneMinusSrcAlpha,
            //     destination_alpha_multiplier: BlendMultiplierType::OneMinusSrcAlpha,
            //     rgb_equation: BlendEquationType::Add,
            //     alpha_equation: BlendEquationType::Add,
            // });

            /*
            unsafe {
                //gl::Viewport(0, 0, 100, 100);
                //gl::ClearColor(0.2, 0.3, 0.3, 0.2);
                //gl::Clear(gl::COLOR_BUFFER_BIT);
                let mut texture = 3;
                gl::GenTextures(1, &mut texture);
                gl::BindTexture(gl::TEXTURE_2D, texture);
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    10,
                    10,
                    0,
                    gl::RGBA as u32,
                    gl::UNSIGNED_BYTE,
                    std::ptr::null_mut(),
                );
            }

            while !realwindow.should_close() {
                // events
                // -----
                // process_events(&mut window, &events);

                // render
                // ------
                unsafe {
                    gl::Viewport(0, 0, 100, 100);

                    gl::Enable(gl::BLEND);
                    gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                    gl::ClearColor(0.0, 0.0, 0.3, 0.2);
                    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                    // draw our first triangle
                    // gl::UseProgram(shaderProgram);
                    // gl::BindVertexArray(VAO); // seeing as we only have a single VAO there's no need to bind it every time, but we'll do so to keep things a bit more organized
                    gl::DrawArrays(gl::TRIANGLES, 0, 3);
                    // glBindVertexArray(0); // no need to unbind it every time
                }

                // glfw: swap buffers and poll IO events (keys pressed/released, mouse moved etc.)
                // -------------------------------------------------------------------------------
                realwindow.swap_buffers();
                //(self.glx.glXSwapBuffers)(self.display, glxwindow as u64);
                glfw.poll_events();
            }
            */
            self.window = Some(realwindow);
            // self.visual_info = Some(visual_info);
            self.screen = Some(screen);
            // self.default_visual = Some(default_visual);
            self.window_size = Some(window_size);
            self.context = Some(context);
            self.viewport = Some(viewport);

            // self.test_three_d();
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
        /*unsafe {
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
        }*/
        todo!()
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        layout: &Rect,
        color: &Color,
        font: &PreparedFont,
    ) -> Result<IDVisual, Error> {
        // println!("would print {text}");
        /*
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
        */
        todo!();
    }

    pub fn load_texture<P: AsRef<std::path::Path>>(
        &mut self,
        path: P,
    ) -> Result<ImageTexture, Error> {
        // This loads the image to a pixmap, that we can utilise later.
        let path: &std::path::Path = path.as_ref();
        let img = image::ImageReader::open(path)?.decode()?.to_rgba8();
        let width = img.width();
        let height = img.height();
        let d: Vec<[u8; 4]> = img
            .into_raw()
            .chunks(4)
            .map(|z| [z[0], z[1], z[2], z[3]])
            .collect();
        let mut data = three_d::core::texture::TextureData::RgbaU8(d);

        let texture = three_d::core::texture::CpuTexture {
            name: path.file_name().unwrap().to_string_lossy().into_owned(),
            data,
            width,
            height,
            min_filter: three_d::Interpolation::Linear,
            mag_filter: three_d::Interpolation::Linear,
            mipmap: None,
            wrap_s: three_d::core::texture::Wrapping::ClampToEdge,
            wrap_t: three_d::core::texture::Wrapping::ClampToEdge,
        };
        Ok(ImageTexture { texture })
    }

    pub fn draw_texture(
        &mut self,
        position: &Point,
        texture: &ImageTexture,
        texture_region: &Rect,
        color: &Color,
        alpha: f32,
    ) -> Result<IDVisual, Error> {
        let context = self.context.as_ref().unwrap();

        const USE_BAD: bool = false;
        let material = if USE_BAD {
            three_d::ColorMaterial {
                color: three_d::Srgba::WHITE,
                texture: Some(three_d::Texture2DRef::from_cpu_texture(
                    &context,
                    &texture.texture,
                )),
                is_transparent: true,

                /*
                render_states: three_d::RenderStates {
                    write_mask: three_d::WriteMask::COLOR,
                    blend: three_d::Blend::TRANSPARENCY,
                    ..Default::default()
                },
                 */
                ..Default::default()
            }
        } else {
            let v: three_d::CpuMaterial = three_d::CpuMaterial {
                albedo_texture: Some(texture.texture.clone()),
                ..Default::default()
            };
            let mut x = three_d::ColorMaterial::new_transparent(&context, &v);
            x.render_states = three_d::RenderStates {
                write_mask: three_d::WriteMask::COLOR,
                blend: three_d::Blend::STANDARD_TRANSPARENCY,
                ..Default::default()
            };
            x
        };

        let rectangle = three_d::Rectangle::new(
            &context,
            three_d::vec2(position.x, position.y),
            three_d::degrees(0.0),
            texture.texture.width as f32,
            texture.texture.height as f32,
        );

        let dt = DrawTexture {
            material,
            rectangle,
        };

        let id = {
            let mut l = self.components.write();
            (*l).add_draw_texture(dt)
        };
        // self.render();

        Ok(IDVisual::DrawId {
            id,
            components: self.components.clone(),
        })
    }

    pub fn remove_visual(&mut self, visual: &IDVisual) -> Result<(), Error> {
        Ok(())
    }

    fn test_three_d(&mut self) {
        let context = self.context.as_ref().unwrap();
        let size = self.window_size.as_ref().unwrap();
        let width = size.width();
        let height = size.height();
        let viewport = *self.viewport.as_ref().unwrap();
        let scale_factor = 1.0;
        {
            use three_d::*;
            let mut rectangle = Gm::new(
                Rectangle::new(
                    &context,
                    vec2(200.0, 200.0) * scale_factor,
                    degrees(45.0),
                    100.0 * scale_factor,
                    200.0 * scale_factor,
                ),
                ColorMaterial {
                    color: Srgba::RED,
                    ..Default::default()
                },
            );
            let mut circle = Gm::new(
                Circle::new(
                    &context,
                    vec2(500.0, 500.0) * scale_factor,
                    200.0 * scale_factor,
                ),
                ColorMaterial {
                    color: Srgba::BLUE,
                    ..Default::default()
                },
            );
            let mut line = Gm::new(
                Line::new(
                    &context,
                    vec2(0.0, 0.0) * scale_factor,
                    vec2(width as f32, height as f32) * scale_factor,
                    5.0 * scale_factor,
                ),
                ColorMaterial {
                    color: Srgba::GREEN,
                    ..Default::default()
                },
            );
            RenderTarget::screen(&context, viewport.width, viewport.height)
                // .clear(ClearState::color_and_depth(0.8, 0.8, 0.8, 1.0, 1.0))
                // .clear(ClearState::none())
                .render(
                    Camera::new_2d(viewport),
                    line.into_iter().chain(&rectangle).chain(&circle),
                    &[],
                );

            self.window.as_mut().unwrap().swap_buffers();
        }
    }

    pub fn render(&mut self) {
        let context = self.context.as_ref().unwrap();
        let size = self.window_size.as_ref().unwrap();
        let viewport = *self.viewport.as_ref().unwrap();
        let c = self.components.read();
        let axes = three_d::Axes::new(&context, 5.0, 100.0);
        let ambient = three_d::AmbientLight::new(&context, 1.0, three_d::Srgba::WHITE);
        let mut line = {
            use three_d::*;
            Gm::new(
                Line::new(
                    &context,
                    vec2(0.0, 0.0),
                    vec2(viewport.width as f32 / 2.0, viewport.height as f32),
                    5.0,
                ),
                ColorMaterial {
                    color: Srgba::GREEN,
                    ..Default::default()
                },
            )
        };

        use three_d::{BlendEquationType, BlendMultiplierType};
        let blender = three_d::Blend::Enabled {
            source_rgb_multiplier: BlendMultiplierType::SrcAlpha,
            source_alpha_multiplier: BlendMultiplierType::SrcAlpha,
            destination_rgb_multiplier: BlendMultiplierType::OneMinusSrcAlpha,
            destination_alpha_multiplier: BlendMultiplierType::OneMinusSrcAlpha,
            rgb_equation: BlendEquationType::Add,
            alpha_equation: BlendEquationType::Add,
        };
        context.set_render_states(three_d::RenderStates {
            write_mask: three_d::WriteMask::COLOR_AND_DEPTH,
            depth_test: Default::default(),
            blend: blender,
            cull: Default::default(),
        });

        three_d::RenderTarget::screen(&context, viewport.width, viewport.height)
            // .clear(three_d::ClearState::color_and_depth(  0.8, 0.8, 0.8, 0.1, 1.0, ))
            .clear(three_d::ClearState::color_and_depth(
                0.0, 0.0, 0.0, 0.0, 1.0,
            ))
            .clear(three_d::ClearState::none())
            .render(
                three_d::Camera::new_2d(viewport),
                axes.into_iter()
                    .chain(c.sprites().iter().map(|z| z as &dyn three_d::Object))
                    .chain(&line),
                &[&ambient],
            );

        self.window.as_mut().unwrap().swap_buffers();
    }
}

type OurApplicationType = ();
pub fn run_msg_loop(app: OurApplicationType) -> Result<(), Error> {
    unsafe {
        let instance = xlib::Xlib::open()?;
        let display = (instance.XOpenDisplay)(std::ptr::null());
        let mut event: xlib::XEvent = std::mem::MaybeUninit::zeroed().assume_init();
        (instance.XNextEvent)(display, &mut event);
    }
    Ok(())
}

pub fn setup() -> Result<OurApplicationType, Error> {
    Ok(())
}
