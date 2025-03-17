#include "../base/base_include.h"
#include "../intrinsics.h"
#include "../random.h"
#include "tile.h"
#include "game.h"

LoadedBitmap 
debug_load_bmp(ThreadContext *thread, DebugPlatformReadEntireFile *read_entire_file, String8 file_name) {
    LoadedBitmap result = {};

    String8 read_result = (*read_entire_file)(thread, file_name);

    BitmapHeader *header = (BitmapHeader *)read_result.str;
    u32 *contents = (u32 *)read_result.str;

    u32 *pixels = (u32 *)((u8 *)contents + header->bitmap_offset);

    result.pixels = pixels;
    result.width = header->width;
    result.height = header->height;

    assert(header->compression == 3);

    u32 red_mask = header->red_mask;
    u32 green_mask = header->green_mask;
    u32 blue_mask = header->blue_mask;
    u32 alpha_mask = ~(red_mask | green_mask | blue_mask);

    BitScanResult red_shift = find_least_significant_set_bit(red_mask);
    BitScanResult green_shift = find_least_significant_set_bit(green_mask);
    BitScanResult blue_shift = find_least_significant_set_bit(blue_mask);
    BitScanResult alpha_shift = find_least_significant_set_bit(alpha_mask);

    assert(red_shift.found);
    assert(green_shift.found);
    assert(blue_shift.found);
    assert(alpha_shift.found);

    u32 *source_dest = pixels;

    for (i32 y = 0; y < header->height; y++) {
        for ( i32 x = 0; x < header->width; x++) {
            u32 c = *source_dest;
            *source_dest = ((((c >> alpha_shift.index) & 0xFF) << 24) |
                            (((c >> red_shift.index)   & 0xFF) << 16) |
                            (((c >> green_shift.index) & 0xFF) << 8)  |
                            (((c >> blue_shift.index)  & 0xFF) << 0));
            *source_dest += 1;
        }
    }
    //Byte order in memory is BB GG RR AA, bottom up
    return result;
}

void
game_update_and_render(ThreadContext *thread, GameOffscreenBuffer *buffer, GameInput *input, GameMemory *memory) {
    // TODO: assert len(input.controllers[0].button_state.buttons) ==
    assert(sizeof(GameState) <= memory->permanent_storage_size);

    f32 player_height = 1.5;
    f32 player_width = player_height * 0.75;

    GameState *game_state = (GameState *)memory->permanent_storage;

    if (!memory->is_initialized) {
        game_state->backdrop = debug_load_bmp(thread, &memory->debug_platform_read_entire_file, str8_lit("../test/test_background.bmp"));

        game_state->player_p.abs_tile_x = 3;
        game_state->player_p.abs_tile_y = 3;
        game_state->player_p.offset = vec2(5.0, 5.0);

        game_state->camera_p.abs_tile_x = game_state->player_p.abs_tile_x;
        game_state->camera_p.abs_tile_y = game_state->player_p.abs_tile_y;
        game_state->camera_p.offset = game_state->player_p.offset;

        arena_init(
            &game_state->world_arena, 
            (u8 *)memory->permanent_storage + sizeof(GameState), 
            memory->permanent_storage_size - sizeof(GameState)
        );
        
        game_state->world = push_array(&game_state->world_arena, World, 1);
        World *world = game_state->world;
        world->tile_map = push_array(&game_state->world_arena, TileMap, 1);
        TileMap *tile_map = world->tile_map;

        tile_map->chunk_shift = 4;
        tile_map->chunk_mask = (1 << tile_map->chunk_shift) - 1;
        tile_map->chunk_dim = 1 << tile_map->chunk_shift;

        tile_map->tile_chunk_count_x = 128;
        tile_map->tile_chunk_count_y = 128;
        tile_map->tile_chunk_count_z = 2;
        
        tile_map->tile_chunks = push_array(
            &game_state->world_arena, 
            TileChunk, 
            tile_map->tile_chunk_count_x * tile_map->tile_chunk_count_y * tile_map->tile_chunk_count_z
        );

        tile_map->tile_side_in_meters = 1.5;

        u64 random_number_index = 0;

        // Build tile_map
        u32 tiles_per_width = 17;
        u32 tiles_per_height = 9;

        u32 screen_x = 0;
        u32 screen_y = 0;
        u32 abs_tile_z = 0;

        b32 door_left = false;
        b32 door_right = false;
        b32 door_top = false;
        b32 door_bottom = false;
        b32 door_up = false;
        b32 door_down = false;

        for (i32 screen_index = 0; screen_index < 100; screen_index++) {
            assert(random_number_index < array_count(RANDOM_NUMBER_TABLE));

            u32 random_choice;

            if (door_up || door_down) {
                random_choice = RANDOM_NUMBER_TABLE[random_number_index] % 2;
            }
            else {
                random_choice = RANDOM_NUMBER_TABLE[random_number_index] % 3;
            }

            random_number_index += 1;

            b32 created_z_door = false;

            if (random_choice == 2) {
                created_z_door = true;
                if (abs_tile_z == 0) {
                    door_up = true;
                }
                else {
                    door_down = true;
                }
            }
            else if (random_choice == 1) {
                door_right = true;
            }
            else {
                door_top = true;
            }

            for (i32 tile_y = 0; tile_y < tiles_per_height; tile_y++) {
                for (i32 tile_x = 0; tile_x < tiles_per_width; tile_x++) {
                    u32 abs_tile_x = (screen_x * tiles_per_height) + tile_x;
                    u32 abs_tile_y = (screen_y * tiles_per_width) + tile_y;

                    u32 tile_value = 1;

                    if (tile_x == 0 && (!door_left || tile_y != tiles_per_height / 2)) {
                        tile_value = 2;
                    }
                    if (tile_x == (tiles_per_width - 1) && (!door_right || tile_y != tiles_per_height / 2)) {
                        tile_value = 2;
                    }
                    if (tile_y == 0 && (!door_bottom || tile_x != tiles_per_width / 2)) {
                        tile_value = 2;
                    }
                    if (tile_y == (tiles_per_height - 1) && (!door_top || tile_x != tiles_per_width / 2)) {
                        tile_value = 2; 
                    }
                    if (tile_x == 10 && tile_y == 5) {
                        if (door_up) {
                            tile_value = 3;
                        }
                        if (door_down) {
                            tile_value = 4;
                        }
                    }

                    // set_tile_value(
                    //     &game_state->world_arena,
                    //     world->tile_map,
                    //     abs_tile_x,
                    //     abs_tile_y,
                    //     abs_tile_z,
                    //     tile_value
                    // );
                }
            }

            door_left = door_right;
            door_bottom = door_top;

            if (created_z_door) {
                door_down = !door_down;
                door_up = !door_up;
            }
            else {
                door_up = false;
                door_down = false;
            }

            door_right = false;
            door_top = false;

            if (random_choice == 2) {
                if (abs_tile_z == 0) {
                    abs_tile_z = 1;
                }
                else {
                    abs_tile_z = 0;
                }
            }
            else if (random_choice == 1) {
                screen_x += 1;
            }
            else {
                screen_y += 1;
            }
        }

        memory->is_initialized = true;
    }

    World *world = game_state->world;
    TileMap *tile_map = world->tile_map;

    i32 tile_side_in_pixels = 60;
    f32 meters_to_pixels = (f32)tile_side_in_pixels / tile_map->tile_side_in_meters;

    for (i32 controller_index = 0; controller_index < array_count(input->controllers); controller_index++) {
        GameControllerInput *controller = get_controller(input, controller_index);

        if (controller->is_analog) {

        }
        else {
            Vec2 dd_player = {};

            //Fix facing directions when we have more bitmaps with facing directions
            if (controller->move_up.ended_down) {
                game_state->hero_facing_direction = 0;
                dd_player.y = 1.0;
            }
            if (controller->move_down.ended_down) {
                game_state->hero_facing_direction = 0;
                dd_player.y = -1.0;
            }
            if (controller->move_left.ended_down) {
                game_state->hero_facing_direction = 0;
                dd_player.x = -1.0;
            }
            if (controller->move_right.ended_down) {
                game_state->hero_facing_direction = 0;
                dd_player.x = 1.0;
            }

            if (dd_player.x != 0.0 && dd_player.y != 0.0) {
                dd_player = vec2_mul(dd_player, 0.707106781); // Sqrt(0.5) = 0.707106781
            }

            // Walking Speed
            f32 player_speed = 100.0; // meters/second^2
            if (controller->back.ended_down) {
                player_speed *= 3.0;
            }
            dd_player = vec2_mul(dd_player, player_speed);

            // dd_player = vec2_add(dd_player, vec2_mul())
        }
    }
}

void
render_gradient(GameOffscreenBuffer *buffer, i32 blue_offset, i32 green_offset) {
    // Draw bitmap to window.
    u8 *row = (u8 *)buffer->memory;
    for (i32 y = 0; y < buffer->height; y++) {
        u32 *pixel = (u32 *)row;
        for (i32 x = 0; x < buffer->width; x++) {
            u8 blue = (x + blue_offset);
            u8 green = (y + green_offset);

            // Shift green up by 8 bits from bottom, to make them sit in the green channel and not overwrite the blue channel (little endian)
            *pixel++ = ((green << 8) | blue);
        }

        row += buffer->pitch;
    }
}
