#![deny(unsafe_op_in_unsafe_fn)]
use crate::macros::*;
use game::game::*;
use std::{
    arch::x86_64::_rdtsc,
    env::current_exe,
    ffi::c_void,
    io,
    mem::{self, size_of, transmute},
    path::PathBuf,
    ptr::{self, addr_of_mut, null_mut, swap},
    slice,
};
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
        UI::{
            Input::{
                KeyboardAndMouse::{
                    GetKeyState, VIRTUAL_KEY, VK_A, VK_D, VK_DOWN, VK_E, VK_ESCAPE, VK_F4, VK_L,
                    VK_LBUTTON, VK_LEFT, VK_MBUTTON, VK_P, VK_Q, VK_RBUTTON, VK_RETURN, VK_RIGHT,
                    VK_S, VK_SPACE, VK_UP, VK_W, VK_XBUTTON1, VK_XBUTTON2,
                },
                XboxController::*,
            },
            WindowsAndMessaging::*,
        },
    },
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
    safety_bytes: u32,
}

#[inline]
fn safe_truncateu64(value: u64) -> u32 {
    debug_assert!(value <= u32::MAX.into());
    value as u32
}

#[derive(Copy, Clone)]
struct Win32DebugTimeMarker {
    output_play_cursor: u32,
    output_write_cursor: u32,
    output_location: u32,
    output_byte_count: u32,
    expected_flip_play_cursor: u32,

    flip_play_cursor: u32,
    flip_write_cursor: u32,
}

impl Default for Win32DebugTimeMarker {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}

//Globals for now, change later
static mut GLOBAL_RUNNING: bool = true;
static mut GLOBAL_PAUSE: bool = false;
static mut PERF_COUNTER_FREQ: i64 = 0;
static mut DIRECT_SOUND: Option<IDirectSound8> = None;
static mut SECONDARY_BUFFER: Option<IDirectSoundBuffer> = None;
static mut GLOBAL_WINDOW_POSITION: WINDOWPLACEMENT = unsafe { mem::zeroed() };
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
    mut top: i32,
    mut bottom: i32,
    color: u32,
) {
    if top <= 0 {
        top = 0;
    }

    if bottom >= back_buffer.height {
        bottom = back_buffer.height - 1;
    }

    if x >= 0 && x < back_buffer.width {
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
}
struct Win32GameCode {
    game_code_dll: Option<HMODULE>,
    dll_last_write_time: FILETIME,
    get_sound_samples: Option<GameGetSoundSamples>,
    update_and_render: Option<GameUpdateAndRender>,
}

struct Win32ReplayBuffer {
    replay_file_name: String,
    memory_block: *mut c_void,
}

impl Default for Win32ReplayBuffer {
    fn default() -> Self {
        Self {
            replay_file_name: String::new(),
            memory_block: null_mut(),
        }
    }
}

struct Win32State {
    total_size: u64,
    game_memory_block: *mut c_void,
    replay_buffers: [Win32ReplayBuffer; 4],

    recording_handle: HANDLE,
    input_recording_index: usize,

    playback_handle: HANDLE,
    input_playing_index: usize,

    exe_full_path: PathBuf,
    exe_name: String,
}

impl Default for Win32State {
    fn default() -> Self {
        Self {
            total_size: 0,
            game_memory_block: null_mut(),
            replay_buffers: core::array::from_fn(|_| Win32ReplayBuffer::default()),
            recording_handle: HANDLE(0),
            input_recording_index: 0,
            playback_handle: HANDLE(0),
            input_playing_index: 0,
            exe_full_path: PathBuf::new(),
            exe_name: String::new(),
        }
    }
}

fn win32_get_last_write_time(file_name: &HSTRING) -> FILETIME {
    let mut last_write_time: FILETIME = Default::default();

    let mut data: WIN32_FILE_ATTRIBUTE_DATA = Default::default();
    unsafe {
        match GetFileAttributesExW(
            file_name,
            GetFileExInfoStandard,
            &mut data as *mut _ as *mut c_void,
        ) {
            Ok(()) => {
                last_write_time = data.ftLastWriteTime;
            }
            Err(e) => {
                dbg!(e);
            }
        };
    }

    last_write_time
}

#[no_mangle]
fn win32_load_game_code(source_dll_name: &HSTRING, temp_dll_name: &HSTRING) -> Win32GameCode {
    let mut result = Win32GameCode {
        game_code_dll: None,
        dll_last_write_time: FILETIME::default(),
        get_sound_samples: None,
        update_and_render: None,
    };

    result.dll_last_write_time = win32_get_last_write_time(source_dll_name);

    unsafe {
        if let Err(e) = CopyFileW(source_dll_name, temp_dll_name, false) {
            dbg!(e);
        }

        match LoadLibraryW(temp_dll_name) {
            Ok(hmodule) => {
                result.game_code_dll = Some(hmodule);
                let p = GetProcAddress(hmodule, s!("game_get_sound_samples"));
                result.get_sound_samples = Some(transmute::<_, GameGetSoundSamples>(p));
                let p = GetProcAddress(hmodule, s!("game_update_and_render"));
                result.update_and_render = Some(transmute::<_, GameUpdateAndRender>(p));
            }
            Err(e) => {
                dbg!(e);
            }
        }
    };

    result
}

fn win32_unload_game_code(game_code: &mut Win32GameCode) {
    if let Some(hmodule) = game_code.game_code_dll {
        unsafe {
            FreeLibrary(hmodule).unwrap_or_else(|e| {
                dbg!(e);
            })
        };
    }
    game_code.game_code_dll = None;
    game_code.get_sound_samples = None;
    game_code.update_and_render = None;
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
    let x_f32 = c * value as f32;
    let x = x_f32 as i32 + pad_x;

    win32_debug_draw_vertical(back_buffer, x, top, bottom, color);
}

fn win32_debug_sync_display(
    back_buffer: &Win32OffscreenBuffer,
    marker_count: usize,
    markers: &[Win32DebugTimeMarker],
    current_marker_index: i32,
    sound_output: &Win32SoundOutput,
    target_seconds_per_frame: f32,
) {
    let pad_x = 16;
    let pad_y = 16;

    let line_height = 64;

    let c = (back_buffer.width - 2 * pad_x) as f32 / sound_output.secondary_buffer_size as f32;
    (0..marker_count).for_each(|marker_index| {
        let this_marker = markers[marker_index];
        debug_assert!(this_marker.flip_play_cursor < sound_output.secondary_buffer_size);
        debug_assert!(this_marker.flip_write_cursor < sound_output.secondary_buffer_size);
        debug_assert!(this_marker.output_play_cursor < sound_output.secondary_buffer_size);
        debug_assert!(this_marker.output_write_cursor < sound_output.secondary_buffer_size);
        debug_assert!(this_marker.output_location < sound_output.secondary_buffer_size);
        debug_assert!(this_marker.output_byte_count < sound_output.secondary_buffer_size);

        let play_color: u32 = 0xFFFFFFFF;
        let write_color: u32 = 0xFFFF0000;
        let expected_flip_color: u32 = 0xFFFFFF00;

        let mut top = pad_y;
        let mut bottom = pad_y + line_height;
        if marker_index == current_marker_index as usize {
            top += line_height + pad_y;
            bottom += line_height + pad_y;

            let first_top = top;

            win32_draw_sound_buffer_marker(
                back_buffer,
                sound_output,
                c,
                pad_x,
                top,
                bottom,
                this_marker.output_play_cursor,
                play_color,
            );
            win32_draw_sound_buffer_marker(
                back_buffer,
                sound_output,
                c,
                pad_x,
                top,
                bottom,
                this_marker.output_write_cursor,
                write_color,
            );
            top += line_height + pad_y;
            bottom += line_height + pad_y;

            win32_draw_sound_buffer_marker(
                back_buffer,
                sound_output,
                c,
                pad_x,
                top,
                bottom,
                this_marker.output_location,
                play_color,
            );
            win32_draw_sound_buffer_marker(
                back_buffer,
                sound_output,
                c,
                pad_x,
                top,
                bottom,
                this_marker.output_location + this_marker.output_byte_count,
                write_color,
            );

            top += line_height + pad_y;
            bottom += line_height + pad_y;

            win32_draw_sound_buffer_marker(
                back_buffer,
                sound_output,
                c,
                pad_x,
                first_top,
                bottom,
                this_marker.expected_flip_play_cursor,
                expected_flip_color,
            );
        }

        win32_draw_sound_buffer_marker(
            back_buffer,
            sound_output,
            c,
            pad_x,
            top,
            bottom,
            this_marker.flip_play_cursor,
            play_color,
        );
        win32_draw_sound_buffer_marker(
            back_buffer,
            sound_output,
            c,
            pad_x,
            top,
            bottom,
            this_marker.flip_write_cursor,
            write_color,
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

fn toggle_fullscreen(window: HWND) {
    unsafe {
        let dw_style = GetWindowLongW(window, GWL_STYLE);
        if dw_style as u32 & WS_OVERLAPPEDWINDOW.0 != 0 {
            let mi: &mut MONITORINFO = &mut mem::zeroed();
            mi.cbSize = size_of::<MONITORINFO>() as u32;

            if let Ok(()) = GetWindowPlacement(window, &mut GLOBAL_WINDOW_POSITION) {
                if GetMonitorInfoW(MonitorFromWindow(window, MONITOR_DEFAULTTOPRIMARY), mi).0 != 0 {
                    SetWindowLongW(window, GWL_STYLE, dw_style & !WS_OVERLAPPEDWINDOW.0 as i32);
                    SetWindowPos(
                        window,
                        HWND_TOP,
                        mi.rcMonitor.left,
                        mi.rcMonitor.top,
                        mi.rcMonitor.right - mi.rcMonitor.left,
                        mi.rcMonitor.bottom - mi.rcMonitor.top,
                        SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
                    )
                    .unwrap();
                }
            }
        } else {
            SetWindowLongW(window, GWL_STYLE, dw_style | WS_OVERLAPPEDWINDOW.0 as i32);
            SetWindowPlacement(window, &mut GLOBAL_WINDOW_POSITION).unwrap();
            SetWindowPos(
                window,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
            )
            .unwrap();
        }
    }
}

fn win32_process_pending_messages(
    win32_state: &mut Win32State,
    keyboard_controller: &mut GameControllerInput,
) {
    let mut msg = MSG::default();

    unsafe {
        while PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE).as_bool() {
            match msg.message {
                WM_QUIT => {
                    GLOBAL_RUNNING = false;
                }
                WM_SYSKEYDOWN | WM_SYSKEYUP | WM_KEYDOWN | WM_KEYUP => {
                    let vk_code = msg.wParam.0 as u16;
                    let was_down = (msg.lParam.0 & (1 << 30)) != 0;
                    let is_down = (msg.lParam.0 & (1 << 31)) == 0;
                    let alt_key_was_down = (msg.lParam.0 & (1 << 29)) != 0;

                    //Check prevents repeat key presses when holding key
                    if was_down != is_down {
                        match VIRTUAL_KEY(vk_code) {
                            VK_W => win32_process_keyboard_message(
                                &mut keyboard_controller.button_state.concrete_buttons.move_up,
                                is_down,
                            ),
                            VK_A => win32_process_keyboard_message(
                                &mut keyboard_controller.button_state.concrete_buttons.move_left,
                                is_down,
                            ),
                            VK_S => win32_process_keyboard_message(
                                &mut keyboard_controller.button_state.concrete_buttons.move_down,
                                is_down,
                            ),
                            VK_D => win32_process_keyboard_message(
                                &mut keyboard_controller.button_state.concrete_buttons.move_right,
                                is_down,
                            ),
                            VK_Q => win32_process_keyboard_message(
                                &mut keyboard_controller
                                    .button_state
                                    .concrete_buttons
                                    .left_shoulder,
                                is_down,
                            ),
                            VK_E => win32_process_keyboard_message(
                                &mut keyboard_controller
                                    .button_state
                                    .concrete_buttons
                                    .right_shoulder,
                                is_down,
                            ),
                            VK_UP => win32_process_keyboard_message(
                                &mut keyboard_controller.button_state.concrete_buttons.action_up,
                                is_down,
                            ),
                            VK_LEFT => win32_process_keyboard_message(
                                &mut keyboard_controller
                                    .button_state
                                    .concrete_buttons
                                    .action_left,
                                is_down,
                            ),
                            VK_DOWN => win32_process_keyboard_message(
                                &mut keyboard_controller
                                    .button_state
                                    .concrete_buttons
                                    .action_down,
                                is_down,
                            ),
                            VK_RIGHT => win32_process_keyboard_message(
                                &mut keyboard_controller
                                    .button_state
                                    .concrete_buttons
                                    .action_right,
                                is_down,
                            ),
                            VK_ESCAPE => win32_process_keyboard_message(
                                &mut keyboard_controller.button_state.concrete_buttons.start,
                                is_down,
                            ),
                            VK_SPACE => win32_process_keyboard_message(
                                &mut keyboard_controller.button_state.concrete_buttons.back,
                                is_down,
                            ),
                            VK_F4 => {
                                if alt_key_was_down {
                                    GLOBAL_RUNNING = false;
                                }
                            }
                            VK_RETURN => {
                                if is_down && alt_key_was_down {
                                    toggle_fullscreen(msg.hwnd);
                                }
                            }
                            VK_P => {
                                if is_down {
                                    GLOBAL_PAUSE = !GLOBAL_PAUSE;
                                }
                            }
                            VK_L => {
                                if is_down {
                                    if win32_state.input_playing_index == 0 {
                                        if win32_state.input_recording_index == 0 {
                                            if let Err(e) =
                                                win32_begin_recording_input(win32_state, 1)
                                            {
                                                dbg!(e);
                                            }
                                        } else {
                                            if let Err(e) = win32_end_recording_input(win32_state) {
                                                dbg!(e);
                                            }
                                            if let Err(e) =
                                                win32_begin_input_playback(win32_state, 1)
                                            {
                                                dbg!(e);
                                            };
                                        }
                                    } else if let Err(e) = win32_end_input_playback(win32_state) {
                                        dbg!(e);
                                    }
                                }
                            }
                            _ => {}
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

#[no_mangle]
pub fn debug_platform_read_entire_file(
    thread: &ThreadContext,
    file_name: PathBuf,
) -> io::Result<DebugReadFileResult> {
    unsafe {
        let mut result: DebugReadFileResult = DebugReadFileResult::default();

        let file_name = &HSTRING::from(file_name.as_os_str());

        let file_handle = CreateFileW(
            file_name,
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
                    debug_platform_free_file_memory(thread, result.contents);
                    result.contents = std::ptr::null_mut();
                    dbg!(e);
                }
            }
        }
        CloseHandle(file_handle)?;

        Ok(result)
    }
}

pub fn debug_platform_free_file_memory(thread: &ThreadContext, memory: *mut c_void) {
    if !memory.is_null() {
        unsafe {
            if let Err(e) = VirtualFree(memory, 0, MEM_RELEASE) {
                dbg!(e);
            }
        }
    }
}

pub fn debug_platform_write_entire_file(
    thread: &ThreadContext,
    file_name: PathBuf,
    memory_size: u32,
    memory: *mut c_void,
) -> io::Result<()> {
    unsafe {
        let file_name = &HSTRING::from(file_name.as_os_str());

        let file_handle = CreateFileW(
            file_name,
            GENERIC_WRITE.0,
            FILE_SHARE_NONE,
            None,
            CREATE_ALWAYS,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            HANDLE(0),
        )?;

        let write_buffer = memory.cast::<u8>();

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
    if new_state.ended_down != is_down {
        new_state.ended_down = is_down;
        new_state.half_transition_count += 1;
    }
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
    wave_format.wFormatTag = WAVE_FORMAT_PCM as u16; // Why does the struct take u16 when all the enum variants are u32?
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
        DirectSoundCreate8(None, addr_of_mut!(DIRECT_SOUND), None).unwrap_or_else(|e| {
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
            .CreateSoundBuffer(&buffer_description, addr_of_mut!(SECONDARY_BUFFER), None)
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

pub fn win32_resize_dib_section(buffer: *mut Win32OffscreenBuffer, width: i32, height: i32) {
    unsafe {
        let buffer = &mut *(buffer);
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
        // Negative height means the bitmap is a top-down DIB and it's origin is the upper-left corner. Rows in window go sequentially from top to bottom.
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
        );

        // Pitch determines when a new row starts in the sequence of bytes in the bitmap.
        buffer.pitch = (width * buffer.bytes_per_pixel) as usize;
    }
}

fn win32_display_buffer_in_window(
    device_context: HDC,
    buffer: &Win32OffscreenBuffer,
    window_width: i32,
    window_height: i32,
) {
    unsafe {
        if window_width >= buffer.width * 2 && window_height >= buffer.height * 2 {
            StretchDIBits(
                device_context,
                0,
                0,
                2 * buffer.width,  //window_width,
                2 * buffer.height, //window_height,
                0,
                0,
                buffer.width,
                buffer.height,
                Some(buffer.memory as *const c_void),
                &buffer.info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        } else {
            let offset_x = 10;
            let offset_y = 10;

            PatBlt(device_context, 0, 0, window_width, offset_y, BLACKNESS);
            PatBlt(
                device_context,
                0,
                offset_y + buffer.height,
                window_width,
                window_height,
                BLACKNESS,
            );
            PatBlt(device_context, 0, 0, offset_x, window_height, BLACKNESS);
            PatBlt(
                device_context,
                offset_x + buffer.width,
                0,
                window_width,
                window_height,
                BLACKNESS,
            );

            //TODO: Aspect ratio correction
            StretchDIBits(
                device_context,
                offset_x,
                offset_y,
                buffer.width,  //window_width,
                buffer.height, //window_height,
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
}

extern "system" fn win32_main_window_callback(
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
                GLOBAL_RUNNING = false;
                // TODO: Handle this as an error
            }
            WM_CLOSE => {
                GLOBAL_RUNNING = false;
                // TODO: Handle this with a message to the user? (Are you sure you want to quit?)
            }
            WM_ACTIVATEAPP => {
                // WPARAM is true or false depending on if window is active or not
                // WPARAM(1) == TRUE, WPARAM(0) == FALSE
                // if wparam == WPARAM(1) {
                //     if let Err(e) = SetLayeredWindowAttributes(window, COLORREF(0), 255, LWA_ALPHA)
                //     {
                //         dbg!(e);
                //     }
                // } else if let Err(e) =
                //     SetLayeredWindowAttributes(window, COLORREF(0), 128, LWA_ALPHA)
                // {
                //     dbg!(e);
                // }
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
                            GLOBAL_BACK_BUFFER.can_into(),
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
        QueryPerformanceFrequency(addr_of_mut!(PERF_COUNTER_FREQ)).unwrap();

        let mut win32_state: Win32State = Default::default();

        win32_get_exe_file_name(&mut win32_state);

        let mut dll_dir_path = PathBuf::new();
        win32_build_exe_path_file_name(&win32_state, "deps", &mut dll_dir_path);

        let source_game_code_dll_full_path =
            &HSTRING::from(dll_dir_path.join("game.dll").as_os_str());

        let temp_game_code_dll_full_path =
            &HSTRING::from(dll_dir_path.join("temp_game.dll").as_os_str());

        // Set schedular granularity to 1ms to make Sleep() more granular
        let desired_scheduler_ms = 1;
        let sleep_is_granular = timeBeginPeriod(desired_scheduler_ms) != TIMERR_NOERROR;

        let mut window_class = WNDCLASSW::default();

        // 1080p is 1920x1080. Half is 960x540 (Because we are software rendering for now)
        win32_resize_dib_section(addr_of_mut!(GLOBAL_BACK_BUFFER), 960, 540);
        window_class.style = CS_HREDRAW | CS_VREDRAW;
        window_class.lpfnWndProc = Some(win32_main_window_callback);
        window_class.hInstance = GetModuleHandleW(PCWSTR::null()).unwrap().into();
        window_class.hCursor = LoadCursorW(None, IDC_ARROW).unwrap();
        // window_class.hIcon = ;
        window_class.lpszClassName = w!("GamingWindowClass");

        if RegisterClassW(&window_class) != 0 {
            let window = CreateWindowExW(
                WINDOW_EX_STYLE(0), // WS_EX_TOPMOST | WS_EX_LAYERED,
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
                let refresh_dc = GetDC(window);
                let mut monitor_refresh_hz = 60;
                let win32_refresh_rate = GetDeviceCaps(refresh_dc, VREFRESH);
                ReleaseDC(window, refresh_dc);

                if win32_refresh_rate > 1 {
                    monitor_refresh_hz = win32_refresh_rate;
                }

                // TODO: If game_update_hz is too big, sound starts bugging. Ideally It should be monitor_refresh_hz / 2
                let game_update_hz: f32 = monitor_refresh_hz as f32 / 2.0;
                let target_seconds_per_frame: f32 = 1.0 / game_update_hz;

                let mut sound_output = Win32SoundOutput::default();
                sound_output.samples_per_second = 48000;
                sound_output.running_sample_index = 0;
                sound_output.bytes_per_sample = size_of::<i16>() as u32 * 2;
                sound_output.secondary_buffer_size =
                    sound_output.samples_per_second * sound_output.bytes_per_sample;
                sound_output.safety_bytes = ((sound_output.samples_per_second as f32
                    * sound_output.bytes_per_sample as f32
                    / game_update_hz)
                    / 3.0) as u32;

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

                GLOBAL_RUNNING = true;

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
                    is_initialized: false,
                    permanent_storage: null_mut(),
                    permanent_storage_size: mega_bytes!(64),
                    transient_storage: null_mut(),
                    transient_storage_size: giga_bytes!(1),
                    debug_platform_read_entire_file,
                    debug_platform_free_file_memory,
                    debug_platform_write_entire_file,
                };

                win32_state.total_size =
                    game_memory.permanent_storage_size + game_memory.transient_storage_size;

                win32_state.game_memory_block = VirtualAlloc(
                    base_address,
                    win32_state.total_size as usize,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_READWRITE,
                );

                game_memory.permanent_storage = win32_state.game_memory_block;

                game_memory.transient_storage = (game_memory.permanent_storage as *mut u8)
                    .add(game_memory.permanent_storage_size as usize)
                    .cast();

                for replay_index in 0..win32_state.replay_buffers.len() {
                    let replay_buffer = &mut win32_state.replay_buffers[replay_index];
                    replay_buffer.memory_block = VirtualAlloc(
                        None,
                        win32_state.total_size as usize,
                        MEM_RESERVE | MEM_COMMIT,
                        PAGE_READWRITE,
                    );

                    debug_assert!(!replay_buffer.memory_block.is_null());
                }

                if !samples.is_null()
                    && !game_memory.permanent_storage.is_null()
                    && !game_memory.transient_storage.is_null()
                {
                    let new_input = &mut GameInput::default();
                    let old_input = &mut GameInput::default();

                    let mut last_counter: i64 = win32_get_wall_clock();
                    let mut flip_wall_clock = win32_get_wall_clock();

                    let mut debug_time_marker_index: usize = 0;
                    let mut debug_time_markers = [Win32DebugTimeMarker::default(); 30];

                    let mut sound_is_valid = false;
                    let mut audio_latency_bytes = 0;
                    let mut audio_latency_seconds = 0.0;

                    let mut last_cycle_count = _rdtsc();

                    let mut game: Win32GameCode = win32_load_game_code(
                        source_game_code_dll_full_path,
                        temp_game_code_dll_full_path,
                    );

                    while GLOBAL_RUNNING {
                        new_input.dt_for_frame = target_seconds_per_frame;

                        let new_dll_write_time =
                            win32_get_last_write_time(source_game_code_dll_full_path);
                        if CompareFileTime(&new_dll_write_time, &game.dll_last_write_time) != 0 {
                            win32_unload_game_code(&mut game);
                            game = win32_load_game_code(
                                source_game_code_dll_full_path,
                                temp_game_code_dll_full_path,
                            );
                        }

                        let old_keyboard_controller = get_controller(old_input, 0);
                        let new_keyboard_controller = get_controller(new_input, 0);
                        *new_keyboard_controller = Default::default();
                        new_keyboard_controller.is_connected = true;

                        for button_index in 0..new_keyboard_controller.button_state.buttons.len() {
                            new_keyboard_controller.button_state.buttons[button_index].ended_down =
                                old_keyboard_controller.button_state.buttons[button_index]
                                    .ended_down;
                        }
                        win32_process_pending_messages(&mut win32_state, new_keyboard_controller);

                        if !GLOBAL_PAUSE {
                            // MOUSE INPUT
                            let mouse_p = ptr::from_mut(&mut POINT::default());
                            if let Err(e) = GetCursorPos(mouse_p) {
                                dbg!(e);
                            };

                            ScreenToClient(window, mouse_p);
                            new_input.mouse_x = (*mouse_p).x;
                            new_input.mouse_y = (*mouse_p).y;
                            new_input.mouse_z = 0;

                            win32_process_keyboard_message(
                                &mut new_input.mouse_buttons[0],
                                (GetKeyState(VK_LBUTTON.0.into()) & (1 << 15)) != 0,
                            );
                            win32_process_keyboard_message(
                                &mut new_input.mouse_buttons[1],
                                (GetKeyState(VK_RBUTTON.0.into()) & (1 << 15)) != 0,
                            );
                            win32_process_keyboard_message(
                                &mut new_input.mouse_buttons[2],
                                (GetKeyState(VK_MBUTTON.0.into()) & (1 << 15)) != 0,
                            );
                            win32_process_keyboard_message(
                                &mut new_input.mouse_buttons[3],
                                (GetKeyState(VK_XBUTTON1.0.into()) & (1 << 15)) != 0,
                            );
                            win32_process_keyboard_message(
                                &mut new_input.mouse_buttons[4],
                                (GetKeyState(VK_XBUTTON2.0.into()) & (1 << 15)) != 0,
                            );
                            // new_input.mouse_buttons[0] =
                            // new_input.mouse_buttons[1] =
                            // new_input.mouse_buttons[2] =
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
                                if XInputGetState(controller_index, controller_state)
                                    == ERROR_SUCCESS.0
                                {
                                    new_controller.is_connected = true;
                                    new_controller.is_analog = old_controller.is_analog;

                                    //Controller is plugged in
                                    let pad = &controller_state.Gamepad;

                                    new_controller.stick_average_x =
                                        win32_process_input_stick_value(
                                            pad.sThumbLX,
                                            XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE.0,
                                        );
                                    new_controller.stick_average_y =
                                        win32_process_input_stick_value(
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
                                        &mut new_controller
                                            .button_state
                                            .concrete_buttons
                                            .move_right,
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
                                        &mut new_controller
                                            .button_state
                                            .concrete_buttons
                                            .action_down,
                                    );
                                    win32_process_xinput_digital_button(
                                        pad.wButtons,
                                        old_controller.button_state.concrete_buttons.action_right,
                                        XINPUT_GAMEPAD_B,
                                        &mut new_controller
                                            .button_state
                                            .concrete_buttons
                                            .action_right,
                                    );
                                    win32_process_xinput_digital_button(
                                        pad.wButtons,
                                        old_controller.button_state.concrete_buttons.action_left,
                                        XINPUT_GAMEPAD_X,
                                        &mut new_controller
                                            .button_state
                                            .concrete_buttons
                                            .action_left,
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
                                        &mut new_controller
                                            .button_state
                                            .concrete_buttons
                                            .left_shoulder,
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

                            let buffer = GameOffscreenBuffer {
                                memory: GLOBAL_BACK_BUFFER.memory,
                                width: GLOBAL_BACK_BUFFER.width,
                                height: GLOBAL_BACK_BUFFER.height,
                                pitch: GLOBAL_BACK_BUFFER.pitch,
                                bytes_per_pixel: GLOBAL_BACK_BUFFER.bytes_per_pixel,
                            };

                            if win32_state.input_recording_index == 1 {
                                win32_record_input(&win32_state, new_input);
                            }

                            if win32_state.input_playing_index == 1 {
                                win32_playback_input(&mut win32_state, new_input);
                            }

                            let thread = ThreadContext::default();

                            if let Some(game_update_and_render) = game.update_and_render {
                                game_update_and_render(
                                    &thread,
                                    &buffer,
                                    new_input,
                                    &mut game_memory,
                                );
                            }

                            let audio_wall_clock = win32_get_wall_clock();
                            let from_begin_to_audio_seconds =
                                win32_get_seconds_elapsed(flip_wall_clock, audio_wall_clock);

                            let mut play_cursor = 0;
                            let mut write_cursor = 0;

                            if let Err(e) = SECONDARY_BUFFER
                                .as_ref()
                                .unwrap()
                                .GetCurrentPosition(Some(&mut play_cursor), Some(&mut write_cursor))
                            {
                                sound_is_valid = false;
                                dbg!(e);
                            }

                            if !sound_is_valid {
                                sound_output.running_sample_index =
                                    write_cursor / sound_output.bytes_per_sample;
                                sound_is_valid = true;
                            }

                            let byte_to_lock = (sound_output.running_sample_index
                                * sound_output.bytes_per_sample)
                                % sound_output.secondary_buffer_size;

                            let expected_sound_bytes_per_frame: u32 =
                                ((sound_output.samples_per_second * sound_output.bytes_per_sample)
                                    as f32
                                    / game_update_hz) as u32;

                            let seconds_left_until_flip =
                                target_seconds_per_frame - from_begin_to_audio_seconds;
                            let expected_bytes_until_flip: u32 = ((seconds_left_until_flip
                                / target_seconds_per_frame)
                                * expected_sound_bytes_per_frame as f32)
                                as u32;

                            let expected_frame_boundary_byte =
                                play_cursor + expected_bytes_until_flip;

                            let mut safe_write_cursor = write_cursor;
                            if safe_write_cursor < play_cursor {
                                safe_write_cursor += sound_output.secondary_buffer_size;
                            }

                            debug_assert!(safe_write_cursor >= play_cursor);
                            safe_write_cursor += sound_output.safety_bytes;

                            let audio_card_is_low_latency =
                                safe_write_cursor < expected_frame_boundary_byte;

                            let mut target_cursor = if audio_card_is_low_latency {
                                expected_frame_boundary_byte + expected_sound_bytes_per_frame
                            } else {
                                write_cursor
                                    + expected_sound_bytes_per_frame
                                    + sound_output.safety_bytes
                            };

                            target_cursor %= sound_output.secondary_buffer_size;

                            let bytes_to_write = if byte_to_lock > target_cursor {
                                sound_output.secondary_buffer_size - byte_to_lock + target_cursor
                            } else {
                                target_cursor - byte_to_lock
                            };

                            let sound_buffer = GameSoundOutputBuffer {
                                samples_per_second: sound_output.samples_per_second,
                                sample_count: bytes_to_write / sound_output.bytes_per_sample,
                                samples: samples as *mut i16,
                            };

                            if let Some(game_get_sound_samples) = game.get_sound_samples {
                                game_get_sound_samples(&thread, &mut game_memory, &sound_buffer);
                            }

                            #[cfg(feature = "gaming_internal")]
                            {
                                let marker = &mut debug_time_markers[debug_time_marker_index];
                                marker.output_play_cursor = play_cursor;
                                marker.output_write_cursor = write_cursor;
                                marker.output_location = byte_to_lock;
                                marker.output_byte_count = bytes_to_write;
                                marker.expected_flip_play_cursor = expected_frame_boundary_byte;

                                let mut unwrapped_write_cursor = write_cursor;
                                if unwrapped_write_cursor < play_cursor {
                                    unwrapped_write_cursor += sound_output.secondary_buffer_size;
                                }
                                audio_latency_bytes = unwrapped_write_cursor - play_cursor;

                                audio_latency_seconds = (audio_latency_bytes as f32
                                    / sound_output.bytes_per_sample as f32)
                                    / sound_output.samples_per_second as f32;

                                let debug_sound_output = format!(
                                            "pc: {play_cursor} wc: {write_cursor} delta: {audio_latency_bytes} {audio_latency_seconds}"
                                        );
                                // dbg!(debug_sound_output);
                            }

                            win32_fill_sound_buffer(
                                &mut sound_output,
                                byte_to_lock,
                                bytes_to_write,
                                sound_buffer,
                            );

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

                                    let test_seconds_elapsed_for_frame = win32_get_seconds_elapsed(
                                        last_counter,
                                        win32_get_wall_clock(),
                                    );

                                    if test_seconds_elapsed_for_frame < target_seconds_per_frame {
                                        // Log missed sleep
                                        dbg!(
                                            test_seconds_elapsed_for_frame,
                                            target_seconds_per_frame
                                        );
                                    }
                                }
                                while seconds_elapsed_for_frame < target_seconds_per_frame {
                                    seconds_elapsed_for_frame = win32_get_seconds_elapsed(
                                        last_counter,
                                        win32_get_wall_clock(),
                                    );
                                }
                            } else {
                                // Logging
                                // dbg!(seconds_elapsed_for_frame, target_seconds_per_frame);
                            }

                            let end_counter = win32_get_wall_clock();
                            let ms_per_frame =
                                1000.0 * win32_get_seconds_elapsed(last_counter, end_counter);
                            last_counter = end_counter;

                            #[cfg(feature = "gaming_internal")]
                            // win32_debug_sync_display(
                            //     GLOBAL_BACK_BUFFER.can_into(),
                            //     debug_time_markers.len(),
                            //     debug_time_markers.as_ref(),
                            //     debug_time_marker_index as i32 - 1,
                            //     &sound_output,
                            //     target_seconds_per_frame,
                            // );
                            let device_context = GetDC(window);

                            match win32_get_window_dimension(window) {
                                Ok(dimension) => win32_display_buffer_in_window(
                                    device_context,
                                    GLOBAL_BACK_BUFFER.can_into(),
                                    dimension.0,
                                    dimension.1,
                                ),
                                Err(e) => {
                                    dbg!(e);
                                }
                            };

                            ReleaseDC(window, device_context);

                            flip_wall_clock = win32_get_wall_clock();

                            #[cfg(feature = "gaming_internal")]
                            {
                                let mut play_cursor = 0;
                                let mut write_cursor = 0;
                                if let Err(e) =
                                    SECONDARY_BUFFER.as_ref().unwrap().GetCurrentPosition(
                                        Some(&mut play_cursor),
                                        Some(&mut write_cursor),
                                    )
                                {
                                    dbg!(e);
                                }

                                debug_assert!(debug_time_marker_index < debug_time_markers.len());

                                let marker = &mut debug_time_markers[debug_time_marker_index];

                                marker.flip_play_cursor = play_cursor;
                                marker.flip_write_cursor = write_cursor;
                            }

                            swap(new_input, old_input);

                            let end_cycle_count = _rdtsc();
                            let cycles_elapsed = end_cycle_count - last_cycle_count;
                            last_cycle_count = end_cycle_count;

                            let fps = 0.0;
                            let megac_per_frame = cycles_elapsed / (1000 * 1000);

                            let timer_output = format!(
                                "{:.02}ms/f, {}f/s, {}mc/f \n",
                                ms_per_frame, fps, megac_per_frame
                            );
                            // dbg!(timer_output);

                            // let debug_sound_output = format!("PC:{play_cursor} BTL:{byte_to_lock} TC:{target_cursor} BTW:{bytes_to_write}");
                            // "{timer_output});
                            // dbg!(debug_sound_output);
                            #[cfg(feature = "gaming_internal")]
                            {
                                debug_time_marker_index += 1;
                                if debug_time_marker_index == debug_time_markers.len() {
                                    debug_time_marker_index = 0;
                                }
                            }
                        }
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

fn win32_playback_input(win32_state: &mut Win32State, new_input: &mut GameInput) {
    let mut bytes_read: u32 = 0;
    let memory_size: usize = size_of::<GameInput>();
    unsafe {
        let read_buffer = slice::from_raw_parts_mut(
            new_input as *mut game::game::GameInput as *mut u8,
            memory_size,
        );

        if let Err(e) = ReadFile(
            win32_state.playback_handle,
            Some(read_buffer),
            Some(&mut bytes_read),
            None,
        ) {
            dbg!(e);
        }

        if bytes_read == 0 {
            let playing_index = win32_state.input_playing_index;
            win32_end_input_playback(win32_state).unwrap();
            win32_begin_input_playback(win32_state, playing_index).unwrap();
            if let Err(e) = ReadFile(
                win32_state.playback_handle,
                Some(read_buffer),
                Some(&mut bytes_read),
                None,
            ) {
                dbg!(e);
            }
        }
    }
}

fn win32_record_input(win32_state: &Win32State, new_input: &mut GameInput) {
    let mut bytes_written: u32 = 0;
    let memory_size = size_of::<GameInput>();
    unsafe {
        let write_buffer = new_input as *mut game::game::GameInput as *mut u8;
        match WriteFile(
            win32_state.recording_handle,
            Some(std::slice::from_raw_parts_mut(write_buffer, memory_size)),
            Some(&mut bytes_written),
            None,
        ) {
            Ok(()) => {
                debug_assert!(bytes_written == memory_size as u32);
                // File is read successfully, dont know if something should happen here
            }
            Err(e) => {
                dbg!(e);
            }
        }
    }
}

fn win32_begin_recording_input(state: &mut Win32State, input_recording_index: usize) -> Result<()> {
    debug_assert!(input_recording_index < state.replay_buffers.len());
    let mut replay_buffer = Win32ReplayBuffer::default();
    win32_get_replay_buffer(state, input_recording_index, &mut replay_buffer);

    if !replay_buffer.memory_block.is_null() {
        state.input_recording_index = input_recording_index;

        unsafe {
            let mut file_name = PathBuf::new();
            win32_get_input_file_location(state, input_recording_index, &mut file_name);

            state.recording_handle = CreateFileW(
                &HSTRING::from(file_name.as_os_str()),
                GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                CREATE_ALWAYS,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                HANDLE(0),
            )?;

            let bytes_to_write = state.total_size as u32;

            debug_assert!(state.total_size == bytes_to_write as u64);

            let file_position: i64 = state.total_size.try_into().unwrap();
            if let Err(e) =
                SetFilePointerEx(state.recording_handle, file_position, None, FILE_BEGIN)
            {
                dbg!(e);
            }

            ptr::copy_nonoverlapping(
                state.game_memory_block,
                replay_buffer.memory_block,
                state.total_size.try_into().unwrap(),
            );
        }
    }

    Ok(())
}

fn win32_end_recording_input(win32_state: &mut Win32State) -> Result<()> {
    unsafe { CloseHandle(win32_state.recording_handle)? }
    win32_state.input_recording_index = 0;

    Ok(())
}

fn win32_begin_input_playback(state: &mut Win32State, input_playing_index: usize) -> Result<()> {
    let mut replay_buffer = Win32ReplayBuffer::default();
    win32_get_replay_buffer(state, input_playing_index, &mut replay_buffer);

    if !replay_buffer.memory_block.is_null() {
        state.input_playing_index = input_playing_index;
        unsafe {
            let mut file_name = PathBuf::new();
            win32_get_input_file_location(state, input_playing_index, &mut file_name);

            state.playback_handle = CreateFileW(
                &HSTRING::from(file_name.as_os_str()),
                GENERIC_READ.0,
                FILE_SHARE_READ,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                HANDLE(0),
            )?;

            let file_position: i64 = state.total_size.try_into().unwrap();
            if let Err(e) = SetFilePointerEx(state.playback_handle, file_position, None, FILE_BEGIN)
            {
                dbg!(e);
            }

            ptr::copy_nonoverlapping(
                replay_buffer.memory_block,
                state.game_memory_block,
                state.total_size.try_into().unwrap(),
            );
        }
    }

    Ok(())
}

fn win32_end_input_playback(win32_state: &mut Win32State) -> Result<()> {
    unsafe { CloseHandle(win32_state.playback_handle)? }
    win32_state.input_playing_index = 0;

    Ok(())
}

fn win32_get_exe_file_name(state: &mut Win32State) {
    let exe_path = current_exe().unwrap();

    state.exe_full_path = exe_path.clone();
    state.exe_name = exe_path.file_name().unwrap().to_str().unwrap().to_string();
}

fn win32_build_exe_path_file_name(state: &Win32State, file_name: &str, dest: &mut PathBuf) {
    let path = state.exe_full_path.parent().unwrap().join(file_name);
    dest.push(path);
}

fn win32_get_input_file_location(state: &Win32State, slot_index: usize, dest: &mut PathBuf) {
    debug_assert!(slot_index == 1);
    win32_build_exe_path_file_name(state, "loop_playback.pb", dest);
}

fn win32_get_replay_buffer(
    state: &Win32State,
    index: usize,
    replay_buffer: &mut Win32ReplayBuffer,
) {
    debug_assert!(index < state.replay_buffers.len());

    let buffer = &state.replay_buffers[index];
    replay_buffer.memory_block = buffer.memory_block;
    replay_buffer.replay_file_name = buffer.replay_file_name.clone();
}
