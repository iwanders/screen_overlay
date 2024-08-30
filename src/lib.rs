use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::ValidateRect,
    Win32::System::LibraryLoader::GetModuleHandleA, Win32::UI::WindowsAndMessaging::*,
};
use windows::{
    core::*,
    Foundation::Numerics::*,
    Win32::{
        Foundation::*, Graphics::Direct2D::Common::*, Graphics::Direct2D::*, Graphics::Direct3D::*,
        Graphics::Direct3D11::*, Graphics::DirectComposition::*, Graphics::DirectWrite::*,
        Graphics::Dxgi::Common::*, Graphics::Dxgi::*, Graphics::Gdi::*, Graphics::Imaging::D2D::*,
        Graphics::Imaging::*, System::Com::*, System::LibraryLoader::*, UI::Animation::*,
        UI::HiDpi::*, UI::Shell::*, UI::WindowsAndMessaging::*,
    },
};
// use windows::Win32::Graphics::GdiPlus::{GdipCreateFromHWND, GpGraphics, GpPen};

/*
How do we make transparent pixels in our new overlay? Just specifying transparency does not work.

Looks like gdi just doesn't support alpha, gdi+ seems to?
https://stackoverflow.com/a/35957469


https://stackoverflow.com/a/22220021
perhaps we need a whole d3d swapchain, and then that magic DwmExtendFrameIntoClientArea function?

debug print issues perhaps similar to 
https://github.com/EasyJellySniper/RustD3D12/blob/ce0c05e51188f7c41b8fbef1d36bdc2c594bbbb0/src/hello_world_triangle.rs
https://www.gamedev.net/blogs/entry/2294005-implement-d3d12-with-the-rust/

https://learn.microsoft.com/en-us/windows/win32/directcomp/initialize-directcomposition


Lets just follow this direct composition example.
https://github.com/microsoft/windows-rs/tree/9f5ec21529ec0530ac16e9a1c5d16eb8bb290535/crates/samples/windows/dcomp
*/


// mod d3d12_sample;
// https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
// https://learn.microsoft.com/en-us/windows/win32/winmsg/window-styles

// Perhaps helpful:
// https://stackoverflow.com/a/3971732

// Do we need:
// https://github.com/microsoft/windows-rs/issues/2737#issuecomment-1852174020
pub fn setup_gdi() -> Result<()> {
    use core::mem::MaybeUninit;

    use windows::{
        // core::Result,
        Win32::Graphics::GdiPlus::{self, GdiplusStartup, GdiplusStartupInput},
    };

    let mut token = MaybeUninit::uninit();
    let mut output = MaybeUninit::uninit();

    let status = unsafe {
        GdiplusStartup(
            token.as_mut_ptr(),
            &GdiplusStartupInput {
                GdiplusVersion: 1,
                ..Default::default()
            },
            output.as_mut_ptr(),
        )
    };

    // assert_eq!(status, GdiPlus::Ok);
    if (status == GdiPlus::Ok) {
        Ok(())
    } else {
        panic!("cant figure out how to make a string error, sort out later")
       // Err("something went wrong")
    }
}


fn rgb(r: u8, g: u8, b: u8) -> windows::Win32::Foundation::COLORREF {
    windows::Win32::Foundation::COLORREF((r as u32) << 16 | (g as u32) << 8 | b as u32)
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> windows::Win32::Foundation::COLORREF {
    windows::Win32::Foundation::COLORREF((a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32)
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


fn create_text_format() -> Result<IDWriteTextFormat> {
    unsafe {
        let factory: IDWriteFactory2 = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;

        let format = factory.CreateTextFormat(
            w!("Candara"),
            None,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            100.0 / 2.0,
            w!("en"),
        )?;

        format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
        Ok(format)
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
        dc.SetDpi(90.0, 90.0);

        dc.SetTransform(&Matrix3x2::translation(
            0.0,
            0.0,
            // physical_to_logical(offset.x as f32, dpi.0),
            // physical_to_logical(offset.y as f32, dpi.1),
        ));

        dc.Clear(Some(&D2D1_COLOR_F {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 0.5,
        }));

        dc.DrawText(
            &[value as _],
            format,
            &D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: 100.0,
                bottom: 100.0,
            },
            brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
            DWRITE_MEASURING_MODE_NATURAL,
        );

        surface.EndDraw()
    }
}


pub fn main() -> Result<()> {
    // return d3d12_sample::main();
    unsafe {
        let instance = GetModuleHandleA(None)?;
        debug_assert!(!instance.0.is_null());

        let window_class = s!("window");

        let wc = WNDCLASSA {
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: instance.into(),
            lpszClassName: window_class,

            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            ..Default::default()
        };

        let atom = RegisterClassA(&wc);
        debug_assert!(atom != 0);

        const WINDOW_TRANSPARENT: bool = true;

        // Extended styles: https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
        let hwnd = CreateWindowExA(
            // if WINDOW_TRANSPARENT {WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_NOACTIVATE} else {WINDOW_EX_STYLE::default()},
            if WINDOW_TRANSPARENT {WS_EX_COMPOSITED | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST} else {WINDOW_EX_STYLE::default()},
            window_class,
            s!("This is a sample window"),
            // WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            WS_OVERLAPPEDWINDOW,
            // WS_VISIBLE,
            // WS_VISIBLE | WS_DLGFRAME,
            0,
            0,
            // https://github.com/microsoft/DirectX-Graphics-Samples/blob/master/Samples/Desktop/D3D12HelloWorld/src/HelloWindow/Win32Application.cpp
            1920, // windowRect.right - windowRect.left, perhaps?
            1080, // windowRect.bottom - windowRect.top, perhaps?
            None,
            None,
            instance,
            None,
        )?;

        let mut window_rect = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        unsafe { AdjustWindowRect(&mut window_rect, WS_OVERLAPPEDWINDOW, false)? };
        // let hwnd = handle;
        if true {
            unsafe {

                let rect = GetWindowRect(hwnd, &mut window_rect)?;
                let rgn = windows::Win32::Graphics::Gdi::CreateRectRgnIndirect(&window_rect);
                windows::Win32::Graphics::Gdi::SetWindowRgn(hwnd, rgn, false);
                ShowWindow(hwnd, SHOW_WINDOW_CMD(1));
            }
        }
        let extended_style = GetWindowLongA(hwnd, GWL_EXSTYLE) as u32;
        println!("GWL_EXSTYLE: {:?}", GWL_EXSTYLE);
        println!("WS_EX_TRANSPARENT: {:?}", WS_EX_TRANSPARENT);
        // https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features#layered-windows
        // SetWindowLongA(hwnd, GWL_EXSTYLE, extended_style | WS_EX_TRANSPARENT.0 as i32 | WS_EX_TOPMOST.0 as i32 | WS_EX_LAYERED.0 as i32);

        /*
        let extended_style = extended_style &  (!(WS_EX_DLGMODALFRAME.0 | WS_EX_CLIENTEDGE.0 | WS_EX_STATICEDGE.0));
        let extended_style = extended_style | WS_EX_TRANSPARENT.0 | WS_EX_TOPMOST.0 | WS_EX_LAYERED.0;
        SetWindowLongA(hwnd, GWL_EXSTYLE, extended_style as i32);
        */

        // If it is a popup, we need to manually set the size.
        // let hdc = GetDC();
        // UpdateLayeredWindow(hwnd, hdc);

        // UpdateLayeredWindow must happen PRIOR to set layered window attributes; https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-updatelayeredwindow
        // see https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setlayeredwindowattributes#remarks
	// SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0x00_FF_FF_FF), 255, LWA_COLORKEY);
	// SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0xFF_FF_FF_FF), 128, LWA_ALPHA);

        use windows::Win32::UI::HiDpi::*;
        use windows::Win32::Graphics::Gdi::*;
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut dpi = (0, 0);
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi.0, &mut dpi.1)?;
        // self.dpi = (dpi.0 as f32, dpi.1 as f32);

        // if cfg!(debug_assertions) {
            // println!("initial dpi: {:?}", self.dpi);
        // }

        // let size = self.effective_window_size()?;

        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            1920,
            1080,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        );

        let device_3d = create_device_3d()?;
        let device_2d = create_device_2d(&device_3d)?;
        let device = Some(device_3d);
        let desktop: IDCompositionDesktopDevice = DCompositionCreateDevice2(&device_2d)?;

        // First release any previous target, otherwise `CreateTargetForHwnd` will find the HWND occupied.
        // let mut target = None;
        let target = desktop.CreateTargetForHwnd(hwnd, true)?;
        let root_visual = create_visual(&desktop)?;
        target.SetRoot(&root_visual)?;
        // self.target = Some(target);

        let dc = device_2d.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;

        let brush = dc.CreateSolidColorBrush(
            &D2D1_COLOR_F {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 0.5,
            },
            None,
        )?;

        let width = 10.0;
        let height = 10.0;

        let back_visual = create_visual(&desktop)?;
        back_visual.SetOffsetX2(0.0)?;
        back_visual.SetOffsetY2(0.0)?;
        root_visual.AddVisual(&back_visual, false, None)?;

        let format = create_text_format()?;

        let front_surface = create_surface(&desktop, width, height)?;
        back_visual.SetContent(&front_surface)?;
        draw_card_front(&front_surface, 0x61, &format, &brush)?;

        let mut message = MSG::default();

        while GetMessageA(&mut message, None, 0, 0).into() {
            DispatchMessageA(&message);
        }

        Ok(())
    }
}

extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_PAINT => {
                println!("WM_PAINT");
                ValidateRect(window, None);
                LRESULT(0)
            }
            WM_DESTROY => {
                println!("WM_DESTROY");
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcA(window, message, wparam, lparam),
        }
    }
}
