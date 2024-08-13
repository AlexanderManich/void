use std::{
    default, io,
    mem::{size_of, transmute},
    ops::{AddAssign, Deref, MulAssign, SubAssign},
    path::PathBuf,
    ptr::{self, null_mut},
    vec,
};

use std::ffi::c_void;

use crate::tile::{
    get_tile_value, is_tile_map_point_empty, recanonicalize_position, TileChunk, TileMap,
    TileMapPosition,
};

pub type GameGetSoundSamples =
    fn(thread: &ThreadContext, memory: &mut GameMemory, sound_buffer: &GameSoundOutputBuffer);
pub type GameUpdateAndRender = fn(
    thread: &ThreadContext,
    buffer: &GameOffscreenBuffer,
    input: &mut GameInput,
    memory: &mut GameMemory,
);

pub struct GameOffscreenBuffer {
    pub width: i32,
    pub height: i32,
    pub memory: *mut c_void,
    pub pitch: usize,
    pub bytes_per_pixel: i32,
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
    pub mouse_buttons: [GameButtonState; 5],
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_z: i32,

    pub controllers: [GameControllerInput; 5],

    pub dt_for_frame: f32,
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

type DebugPlatformReadEntireFile =
    fn(thread: &ThreadContext, filename: PathBuf) -> io::Result<DebugReadFileResult>;
type DebugPlatformFreeFileMemory = fn(thread: &ThreadContext, memory: *mut c_void);
type DebugPlatformWriteEntireFile = fn(
    thread: &ThreadContext,
    filename: PathBuf,
    memory_size: u32,
    memory: *mut c_void,
) -> io::Result<()>;

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

pub struct ThreadContext {
    pub placeholder: i32,
}

impl Default for ThreadContext {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}

pub struct GameState {
    pub player_p: TileMapPosition,
}

pub struct World {
    pub tile_map: TileMap,
}

impl Default for World {
    fn default() -> Self {
        Self {
            tile_map: TileMap::default(),
        }
    }
}

#[inline]
pub fn get_controller(input: &mut GameInput, controller_index: usize) -> &mut GameControllerInput {
    assert!(controller_index < input.controllers.len());
    &mut input.controllers[controller_index]
}

#[no_mangle]
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
                *pixel = ((green << 16) | blue) as u32;
                pixel = pixel.add(1);
            }

            row = row.add(buffer.pitch);
        }
    }
}

#[no_mangle]
pub fn draw_rectangle(
    buffer: &GameOffscreenBuffer,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    r: f32,
    g: f32,
    b: f32,
) {
    let mut min_x: i32 = min_x.round() as i32;
    let mut min_y: i32 = min_y.round() as i32;
    let mut max_x: i32 = max_x.round() as i32;
    let mut max_y: i32 = max_y.round() as i32;

    if min_x < 0 {
        min_x = 0;
    }
    if min_y < 0 {
        min_y = 0;
    }
    if max_x > buffer.width {
        max_x = buffer.width;
    }
    if max_y > buffer.height {
        max_y = buffer.height;
    }

    unsafe {
        let color: u32 = ((((r * 255.0).round() as u32) << 16)
            | (((g * 255.0).round() as u32) << 8)
            | (((b * 255.0).round() as u32) << 0)) as u32;

        let mut row = buffer
            .memory
            .offset(((min_x * buffer.bytes_per_pixel) + (min_y * buffer.pitch as i32)) as isize)
            as *mut u8;

        for _y in min_y..max_y {
            let mut pixel = row as *mut u32;
            for _x in min_x..max_x {
                *pixel = color;
                pixel = pixel.add(1);
            }
            row = row.add(buffer.pitch);
        }
    }
}

#[no_mangle]
pub fn game_output_sound(
    game_state: &mut GameState,
    sound_buffer: &GameSoundOutputBuffer,
    tone_hz: u32,
) {
    let tone_volume: f32 = 0.0;
    let wave_period = sound_buffer.samples_per_second / tone_hz;

    let mut sample_out = sound_buffer.samples;
    for _ in 0..sound_buffer.sample_count {
        unsafe {
            // let sine_value: f32 = game_state.t_sine.sin();
            // let sample_value: i16 = (sine_value * tone_volume) as i16;
            let sample_value: i16 = 0;
            *sample_out = sample_value;
            sample_out = sample_out.add(1);
            *sample_out = sample_value;
            sample_out = sample_out.add(1);

            // game_state.t_sine += 2.0 * PI / wave_period as f32;
            // if game_state.t_sine > 2.0 * PI {
            //     game_state.t_sine -= 2.0 * PI;
            // }
        }
    }
}

#[no_mangle]
pub fn game_get_sound_samples(
    thread: &ThreadContext,
    memory: &mut GameMemory,
    sound_buffer: &GameSoundOutputBuffer,
) {
    unsafe {
        let game_state = transmute::<*mut c_void, &mut GameState>(memory.permanent_storage);
        game_output_sound(game_state, sound_buffer, 400);
    }
}

#[no_mangle]
pub fn game_update_and_render(
    thread: &ThreadContext,
    buffer: &GameOffscreenBuffer,
    input: &mut GameInput,
    memory: &mut GameMemory,
) {
    debug_assert!(
        size_of::<ConcreteGameButtonState>() / size_of::<GameButtonState>()
            == unsafe { input.controllers[0].button_state.buttons.len() }
    );
    debug_assert!(size_of::<GameState>() <= memory.permanent_storage_size.try_into().unwrap());

    unsafe {
        const TILE_MAP_COUNT_X: usize = 256;
        const TILE_MAP_COUNT_Y: usize = 256;

        let mut temp_tiles = vec![vec![0; 256]; 256];

        temp_tiles[0][0] = 1;
        temp_tiles[0][1] = 1;
        temp_tiles[0][2] = 1;
        temp_tiles[0][3] = 1;
        temp_tiles[0][4] = 1;
        temp_tiles[0][5] = 1;

        temp_tiles[0][0] = 1;
        temp_tiles[1][0] = 1;
        temp_tiles[2][0] = 1;
        temp_tiles[3][0] = 1;
        temp_tiles[4][0] = 1;
        temp_tiles[5][0] = 1;

        let mut tile_map = TileMap::default();

        tile_map.chunk_shift = 8;
        tile_map.chunk_mask = (1 << tile_map.chunk_shift) - 1;
        tile_map.chunk_dim = 256;

        tile_map.tile_chunk_count_x = 1;
        tile_map.tile_chunk_count_y = 1;

        let tile_chunk = ptr::from_mut(&mut TileChunk::default());
        (*tile_chunk).tiles = temp_tiles;
        tile_map.tile_chunks = tile_chunk;

        tile_map.tile_side_in_meters = 1.5;
        tile_map.tile_side_in_pixels = 60;

        let lower_left_x = -(tile_map.tile_side_in_pixels / 2) as f32;
        let lower_left_y = buffer.height as f32;

        tile_map.meters_to_pixels =
            tile_map.tile_side_in_pixels as f32 / tile_map.tile_side_in_meters;

        let player_height = 1.5;
        let player_width = player_height * 0.75;

        let game_state = transmute::<*mut c_void, &mut GameState>(memory.permanent_storage);

        if !memory.is_initialized {
            memory.is_initialized = true;

            game_state.player_p.abs_tile_x = 5;
            game_state.player_p.abs_tile_y = 5;
            game_state.player_p.tile_rel_x = 5.0;
            game_state.player_p.tile_rel_y = 5.0;
        }

        for controller_index in 0..input.controllers.len() {
            let controller = get_controller(input, controller_index);

            if controller.is_analog {
            } else {
                let mut d_player_x = 0.0; // pixels/second
                let mut d_player_y = 0.0; // pixels/second

                if controller.button_state.concrete_buttons.move_up.ended_down {
                    d_player_y = 1.0;
                }
                if controller
                    .button_state
                    .concrete_buttons
                    .move_down
                    .ended_down
                {
                    d_player_y = -1.0;
                }
                if controller
                    .button_state
                    .concrete_buttons
                    .move_left
                    .ended_down
                {
                    d_player_x = -1.0
                }
                if controller
                    .button_state
                    .concrete_buttons
                    .move_right
                    .ended_down
                {
                    d_player_x = 1.0;
                }

                // Walking Speed
                let mut player_speed = 10.0;
                if controller.button_state.concrete_buttons.back.ended_down {
                    player_speed = 20.0;
                }

                d_player_x.mul_assign(player_speed);
                d_player_y.mul_assign(player_speed);

                let mut new_player_p = game_state.player_p;
                new_player_p
                    .tile_rel_x
                    .add_assign(d_player_x * input.dt_for_frame);
                new_player_p
                    .tile_rel_y
                    .add_assign(d_player_y * input.dt_for_frame);

                new_player_p = recanonicalize_position(&tile_map, new_player_p);

                let mut player_left = new_player_p;
                player_left.tile_rel_x.sub_assign(player_width * 0.5);
                player_left = recanonicalize_position(&tile_map, player_left);

                let mut player_right = new_player_p;
                player_right.tile_rel_x.add_assign(player_width * 0.5);
                player_right = recanonicalize_position(&tile_map, player_right);

                if is_tile_map_point_empty(&tile_map, new_player_p)
                    && is_tile_map_point_empty(&tile_map, player_left)
                    && is_tile_map_point_empty(&tile_map, player_right)
                {
                    game_state.player_p = new_player_p;
                }
            }
        }

        draw_rectangle(
            buffer,
            0.0,
            0.0,
            buffer.width as f32,
            buffer.height as f32,
            1.0,
            0.0,
            1.0,
        );

        let screen_center_x: f32 = 0.5 * buffer.width as f32;
        let screen_center_y: f32 = 0.5 * buffer.height as f32;

        for rel_row in -10..10 {
            for rel_column in -20..20 {
                let column: u32 = game_state
                    .player_p
                    .abs_tile_x
                    .wrapping_add_signed(rel_column);
                let row: u32 = game_state.player_p.abs_tile_y.wrapping_add_signed(rel_row);

                let tile_idx: u32 = get_tile_value(&tile_map, column, row);
                let mut gray: f32 = 0.5;
                if tile_idx == 1 {
                    gray = 1.0;
                }

                if column == game_state.player_p.abs_tile_x && row == game_state.player_p.abs_tile_y
                {
                    gray = 0.0;
                }

                let cen_x: f32 = screen_center_x
                    - tile_map.meters_to_pixels * game_state.player_p.tile_rel_x
                    + rel_column as f32 * tile_map.tile_side_in_pixels as f32;
                let cen_y: f32 = screen_center_y
                    + tile_map.meters_to_pixels * game_state.player_p.tile_rel_y
                    - (rel_row as f32 * tile_map.tile_side_in_pixels as f32);
                let min_x: f32 = cen_x - 0.5 * tile_map.tile_side_in_pixels as f32;
                let min_y: f32 = cen_y - 0.5 * tile_map.tile_side_in_pixels as f32;
                let max_x: f32 = cen_x + 0.5 * tile_map.tile_side_in_pixels as f32;
                let max_y: f32 = cen_y + 0.5 * tile_map.tile_side_in_pixels as f32;

                draw_rectangle(buffer, min_x, min_y, max_x, max_y, gray, gray, gray)
            }
        }

        let player_left = screen_center_x - 0.5 * player_width * tile_map.meters_to_pixels;
        let player_top = screen_center_y - player_height * tile_map.meters_to_pixels;

        draw_rectangle(
            buffer,
            player_left,
            player_top,
            player_left + tile_map.meters_to_pixels * player_width,
            player_top + tile_map.meters_to_pixels * player_height,
            1.0,
            0.0,
            0.0,
        )
    }
}
