use std::{f32::consts::PI, ffi::c_void, mem::size_of};

use windows::core::{w, HSTRING, PCWSTR};

use crate::{debug_platform_free_file_memory, debug_platform_read_entire_file};
pub struct GameOffscreenBuffer {
    pub width: i32,
    pub height: i32,
    pub memory: *mut c_void,
    pub pitch: usize,
}

impl Default for GameOffscreenBuffer {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}

pub struct GameSoundOutputBuffer {
    pub samples_per_second: u32,
    pub sample_count: u32,
    pub samples: *mut i16,
}

impl Default for GameSoundOutputBuffer {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}

#[derive(Copy, Clone)]
pub struct GameButtonState {
    pub half_transition_count: i32,
    pub ended_down: bool,
}

pub union GameButtonStateUnion {
    pub buttons: [GameButtonState; 6],
    pub concrete_buttons: ConcreteGameButtonState,
}

#[derive(Copy, Clone)]
pub struct ConcreteGameButtonState {
    pub up: GameButtonState,
    pub down: GameButtonState,
    pub left: GameButtonState,
    pub right: GameButtonState,
    pub left_shoulder: GameButtonState,
    pub right_shoulder: GameButtonState,
}

pub struct GameControllerInput {
    pub is_analog: bool,

    pub start_x: f32,
    pub start_y: f32,

    pub min_x: f32,
    pub min_y: f32,

    pub max_x: f32,
    pub max_y: f32,

    pub end_x: f32,
    pub end_y: f32,

    pub button_state: GameButtonStateUnion,
}

pub struct GameInput {
    pub controllers: [GameControllerInput; 4],
}

impl Default for GameInput {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}

pub struct GameMemory {
    pub is_initialized: bool,

    pub permanent_storage_size: u64,
    pub permanent_storage: *mut c_void, // Required to be cleared to zero at startup

    pub transient_storage_size: u64,
    pub transient_storage: *mut c_void, // Required to be cleared to zero at startup
}

impl Default for GameMemory {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}

pub struct GameState {
    blue_offset: i32,
    green_offset: i32,
    tone_hz: u32,
}

pub fn render_gradient(buffer: &GameOffscreenBuffer, blue_offset: i32, green_offset: i32) {
    unsafe {
        // Draw bitmap to window.
        let mut row: *mut u8 = buffer.memory.cast();
        for y in 0..buffer.height {
            let mut pixel: *mut u32 = row.cast();
            for x in 0..buffer.width {
                // Truncate to 8 bits using mask, because i couldn't figure out how to cast it :)
                let blue = (x + blue_offset) & 0xFF;
                let green = (y + green_offset) & 0xFF;

                // Shift green up by 8 bits from bottom, to make them sit in the green channel and not overwrite the blue channel (little endian)
                *pixel = ((green << 8) | blue) as u32;
                pixel = pixel.add(1);
            }

            row = row.add(buffer.pitch);
        }
    }
}

pub fn game_output_sound(sound_buffer: &GameSoundOutputBuffer, tone_hz: u32) {
    static mut T_SINE: f32 = 0.0;
    let tone_volume: f32 = 200.0;
    let wave_period = sound_buffer.samples_per_second / tone_hz;

    let mut sample_out = sound_buffer.samples;
    for _ in 0..sound_buffer.sample_count {
        unsafe {
            let sine_value: f32 = T_SINE.sin();
            let sample_value: i16 = (sine_value * tone_volume) as i16;
            *sample_out = sample_value;
            sample_out = sample_out.add(1);
            *sample_out = sample_value;
            sample_out = sample_out.add(1);

            T_SINE += 2.0 * PI / wave_period as f32;
        }
    }
}

pub fn game_update_and_render(
    buffer: &GameOffscreenBuffer,
    sound_buffer: &GameSoundOutputBuffer,
    input: &GameInput,
    memory: &mut GameMemory,
) {
    debug_assert!(size_of::<GameState>() <= memory.permanent_storage_size.try_into().unwrap());

    unsafe {
        let game_state = memory.permanent_storage as *mut GameState;
        if !memory.is_initialized {
            #[cfg(feature = "gaming_internal")]
            {
                // TODO: File path of THIS file right now. change to something idk
                let filename = file!().encode_utf16().chain([0u16]).collect::<Vec<u16>>();
                let filename_test = PCWSTR(filename.as_ptr());
                match debug_platform_read_entire_file(filename_test) {
                    Ok(bitmap_memory) => {
                        debug_platform_free_file_memory(bitmap_memory);
                    }
                    Err(e) => {
                        dbg!(e);
                    }
                };
            }

            (*game_state).tone_hz = 256;
            memory.is_initialized = true;
        }

        let input0 = &input.controllers[0];
        if input0.is_analog {
            (*game_state).blue_offset = (4.0 * input0.end_x) as i32;
            (*game_state).tone_hz = 256 + ((128.0 * input0.end_y) as u32);
        } else {
        }

        if input0.button_state.concrete_buttons.down.ended_down {
            (*game_state).green_offset += 1;
        }

        game_output_sound(sound_buffer, (*game_state).tone_hz);
        render_gradient(
            buffer,
            (*game_state).blue_offset,
            (*game_state).green_offset,
        );
    }
}
