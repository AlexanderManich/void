#![deny(unsafe_op_in_unsafe_fn)]

use std::{env, ffi::c_void, mem::size_of};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        Media::Audio::{
            DirectSound::{
                DirectSoundCreate8, IDirectSound8, IDirectSoundBuffer, DSBCAPS_PRIMARYBUFFER,
                DSBPLAY_LOOPING, DSBUFFERDESC, DSSCL_PRIORITY,
            },
            WAVEFORMATEX, WAVE_FORMAT_PCM,
        },
        System::LibraryLoader::*,
        System::Memory::*,
        UI::{
            Input::XboxController::{
                XInputGetState, XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_BACK,
                XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT,
                XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_LEFT_SHOULDER,
                XINPUT_GAMEPAD_RIGHT_SHOULDER, XINPUT_GAMEPAD_START, XINPUT_GAMEPAD_X,
                XINPUT_GAMEPAD_Y, XINPUT_STATE, XUSER_MAX_COUNT,
            },
            WindowsAndMessaging::*,
        },
    },
};

//Global for now, change later
static mut RUNNING: bool = true;
static mut DIRECT_SOUND: Option<IDirectSound8> = None;
static mut SECONDARY_BUFFER: Option<IDirectSoundBuffer> = None;

static mut X_OFFSET: i32 = 0;
static mut Y_OFFSET: i32 = 0;

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

//DirectSound api (very old)
pub fn win32_init_d_sound(window: HWND, samples_per_second: u32, buffer_size: u32) -> Result<()> {
    // Define WaveFormat here since we're going to use it for multiple buffers
    let mut wave_format = WAVEFORMATEX::default();
    wave_format.wFormatTag = WAVE_FORMAT_PCM as u16; // This might introduce a bug, idk why the struct takes u16 when all the enum variants are u32?
    wave_format.nChannels = 2;
    wave_format.nSamplesPerSec = samples_per_second;
    wave_format.wBitsPerSample = 16;
    wave_format.nBlockAlign = (wave_format.nChannels * wave_format.wBitsPerSample) / 8;
    wave_format.nAvgBytesPerSec = u32::from(wave_format.nBlockAlign) * wave_format.nSamplesPerSec;
    wave_format.cbSize = 0;

    //Get a DirectSound object
    /* DIRECT_SOUND declared globally to fix bug where the pointer to SECONDARY_BUFFER got dropped when DIRECT_SOUND went out of scope,
    making the SECONDARY_BUFFER pointer unaligned, causing the program to crash when calling any of the methods on the buffer.
    A solution might be to just returning the buffer from the function, but that seems to cause the same problem */

    unsafe {
        match DirectSoundCreate8(Some(std::ptr::null()), &mut DIRECT_SOUND, None) {
            Ok(_) => {}
            Err(e) => {
                println!("DirectSoundCreate() Error: {}", e);
            }
        };
        match DIRECT_SOUND
            .as_ref()
            .unwrap()
            .SetCooperativeLevel(window, DSSCL_PRIORITY)
        {
            Ok(_) => {}
            Err(e) => {
                println!("SetCooperativeLevel() Error: {}", e);
            }
        }
    }

    // Create a primary buffer. Primary buffer is only created to get a handle, to be able to call SetFormat.
    let mut primary_buffer = None;

    //BufferDescription for primary buffer
    let mut buffer_description = DSBUFFERDESC::default();
    buffer_description.dwSize = size_of::<DSBUFFERDESC>() as u32;
    buffer_description.dwFlags = DSBCAPS_PRIMARYBUFFER;

    unsafe {
        match DIRECT_SOUND.as_ref().unwrap().CreateSoundBuffer(
            &buffer_description,
            &mut primary_buffer,
            None,
        ) {
            Ok(_) => {}
            Err(e) => {
                println!("CreateSoundBuffer() Error: {}", e);
            }
        }
        match primary_buffer.unwrap().SetFormat(&wave_format) {
            Ok(_) => {}
            Err(e) => {
                println!("SetFormat() Error: {}", e);
            }
        };
    }

    // Create a secondary buffer that we are actually going to write into. (Declared globally)

    // BufferDescription for secondary buffer
    let mut buffer_description = DSBUFFERDESC::default();
    buffer_description.dwSize = size_of::<DSBUFFERDESC>() as u32;
    buffer_description.dwFlags = 0;
    buffer_description.dwBufferBytes = buffer_size;
    buffer_description.lpwfxFormat = &mut wave_format;
    unsafe {
        match DIRECT_SOUND.as_ref().unwrap().CreateSoundBuffer(
            &buffer_description,
            &mut SECONDARY_BUFFER,
            None,
        ) {
            Ok(_) => {}
            Err(e) => {
                println!("CreateSoundBuffer() Error: {}", e);
            }
        }
        // Start playing

        Ok(())
    }
}

pub fn win32_get_window_dimension(window: HWND) -> Result<(i32, i32)> {
    let mut client_rect = RECT::default();
    unsafe {
        GetClientRect(window, &mut client_rect).unwrap();
    }
    let width = client_rect.right - client_rect.left;
    let height = client_rect.bottom - client_rect.top;

    Ok((width, height))
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
                //Check prevents repeat key presses when holding key
                if was_down != is_down {
                    match vk_code {
                        WPARAM(0x57) => {
                            //W key
                            Y_OFFSET += 1;
                        }
                        WPARAM(0x41) => {
                            //A key
                            X_OFFSET -= 1;
                        }
                        WPARAM(0x53) => {
                            //S key
                            Y_OFFSET -= 1;
                        }
                        WPARAM(0x44) => {
                            //D key
                            X_OFFSET += 1;
                        }
                        WPARAM(0x73) => {
                            let alt_key_was_down = (lparam.0 & (1 << 29)) != 0;
                            if alt_key_was_down {
                                RUNNING = false;
                            }
                        }
                        WPARAM(_) => {}
                    }
                }
            }
            WM_PAINT => {
                let paint = &mut PAINTSTRUCT::default();
                let device_context = BeginPaint(window, paint);

                let dimension = win32_get_window_dimension(window).unwrap_or_default();
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
    env::set_var("RUST_BACKTRACE", "0");

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
                let device_context = GetDC(window);

                // Sound test
                let samples_per_second = 48000;
                let tone_hz = 256;
                let tone_volume = 200;
                let mut running_sample_index = 0;
                let square_wave_period = samples_per_second / tone_hz;
                let half_square_wave_period = square_wave_period / 2;
                let bytes_per_sample = size_of::<i16>() as u32 * 2;
                let secondary_buffer_size = samples_per_second * bytes_per_sample;

                win32_init_d_sound(window, samples_per_second, secondary_buffer_size).unwrap();

                match SECONDARY_BUFFER
                    .as_ref()
                    .unwrap()
                    .Play(0, 0, DSBPLAY_LOOPING)
                {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Play() Error: {}", e);
                    }
                }

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
                    for controller_index in 0..XUSER_MAX_COUNT {
                        let controller_state = &mut XINPUT_STATE::default();
                        if XInputGetState(controller_index, controller_state) == ERROR_SUCCESS.0 {
                            //Controller is plugged in
                            let pad = &controller_state.Gamepad;

                            let up = pad.wButtons & XINPUT_GAMEPAD_DPAD_UP;
                            let down = pad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN;
                            let left = pad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT;
                            let right = pad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT;
                            let start = pad.wButtons & XINPUT_GAMEPAD_START;
                            let back = pad.wButtons & XINPUT_GAMEPAD_BACK;
                            let left_shoulder = pad.wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER;
                            let right_shoulder = pad.wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER;
                            let a_button = pad.wButtons & XINPUT_GAMEPAD_A;
                            let b_button = pad.wButtons & XINPUT_GAMEPAD_B;
                            let x_button = pad.wButtons & XINPUT_GAMEPAD_X;
                            let y_button = pad.wButtons & XINPUT_GAMEPAD_Y;

                            let stick_x = pad.sThumbLX;
                            let stick_y = pad.sThumbLY;
                        } else {
                            //Controller is not available
                        }
                    }

                    render_gradient(&GLOBAL_BACK_BUFFER, X_OFFSET, Y_OFFSET);

                    // DirectSound output test
                    let mut play_cursor = 0;
                    let mut write_cursor = 0;
                    match SECONDARY_BUFFER
                        .as_ref()
                        .unwrap()
                        .GetCurrentPosition(Some(&mut play_cursor), Some(&mut write_cursor))
                    {
                        Ok(_) => {}
                        Err(e) => {
                            println!("GetCurrentPosition() Error: {}", e);
                        }
                    }
                    {
                        let byte_to_lock =
                            (running_sample_index * bytes_per_sample) % secondary_buffer_size;
                        let mut bytes_to_write;
                        if byte_to_lock > play_cursor {
                            bytes_to_write = secondary_buffer_size - byte_to_lock;
                            bytes_to_write += play_cursor;
                        } else {
                            bytes_to_write = play_cursor - byte_to_lock;
                        }
                        let mut region1 = std::ptr::null_mut();
                        let mut region1_size = 0;
                        let mut region2 = std::ptr::null_mut();
                        let mut region2_size = 0;

                        // println!("Byte to lock: {:?}\n bytes_to_write: {:?}\n region1: {:?}\n region1_size: {:?}\n region2: {:?}\n region2_size {:?}\n", byte_to_lock, bytes_to_write, region1, region1_size, region2, region2_size);
                        match SECONDARY_BUFFER.as_ref().unwrap().Lock(
                            byte_to_lock,
                            bytes_to_write,
                            &mut region1,
                            &mut region1_size,
                            Some(&mut region2),
                            Some(&mut region2_size),
                            0,
                        ) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Lock() Error: {}", e);
                            }
                        }

                        // println!("Byte to lock: {:?}\n bytes_to_write: {:?}\n region1: {:?}\n region1_size: {:?}\n region2: {:?}\n region2_size {:?}\n", byte_to_lock, bytes_to_write, region1, region1_size, region2, region2_size);

                        let mut sample_out = region1 as *mut i16;
                        let region1_sample_count = region1_size / bytes_per_sample;

                        for _ in 0..region1_sample_count {
                            let sample_value: i16;
                            if ((running_sample_index / half_square_wave_period) % 2) == 0 {
                                sample_value = tone_volume;
                            } else {
                                sample_value = -tone_volume;
                            }
                            *sample_out = sample_value;
                            sample_out = sample_out.add(1);
                            *sample_out = sample_value;
                            sample_out = sample_out.add(1);
                            running_sample_index += 1;
                        }

                        sample_out = region2 as *mut i16;
                        let region2_sample_count = region2_size / bytes_per_sample;

                        for _ in 0..region2_sample_count {
                            let sample_value: i16;
                            if ((running_sample_index / half_square_wave_period) % 2) == 0 {
                                sample_value = tone_volume;
                            } else {
                                sample_value = -tone_volume;
                            }
                            *sample_out = sample_value;
                            sample_out = sample_out.add(1);
                            *sample_out = sample_value;
                            sample_out = sample_out.add(1);
                            running_sample_index += 1;
                        }
                        match SECONDARY_BUFFER.as_ref().unwrap().Unlock(
                            region1,
                            region1_size,
                            Some(region2),
                            region2_size,
                        ) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Unlock() Error: {}", e);
                            }
                        }
                    }
                    let dimension = win32_get_window_dimension(window).unwrap();

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
