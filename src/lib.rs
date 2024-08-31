use windows::{
    core::*,
    Foundation::Numerics::*,
    Win32::{
        Foundation::*, Graphics::Direct2D::Common::*, Graphics::Direct2D::*, Graphics::Direct3D::*,
        Graphics::Direct3D11::*, Graphics::DirectComposition::*, Graphics::DirectWrite::*,
        Graphics::Dxgi::Common::*, Graphics::Dxgi::*, Graphics::Gdi::*, Graphics::Imaging::D2D::*,
        Graphics::Imaging::*, System::Com::*, System::LibraryLoader::*, UI::Animation::*,
        // UI::HiDpi::*,
        UI::Shell::*, UI::WindowsAndMessaging::*,
    },
};

const CARD_ROWS: usize = 3;
const CARD_COLUMNS: usize = 6;
const CARD_MARGIN: f32 = 1.0;
const CARD_WIDTH: f32 = 150.0;
const CARD_HEIGHT: f32 = 210.0;
const WINDOW_WIDTH: f32 = CARD_COLUMNS as f32 * (CARD_WIDTH + CARD_MARGIN) + CARD_MARGIN;
const WINDOW_HEIGHT: f32 = CARD_ROWS as f32 * (CARD_HEIGHT + CARD_MARGIN) + CARD_MARGIN;

pub fn main() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        // SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)?;
    }
    let mut window = Window::new()?;
    window.run()
}

#[derive(PartialEq)]
enum Status {
    Hidden,
    Selected,
    Matched,
}

struct Card {
    status: Status,
    value: u8,
    offset: (f32, f32),
    variable: IUIAnimationVariable2,
    rotation: Option<IDCompositionRotateTransform3D>,
}

struct Window {
    handle: HWND,
    // dpi: (f32, f32),
    format: IDWriteTextFormat,
    image: IWICFormatConverter,
    manager: IUIAnimationManager2,
    library: IUIAnimationTransitionLibrary2,
    first: Option<usize>,
    cards: Vec<Card>,
    device: Option<ID3D11Device>,
    desktop: Option<IDCompositionDesktopDevice>,
    target: Option<IDCompositionTarget>,
}

impl Window {
    fn new() -> Result<Self> {
        unsafe {
            let manager: IUIAnimationManager2 =
                CoCreateInstance(&UIAnimationManager2, None, CLSCTX_INPROC_SERVER)?;

            let values = [0x61; CARD_ROWS * CARD_COLUMNS];

            // values.shuffle(&mut rng);
            let mut cards = Vec::new();

            for value in values {
                cards.push(Card {
                    status: Status::Hidden,
                    value,
                    offset: (0.0, 0.0),
                    variable: manager.CreateAnimationVariable(0.0)?,
                    rotation: None,
                });
            }

            if cfg!(debug_assertions) {
                println!("deck:");
                for row in 0..CARD_ROWS {
                    for column in 0..CARD_COLUMNS {
                        print!(
                            " {}",
                            char::from_u32(cards[row * CARD_COLUMNS + column].value as u32)
                                .expect("char")
                        );
                    }
                    println!();
                }
            }

            let library =
                CoCreateInstance(&UIAnimationTransitionLibrary2, None, CLSCTX_INPROC_SERVER)?;

            Ok(Window {
                handle: Default::default(),
                // dpi: (0.0, 0.0),
                format: create_text_format()?,
                image: create_image()?,
                manager,
                library,
                first: None,
                cards,
                device: None,
                desktop: None,
                target: None,
            })
        }
    }

    fn create_device_resources(&mut self) -> Result<()> {
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
            self.target = Some(target);

            let dc = device_2d.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            dc.SetUnitMode(D2D1_UNIT_MODE_PIXELS); // set the device mode to pixels.

            let brush = dc.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
                None,
            )?;

            let bitmap = dc.CreateBitmapFromWicBitmap(&self.image, None)?;
            let width = CARD_WIDTH;
            let height = CARD_HEIGHT;

            for row in 0..CARD_ROWS {
                for column in 0..CARD_COLUMNS {
                    let card = &mut self.cards[row * CARD_COLUMNS + column];

                    card.offset = (
                            column as f32 * (CARD_WIDTH + CARD_MARGIN) + CARD_MARGIN,
                            row as f32 * (CARD_HEIGHT + CARD_MARGIN) + CARD_MARGIN,
                    );

                    if card.status == Status::Matched {
                        continue;
                    }

                    let front_visual = create_visual(&desktop)?;
                    front_visual.SetOffsetX2(card.offset.0)?;
                    front_visual.SetOffsetY2(card.offset.1)?;
                    root_visual.AddVisual(&front_visual, false, None)?;

                    let back_visual = create_visual(&desktop)?;
                    back_visual.SetOffsetX2(card.offset.0)?;
                    back_visual.SetOffsetY2(card.offset.1)?;
                    root_visual.AddVisual(&back_visual, false, None)?;

                    let front_surface = create_surface(&desktop, width, height)?;
                    front_visual.SetContent(&front_surface)?;
                    draw_card_front(&front_surface, card.value, &self.format, &brush)?;

                    let back_surface = create_surface(&desktop, width, height)?;
                    back_visual.SetContent(&back_surface)?;
                    draw_card_back(&back_surface, &bitmap, card.offset)?;

                    let rotation = desktop.CreateRotateTransform3D()?;

                    if card.status == Status::Selected {
                        rotation.SetAngle2(180.0)?;
                    }

                    rotation.SetAxisZ2(0.0)?;
                    rotation.SetAxisY2(1.0)?;
                    create_effect(&desktop, &front_visual, &rotation, true)?;
                    create_effect(&desktop, &back_visual, &rotation, false)?;
                    card.rotation = Some(rotation);

                    let mut stats = Default::default();
                    desktop.GetFrameStatistics(&mut stats)?;

                    let next_frame: f64 =
                        stats.nextEstimatedFrameTime as f64 / stats.timeFrequency as f64;

                    self.manager.Update(next_frame, None)?;
                    let storyboard = self.manager.CreateStoryboard()?;
                    let key_frame = add_show_transition(&self.library, &storyboard, &card)?;

                    // storyboard.Schedule(next_frame, None)?;
                    update_animation(&desktop, &card)?;
                    desktop.Commit()?;
                }
            }

            desktop.Commit()?;
            self.desktop = Some(desktop);
            Ok(())
        }
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
            // MONITORINFO monitor_info;
            // monitor_info.cbSize = sizeof(monitor_info);
            // GetMonitorInfo(MonitorFromWindow(hwnd_, MONITOR_DEFAULTTONEAREST), &monitor_info);
            GetMonitorInfoA(monitor, &mut monitor_info);
            // gfx::Rect window_rect(monitor_info.rcMonitor);
            println!("Setting size to: {:?}", monitor_info.rcMonitor);
            Ok(monitor_info.rcMonitor)
        }
    }

    fn create_handler(&mut self) -> Result<()> {
        unsafe {
            let monitor = MonitorFromWindow(self.handle, MONITOR_DEFAULTTOPRIMARY);
            let desired_size = self.desired_window_size()?;
            println!("Setting size to: {:?}", desired_size);
            SetWindowPos(self.handle, None,
                desired_size.left, desired_size.top, desired_size.right, desired_size.bottom,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED)
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

    fn run(&mut self) -> Result<()> {
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

            debug_assert!(!handle.is_invalid());
            debug_assert!(handle == self.handle);
            let mut message = MSG::default();

            while GetMessageA(&mut message, HWND::default(), 0, 0).into() {
                DispatchMessageA(&message);
            }

            Ok(())
        }
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

fn create_image() -> Result<IWICFormatConverter> {
    unsafe {
        let factory: IWICImagingFactory2 =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

        // Just a little hack to make it simpler to run the sample from the root of the workspace.
        let path = if PathFileExistsW(w!("image.jpg")).is_ok() {
            w!("image.jpg")
        } else {
            w!("crates/samples/windows/dcomp/image.jpg")
        };

        let decoder = factory.CreateDecoderFromFilename(
            path,
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        )?;

        let source = decoder.GetFrame(0)?;
        let image = factory.CreateFormatConverter()?;

        image.Initialize(
            &source,
            &GUID_WICPixelFormat32bppBGR,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeMedianCut,
        )?;

        Ok(image)
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

fn add_show_transition(
    library: &IUIAnimationTransitionLibrary2,
    storyboard: &IUIAnimationStoryboard2,
    card: &Card,
) -> Result<UI_ANIMATION_KEYFRAME> {
    unsafe {
        let duration = (180.0 - card.variable.GetValue()?) / 180.0;
        let transition = create_transition(library, duration, 180.0)?;
        storyboard.AddTransition(&card.variable, &transition)?;
        storyboard.AddKeyframeAfterTransition(&transition)
    }
}

fn add_hide_transition(
    library: &IUIAnimationTransitionLibrary2,
    storyboard: &IUIAnimationStoryboard2,
    key_frame: UI_ANIMATION_KEYFRAME,
    final_value: f64,
    card: &Card,
) -> Result<()> {
    unsafe {
        let transition = create_transition(library, 1.0, final_value)?;
        storyboard.AddTransitionAtKeyframe(&card.variable, &transition, key_frame)
    }
}

fn update_animation(device: &IDCompositionDesktopDevice, card: &Card) -> Result<()> {
    unsafe {
        let animation = device.CreateAnimation()?;
        card.variable.GetCurve(&animation)?;

        card.rotation
            .as_ref()
            .expect("IDCompositionRotateTransform3D")
            .SetAngle(&animation)
    }
}

fn create_transition(
    library: &IUIAnimationTransitionLibrary2,
    duration: f64,
    final_value: f64,
) -> Result<IUIAnimationTransition2> {
    unsafe { library.CreateAccelerateDecelerateTransition(duration, final_value, 0.2, 0.8) }
}

fn create_effect(
    device: &IDCompositionDesktopDevice,
    visual: &IDCompositionVisual2,
    rotation: &IDCompositionRotateTransform3D,
    front: bool,
) -> Result<()> {
    unsafe {
        let width = CARD_WIDTH;
        let height = CARD_HEIGHT;

        let pre_matrix = Matrix4x4::translation(-width / 2.0, -height / 2.0, 0.0)
            * Matrix4x4::rotation_y(if front { 180.0 } else { 0.0 });

        let pre_transform = device.CreateMatrixTransform3D()?;
        pre_transform.SetMatrix(&pre_matrix)?;

        let post_matrix = Matrix4x4::perspective_projection(width * 2.0)
            * Matrix4x4::translation(width / 2.0, height / 2.0, 0.0);

        let post_transform = device.CreateMatrixTransform3D()?;
        post_transform.SetMatrix(&post_matrix)?;

        let transform = device.CreateTransform3DGroup(&[
            pre_transform.cast().ok(),
            rotation.cast().ok(),
            post_transform.cast().ok(),
        ])?;

        visual.SetEffect(&transform)
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

        dc.SetTransform(&Matrix3x2::translation(
            offset.x as f32,
            offset.y as f32,
        ));

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

