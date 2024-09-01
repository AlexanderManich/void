use std::{
    io::{self},
    mem::{size_of, transmute},
    path::PathBuf,
    ptr::null_mut,
};

use std::ffi::c_void;

use crate::{intrinsics::find_least_significant_set_bit, macros::*, math::*, random, tile::*};

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

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

struct LoadedBitmap {
    width: i32,
    height: i32,
    pixels: *mut u32,
}

impl Default for LoadedBitmap {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            pixels: null_mut(),
        }
    }
}

#[derive(Default)]
struct HeroBitMaps {
    align_x: i32,
    align_y: i32,

    whole_player: LoadedBitmap,
    cape: LoadedBitmap,
}
pub struct GameState {
    world_arena: MemoryArena,
    world: *mut World,

    pub camera_p: TileMapPosition,
    pub player_p: TileMapPosition,
    d_player_p: Vec2,

    backdrop: LoadedBitmap,

    hero_facing_direction: usize,
    hero_bitmaps: Vec<HeroBitMaps>,
}

pub struct World {
    pub tile_map: *mut TileMap,
}

impl Default for World {
    fn default() -> Self {
        Self {
            tile_map: null_mut(),
        }
    }
}

#[derive(Debug)]
pub struct MemoryArena {
    size: usize,
    base: *mut u8,
    used: usize,
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

pub fn draw_rectangle(
    buffer: &GameOffscreenBuffer,
    v_min: Vec2,
    v_max: Vec2,
    r: f32,
    g: f32,
    b: f32,
) {
    let mut min_x: i32 = v_min.x.round() as i32;
    let mut min_y: i32 = v_min.y.round() as i32;
    let mut max_x: i32 = v_max.x.round() as i32;
    let mut max_y: i32 = v_max.y.round() as i32;

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

#[derive(Debug, Clone, Copy)]
#[repr(packed(1))]
struct BitmapHeader {
    file_type: u16,
    file_size: u32,
    reserved_1: u16,
    reserved_2: u16,
    bitmap_offset: u32,
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bits_per_pixel: u16,
    compression: u32,
    size_of_bitmap: u32,
    horz_resolution: i32,
    vert_resolution: i32,
    colors_used: u32,
    colors_important: u32,

    red_mask: u32,
    green_mask: u32,
    blue_mask: u32,
}

fn debug_load_bmp(
    thread: &ThreadContext,
    read_entire_file: &DebugPlatformReadEntireFile,
    file_name: &str,
) -> MyResult<LoadedBitmap> {
    let mut result = LoadedBitmap::default();

    let read_result = read_entire_file(thread, PathBuf::from(file_name))
        .map_err(|e| format!("{e}, {file_name}"))?;

    let header = unsafe { *read_result.contents.cast::<BitmapHeader>() };
    let contents = read_result.contents as *mut u32;

    let pixels = unsafe { contents.byte_add(header.bitmap_offset as usize) };

    result.pixels = pixels;
    result.width = header.width;
    result.height = header.height;

    debug_assert!(header.compression == 3);

    let red_mask = header.red_mask;
    let green_mask = header.green_mask;
    let blue_mask = header.blue_mask;
    let alpha_mask = !(red_mask | green_mask | blue_mask);

    let red_shift = find_least_significant_set_bit(red_mask);
    let green_shift = find_least_significant_set_bit(green_mask);
    let blue_shift = find_least_significant_set_bit(blue_mask);
    let alpha_shift = find_least_significant_set_bit(alpha_mask);

    debug_assert!(red_shift.is_some());
    debug_assert!(green_shift.is_some());
    debug_assert!(blue_shift.is_some());
    debug_assert!(alpha_shift.is_some());

    let mut source_dest = pixels;

    for _y in 0..header.height {
        for _x in 0..header.width {
            let c = unsafe { source_dest.read_unaligned() };
            unsafe {
                source_dest.write_unaligned(
                    (((c >> alpha_shift.unwrap()) & 0xFF) << 24)
                        | (((c >> red_shift.unwrap()) & 0xFF) << 16)
                        | (((c >> green_shift.unwrap()) & 0xFF) << 8)
                        | (((c >> blue_shift.unwrap()) & 0xFF) << 0),
                );
                source_dest = source_dest.add(1);
            }
        }
    }

    //Byte order in memory is BB GG RR AA, bottom up
    Ok(result)
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
        let player_height = 1.5;
        let player_width = player_height * 0.75;

        let game_state = memory
            .permanent_storage
            .cast::<GameState>()
            .as_mut()
            .unwrap();

        if !memory.is_initialized {
            game_state.backdrop = debug_load_bmp(
                thread,
                &memory.debug_platform_read_entire_file,
                "game/test/test_background.bmp",
            )
            .unwrap();

            game_state.hero_bitmaps.push({
                HeroBitMaps {
                    align_x: 22,
                    align_y: 56,
                    whole_player: debug_load_bmp(
                        thread,
                        &memory.debug_platform_read_entire_file,
                        "game/test/player.bmp",
                    )
                    .unwrap(),
                    cape: LoadedBitmap::default(),
                }
            });
            game_state.hero_bitmaps.push(HeroBitMaps {
                align_x: 22,
                align_y: 56,
                whole_player: debug_load_bmp(
                    thread,
                    &memory.debug_platform_read_entire_file,
                    "game/test/player_cape_front.bmp",
                )
                .unwrap(),
                cape: LoadedBitmap::default(),
            });

            game_state.player_p.abs_tile_x = 3;
            game_state.player_p.abs_tile_y = 3;
            game_state.player_p.offset = Vec2::new(5.0, 5.0);

            game_state.camera_p.abs_tile_x = game_state.player_p.abs_tile_x;
            game_state.camera_p.abs_tile_y = game_state.player_p.abs_tile_y;
            game_state.camera_p.offset = game_state.player_p.offset;

            initialize_arena(
                &mut game_state.world_arena,
                memory.permanent_storage_size as usize - size_of::<GameState>(),
                memory
                    .permanent_storage
                    .add(size_of::<GameState>())
                    .cast::<u8>(),
            );

            game_state.world = push_size!(&mut game_state.world_arena, World);
            let world = game_state.world.as_mut().unwrap();
            world.tile_map = push_size!(&mut game_state.world_arena, TileMap);
            let tile_map = world.tile_map.as_mut().unwrap();

            tile_map.chunk_shift = 4;
            tile_map.chunk_mask = (1 << tile_map.chunk_shift) - 1;
            tile_map.chunk_dim = 1 << tile_map.chunk_shift;

            tile_map.tile_chunk_count_x = 128;
            tile_map.tile_chunk_count_y = 128;
            tile_map.tile_chunk_count_z = 2;

            tile_map.tile_chunks = push_array!(
                &mut game_state.world_arena,
                tile_map.tile_chunk_count_x
                    * tile_map.tile_chunk_count_y
                    * tile_map.tile_chunk_count_z,
                TileChunk
            );

            tile_map.tile_side_in_meters = 1.5;

            // let lower_left_x = -(tile_side_in_pixels / 2) as f32;
            // let lower_left_y = buffer.height as f32;

            let mut random_number_index = 0;

            // Build tile_map
            let tiles_per_width: u32 = 17;
            let tiles_per_height: u32 = 9;

            let mut screen_x = 0;
            let mut screen_y = 0;
            let mut abs_tile_z = 0;

            let mut door_left = false;
            let mut door_right = false;
            let mut door_top = false;
            let mut door_bottom = false;
            let mut door_up = false;
            let mut door_down = false;

            for screen_index in 0..100 {
                debug_assert!(random_number_index < random::RANDOM_NUMBER_TABLE.len());

                let random_choice;

                if door_up || door_down {
                    random_choice = random::RANDOM_NUMBER_TABLE[random_number_index] % 2;
                } else {
                    random_choice = random::RANDOM_NUMBER_TABLE[random_number_index] % 3;
                }

                random_number_index += 1;

                let mut created_z_door = false;

                if random_choice == 2 {
                    created_z_door = true;
                    if abs_tile_z == 0 {
                        door_up = true;
                    } else {
                        door_down = true;
                    }
                } else if random_choice == 1 {
                    door_right = true;
                } else {
                    door_top = true;
                }

                for tile_y in 0..tiles_per_height {
                    for tile_x in 0..tiles_per_width {
                        let abs_tile_x = (screen_x * tiles_per_width) + tile_x;
                        let abs_tile_y = (screen_y * tiles_per_height) + tile_y;

                        let mut tile_value = 1;
                        if tile_x == 0 && (!door_left || tile_y != tiles_per_height / 2) {
                            tile_value = 2;
                        }
                        if tile_x == (tiles_per_width - 1)
                            && (!door_right || tile_y != tiles_per_height / 2)
                        {
                            tile_value = 2;
                        }
                        if tile_y == 0 && (!door_bottom || tile_x != tiles_per_width / 2) {
                            tile_value = 2;
                        }
                        if tile_y == (tiles_per_height - 1)
                            && (!door_top || tile_x != tiles_per_width / 2)
                        {
                            tile_value = 2;
                        }
                        if tile_x == 10 && tile_y == 5 {
                            if door_up {
                                tile_value = 3;
                            }
                            if door_down {
                                tile_value = 4;
                            }
                        }

                        set_tile_value(
                            &mut game_state.world_arena,
                            world.tile_map.as_mut().unwrap(),
                            abs_tile_x,
                            abs_tile_y,
                            abs_tile_z,
                            tile_value,
                        )
                    }
                }

                door_left = door_right;
                door_bottom = door_top;

                if created_z_door {
                    door_down = !door_down;
                    door_up = !door_up;
                } else {
                    door_up = false;
                    door_down = false;
                }

                door_right = false;
                door_top = false;

                if random_choice == 2 {
                    if abs_tile_z == 0 {
                        abs_tile_z = 1;
                    } else {
                        abs_tile_z = 0;
                    }
                } else if random_choice == 1 {
                    screen_x += 1
                } else {
                    screen_y += 1
                }
            }

            memory.is_initialized = true;
        }

        let world = game_state.world;
        let tile_map = (*world).tile_map.as_ref().unwrap();

        let tile_side_in_pixels: i32 = 60;
        let meters_to_pixels: f32 = tile_side_in_pixels as f32 / tile_map.tile_side_in_meters;

        for controller_index in 0..input.controllers.len() {
            let controller = get_controller(input, controller_index);

            if controller.is_analog {
            } else {
                let mut dd_player = Vec2::default(); // pixels/second

                //Fix facing directions when we have more bitmaps with facing directions
                if controller.button_state.concrete_buttons.move_up.ended_down {
                    game_state.hero_facing_direction = 0;
                    dd_player.y = 1.0;
                }
                if controller
                    .button_state
                    .concrete_buttons
                    .move_down
                    .ended_down
                {
                    game_state.hero_facing_direction = 0;
                    dd_player.y = -1.0;
                }
                if controller
                    .button_state
                    .concrete_buttons
                    .move_left
                    .ended_down
                {
                    game_state.hero_facing_direction = 0;
                    dd_player.x = -1.0
                }
                if controller
                    .button_state
                    .concrete_buttons
                    .move_right
                    .ended_down
                {
                    game_state.hero_facing_direction = 1;
                    dd_player.x = 1.0;
                }

                if dd_player.x != 0.0 && dd_player.y != 0.0 {
                    dd_player *= 0.5f32.sqrt();
                }

                // Walking Speed
                let mut player_speed = 100.0; // meters/second^2
                if controller.button_state.concrete_buttons.back.ended_down {
                    player_speed *= 3.0; // meters/second^2
                }
                dd_player *= player_speed;

                dd_player += 3.5 * -game_state.d_player_p;

                let mut new_player_p = game_state.player_p;
                // 1/2 * a * dt + v * dt + p
                new_player_p.offset = 0.5 * dd_player * square(input.dt_for_frame)
                    + game_state.d_player_p * input.dt_for_frame
                    + new_player_p.offset;

                game_state.d_player_p = dd_player * input.dt_for_frame + game_state.d_player_p;

                new_player_p = recanonicalize_position(&tile_map, new_player_p);

                let mut player_left = new_player_p;
                player_left.offset.x -= player_width * 0.5;
                player_left = recanonicalize_position(&tile_map, player_left);

                let mut player_right = new_player_p;
                player_right.offset.x += player_width * 0.5;
                player_right = recanonicalize_position(&tile_map, player_right);

                let mut collided = false;
                let mut col_p = TileMapPosition::default();

                if !is_tile_map_point_empty(tile_map, new_player_p) {
                    col_p = new_player_p;
                    collided = true;
                }

                if !is_tile_map_point_empty(tile_map, player_left) {
                    col_p = player_left;
                    collided = true;
                }

                if !is_tile_map_point_empty(tile_map, player_right) {
                    col_p = player_right;
                    collided = true;
                }

                if collided {
                    let mut r = Vec2::default();

                    if col_p.abs_tile_x < game_state.player_p.abs_tile_x {
                        r = Vec2::new(1.0, 0.0);
                    }
                    if col_p.abs_tile_x > game_state.player_p.abs_tile_x {
                        r = Vec2::new(-1.0, 0.0);
                    }
                    if col_p.abs_tile_y < game_state.player_p.abs_tile_y {
                        r = Vec2::new(0.0, 1.0);
                    }
                    if col_p.abs_tile_y > game_state.player_p.abs_tile_y {
                        r = Vec2::new(0.0, -1.0);
                    }

                    game_state.d_player_p =
                        game_state.d_player_p - 1.0 * game_state.d_player_p.dot(r) * r;
                } else {
                    if !are_on_same_tile(&game_state.player_p, &new_player_p) {
                        let new_tile_value = get_tile_value_from_map_pos(tile_map, new_player_p);
                        if new_tile_value == 3 {
                            new_player_p.abs_tile_z += 1;
                        } else if new_tile_value == 4 {
                            new_player_p.abs_tile_z -= 1;
                        }
                    }
                    game_state.player_p = new_player_p;
                    game_state.camera_p = new_player_p;
                }
            }
        }

        //Clear Screen
        draw_rectangle(
            buffer,
            Vec2::new(0.0, 0.0),
            Vec2::new(buffer.width as f32, buffer.height as f32),
            1.0,
            0.0,
            1.0,
        );

        // Draw background
        // draw_bitmap(buffer, &game_state.backdrop, 0.0, 0.0, None, None);

        //draw screen
        let screen_center_x: f32 = 0.5 * buffer.width as f32;
        let screen_center_y: f32 = 0.5 * buffer.height as f32;

        for rel_row in -100..100 {
            for rel_column in -200..200 {
                let column: u32 = game_state
                    .player_p
                    .abs_tile_x
                    .wrapping_add_signed(rel_column);
                let row: u32 = game_state.camera_p.abs_tile_y.wrapping_add_signed(rel_row);

                let tile_id: u32 =
                    get_tile_value(tile_map, column, row, game_state.camera_p.abs_tile_z);

                if tile_id > 1 {
                    let mut gray: f32 = 0.5;

                    if tile_id == 2 {
                        gray = 1.0;
                    }

                    if tile_id > 2 {
                        gray = 0.25;
                    }

                    if column == game_state.camera_p.abs_tile_x
                        && row == game_state.camera_p.abs_tile_y
                    {
                        gray = 0.0;
                    }

                    let tile_side = Vec2::new(
                        0.5 * tile_side_in_pixels as f32,
                        0.5 * tile_side_in_pixels as f32,
                    );
                    let cen = Vec2::new(
                        screen_center_x - meters_to_pixels * game_state.camera_p.offset.x
                            + rel_column as f32 * tile_side_in_pixels as f32,
                        screen_center_y + meters_to_pixels * game_state.camera_p.offset.y
                            - (rel_row as f32 * tile_side_in_pixels as f32),
                    );
                    let min = cen - tile_side;
                    let max = cen + tile_side;

                    draw_rectangle(buffer, min, max, gray, gray, gray)
                }
            }
        }

        // camera stuff that we might need? for now just scroll the screen.
        //let diff = subtract(tile_map, &game_state.player_p, &game_state.camera_p);

        let player_ground_point_x = screen_center_x; // + meters_to_pixels * diff.;
        let player_ground_point_y = screen_center_y; // - meters_to_pixels * diff.dy;

        let player_left_top = Vec2::new(
            player_ground_point_x - 0.5 * player_width * meters_to_pixels,
            player_ground_point_y - player_height * meters_to_pixels,
        );

        let player_width_height = Vec2::new(player_width, player_height);

        // Draw Player
        draw_rectangle(
            buffer,
            player_left_top,
            player_left_top + meters_to_pixels * player_width_height,
            1.0,
            0.0,
            0.0,
        );

        let hero_bitmaps = &game_state.hero_bitmaps[game_state.hero_facing_direction];

        draw_bitmap(
            buffer,
            &hero_bitmaps.whole_player,
            player_ground_point_x,
            player_ground_point_y,
            Some(hero_bitmaps.align_x),
            Some(hero_bitmaps.align_y),
        );
        draw_bitmap(
            buffer,
            &hero_bitmaps.cape,
            player_ground_point_x,
            player_ground_point_y,
            Some(hero_bitmaps.align_x),
            Some(hero_bitmaps.align_y),
        );
    }
}

fn draw_bitmap(
    buffer: &GameOffscreenBuffer,
    bitmap: &LoadedBitmap,
    mut real_x: f32,
    mut real_y: f32,
    align_x: Option<i32>,
    align_y: Option<i32>,
) {
    if align_x.is_some() {
        real_x -= align_x.unwrap() as f32;
    }
    if align_y.is_some() {
        real_y -= align_y.unwrap() as f32;
    }

    let mut min_x: i32 = real_x.round() as i32;
    let mut min_y: i32 = real_y.round() as i32;
    let mut max_x: i32 = (real_x + bitmap.width as f32).round() as i32;
    let mut max_y: i32 = (real_y + bitmap.height as f32).round() as i32;

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

    let mut source_row = bitmap.pixels;

    source_row = unsafe { source_row.add((bitmap.width * (bitmap.height - 1)) as usize) };
    let mut dest_row = unsafe {
        buffer
            .memory
            .offset(((min_x * buffer.bytes_per_pixel) + (min_y * buffer.pitch as i32)) as isize)
    } as *mut u8;

    unsafe {
        // This is really fucking slow
        for y in min_y..max_y {
            let mut dest = dest_row as *mut u32;
            let mut source = source_row;
            for x in min_x..max_x {
                let a = ((source.read_unaligned() >> 24) & 0xFF) as f32 / 255.0;
                let sr = ((source.read_unaligned() >> 16) & 0xFF) as f32;
                let sg = ((source.read_unaligned() >> 8) & 0xFF) as f32;
                let sb = ((source.read_unaligned() >> 0) & 0xFF) as f32;

                let dr = ((*dest >> 16) & 0xFF) as f32;
                let dg = ((*dest >> 8) & 0xFF) as f32;
                let db = ((*dest >> 0) & 0xFF) as f32;

                // Linear alpha interpolation
                let r = (1.0 - a) * dr + a * sr;
                let g = (1.0 - a) * dg + a * sg;
                let b = (1.0 - a) * db + a * sb;

                *dest =
                    ((r + 0.5) as u32) << 16 | ((g + 0.5) as u32) << 8 | ((b + 0.5) as u32) << 0;

                dest = dest.add(1);
                source = source.add(1);
            }
            dest_row = dest_row.add(buffer.pitch);
            source_row = source_row.sub(bitmap.width as usize);
        }
    }
}

pub fn push_size_(arena: &mut MemoryArena, size: usize) -> *mut c_void {
    debug_assert!((arena.used + size) <= arena.size);
    let result = unsafe { arena.base.add(arena.used) };
    arena.used += size;

    result as *mut c_void
}

fn initialize_arena(arena: &mut MemoryArena, size: usize, base: *mut u8) {
    arena.size = size;
    arena.base = base;
    arena.used = 0;
}
