use std::{ffi::c_void, fmt::Debug};

use windows::core::*;

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
    pub buttons: [GameButtonState; 12],
    pub concrete_buttons: ConcreteGameButtonState,
}

#[derive(Copy, Clone)]
pub struct ConcreteGameButtonState {
    pub move_up: GameButtonState,
    pub move_down: GameButtonState,
    pub move_left: GameButtonState,
    pub move_right: GameButtonState,

    pub action_up: GameButtonState,
    pub action_down: GameButtonState,
    pub action_left: GameButtonState,
    pub action_right: GameButtonState,

    pub left_shoulder: GameButtonState,
    pub right_shoulder: GameButtonState,

    pub back: GameButtonState,
    pub start: GameButtonState,
}

pub struct GameControllerInput {
    pub is_analog: bool,
    pub is_connected: bool,

    pub stick_average_x: f32,
    pub stick_average_y: f32,

    pub button_state: GameButtonStateUnion,
}

impl Default for GameControllerInput {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}

pub struct GameInput {
    pub controllers: [GameControllerInput; 5],
}

impl Default for GameInput {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
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

type DebugPlatformReadEntireFile = extern "C" fn(filename: PCWSTR) -> Result<DebugReadFileResult>;
type DebugPlatformFreeFileMemory = extern "C" fn(memory: *mut c_void);
type DebugPlatformWriteEntireFile =
    extern "C" fn(filename: PCWSTR, memory_size: u32, memory: *mut c_void) -> Result<()>;

pub struct GameMemory {
    pub is_initialized: bool,

    pub permanent_storage_size: u64,
    pub permanent_storage: *mut c_void, // Required to be cleared to zero at startup

    pub transient_storage_size: u64,
    pub transient_storage: *mut c_void, // Required to be cleared to zero at startup

    pub debug_platform_read_entire_file: DebugPlatformReadEntireFile,
    pub debug_platform_free_file_memory: DebugPlatformFreeFileMemory,
    pub debug_platform_write_entire_file: DebugPlatformWriteEntireFile,
}

pub struct GameState {
    pub blue_offset: i32,
    pub green_offset: i32,
    pub tone_hz: u32,
}
