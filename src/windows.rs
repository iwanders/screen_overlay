use windows::{
    core::*,
    Foundation::Numerics::*,
    Win32::{
        Foundation::*,
        Graphics::Direct2D::Common::*,
        Graphics::Direct2D::*,
        Graphics::Direct3D::*,
        Graphics::Direct3D11::*,
        Graphics::DirectComposition::*,
        Graphics::DirectWrite::*,
        Graphics::Dxgi::Common::*,
        Graphics::Dxgi::*,
        Graphics::Gdi::*,
        Graphics::Imaging::D2D::*,
        Graphics::Imaging::*,
        System::Com::*,
        System::LibraryLoader::*,
        UI::Animation::*,
        // UI::HiDpi::*,
        UI::Shell::*,
        UI::WindowsAndMessaging::*,
    },
};

use crate::{Color, Rect, Point, DrawGeometry, Stroke, GeometryElement};

use std::sync::Arc;

// This is helpful; https://learn.microsoft.com/en-us/windows/win32/directcomp/basic-concepts

#[derive(Clone)]
pub struct ImageTexture {
    image: Arc<IWICFormatConverter>,
}
impl std::fmt::Debug for ImageTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "ImageTexture {:?}", &self)
    }
}

impl From<Color> for D2D1_COLOR_F {
    fn from(c: Color) -> Self {
        D2D1_COLOR_F {
            r: c.r as f32 / 255.0,
            g: c.g as f32 / 255.0,
            b: c.b as f32 / 255.0,
            a: c.a as f32 / 255.0,
        }
    }
}
impl From<Point> for D2D_POINT_2F {
    fn from(p: Point) -> Self {
        D2D_POINT_2F { x: p.x, y: p.y }
    }
}

impl From<Rect> for D2D_RECT_F {
    fn from(r: Rect) -> Self {
        D2D_RECT_F {
            left: r.min.x,
            top: r.min.y,
            right: r.max.x,
            bottom: r.max.y
        }
    }
}




const CARD_WIDTH: f32 = 150.0;
const CARD_HEIGHT: f32 = 210.0;

// The IDCompositionVisual appears to be a tree, as per;
// https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Graphics/DirectComposition/trait.IDCompositionVisual_Impl.html#tymethod.AddVisual
pub type IDVisual = IDCompositionVisual2;
struct DrawElement {
    position: (f32, f32),
    visual: IDVisual,
    surface: IDCompositionSurface,
}

pub struct OverlayImpl {
    handle: HWND,
    format: IDWriteTextFormat,
    manager: IUIAnimationManager2,
    library: IUIAnimationTransitionLibrary2,
    device: Option<ID3D11Device>,
    desktop: Option<IDCompositionDesktopDevice>,
    target: Option<IDCompositionTarget>,
    root_visual: Option<IDCompositionVisual2>,
}
// Is this legal?
unsafe impl Send for OverlayImpl {}

pub fn run_msg_loop() -> Result<()> {
    unsafe {
        let mut message = MSG::default();
        while GetMessageA(&mut message, HWND::default(), 0, 0).into() {
            println!("message: {message:?}");
            DispatchMessageA(&message);
        }
        Ok(())
    }
}

impl OverlayImpl {
    pub fn new() -> Result<Self> {
        unsafe {
            let manager: IUIAnimationManager2 =
                CoCreateInstance(&UIAnimationManager2, None, CLSCTX_INPROC_SERVER)?;

            let library =
                CoCreateInstance(&UIAnimationTransitionLibrary2, None, CLSCTX_INPROC_SERVER)?;

            Ok(Self {
                handle: Default::default(),
                format: create_text_format()?,
                manager,
                library,
                device: None,
                desktop: None,
                target: None,
                root_visual: None,
            })
        }
    }

    pub fn create_window(&mut self) -> Result<()> {
        unsafe {
            let instance = GetModuleHandleA(None)?;
            let window_class = s!("window");

            let wc = WNDCLASSA {
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hInstance: instance.into(),
                lpszClassName: window_class,

                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(Self::wndproc),
                ..Default::default()
            };

            let atom = RegisterClassA(&wc);
            debug_assert!(atom != 0);

            let handle = CreateWindowExA(
                // WS_EX_NOREDIRECTIONBITMAP,
                WS_EX_COMPOSITED | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST, //  |WS_EX_NOACTIVATE  <- hides taskbar
                window_class,
                s!("Sample Window"),
                // WS_OVERLAPPED, // | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
                WS_POPUP, // use popup, that disables the titlebar and border.
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                instance,
                Some(self as *mut _ as _),
            )?;
            let hwnd = handle;

            if true {
                let window_rect = self.desired_window_size()?;
                let rgn = windows::Win32::Graphics::Gdi::CreateRectRgnIndirect(&window_rect);
                windows::Win32::Graphics::Gdi::SetWindowRgn(hwnd, rgn, false);
                ShowWindow(hwnd, SHOW_WINDOW_CMD(1));
            }
            // self.create_handler()?;

            debug_assert!(!handle.is_invalid());
            debug_assert!(handle == self.handle);
        }
        Ok(())
    }

    pub fn create_device_resources(&mut self) -> Result<()> {
        unsafe {
            debug_assert!(self.device.is_none());
            let device_3d = create_device_3d()?;
            let device_2d = create_device_2d(&device_3d)?;
            self.device = Some(device_3d);
            let desktop: IDCompositionDesktopDevice = DCompositionCreateDevice2(&device_2d)?;

            // First release any previous target, otherwise `CreateTargetForHwnd` will find the HWND occupied.
            self.target = None;
            let target = desktop.CreateTargetForHwnd(self.handle, true)?;
            let root_visual = create_visual(&desktop)?;
            target.SetRoot(&root_visual)?;
            self.root_visual = Some(root_visual.clone());
            self.target = Some(target);

            desktop.Commit()?;
            self.desktop = Some(desktop);

            Ok(())
        }
    }

    pub fn draw_line(&mut self) -> Result<IDVisual> {
        // Objects used together must be created from the same factory instance.
        unsafe {
            let visual = create_visual(self.desktop.as_ref().unwrap())?;
            visual.SetOffsetX2(100.0)?;
            visual.SetOffsetY2(100.0)?;
            self.root_visual
                .as_ref()
                .unwrap()
                .AddVisual(&visual, false, None)?;
            let width = 100.0;
            let height = 100.0;
            let surface = create_surface(self.desktop.as_ref().unwrap(), width, height)?;
            visual.SetContent(&surface)?;

            let mut offset = Default::default();
            let dc: ID2D1DeviceContext = surface.BeginDraw(None, &mut offset)?;

            dc.SetTransform(&Matrix3x2::translation(offset.x as f32, offset.y as f32));

            dc.Clear(Some(&D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.0,
            }));

            let p0 = D2D_POINT_2F { x: 0.0, y: 0.0 };
            let p1 = D2D_POINT_2F {
                x: 1000.0,
                y: 1000.0,
            };
            let brush: ID2D1Brush = dc
                .CreateSolidColorBrush(
                    &D2D1_COLOR_F {
                        r: 1.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    },
                    None,
                )?
                .cast()?;
            let strokewidth = 5.0;

            let stroke_props = D2D1_STROKE_STYLE_PROPERTIES {
                ..Default::default()
            };
            // let stroke_style = self.factory.as_ref().unwrap().CreateStrokeStyle(&stroke_props, None)?;
            let stroke_style = dc.GetFactory()?.CreateStrokeStyle(&stroke_props, None)?;

            dc.DrawLine(p0, p1, &brush, strokewidth, &stroke_style);

            surface.EndDraw();

            self.desktop.as_ref().map(|v| v.Commit()).unwrap()?;

            Ok(visual.clone())
        }
    }

    pub fn draw_geometry(&mut self, geometry: &DrawGeometry, stroke: &Stroke) -> Result<IDVisual> {
        // Objects used together must be created from the same factory instance.
        unsafe {
            let visual = create_visual(self.desktop.as_ref().unwrap())?;
            visual.SetOffsetX2(0.0)?;
            visual.SetOffsetY2(0.0)?;
            self.root_visual
                .as_ref()
                .unwrap()
                .AddVisual(&visual, false, None)?;
            let width = 1000.0;
            let height = 1000.0;
            let surface = create_surface(self.desktop.as_ref().unwrap(), width, height)?;
            visual.SetContent(&surface)?;

            let mut offset = Default::default();
            let dc: ID2D1DeviceContext = surface.BeginDraw(None, &mut offset)?;

            dc.SetTransform(&Matrix3x2::translation(offset.x as f32, offset.y as f32));

            dc.Clear(Some(&D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.0,
            }));

            let path_geom = dc.GetFactory()?.CreatePathGeometry()?;
            let sink: ID2D1SimplifiedGeometrySink = path_geom.Open()?.cast()?;
            for el in geometry.elements.iter() {
                match el {
                    GeometryElement::Start { start, filled } => {
                        let start_style = if *filled {
                            D2D1_FIGURE_BEGIN_FILLED
                        } else {
                            D2D1_FIGURE_BEGIN_HOLLOW
                        };
                        sink.BeginFigure((*start).into(), start_style);
                    }
                    GeometryElement::End { closed } => {
                        let close_style = if *closed {
                            D2D1_FIGURE_END_CLOSED
                        } else {
                            D2D1_FIGURE_END_OPEN
                        };
                        sink.EndFigure(close_style);
                    }
                    GeometryElement::Line(point) => {
                        sink.AddLines(&[(*point).into()]);
                    }
                    z => todo!("Missing implementation for {z:?}"),
                }
            }
            sink.Close()?;

            let brush: ID2D1Brush = dc
                .CreateSolidColorBrush(&stroke.color.into(), None)?
                .cast()?;
            let strokewidth = stroke.width;

            let stroke_props = D2D1_STROKE_STYLE_PROPERTIES {
                ..Default::default()
            };
            // let stroke_style = self.factory.as_ref().unwrap().CreateStrokeStyle(&stroke_props, None)?;
            let stroke_style = dc.GetFactory()?.CreateStrokeStyle(&stroke_props, None)?;

            dc.DrawGeometry(&path_geom, &brush, strokewidth, &stroke_style);

            surface.EndDraw();

            self.desktop.as_ref().map(|v| v.Commit()).unwrap()?;

            Ok(visual.clone())
        }
    }

    pub fn draw_text(&mut self, text: &str, layout: &Rect, color: &Color) -> Result<IDVisual> {
        unsafe {
            let visual = create_visual(self.desktop.as_ref().unwrap())?;
            visual.SetOffsetX2(0.0)?;
            visual.SetOffsetY2(0.0)?;
            self.root_visual
                .as_ref()
                .unwrap()
                .AddVisual(&visual, false, None)?;
            let width = layout.width();
            let height = layout.height();
            let surface = create_surface(self.desktop.as_ref().unwrap(), width, height)?;
            visual.SetContent(&surface)?;

            let mut offset = Default::default();
            let dc: ID2D1DeviceContext = surface.BeginDraw(None, &mut offset)?;

            dc.SetTransform(&Matrix3x2::translation(layout.min.x, layout.min.y));

            dc.Clear(Some(&D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.0,
            }));

            let format = create_text_format()?;

            let brush: ID2D1Brush = dc
                .CreateSolidColorBrush(&(*color).into(), None)?
                .cast()?;
            use std::os::windows::ffi::OsStrExt;
            let windows_string: Vec<u16> = std::ffi::OsStr::new(text).encode_wide().chain([0u16].iter().copied()).collect();
            dc.DrawText(
                &windows_string,
                &format,
                &D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: width,
                    bottom: height,
                },
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            surface.EndDraw();
            self.desktop.as_ref().map(|v| v.Commit()).unwrap()?;

            Ok(visual.clone())
        }
    }


    pub fn load_texture<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<ImageTexture> {
        unsafe {
            // This should create a singleton.
            let factory: IWICImagingFactory2 =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

            let path : std::path::PathBuf = std::path::PathBuf::from(path.as_ref());
            use std::os::windows::ffi::OsStrExt;
            let windows_string: Vec<u16>  = std::ffi::OsString::from(&path).encode_wide().chain([0u16].iter().copied()).collect();
            println!("windows string: {windows_string:?}");
            let z = PCWSTR::from_raw(windows_string.as_ptr());

            // https://learn.microsoft.com/en-us/windows/win32/wic/-wic-bitmapsources
            let decoder = factory.CreateDecoderFromFilename(
                z,
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )?;

            let source = decoder.GetFrame(0)?;
            let image = factory.CreateFormatConverter()?;

            image.Initialize(
                &source,
                &GUID_WICPixelFormat32bppPBGRA , // not GUID_WICPixelFormat32bppBGRA; https://stackoverflow.com/a/25009124
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeMedianCut,
            )?;
            let image = Arc::new(image);
            println!("bitmap good");

            Ok(ImageTexture{image})
        }
    }
    pub fn draw_texture(&mut self, position: &Point, texture: &ImageTexture, texture_region: &Rect) -> Result<IDVisual> {
        unsafe {
            let visual = create_visual(self.desktop.as_ref().unwrap())?;
            visual.SetOffsetX2(position.x)?;
            visual.SetOffsetY2(position.y)?;
            self.root_visual
                .as_ref()
                .unwrap()
                .AddVisual(&visual, false, None)?;
            let width = texture_region.width();
            let height = texture_region.height();
            let surface = create_surface(self.desktop.as_ref().unwrap(), width, height)?;
            visual.SetContent(&surface)?;

            let mut dc_offset = Default::default();
            let dc: ID2D1DeviceContext = surface.BeginDraw(None, &mut dc_offset)?;

            // dc.Clear(Some(&D2D1_COLOR_F {
                // r: 1.0,
                // g: 1.0,
                // b: 1.0,
                // a: 0.0,
            // }));

            println!("Convert image to bitmap");
            let properties = D2D1_BITMAP_PROPERTIES1  {
                pixelFormat: D2D1_PIXEL_FORMAT  {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    // ..Default::default()
                },
                ..Default::default()
            };
            let bitmap = dc.CreateBitmapFromWicBitmap(&*(*texture).image, Some(&properties))?;

            println!("below  image to bitmap");
            dc.SetTransform(&Matrix3x2::translation(
                dc_offset.x as f32,
                dc_offset.y as f32,
            ));


            dc.DrawBitmap(
                &bitmap,
                None,
                1.0, // alpha
                D2D1_INTERPOLATION_MODE_LINEAR,
                Some(&D2D_RECT_F {
                    left: texture_region.min.x,
                    top: texture_region.min.y,
                    right: texture_region.max.x,
                    bottom: texture_region.max.y,
                }),
                None,
            );

            surface.EndDraw();
            self.desktop.as_ref().map(|v| v.Commit()).unwrap()?;
            Ok(visual.clone())
        }

    }


    pub fn remove_visual(&mut self, visual: &IDVisual) -> Result<()> {
        unsafe {
            self.root_visual.as_ref().unwrap().RemoveVisual(visual)?;
            self.desktop.as_ref().map(|v| v.Commit()).unwrap()?;
        }
        Ok(())
    }

    fn paint_handler(&mut self) -> Result<()> {
        unsafe {
            if let Some(device) = &self.device {
                if cfg!(debug_assertions) {
                    println!("check device");
                }
                device.GetDeviceRemovedReason()?;
            } else {
                if cfg!(debug_assertions) {
                    println!("build device");
                }
                self.create_device_resources()?;
            }

            ValidateRect(self.handle, None).ok()
        }
    }

    fn desired_window_size(&self) -> Result<RECT> {
        unsafe {
            let monitor = MonitorFromWindow(self.handle, MONITOR_DEFAULTTOPRIMARY);
            let mut monitor_info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            GetMonitorInfoA(monitor, &mut monitor_info);
            // println!("Setting size to: {:?}", monitor_info.rcMonitor);
            Ok(monitor_info.rcMonitor)
        }
    }

    fn create_handler(&mut self) -> Result<()> {
        unsafe {
            let monitor = MonitorFromWindow(self.handle, MONITOR_DEFAULTTOPRIMARY);
            let desired_size = self.desired_window_size()?;
            println!("Setting size to: {:?}", desired_size);
            SetWindowPos(
                self.handle,
                None,
                desired_size.left,
                desired_size.top,
                desired_size.right,
                desired_size.bottom,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )
        }
    }

    fn message_handler(&mut self, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            match message {
                // WM_LBUTTONUP => self.click_handler(lparam).expect("WM_LBUTTONUP"),
                WM_PAINT => {
                    self.paint_handler().unwrap_or_else(|_| {
                        // Device loss can cause rendering to fail and should not be considered fatal.
                        if cfg!(debug_assertions) {
                            println!("WM_PAINT failed");
                        }
                        self.device = None;
                    });
                }
                // WM_DPICHANGED => self.dpi_changed_handler(wparam, lparam).expect("WM_DPICHANGED"),
                WM_CREATE => self.create_handler().expect("WM_CREATE"),
                WM_WINDOWPOSCHANGING => {
                    // Prevents window resizing due to device loss
                }
                WM_DESTROY => PostQuitMessage(0),
                _ => return DefWindowProcA(self.handle, message, wparam, lparam),
            }
        }

        LRESULT(0)
    }

    extern "system" fn wndproc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            if message == WM_NCCREATE {
                let cs = lparam.0 as *const CREATESTRUCTA;
                let this = (*cs).lpCreateParams as *mut Self;
                (*this).handle = window;

                SetWindowLongPtrA(window, GWLP_USERDATA, this as _);
            } else {
                let this = GetWindowLongPtrA(window, GWLP_USERDATA) as *mut Self;

                if !this.is_null() {
                    return (*this).message_handler(message, wparam, lparam);
                }
            }

            DefWindowProcA(window, message, wparam, lparam)
        }
    }
}

fn create_text_format() -> Result<IDWriteTextFormat> {
    unsafe {
        let factory: IDWriteFactory2 = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;

        let format = factory.CreateTextFormat(
            w!("Candara"),
            None,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            CARD_HEIGHT / 2.0,
            w!("en"),
        )?;

        format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
        Ok(format)
    }
}

fn create_device_3d() -> Result<ID3D11Device> {
    let mut device = None;

    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )
        .map(|()| device.unwrap())
    }
}

fn create_device_2d(device_3d: &ID3D11Device) -> Result<ID2D1Device> {
    let dxgi: IDXGIDevice3 = device_3d.cast()?;
    unsafe { D2D1CreateDevice(&dxgi, None) }
}

fn create_visual(device: &IDCompositionDesktopDevice) -> Result<IDCompositionVisual2> {
    unsafe {
        let visual = device.CreateVisual()?;
        visual.SetBackFaceVisibility(DCOMPOSITION_BACKFACE_VISIBILITY_HIDDEN)?;
        Ok(visual)
    }
}

fn create_surface(
    device: &IDCompositionDesktopDevice,
    width: f32,
    height: f32,
) -> Result<IDCompositionSurface> {
    unsafe {
        device.CreateSurface(
            width as u32,
            height as u32,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_ALPHA_MODE_PREMULTIPLIED,
        )
    }
}

fn draw_card_front(
    surface: &IDCompositionSurface,
    value: u8,
    format: &IDWriteTextFormat,
    brush: &ID2D1SolidColorBrush,
) -> Result<()> {
    unsafe {
        let mut offset = Default::default();
        let dc: ID2D1DeviceContext = surface.BeginDraw(None, &mut offset)?;

        dc.SetTransform(&Matrix3x2::translation(offset.x as f32, offset.y as f32));

        dc.Clear(Some(&D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.8,
        }));

        dc.DrawText(
            &[value as _],
            format,
            &D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: CARD_WIDTH,
                bottom: CARD_HEIGHT,
            },
            brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
            DWRITE_MEASURING_MODE_NATURAL,
        );

        surface.EndDraw()
    }
}

fn draw_card_back(
    surface: &IDCompositionSurface,
    bitmap: &ID2D1Bitmap1,
    offset: (f32, f32),
) -> Result<()> {
    unsafe {
        let mut dc_offset = Default::default();
        let dc: ID2D1DeviceContext = surface.BeginDraw(None, &mut dc_offset)?;

        dc.SetTransform(&Matrix3x2::translation(
            dc_offset.x as f32,
            dc_offset.y as f32,
        ));

        let left = offset.0;
        let top = offset.1;

        dc.DrawBitmap(
            bitmap,
            None,
            0.5, // alpha
            D2D1_INTERPOLATION_MODE_LINEAR,
            Some(&D2D_RECT_F {
                left,
                top,
                right: left + CARD_WIDTH,
                bottom: top + CARD_HEIGHT,
            }),
            None,
        );

        surface.EndDraw()
    }
}

pub fn setup() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    Ok(())
}
