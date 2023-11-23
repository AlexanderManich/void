#![deny(unsafe_op_in_unsafe_fn)]
use std::{arch::x86_64::_rdtsc, ffi::c_void, mem::size_of, ptr::swap};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        Media::{
            timeBeginPeriod,
            Audio::{DirectSound::*, WAVEFORMATEX, WAVE_FORMAT_PCM},
            TIMERR_NOERROR,
        },
        Storage::FileSystem::*,
        System::{LibraryLoader::*, Memory::*, Performance::*, Threading::Sleep},
        UI::{Input::XboxController::*, WindowsAndMessaging::*},
    },
};

use crate::{gaming_engine::*, macros::*};

#[derive(Clone, Copy)]
pub struct Win32OffscreenBuffer {
    width: i32,
    height: i32,
    memory: *mut c_void,
    info: BITMAPINFO,
    pitch: usize,
    bytes_per_pixel: i32,
}

impl Default for Win32OffscreenBuffer {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}

#[derive(Default, Clone)]
pub struct Win32SoundOutput {
    samples_per_second: u32,
    running_sample_index: u32,
    bytes_per_sample: u32,
    secondary_buffer_size: u32,
    latency_sample_count: u32,
}

#[inline]
fn safe_truncateu64(value: u64) -> u32 {
    debug_assert!(value <= u32::MAX.into());
    value as u32
}

#[cfg(feature = "gaming_internal")]
pub struct DebugReadFileResult {
    pub contents: *mut c_void,
    pub contents_size: u32,
}

impl Default for DebugReadFileResult {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}

#[derive(Copy, Clone)]
struct Win32DebugTimeMarker {
    play_cursor: u32,
    write_cursor: u32,
}

impl Default for Win32DebugTimeMarker {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}

//Global for now, change later
static mut RUNNING: bool = true;
static mut PERF_COUNTER_FREQ: i64 = 0;
static mut DIRECT_SOUND: Option<IDirectSound8> = None;
static mut SECONDARY_BUFFER: Option<IDirectSoundBuffer> = None;
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

fn win32_debug_draw_vertical(
    back_buffer: &Win32OffscreenBuffer,
    x: i32,
    top: i32,
    bottom: i32,
    color: u32,
) {
    let mut pixel = back_buffer.memory as *mut u8;
    unsafe {
        pixel = pixel.add((x * back_buffer.bytes_per_pixel) as usize);
        pixel = pixel.add((top * back_buffer.pitch as i32) as usize);
    }
    for _y in top..bottom {
        unsafe {
            *(pixel as *mut u32) = color;
            pixel = pixel.add(back_buffer.pitch);
        }
    }
}

#[inline]
fn win32_draw_sound_buffer_marker(
    back_buffer: &Win32OffscreenBuffer,
    sound_output: &Win32SoundOutput,
    c: f32,
    pad_x: i32,
    top: i32,
    bottom: i32,
    value: u32,
    color: u32,
) {
    debug_assert!(value < sound_output.secondary_buffer_size);
    let x_f32 = c * value as f32;
    let x = x_f32 as i32 + pad_x;

    win32_debug_draw_vertical(back_buffer, x, top, bottom, color);
}

fn win32_debug_sync_display(
    back_buffer: &Win32OffscreenBuffer,
    marker_count: usize,
    markers: &[Win32DebugTimeMarker],
    sound_output: &Win32SoundOutput,
    target_seconds_per_frame: f32,
) {
    let pad_x = 16;
    let pad_y = 16;

    let top = pad_y;
    let bottom = back_buffer.height - pad_y;

    let c = (back_buffer.width - 2 * pad_x) as f32 / sound_output.secondary_buffer_size as f32;
    (0..marker_count).for_each(|marker_index| {
        let this_marker = markers[marker_index];
        win32_draw_sound_buffer_marker(
            back_buffer,
            sound_output,
            c,
            pad_x,
            top,
            bottom,
            this_marker.play_cursor,
            0xFFFFFFFF,
        );
        win32_draw_sound_buffer_marker(
            back_buffer,
            sound_output,
            c,
            pad_x,
            top,
            bottom,
            this_marker.write_cursor,
            0xFFFF0000,
        );
    });
}

#[inline]
fn win32_get_wall_clock() -> i64 {
    let mut result = 0;
    unsafe { QueryPerformanceCounter(&mut result).unwrap() }
    result
}

#[inline]
fn win32_get_seconds_elapsed(start: i64, end: i64) -> f32 {
    (end - start) as f32 / unsafe { PERF_COUNTER_FREQ } as f32
}

fn win32_process_input_stick_value(value: i16, dead_zone_threshold: u16) -> f32 {
    let mut x: f32 = 0.0;
    if value < -(dead_zone_threshold as i16) {
        x = (value + dead_zone_threshold as i16) as f32 / (32760.0 - dead_zone_threshold as f32);
    } else if value > dead_zone_threshold as i16 {
        x = (value - dead_zone_threshold as i16) as f32 / (32767.0 - dead_zone_threshold as f32);
    }
    x
}

fn win32_process_pending_messages(keyboard_controller: &mut GameControllerInput) {
    let mut msg = MSG::default();

    unsafe {
        while PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).as_bool() {
            match msg.message {
                WM_QUIT => {
                    RUNNING = false;
                }
                WM_SYSKEYDOWN | WM_SYSKEYUP | WM_KEYDOWN | WM_KEYUP => {
                    let vk_code = msg.wParam;
                    let was_down = (msg.lParam.0 & (1 << 30)) != 0;
                    let is_down = (msg.lParam.0 & (1 << 31)) == 0;
                    //Check prevents repeat key presses when holding key
                    if was_down != is_down {
                        match vk_code {
                            WPARAM(0x57) => {
                                //W key
                                win32_process_keyboard_message(
                                    &mut keyboard_controller.button_state.concrete_buttons.move_up,
                                    is_down,
                                )
                            }
                            WPARAM(0x41) => {
                                //A key
                                win32_process_keyboard_message(
                                    &mut keyboard_controller
                                        .button_state
                                        .concrete_buttons
                                        .move_left,
                                    is_down,
                                )
                            }
                            WPARAM(0x53) => {
                                //S key
                                win32_process_keyboard_message(
                                    &mut keyboard_controller
                                        .button_state
                                        .concrete_buttons
                                        .move_down,
                                    is_down,
                                )
                            }
                            WPARAM(0x44) => {
                                //D key
                                win32_process_keyboard_message(
                                    &mut keyboard_controller
                                        .button_state
                                        .concrete_buttons
                                        .move_right,
                                    is_down,
                                )
                            }
                            WPARAM(0x51) => {
                                //Q Key
                                win32_process_keyboard_message(
                                    &mut keyboard_controller
                                        .button_state
                                        .concrete_buttons
                                        .left_shoulder,
                                    is_down,
                                )
                            }
                            WPARAM(0x45) => {
                                //E Key
                                win32_process_keyboard_message(
                                    &mut keyboard_controller
                                        .button_state
                                        .concrete_buttons
                                        .right_shoulder,
                                    is_down,
                                )
                            }
                            WPARAM(0x26) => {
                                //Up arrow
                                win32_process_keyboard_message(
                                    &mut keyboard_controller
                                        .button_state
                                        .concrete_buttons
                                        .action_up,
                                    is_down,
                                );
                            }
                            WPARAM(0x25) => {
                                //Left arrow
                                win32_process_keyboard_message(
                                    &mut keyboard_controller
                                        .button_state
                                        .concrete_buttons
                                        .action_left,
                                    is_down,
                                );
                            }
                            WPARAM(0x28) => {
                                //Down arrow
                                win32_process_keyboard_message(
                                    &mut keyboard_controller
                                        .button_state
                                        .concrete_buttons
                                        .action_down,
                                    is_down,
                                );
                            }
                            WPARAM(0x27) => {
                                //Right arrow
                                win32_process_keyboard_message(
                                    &mut keyboard_controller
                                        .button_state
                                        .concrete_buttons
                                        .action_right,
                                    is_down,
                                );
                            }
                            WPARAM(0x1b) => {
                                //Escape key
                                win32_process_keyboard_message(
                                    &mut keyboard_controller.button_state.concrete_buttons.start,
                                    is_down,
                                );
                            }
                            WPARAM(0x20) => {
                                //Space key
                                win32_process_keyboard_message(
                                    &mut keyboard_controller.button_state.concrete_buttons.back,
                                    is_down,
                                );
                            }
                            WPARAM(0x73) => {
                                //F4 key
                                let alt_key_was_down = (msg.lParam.0 & (1 << 29)) != 0;
                                if alt_key_was_down {
                                    RUNNING = false;
                                }
                            }
                            WPARAM(_) => {}
                        }
                    }
                }
                _ => {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }
}

pub fn debug_platform_read_entire_file(filename: PCWSTR) -> Result<DebugReadFileResult> {
    unsafe {
        let mut result: DebugReadFileResult = Default::default();
        let file_handle = CreateFileW(
            filename,
            GENERIC_READ.0,
            FILE_SHARE_READ,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            HANDLE(0),
        )?;

        let mut file_size: i64 = 0;
        GetFileSizeEx(file_handle, &mut file_size)?;
        let file_size32 = safe_truncateu64(file_size as u64);
        result.contents = VirtualAlloc(
            None,
            file_size32 as usize,
            MEM_RESERVE | MEM_COMMIT,
            PAGE_READWRITE,
        );

        let read_buffer = &mut *result.contents.cast::<u8>();

        if !result.contents.is_null() {
            let mut bytes_read: u32 = 0;
            match ReadFile(
                file_handle,
                Some(std::slice::from_raw_parts_mut(
                    read_buffer,
                    file_size32 as usize,
                )),
                Some(&mut bytes_read),
                None,
            ) {
                Ok(()) => {
                    debug_assert!(bytes_read == file_size32);
                    result.contents_size = file_size32;
                }
                Err(e) => {
                    dbg!(debug_platform_free_file_memory(result.contents));
                    dbg!(e);
                    result.contents = std::ptr::null_mut();
                }
            }
        }
        CloseHandle(file_handle)?;

        Ok(result)
    }
}

pub fn debug_platform_free_file_memory(memory: *mut c_void) {
    if !memory.is_null() {
        unsafe {
            VirtualFree(memory, 0, MEM_RELEASE).unwrap_or_else(|e| {
                dbg!(e);
            });
        }
    }
}

pub fn debug_platform_write_entire_file(
    filename: PCWSTR,
    memory_size: u32,
    memory: *mut c_void,
) -> Result<()> {
    unsafe {
        let file_handle = CreateFileW(
            filename,
            GENERIC_WRITE.0,
            FILE_SHARE_NONE,
            None,
            CREATE_ALWAYS,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            HANDLE(0),
        )?;

        let write_buffer = &mut *memory.cast::<u8>();

        let mut bytes_written: u32 = 0;
        match WriteFile(
            file_handle,
            Some(std::slice::from_raw_parts_mut(
                write_buffer,
                memory_size as usize,
            )),
            Some(&mut bytes_written),
            None,
        ) {
            Ok(()) => {
                debug_assert!(bytes_written == memory_size);
                // File is read successfully, dont know if something should happen here
            }
            Err(e) => {
                dbg!(e);
            }
        }

        CloseHandle(file_handle)?;

        Ok(())
    }
}

fn win32_process_keyboard_message(new_state: &mut GameButtonState, is_down: bool) {
    debug_assert!(new_state.ended_down != is_down);
    new_state.ended_down = is_down;
    new_state.half_transition_count += 1;
}

pub fn win32_process_xinput_digital_button(
    //Maybe this function should just take u16 instead of XINPUT_GAMEPAD_BUTTON_FLAGS, and then just get the int from the flag
    x_input_button_state: XINPUT_GAMEPAD_BUTTON_FLAGS,
    old_state: GameButtonState,
    button_bit: XINPUT_GAMEPAD_BUTTON_FLAGS,
    new_state: &mut GameButtonState,
) {
    new_state.ended_down = (x_input_button_state & button_bit) == button_bit;

    if old_state.ended_down != new_state.ended_down {
        new_state.half_transition_count = 1;
    } else {
        new_state.half_transition_count = 0;
    }
}

pub fn win32_clear_sound_buffer(sound_output: &Win32SoundOutput) {
    let mut region1: *mut c_void = std::ptr::null_mut();
    let mut region1_size = 0;
    let mut region2: *mut c_void = std::ptr::null_mut();
    let mut region2_size = 0;

    unsafe {
        SECONDARY_BUFFER
            .as_ref()
            .unwrap()
            .Lock(
                0,
                sound_output.secondary_buffer_size,
                &mut region1,
                &mut region1_size,
                Some(&mut region2),
                Some(&mut region2_size),
                0,
            )
            .unwrap_or_else(|e| {
                dbg!(e);
            })
    }

    let mut dest_sample = region1 as *mut u8;

    for _ in 0..region1_size {
        unsafe {
            *dest_sample = 0;
            dest_sample = dest_sample.add(1);
        }
    }

    dest_sample = region2 as *mut u8;

    for _ in 0..region2_size {
        unsafe {
            *dest_sample = 0;
            dest_sample = dest_sample.add(1);
        }
    }

    unsafe {
        SECONDARY_BUFFER
            .as_ref()
            .unwrap()
            .Unlock(region1, region1_size, Some(region2), region2_size)
            .unwrap_or_else(|e| {
                dbg!(e);
            });
    }
}

pub fn win32_fill_sound_buffer(
    sound_output: &mut Win32SoundOutput,
    byte_to_lock: u32,
    bytes_to_write: u32,
    source_buffer: GameSoundOutputBuffer,
) {
    let mut region1 = std::ptr::null_mut();
    let mut region1_size = 0;
    let mut region2 = std::ptr::null_mut();
    let mut region2_size = 0;

    unsafe {
        // TODO: There is a bug here, where sometimes the parameters are not correct
        SECONDARY_BUFFER
            .as_ref()
            .unwrap()
            .Lock(
                byte_to_lock,
                bytes_to_write,
                &mut region1,
                &mut region1_size,
                Some(&mut region2),
                Some(&mut region2_size),
                0,
            )
            .unwrap_or_else(|e| {
                dbg!(e);
            });
    }

    let region1_sample_count = region1_size / sound_output.bytes_per_sample;
    let mut dest_sample = region1 as *mut i16;
    let mut source_sample = source_buffer.samples;

    for _ in 0..region1_sample_count {
        unsafe {
            *dest_sample = *source_sample;
            source_sample = source_sample.add(1);
            dest_sample = dest_sample.add(1);
            *dest_sample = *source_sample;
            source_sample = source_sample.add(1);
            dest_sample = dest_sample.add(1);
            sound_output.running_sample_index += 1;
        }
    }

    dest_sample = region2 as *mut i16;
    let region2_sample_count = region2_size / sound_output.bytes_per_sample;

    for _ in 0..region2_sample_count {
        unsafe {
            *dest_sample = *source_sample;
            source_sample = source_sample.add(1);
            dest_sample = dest_sample.add(1);
            *dest_sample = *source_sample;
            source_sample = source_sample.add(1);
            dest_sample = dest_sample.add(1);
            sound_output.running_sample_index += 1;
        }
    }
    unsafe {
        SECONDARY_BUFFER
            .as_ref()
            .unwrap()
            .Unlock(region1, region1_size, Some(region2), region2_size)
            .unwrap_or_else(|e| {
                dbg!(e);
            })
    }
}

//DirectSound api (very old)
pub fn win32_init_d_sound(window: HWND, samples_per_second: u32, buffer_size: u32) -> Result<()> {
    // Define WaveFormat here since we're going to use it for multiple buffers
    let mut wave_format = WAVEFORMATEX::default();
    wave_format.wFormatTag = WAVE_FORMAT_PCM as u16; // Why the struct takes u16 when all the enum variants are u32?
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
        DirectSoundCreate8(None, &mut DIRECT_SOUND, None).unwrap_or_else(|e| {
            dbg!(e);
        });
        DIRECT_SOUND
            .as_ref()
            .unwrap()
            .SetCooperativeLevel(window, DSSCL_PRIORITY)
            .unwrap_or_else(|e| {
                dbg!(e);
            });
    }

    // Create a primary buffer. Primary buffer is only created to get a handle, to be able to call SetFormat.
    let mut primary_buffer = None;

    //BufferDescription for primary buffer
    let buffer_description = DSBUFFERDESC {
        dwSize: size_of::<DSBUFFERDESC>() as u32,
        dwFlags: DSBCAPS_PRIMARYBUFFER,
        ..Default::default()
    };

    unsafe {
        DIRECT_SOUND
            .as_ref()
            .unwrap()
            .CreateSoundBuffer(&buffer_description, &mut primary_buffer, None)
            .unwrap_or_else(|e| {
                dbg!(e);
            });
        primary_buffer
            .unwrap()
            .SetFormat(&wave_format)
            .unwrap_or_else(|e| {
                dbg!(e);
            });
    }

    // Create a secondary buffer that we are actually going to write into. (Declared globally)

    // BufferDescription for secondary buffer
    let buffer_description = DSBUFFERDESC {
        dwSize: size_of::<DSBUFFERDESC>() as u32,
        dwFlags: DSBCAPS_GETCURRENTPOSITION2,
        dwBufferBytes: buffer_size,
        lpwfxFormat: &mut wave_format,
        ..Default::default()
    };

    unsafe {
        DIRECT_SOUND
            .as_ref()
            .unwrap()
            .CreateSoundBuffer(&buffer_description, &mut SECONDARY_BUFFER, None)
            .unwrap_or_else(|e| {
                dbg!(e);
            });
        // Start playing

        Ok(())
    }
}

pub fn win32_get_window_dimension(window: HWND) -> Result<(i32, i32)> {
    let mut client_rect = RECT::default();
    unsafe {
        GetClientRect(window, &mut client_rect)?;
    }
    let width = client_rect.right - client_rect.left;
    let height = client_rect.bottom - client_rect.top;

    Ok((width, height))
}

pub fn win32_resize_dib_section(buffer: &mut Win32OffscreenBuffer, width: i32, height: i32) {
    unsafe {
        if !buffer.memory.is_null() {
            VirtualFree(buffer.memory, 0, MEM_RELEASE).unwrap_or_else(|e| {
                dbg!(e);
            });
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
            None,
            bit_map_memory_size,
            MEM_RESERVE | MEM_COMMIT,
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
                debug_assert!(false);
            }
            WM_PAINT => {
                let paint = &mut PAINTSTRUCT::default();
                let device_context = BeginPaint(window, paint);

                match win32_get_window_dimension(window) {
                    Ok(dimension) => {
                        win32_display_buffer_in_window(
                            device_context,
                            &GLOBAL_BACK_BUFFER,
                            dimension.0,
                            dimension.1,
                        );
                    }
                    Err(e) => {
                        dbg!(e);
                    }
                }

                EndPaint(window, paint);
            }
            _ => {
                result = DefWindowProcW(window, msg, wparam, lparam);
            }
        }

        result
    }
}

pub fn win32_main() -> Result<()> {
    unsafe {
        QueryPerformanceFrequency(&mut PERF_COUNTER_FREQ).unwrap();

        // Set schedular granularity to 1ms to make Sleep() more granular
        let desired_scheduler_ms = 1;
        let sleep_is_granular = timeBeginPeriod(desired_scheduler_ms) != TIMERR_NOERROR;

        let mut window_class = WNDCLASSW::default();

        win32_resize_dib_section(&mut GLOBAL_BACK_BUFFER, 1280, 720);
        window_class.style = CS_HREDRAW | CS_VREDRAW;
        window_class.lpfnWndProc = Some(win32_main_window_callback);
        window_class.hInstance = GetModuleHandleW(PCWSTR::null()).unwrap().into();
        // window_class.hIcon = ;
        window_class.lpszClassName = w!("GamingWindowClass");

        // We should query and get the actual refresh rate of the monitor
        const MONITER_REFRESH_HZ: usize = 144;
        const GAME_UPDATE_HZ: usize = MONITER_REFRESH_HZ / 2;
        const FRAMES_OF_AUDIO_LATENCY: usize = 2;

        let target_seconds_per_frame = 1.0 / GAME_UPDATE_HZ as f32;

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
                let device_context = GetDC(window);

                let mut sound_output = Win32SoundOutput::default();
                sound_output.samples_per_second = 48000;
                sound_output.running_sample_index = 0;
                sound_output.bytes_per_sample = size_of::<i16>() as u32 * 2;
                sound_output.secondary_buffer_size =
                    sound_output.samples_per_second * sound_output.bytes_per_sample;
                sound_output.latency_sample_count = FRAMES_OF_AUDIO_LATENCY as u32
                    * (sound_output.samples_per_second / GAME_UPDATE_HZ as u32);

                win32_init_d_sound(
                    window,
                    sound_output.samples_per_second,
                    sound_output.secondary_buffer_size,
                )
                .unwrap();

                win32_clear_sound_buffer(&sound_output);

                SECONDARY_BUFFER
                    .as_ref()
                    .unwrap()
                    .Play(0, 0, DSBPLAY_LOOPING)
                    .unwrap_or_else(|e| {
                        dbg!(e);
                    });

                RUNNING = true;

                let samples = VirtualAlloc(
                    None,
                    sound_output.secondary_buffer_size as usize,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_READWRITE,
                );

                #[cfg(feature = "gaming_internal")]
                let base_address = Some(tera_bytes!(2) as *const c_void);

                #[cfg(not(feature = "gaming_internal"))]
                let base_address: Option<*const c_void> = None;

                let mut game_memory = GameMemory {
                    permanent_storage_size: mega_bytes!(64),
                    transient_storage_size: giga_bytes!(1),
                    ..Default::default()
                };

                let total_size =
                    game_memory.permanent_storage_size + game_memory.transient_storage_size;
                game_memory.permanent_storage = VirtualAlloc(
                    base_address,
                    total_size as usize,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_READWRITE,
                );

                game_memory.transient_storage = (game_memory.permanent_storage as *mut u8)
                    .add(game_memory.permanent_storage_size as usize)
                    .cast();

                if !samples.is_null()
                    && !game_memory.permanent_storage.is_null()
                    && !game_memory.transient_storage.is_null()
                {
                    let new_input = &mut GameInput::default();
                    let old_input = &mut GameInput::default();

                    let mut last_counter: i64 = win32_get_wall_clock();

                    let mut debug_time_marker_index = 0;
                    let mut debug_time_markers =
                        [Win32DebugTimeMarker::default(); GAME_UPDATE_HZ / 2];

                    let mut last_play_cursor = 0;
                    let mut sound_is_valid = false;

                    let mut last_cycle_count = _rdtsc();

                    while RUNNING {
                        let old_keyboard_controller = get_controller(old_input, 0);
                        let new_keyboard_controller = get_controller(new_input, 0);
                        *new_keyboard_controller = Default::default();
                        new_keyboard_controller.is_connected = true;

                        for button_index in 0..new_keyboard_controller.button_state.buttons.len() {
                            new_keyboard_controller.button_state.buttons[button_index].ended_down =
                                old_keyboard_controller.button_state.buttons[button_index]
                                    .ended_down;
                        }
                        win32_process_pending_messages(new_keyboard_controller);

                        //CONTROLLER INPUT
                        //Should we poll this more frequently?
                        let mut max_controller_count = XUSER_MAX_COUNT;

                        if max_controller_count > (new_input.controllers.len() - 1) as u32 {
                            max_controller_count = (new_input.controllers.len() - 1) as u32;
                        }

                        for controller_index in 0..XUSER_MAX_COUNT {
                            let our_controller_index = controller_index + 1;
                            let old_controller =
                                get_controller(old_input, our_controller_index as usize);
                            let new_controller =
                                get_controller(new_input, our_controller_index as usize);
                            let controller_state = &mut XINPUT_STATE::default();
                            if XInputGetState(controller_index, controller_state) == ERROR_SUCCESS.0
                            {
                                new_controller.is_connected = true;
                                //Controller is plugged in
                                let pad = &controller_state.Gamepad;

                                new_controller.stick_average_x = win32_process_input_stick_value(
                                    pad.sThumbLX,
                                    XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE.0,
                                );
                                new_controller.stick_average_y = win32_process_input_stick_value(
                                    pad.sThumbLY,
                                    XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE.0,
                                );
                                if new_controller.stick_average_x != 0.0
                                    || new_controller.stick_average_y != 0.0
                                {
                                    new_controller.is_analog = true;
                                }

                                if (pad.wButtons & XINPUT_GAMEPAD_DPAD_UP).0 != 0 {
                                    new_controller.stick_average_y = 1.0;
                                    new_controller.is_analog = false;
                                }
                                if (pad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN).0 != 0 {
                                    new_controller.stick_average_y = -1.0;
                                    new_controller.is_analog = false;
                                }
                                if (pad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT).0 != 0 {
                                    new_controller.stick_average_x = -1.0;
                                    new_controller.is_analog = false;
                                }
                                if (pad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT).0 != 0 {
                                    new_controller.stick_average_x = 1.0;
                                    new_controller.is_analog = false;
                                }

                                let threshold: f32 = 0.5;

                                win32_process_xinput_digital_button(
                                    //TODO: refer to comment in function definition
                                    XINPUT_GAMEPAD_BUTTON_FLAGS(
                                        if new_controller.stick_average_x < -threshold {
                                            1
                                        } else {
                                            0
                                        },
                                    ),
                                    old_controller.button_state.concrete_buttons.move_left,
                                    XINPUT_GAMEPAD_BUTTON_FLAGS(1),
                                    &mut new_controller.button_state.concrete_buttons.move_left,
                                );
                                win32_process_xinput_digital_button(
                                    //TODO: refer to comment in function definition
                                    XINPUT_GAMEPAD_BUTTON_FLAGS(
                                        if new_controller.stick_average_x > threshold {
                                            1
                                        } else {
                                            0
                                        },
                                    ),
                                    old_controller.button_state.concrete_buttons.move_right,
                                    XINPUT_GAMEPAD_BUTTON_FLAGS(1),
                                    &mut new_controller.button_state.concrete_buttons.move_right,
                                );
                                win32_process_xinput_digital_button(
                                    //TODO: refer to comment in function definition
                                    XINPUT_GAMEPAD_BUTTON_FLAGS(
                                        if new_controller.stick_average_y < -threshold {
                                            1
                                        } else {
                                            0
                                        },
                                    ),
                                    old_controller.button_state.concrete_buttons.move_down,
                                    XINPUT_GAMEPAD_BUTTON_FLAGS(1),
                                    &mut new_controller.button_state.concrete_buttons.move_down,
                                );
                                win32_process_xinput_digital_button(
                                    //TODO: refer to comment in function definition
                                    XINPUT_GAMEPAD_BUTTON_FLAGS(
                                        if new_controller.stick_average_y > threshold {
                                            1
                                        } else {
                                            0
                                        },
                                    ),
                                    old_controller.button_state.concrete_buttons.move_up,
                                    XINPUT_GAMEPAD_BUTTON_FLAGS(1),
                                    &mut new_controller.button_state.concrete_buttons.move_up,
                                );

                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.action_down,
                                    XINPUT_GAMEPAD_A,
                                    &mut new_controller.button_state.concrete_buttons.action_down,
                                );
                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.action_right,
                                    XINPUT_GAMEPAD_B,
                                    &mut new_controller.button_state.concrete_buttons.action_right,
                                );
                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.action_left,
                                    XINPUT_GAMEPAD_X,
                                    &mut new_controller.button_state.concrete_buttons.action_left,
                                );
                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.action_up,
                                    XINPUT_GAMEPAD_Y,
                                    &mut new_controller.button_state.concrete_buttons.action_up,
                                );
                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.left_shoulder,
                                    XINPUT_GAMEPAD_LEFT_SHOULDER,
                                    &mut new_controller.button_state.concrete_buttons.left_shoulder,
                                );
                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.right_shoulder,
                                    XINPUT_GAMEPAD_RIGHT_SHOULDER,
                                    &mut new_controller
                                        .button_state
                                        .concrete_buttons
                                        .right_shoulder,
                                );
                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.back,
                                    XINPUT_GAMEPAD_BACK,
                                    &mut new_controller.button_state.concrete_buttons.back,
                                );
                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.start,
                                    XINPUT_GAMEPAD_START,
                                    &mut new_controller.button_state.concrete_buttons.start,
                                );
                            } else {
                                //Controller is not available
                                // dbg!(controller_state);
                                new_controller.is_connected = false;
                            }
                        }

                        // Compute how much sound to write and where
                        let mut byte_to_lock = 0;
                        let target_cursor;
                        let mut bytes_to_write = 0;
                        if sound_is_valid {
                            byte_to_lock = (sound_output.running_sample_index
                                * sound_output.bytes_per_sample)
                                % sound_output.secondary_buffer_size;

                            target_cursor = (last_play_cursor
                                + (sound_output.latency_sample_count
                                    * sound_output.bytes_per_sample))
                                % sound_output.secondary_buffer_size;
                            if byte_to_lock > target_cursor {
                                bytes_to_write = sound_output.secondary_buffer_size - byte_to_lock;
                                bytes_to_write += target_cursor;
                            } else {
                                bytes_to_write = target_cursor - byte_to_lock;
                                /* TODO: Passing bytes_to_write as 0 throws an error.
                                UPDATE: This happens when resizing the window, so something with buffer sizes?
                                Also the rendering is bugged on the default resolution of the window but fixes when you make it bigger.
                                But that in turn trigger the audio bug :/ */
                            }
                        }

                        let sound_buffer = GameSoundOutputBuffer {
                            samples_per_second: sound_output.samples_per_second,
                            sample_count: bytes_to_write / sound_output.bytes_per_sample,
                            samples: samples as *mut i16,
                        };

                        let buffer = GameOffscreenBuffer {
                            memory: GLOBAL_BACK_BUFFER.memory,
                            width: GLOBAL_BACK_BUFFER.width,
                            height: GLOBAL_BACK_BUFFER.height,
                            pitch: GLOBAL_BACK_BUFFER.pitch,
                        };

                        game_update_and_render(&buffer, &sound_buffer, new_input, &mut game_memory);

                        if sound_is_valid {
                            win32_fill_sound_buffer(
                                &mut sound_output,
                                byte_to_lock,
                                bytes_to_write,
                                sound_buffer,
                            );
                        }

                        let work_counter = win32_get_wall_clock();
                        let work_seconds_elapsed =
                            win32_get_seconds_elapsed(last_counter, work_counter);

                        let mut seconds_elapsed_for_frame = work_seconds_elapsed;

                        if seconds_elapsed_for_frame < target_seconds_per_frame {
                            if sleep_is_granular {
                                let sleep_ms = (1000.0
                                    * (target_seconds_per_frame - seconds_elapsed_for_frame))
                                    as u32;
                                if sleep_ms > 0 {
                                    Sleep(sleep_ms);
                                }
                            }
                            while seconds_elapsed_for_frame < target_seconds_per_frame {
                                seconds_elapsed_for_frame =
                                    win32_get_seconds_elapsed(last_counter, win32_get_wall_clock());
                            }
                        } else {
                            // Logging
                            dbg!(seconds_elapsed_for_frame, target_seconds_per_frame);
                        }

                        let end_counter = win32_get_wall_clock();
                        let ms_per_frame =
                            1000.0 * win32_get_seconds_elapsed(last_counter, end_counter);
                        last_counter = end_counter;

                        let dimension = win32_get_window_dimension(window).unwrap_or_else(|e| {
                            dbg!(e);
                            Default::default()
                        });

                        #[cfg(feature = "gaming_internal")]
                        win32_debug_sync_display(
                            &GLOBAL_BACK_BUFFER,
                            debug_time_markers.len(),
                            debug_time_markers.as_ref(),
                            &sound_output,
                            target_seconds_per_frame,
                        );

                        win32_display_buffer_in_window(
                            device_context,
                            &GLOBAL_BACK_BUFFER,
                            dimension.0,
                            dimension.1,
                        );

                        let mut play_cursor = 0;
                        let mut write_cursor = 0;
                        match SECONDARY_BUFFER
                            .as_ref()
                            .unwrap()
                            .GetCurrentPosition(Some(&mut play_cursor), Some(&mut write_cursor))
                        {
                            Ok(_) => {
                                last_play_cursor = play_cursor;
                                sound_is_valid = true;
                            }
                            Err(e) => {
                                sound_is_valid = false;
                                dbg!(e);
                            }
                        }

                        #[cfg(feature = "gaming_internal")]
                        {
                            let marker = &mut debug_time_markers[debug_time_marker_index];
                            debug_time_marker_index += 1;

                            marker.play_cursor = play_cursor;
                            marker.write_cursor = write_cursor;

                            if debug_time_marker_index >= debug_time_markers.len() {
                                debug_time_marker_index = 0;
                            }
                        }

                        swap(new_input, old_input);

                        let end_cycle_count = _rdtsc();
                        let cycles_elapsed = end_cycle_count - last_cycle_count;
                        last_cycle_count = end_cycle_count;

                        let fps = 0.0;
                        let megac_per_frame = cycles_elapsed / (1000 * 1000);

                        let timer_output =
                            format!("{}ms/f, {}f/s, {}mc/f", ms_per_frame, fps, megac_per_frame);

                        dbg!(timer_output);
                    }
                } else {
                    //Logging
                    dbg!(samples);
                    dbg!(game_memory.permanent_storage);
                }

                ReleaseDC(window, device_context);
            } else {
                //Logging
                dbg!(window);
            }
        }
    }

    Ok(())
}
