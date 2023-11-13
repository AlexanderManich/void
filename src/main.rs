#![deny(unsafe_op_in_unsafe_fn)]
mod gaming_engine;
mod macros;

use std::{arch::x86_64::_rdtsc, ffi::c_void, mem::size_of, ptr::swap};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        Media::Audio::{DirectSound::*, WAVEFORMATEX, WAVE_FORMAT_PCM},
        Storage::FileSystem::{
            CreateFileW, GetFileSizeEx, ReadFile, WriteFile, CREATE_ALWAYS,
            FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_NONE, FILE_SHARE_READ, OPEN_EXISTING,
        },
        System::{LibraryLoader::*, Memory::*, Performance::*},
        UI::{Input::XboxController::*, WindowsAndMessaging::*},
    },
};

use gaming_engine::*;
use macros::*;

//Global for now, change later
static mut RUNNING: bool = true;
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
    contents: *mut c_void,
    contents_size: u32,
}

impl Default for DebugReadFileResult {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
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

        let read_buffer = std::mem::transmute::<*mut c_void, &mut u8>(result.contents);

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

        let write_buffer = std::mem::transmute::<*mut c_void, &mut u8>(memory);

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

pub fn win32_process_xinput_digital_button(
    x_input_button_state: XINPUT_GAMEPAD_BUTTON_FLAGS,
    old_state: GameButtonState,
    button_bit: XINPUT_GAMEPAD_BUTTON_FLAGS,
    mut new_state: GameButtonState,
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
    let mut buffer_description = DSBUFFERDESC::default();
    buffer_description.dwSize = size_of::<DSBUFFERDESC>() as u32;
    buffer_description.dwFlags = DSBCAPS_PRIMARYBUFFER;

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
    let mut buffer_description = DSBUFFERDESC::default();
    buffer_description.dwSize = size_of::<DSBUFFERDESC>() as u32;
    buffer_description.dwFlags = 0;
    buffer_description.dwBufferBytes = buffer_size;
    buffer_description.lpwfxFormat = &mut wave_format;

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
        GetClientRect(window, &mut client_rect).unwrap();
    }
    let width = client_rect.right - client_rect.left;
    let height = client_rect.bottom - client_rect.top;

    Ok((width, height))
}

pub fn win32_resize_dib_section(buffer: &mut Win32OffscreenBuffer, width: i32, height: i32) {
    unsafe {
        if !buffer.memory.is_null() {
            VirtualFree(buffer.memory as *mut c_void, 0, MEM_RELEASE).unwrap_or_else(|e| {
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
                let vk_code = wparam;
                let was_down = (lparam.0 & (1 << 30)) != 0;
                let is_down = (lparam.0 & (1 << 31)) == 0;
                //Check prevents repeat key presses when holding key
                // if was_down != is_down {
                match vk_code {
                    WPARAM(0x57) => {
                        //W key
                    }
                    WPARAM(0x41) => {
                        //A key
                    }
                    WPARAM(0x53) => {
                        //S key
                    }
                    WPARAM(0x44) => {
                        //D key
                    }
                    WPARAM(0x73) => {
                        //F4 key
                        let alt_key_was_down = (lparam.0 & (1 << 29)) != 0;
                        if alt_key_was_down {
                            RUNNING = false;
                        }
                    }
                    WPARAM(_) => {}
                }
                // }
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

        return result;
    }
}
fn main() -> Result<()> {
    unsafe {
        let mut perf_counter_freq: i64 = 0;
        QueryPerformanceFrequency(&mut perf_counter_freq).unwrap();

        let mut window_class = WNDCLASSW::default();

        win32_resize_dib_section(&mut GLOBAL_BACK_BUFFER, 1280, 720);
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
                let device_context = GetDC(window);

                // Sound test
                let mut sound_output = Win32SoundOutput::default();
                sound_output.samples_per_second = 48000;
                sound_output.running_sample_index = 0;
                sound_output.bytes_per_sample = size_of::<i16>() as u32 * 2;
                sound_output.secondary_buffer_size =
                    sound_output.samples_per_second * sound_output.bytes_per_sample;
                sound_output.latency_sample_count = sound_output.samples_per_second / 15;

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
                        ()
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

                let mut game_memory = GameMemory::default();
                game_memory.permanent_storage_size = mega_bytes!(64);
                game_memory.transient_storage_size = giga_bytes!(1);

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

                // TODO: I dont know if this semantically correct?
                if samples != std::ptr::null_mut()
                    && game_memory.permanent_storage != std::ptr::null_mut()
                    && game_memory.transient_storage != std::ptr::null_mut()
                {
                    let mut input: [GameInput; 2] = [GameInput::default(), GameInput::default()];
                    let (new_input, old_input) = input.split_at_mut(1);
                    let new_input = &mut new_input[0];
                    let old_input = &mut old_input[0];

                    let mut last_counter: i64 = 0;
                    QueryPerformanceCounter(&mut last_counter).unwrap();
                    let mut last_cycle_count = _rdtsc();

                    while RUNNING {
                        let mut msg = MSG::default();

                        while (PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE)).into() {
                            if msg.message == WM_QUIT {
                                RUNNING = false;
                            }

                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }

                        //CONTROLLER INPUT
                        //Should we poll this more frequently?
                        let mut max_controller_count = XUSER_MAX_COUNT;

                        if max_controller_count > new_input.controllers.len() as u32 {
                            max_controller_count = new_input.controllers.len() as u32;
                        }

                        for controller_index in 0..XUSER_MAX_COUNT {
                            let old_controller =
                                &mut old_input.controllers[controller_index as usize];
                            let new_controller =
                                &mut new_input.controllers[controller_index as usize];

                            let controller_state = &mut XINPUT_STATE::default();
                            if XInputGetState(controller_index, controller_state) == ERROR_SUCCESS.0
                            {
                                //Controller is plugged in
                                let pad = &controller_state.Gamepad;

                                let up = pad.wButtons & XINPUT_GAMEPAD_DPAD_UP;
                                let down = pad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN;
                                let left = pad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT;
                                let right = pad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT;

                                new_controller.is_analog = true;
                                new_controller.start_x = old_controller.end_x;
                                new_controller.start_y = old_controller.end_y;

                                // Deadzone handling? whatever that is using:
                                // XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE
                                // XINPUT_GAMEPAD_RIGHT_THUMB_DEADZONE

                                let x: f32;
                                if pad.sThumbLX < 0 {
                                    x = pad.sThumbLX as f32 / 32760.0;
                                } else {
                                    x = pad.sThumbLX as f32 / 32767.0;
                                }

                                new_controller.end_x = x;
                                new_controller.max_x = new_controller.end_x;
                                new_controller.min_x = new_controller.max_x;

                                let y: f32;
                                if pad.sThumbLY < 0 {
                                    y = pad.sThumbLY as f32 / 32760.0;
                                } else {
                                    y = pad.sThumbLY as f32 / 32767.0;
                                }
                                new_controller.end_y = y;
                                new_controller.max_y = new_controller.end_y;
                                new_controller.min_y = new_controller.max_y;

                                let stick_x = pad.sThumbLX;
                                let stick_y = pad.sThumbLY;

                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.down,
                                    XINPUT_GAMEPAD_A,
                                    new_controller.button_state.concrete_buttons.down,
                                );
                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.right,
                                    XINPUT_GAMEPAD_B,
                                    new_controller.button_state.concrete_buttons.right,
                                );
                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.left,
                                    XINPUT_GAMEPAD_X,
                                    new_controller.button_state.concrete_buttons.left,
                                );
                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.up,
                                    XINPUT_GAMEPAD_Y,
                                    new_controller.button_state.concrete_buttons.up,
                                );
                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.left_shoulder,
                                    XINPUT_GAMEPAD_LEFT_SHOULDER,
                                    new_controller.button_state.concrete_buttons.left_shoulder,
                                );
                                win32_process_xinput_digital_button(
                                    pad.wButtons,
                                    old_controller.button_state.concrete_buttons.right_shoulder,
                                    XINPUT_GAMEPAD_RIGHT_SHOULDER,
                                    new_controller.button_state.concrete_buttons.right_shoulder,
                                );

                                // let start = pad.wButtons & XINPUT_GAMEPAD_START;
                                // let back = pad.wButtons & XINPUT_GAMEPAD_BACK;
                            } else {
                                //Controller is not available
                            }
                        }

                        // DirectSound output test
                        let mut byte_to_lock = 0;
                        let target_cursor;
                        let mut bytes_to_write = 0;
                        let mut play_cursor = 0;
                        let mut write_cursor = 0;
                        let mut sound_is_valid = false;
                        match SECONDARY_BUFFER
                            .as_ref()
                            .unwrap()
                            .GetCurrentPosition(Some(&mut play_cursor), Some(&mut write_cursor))
                        {
                            Ok(_) => {
                                sound_is_valid = true;
                                byte_to_lock = (sound_output.running_sample_index
                                    * sound_output.bytes_per_sample)
                                    % sound_output.secondary_buffer_size;

                                target_cursor = (play_cursor
                                    + (sound_output.latency_sample_count
                                        * sound_output.bytes_per_sample))
                                    % sound_output.secondary_buffer_size;
                                if byte_to_lock > target_cursor {
                                    bytes_to_write =
                                        sound_output.secondary_buffer_size - byte_to_lock;
                                    bytes_to_write += target_cursor;
                                } else {
                                    bytes_to_write = target_cursor - byte_to_lock;
                                    // TODO: Passing 0 to bytes to write throws an error. This is probably scuffed but it kinda works? idk
                                    if bytes_to_write == 0 {
                                        bytes_to_write = 1;
                                    }
                                }
                            }
                            Err(e) => {
                                dbg!(e);
                            }
                        }

                        let mut sound_buffer = GameSoundOutputBuffer::default();
                        sound_buffer.samples_per_second = sound_output.samples_per_second;
                        sound_buffer.sample_count = bytes_to_write / sound_output.bytes_per_sample;
                        sound_buffer.samples = samples as *mut i16;

                        let mut buffer = GameOffscreenBuffer::default();
                        buffer.memory = GLOBAL_BACK_BUFFER.memory;
                        buffer.width = GLOBAL_BACK_BUFFER.width;
                        buffer.height = GLOBAL_BACK_BUFFER.height;
                        buffer.pitch = GLOBAL_BACK_BUFFER.pitch;

                        gaming_engine::game_update_and_render(
                            &buffer,
                            &sound_buffer,
                            &new_input,
                            &mut game_memory,
                        );

                        if sound_is_valid {
                            win32_fill_sound_buffer(
                                &mut sound_output,
                                byte_to_lock,
                                bytes_to_write,
                                sound_buffer,
                            );
                        }

                        let dimension = win32_get_window_dimension(window).unwrap();

                        win32_display_buffer_in_window(
                            device_context,
                            &GLOBAL_BACK_BUFFER,
                            dimension.0,
                            dimension.1,
                        );

                        // This makes the window not live update, have to look into how this works
                        // ReleaseDC(window, device_context);

                        let end_cycle_count = _rdtsc();

                        let mut end_counter: i64 = 0;
                        QueryPerformanceCounter(&mut end_counter).unwrap();

                        let cycles_elapsed = end_cycle_count - last_cycle_count;
                        let counter_elapsed: i64 = end_counter - last_counter;
                        let millis_per_frame = (1000 * counter_elapsed) / perf_counter_freq;
                        let fps = perf_counter_freq / counter_elapsed;
                        let megac_per_frame = cycles_elapsed / (1000 * 1000);

                        let timer_output = format!(
                            "{}ms/f, {}f/s, {}mc/f",
                            millis_per_frame, fps, megac_per_frame
                        );

                        // dbg!(timer_output);

                        last_counter = end_counter;
                        last_cycle_count = end_cycle_count;

                        swap(new_input, old_input);
                        // let temp = new_input;
                        // new_input = old_input;
                        // old_input = temp;
                    }
                } else {
                    //Logging
                    dbg!(samples);
                    dbg!(game_memory.permanent_storage);
                }
            } else {
                //Logging
                dbg!(window);
            }
        }
    }

    Ok(())
}
