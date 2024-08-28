use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::ValidateRect,
    Win32::System::LibraryLoader::GetModuleHandleA, Win32::UI::WindowsAndMessaging::*,
};

use windows::Win32::Graphics::GdiPlus::{GdipCreateFromHWND, GpGraphics, GpPen};

/*
How do we make transparent pixels in our new overlay? Just specifying transparency does not work.

Looks like gdi just doesn't support alpha, gdi+ seems to?
https://stackoverflow.com/a/35957469
*/

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

pub fn main() -> Result<()> {

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
            if WINDOW_TRANSPARENT {WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT   } else {WINDOW_EX_STYLE::default()},
            window_class,
            s!("This is a sample window"),
            // WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            WS_POPUP | WS_VISIBLE,
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
	SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0x00_FF_FF_FF), 255, LWA_COLORKEY);
	// SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0xFF_FF_FF_FF), 128, LWA_ALPHA);

        setup_gdi()?;
        // let mut graphics: GpGraphics = Default::default();
        let mut graphics: *mut GpGraphics = std::ptr::null_mut();
        let gdip = GdipCreateFromHWND(hwnd, &mut graphics);

        // let mut white_pen: *mut GpPen = std::ptr::null_mut();
        // windows::Win32::Graphics::GdiPlus::GdipCreatePen1(0xFFFFFFFF, 3.0, windows::Win32::Graphics::GdiPlus::UnitPixel, &mut white_pen);
        // let white_brush = windows::Win32::Graphics::Gdi::CreateSolidBrush(0xFFFFFFFF);
        // windows::Win32::Graphics::GdiPlus::GdipFillRectangle(graphics, 
        // windows::Win32::Graphics::Gdi::SetBkColor(windows::Win32::Graphics::Gdi::GetDC(hwnd), windows::Win32::Foundation::COLORREF(0xFF_FF_FF_FF));
        // windows::Win32::Graphics::Gdi::SetBkMode(windows::Win32::Graphics::Gdi::GetDC(hwnd), windows::Win32::Graphics::Gdi::OPAQUE);

        let mut white_pen: *mut GpPen = std::ptr::null_mut();
        windows::Win32::Graphics::GdiPlus::GdipCreatePen1(0x00_FF_FF_FF, 300000.0, windows::Win32::Graphics::GdiPlus::UnitPixel, &mut white_pen);
        windows::Win32::Graphics::GdiPlus::GdipDrawLine(graphics, white_pen, 0.0, 0.0, 1920.0, 1080.0);

        let mut blue_pen: *mut GpPen = std::ptr::null_mut();
        let color = rgba(0, 0, 0xff, 0x10);
        windows::Win32::Graphics::GdiPlus::GdipCreatePen1(color.0, 3.0, windows::Win32::Graphics::GdiPlus::UnitPixel, &mut blue_pen);
        windows::Win32::Graphics::GdiPlus::GdipDrawLine(graphics, blue_pen, 0.0, 0.0, 100.0, 100.0);

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
