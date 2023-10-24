use std::{ffi::c_void, mem::size_of};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::*,
        System::Memory::*,
        UI::{Input::XboxController::*, WindowsAndMessaging::*},
    },
};

//Global for now, change later
static mut RUNNING: bool = true;
static mut x_offset: i32 = 0;
static mut y_offset: i32 = 0;

static mut GLOBAL_BACK_BUFFER: Win32OffscreenBuffer = Win32OffscreenBuffer {
    width: 0,
    height: 0,
    memory: std::ptr::null_mut(),
    info: BITMAPINFO {
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
    },
    pitch: 0,
    bytes_per_pixel: 0,
};

#[derive(Clone, Copy)]
pub struct Win32OffscreenBuffer {
    width: i32,
    height: i32,
    memory: *mut c_void,
    info: BITMAPINFO,
    pitch: usize,
    bytes_per_pixel: i32,
}

pub struct Win32WindowDimension {
    pub width: i32,
    pub height: i32,
}

pub fn win32_get_window_dimension(window: HWND) -> (i32, i32) {
    unsafe {
        let mut client_rect = RECT::default();
        GetClientRect(window, &mut client_rect).unwrap_or_default();
        let width = client_rect.right - client_rect.left;
        let height = client_rect.bottom - client_rect.top;

        return (width, height);
    }
}

pub fn render_gradient(buffer: &Win32OffscreenBuffer, blue_offset: i32, green_offset: i32) {
    unsafe {
        // Draw bitmap to window.
        let mut row: *mut u8 = buffer.memory.cast();
        for y in 0..buffer.height {
            let mut pixel: *mut u32 = row.cast();
            for x in 0..buffer.width {
                //truncate to 8 bits using mask, because i couldn't figure out how to cast it :)
                let blue = x + blue_offset & 0xFF;
                let green = y + green_offset & 0xFF;

                /*
                Memory: BB GG RR xx
                Register: xx RR GG BB

                Shift GG up by 8 bits from bottom, to make them sit correctly in regsiter (little endian)
                 */
                *pixel = ((green << 8) | blue) as u32;
                pixel = pixel.add(1);
            }

            row = row.add(buffer.pitch);
        }
    }
}

pub fn win32_resize_db_section(buffer: &mut Win32OffscreenBuffer, width: i32, height: i32) {
    unsafe {
        if !buffer.memory.is_null() {
            VirtualFree(buffer.memory as *mut c_void, 0, MEM_RELEASE).unwrap_or_default();
        }

        buffer.width = width;
        buffer.height = height;
        buffer.bytes_per_pixel = 4;

        buffer.info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        buffer.info.bmiHeader.biWidth = buffer.width;
        //Negative height means the bitmap is a top-down DIB and it's origin is the upper-left corner. Rows in window go sequentially from top to bottom.
        buffer.info.bmiHeader.biHeight = -buffer.height;
        buffer.info.bmiHeader.biPlanes = 1;
        buffer.info.bmiHeader.biBitCount = 32;
        buffer.info.bmiHeader.biCompression = BI_RGB.0;

        let bit_map_memory_size = ((width * height) * buffer.bytes_per_pixel) as usize;
        buffer.memory = VirtualAlloc(
            Some(std::ptr::null()),
            bit_map_memory_size,
            MEM_COMMIT,
            PAGE_READWRITE,
        )
        .cast();

        // Pitch determies when a new row starts in the sequence of bytes in the bitmap.
        buffer.pitch = (width * buffer.bytes_per_pixel) as usize;
    }
}

pub fn win32_display_buffer_in_window(
    device_context: HDC,
    buffer: &Win32OffscreenBuffer,
    window_width: i32,
    window_height: i32,
) {
    unsafe {
        //Draw whole bitmap onto whole window initially
        //TODO: Aspect ratio correction
        StretchDIBits(
            device_context,
            0,
            0,
            window_width,
            window_height,
            0,
            0,
            buffer.width,
            buffer.height,
            Some(buffer.memory as *const c_void),
            &buffer.info,
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
            WM_SIZE => {}
            WM_DESTROY => {
                RUNNING = false;
                // TODO: Handle this as an error
            }
            WM_CLOSE => {
                RUNNING = false;
                // TODO: Handle this with a message to the user? (Are you sure you want to quit?)
            }
            WM_ACTIVATEAPP => {
                // Called when window is opened?
            }
            WM_SYSKEYDOWN | WM_SYSKEYUP | WM_KEYDOWN | WM_KEYUP => {
                let vk_code = wparam;
                let was_down = (lparam.0 & (1 << 30)) != 0;
                let is_down = (lparam.0 & (1 << 31)) == 0;
                //if was_down != is_down {
                match vk_code {
                    WPARAM(0x57) => {
                        //W key
                        y_offset += 1;
                    }
                    WPARAM(0x41) => {
                        //A key
                        x_offset -= 1;
                    }
                    WPARAM(0x53) => {
                        //S key
                        y_offset -= 1;
                    }
                    WPARAM(0x44) => {
                        //D key
                        x_offset += 1;
                    }
                    WPARAM(0x73) => {
                        let alt_key_was_down = (lparam.0 & (1 << 29)) != 0;
                        if alt_key_was_down {
                            RUNNING = false;
                        }
                    }
                    WPARAM(_) => {}
                }
                //            }
            }
            WM_PAINT => {
                let paint = &mut PAINTSTRUCT::default();
                let device_context = BeginPaint(window, paint);

                let dimension = win32_get_window_dimension(window);
                win32_display_buffer_in_window(
                    device_context,
                    &GLOBAL_BACK_BUFFER,
                    dimension.0,
                    dimension.1,
                );
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
        win32_resize_db_section(&mut GLOBAL_BACK_BUFFER, 1280, 720);

        window_class.style = CS_HREDRAW | CS_VREDRAW;
        window_class.lpfnWndProc = Some(win32_main_window_callback);
        window_class.hInstance = GetModuleHandleW(PCWSTR::null()).unwrap().into();
        // window_class.hIcon = ;
        window_class.lpszClassName = w!("GamingWindowClass");

        if RegisterClassW(&window_class) != 0 {
            let window = CreateWindowExW(
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
            if window != HWND(0) {
                let mut msg = MSG::default();

                while RUNNING {
                    while (PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE)).into() {
                        if msg.message == WM_QUIT {
                            RUNNING = false;
                        }

                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }

                    //CONTROLLER INPUT
                    //Should we poll this more frequently?
                    // for controller_index in 0..XUSER_MAX_COUNT {
                    //     let controller_state = &mut XINPUT_STATE::default();
                    //     if XInputGetState(controller_index, controller_state) == ERROR_SUCCESS.0 {
                    //         //Controller is plugged in
                    //         let pad = &controller_state.Gamepad;

                    //         let up = pad.wButtons & XINPUT_GAMEPAD_DPAD_UP;
                    //         let down = pad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN;
                    //         let left = pad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT;
                    //         let right = pad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT;
                    //         let start = pad.wButtons & XINPUT_GAMEPAD_START;
                    //         let back = pad.wButtons & XINPUT_GAMEPAD_BACK;
                    //         let left_shoulder = pad.wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER;
                    //         let right_shoulder = pad.wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER;
                    //         let a_button = pad.wButtons & XINPUT_GAMEPAD_A;
                    //         let b_button = pad.wButtons & XINPUT_GAMEPAD_B;
                    //         let x_button = pad.wButtons & XINPUT_GAMEPAD_X;
                    //         let y_button = pad.wButtons & XINPUT_GAMEPAD_Y;

                    //         let stick_x = pad.sThumbLX;
                    //         let stick_y = pad.sThumbLY;
                    //     } else {
                    //         //Controller is not available
                    //     }
                    // }

                    render_gradient(&GLOBAL_BACK_BUFFER, x_offset, y_offset);

                    let device_context = GetDC(window);
                    let dimension = win32_get_window_dimension(window);

                    win32_display_buffer_in_window(
                        device_context,
                        &GLOBAL_BACK_BUFFER,
                        dimension.0,
                        dimension.1,
                    );
                    ReleaseDC(window, device_context);
                }
            } else {
                //Logging
            }
        }
    }

    Ok(())
}
