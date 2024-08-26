use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::ValidateRect,
    Win32::System::LibraryLoader::GetModuleHandleA, Win32::UI::WindowsAndMessaging::*,
};

// https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
// https://learn.microsoft.com/en-us/windows/win32/winmsg/window-styles

pub fn main() -> Result<()> {
    unsafe {
        let instance = GetModuleHandleA(None)?;
        debug_assert!(instance.0 != 0);

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

        // Extended styles: https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
        let hwnd = CreateWindowExA(
            WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_NOACTIVATE, // | WINDOW_EX_STYLE::default(),
            // WINDOW_EX_STYLE::default(),
            window_class,
            s!("This is a sample window"),
            // WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            // WS_POPUP | WS_VISIBLE,
            WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            instance,
            None,
        );

        let extended_style = GetWindowLongA(hwnd, GWL_EXSTYLE);
        println!("GWL_EXSTYLE: {:?}", GWL_EXSTYLE);
        println!("WS_EX_TRANSPARENT: {:?}", WS_EX_TRANSPARENT);
        // https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features#layered-windows
        //SetWindowLongA(hwnd, GWL_EXSTYLE, extended_style | WS_EX_TRANSPARENT.0 as i32 | WS_EX_TOPMOST.0 as i32 | WS_EX_LAYERED.0 as i32);

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
