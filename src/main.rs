use std::{ffi::c_void, mem::size_of};

use windows::{
    core::*,
    Win32::{
        Foundation::*, Graphics::Gdi::*, System::LibraryLoader::*, UI::WindowsAndMessaging::*,
    },
};

//Global for now, change later
static mut RUNNING: bool = true;
//These variables are unbelievably scuffed change this shit asap
static mut BIT_MAP_HANDLE: HBITMAP = HBITMAP(0);
static mut BIT_MAP_MEMORY: *mut *mut c_void = std::ptr::null_mut();
static mut BIT_MAP_DEVICE_CONTEXT: HDC = HDC(0);
static mut BIT_MAP_INFO: BITMAPINFO = BITMAPINFO {
    bmiHeader: BITMAPINFOHEADER {
        biSize: 0,
        biWidth: 0,
        biHeight: 0,
        biPlanes: 0,
        biBitCount: 0,
        biCompression: 0,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    },
    bmiColors: [RGBQUAD {
        rgbBlue: 0,
        rgbGreen: 0,
        rgbRed: 0,
        rgbReserved: 0,
    }; 1],
};

pub fn win32_resize_db_section(width: i32, height: i32) {
    unsafe {
        if BIT_MAP_HANDLE.is_invalid() {
            DeleteObject(BIT_MAP_HANDLE);
        }

        if !BIT_MAP_DEVICE_CONTEXT.is_invalid() {
            // Should we recreate these under certain special circumstances?
            BIT_MAP_DEVICE_CONTEXT = CreateCompatibleDC(HDC(0));
        }

        BIT_MAP_INFO.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        BIT_MAP_INFO.bmiHeader.biWidth = width;
        BIT_MAP_INFO.bmiHeader.biHeight = height;
        BIT_MAP_INFO.bmiHeader.biPlanes = 1;
        BIT_MAP_INFO.bmiHeader.biBitCount = 32;
        BIT_MAP_INFO.bmiHeader.biCompression = BI_RGB.0;

        BIT_MAP_HANDLE = match CreateDIBSection(
            BIT_MAP_DEVICE_CONTEXT,
            &BIT_MAP_INFO,
            DIB_RGB_COLORS,
            BIT_MAP_MEMORY,
            HANDLE(0),
            0,
        ) {
            // .unwrap() fails if you try to resize window to below the minimum window size. Temporarily handled like this so program doesn't crash
            Ok(result) => result,
            Err(e) => {
                println!("{}", e);
                HBITMAP::default()
            }
        }
    }
}

pub fn win32_update_window(device_context: HDC, x: i32, y: i32, width: i32, height: i32) {
    unsafe {
        StretchDIBits(
            device_context,
            x,
            y,
            width,
            height,
            x,
            y,
            width,
            height,
            Some(BIT_MAP_MEMORY as *const c_void),
            &BIT_MAP_INFO,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}

pub unsafe extern "system" fn win32_main_window_callback(
    window: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let mut result = LRESULT(0);
        match msg {
            WM_SIZE => {
                let client_rect = &mut RECT::default();
                GetClientRect(window, client_rect);
                let width = client_rect.right - client_rect.left;
                let height = client_rect.bottom - client_rect.top;
                win32_resize_db_section(width, height);
            }
            WM_DESTROY => {
                RUNNING = false;
                // todo!("Handle this as an error")
            }
            WM_CLOSE => {
                RUNNING = false;
                // todo!("Handle this with a message to the user? (Are you sure you want to quit?))")
            }
            WM_ACTIVATEAPP => {
                // todo!()
            }
            WM_PAINT => {
                let paint = &mut PAINTSTRUCT::default();
                let device_context = BeginPaint(window, paint);
                let x = paint.rcPaint.left;
                let y = paint.rcPaint.top;
                let width = paint.rcPaint.right - paint.rcPaint.left;
                let height = paint.rcPaint.bottom - paint.rcPaint.top;
                win32_update_window(device_context, x, y, width, height);
                EndPaint(window, paint);
            }
            _ => {
                result = DefWindowProcW(window, msg, wparam, lparam);
            }
        }

        return result;
    }
}
fn main() -> Result<()> {
    let mut window_class = WNDCLASSW::default();
    unsafe {
        window_class.style = CS_OWNDC | CS_HREDRAW | CS_VREDRAW;
        window_class.lpfnWndProc = Some(win32_main_window_callback);
        window_class.hInstance = GetModuleHandleW(PCWSTR::null()).unwrap().into();
        // window_class.hIcon = ;
        window_class.lpszClassName = w!("GamingWindowClass");

        if RegisterClassW(&window_class) != 0 {
            let window_handle = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                window_class.lpszClassName,
                w!("Gaming"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                HWND(0),
                HMENU(0),
                window_class.hInstance,
                None,
            );
            if window_handle != HWND(0) {
                let mut msg = MSG::default();
                while RUNNING {
                    let message_result = GetMessageW(&mut msg, HWND(0), 0, 0);
                    // Returns 0 if WM_QUIT is received, -1 if there is an error with the message loop
                    if message_result == BOOL(0) || message_result == BOOL(-1) {
                        break;
                    } else {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        }
    }

    Ok(())
}
