use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::ValidateRect,
    Win32::System::LibraryLoader::GetModuleHandleA, Win32::UI::WindowsAndMessaging::*,
};

use windows::Win32::Graphics::GdiPlus::{GdipCreateFromHWND, GpGraphics, GpPen};

// https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
// https://learn.microsoft.com/en-us/windows/win32/winmsg/window-styles



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

        const WINDOW_TRANSPARENT: bool = false;

        // Extended styles: https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
        let hwnd = CreateWindowExA(
            if WINDOW_TRANSPARENT {WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_NOACTIVATE} else {WINDOW_EX_STYLE::default()},
            window_class,
            s!("This is a sample window"),
            // WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            // WS_POPUP | WS_VISIBLE,
            WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            // https://github.com/microsoft/DirectX-Graphics-Samples/blob/master/Samples/Desktop/D3D12HelloWorld/src/HelloWindow/Win32Application.cpp
            CW_USEDEFAULT, // windowRect.right - windowRect.left, perhaps?
            CW_USEDEFAULT, // windowRect.bottom - windowRect.top, perhaps?
            None,
            None,
            instance,
            None,
        )?;

        let extended_style = GetWindowLongA(hwnd, GWL_EXSTYLE);
        println!("GWL_EXSTYLE: {:?}", GWL_EXSTYLE);
        println!("WS_EX_TRANSPARENT: {:?}", WS_EX_TRANSPARENT);
        // https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features#layered-windows
        //SetWindowLongA(hwnd, GWL_EXSTYLE, extended_style | WS_EX_TRANSPARENT.0 as i32 | WS_EX_TOPMOST.0 as i32 | WS_EX_LAYERED.0 as i32);

        setup_gdi()?;
        // let mut graphics: GpGraphics = Default::default();
        let mut graphics: *mut GpGraphics = std::ptr::null_mut();
        let gdip = GdipCreateFromHWND(hwnd, &mut graphics);

        let mut pen: *mut GpPen = std::ptr::null_mut();
        windows::Win32::Graphics::GdiPlus::GdipCreatePen1(0xFF0000FF, 3.0, windows::Win32::Graphics::GdiPlus::UnitPixel, &mut pen);
        windows::Win32::Graphics::GdiPlus::GdipDrawLine(graphics, pen, 0.0, 0.0, 100.0, 100.0);
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
