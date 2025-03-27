#include "../base/base_include.h"
#include "../random.h"
#include "../intrinsics.h"
#include "tile.h"
#include "game.h"

#include "../base/base_include.c"
#include "tile.cpp"
#include <stdio.h>

LoadedBitmap 
debug_load_bmp(ThreadContext *thread, DebugPlatformReadEntireFile *read_entire_file, String8 file_name) {
    LoadedBitmap result = {};

    String8 read_result = (*read_entire_file)(thread, file_name);

    if (read_result.size) {

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
            for (i32 x = 0; x < header->width; x++) {
                u32 c = *source_dest;
                *source_dest = ((((c >> alpha_shift.index) & 0xFF) << 24) |
                            (((c >> red_shift.index)   & 0xFF) << 16) |
                            (((c >> green_shift.index) & 0xFF) << 8)  |
                            (((c >> blue_shift.index)  & 0xFF) << 0));
                source_dest += 1;
            }
        }
    }
    //Byte order in memory is BB GG RR AA, bottom up
    return result;
}

void draw_bitmap(GameOffscreenBuffer *buffer, LoadedBitmap *bitmap, f32 real_x, f32 real_y, i32 align_x, i32 align_y) {
    if (align_x) {
        real_x -= (f32)align_x;
    }
    if (align_y) {
        real_y -= (f32)align_y;
    }

    i32 min_x = round_f32_to_i32(real_x);
    i32 min_y = round_f32_to_i32(real_y);
    i32 max_x = round_f32_to_i32(real_x + (f32)bitmap->width);
    i32 max_y = round_f32_to_i32(real_y + (f32)bitmap->height);

    if (min_x < 0) {
        min_x = 0;
    }
    if (min_y < 0) {
        min_y = 0;
    }
    if (max_x > buffer->width) {
        max_x = buffer->width;
    }
    if (max_y > buffer->height) {
        max_y = buffer->height;
    }

    u32 *source_row = bitmap->pixels + bitmap->width * (bitmap->height - 1);
    u8 *dest_row = ((u8 *)buffer->memory + min_x * buffer->bytes_per_pixel + min_y * buffer->pitch);

    for (i32 y = min_y; y < max_y; y++) {
        u32 *dest = (u32 *)dest_row;
        u32 *source = source_row;
        for (i32 x = min_x; x < max_x; x++) {
            f32 a  = (f32)((*source >> 24) & 0xFF) / 255.0;
            f32 sr = (f32)((*source >> 16) & 0xFF);
            f32 sg = (f32)((*source >> 8)  & 0xFF);
            f32 sb = (f32)((*source >> 0)  & 0xFF);
            
            f32 dr = (f32)((*dest >> 16) & 0xFF);
            f32 dg = (f32)((*dest >> 8)  & 0xFF);
            f32 db = (f32)((*dest >> 0)  & 0xFF);
            
            f32 r = (1.0 - a) * dr + a * sr;
            f32 g = (1.0 - a) * dg + a * sg;
            f32 b = (1.0 - a) * db + a * sb;
            
            *dest = (((u32)(r + 0.5) << 16) | ((u32)(g + 0.5) << 8) | ((u32)(b + 0.5) << 0));
            
            dest += 1;
            source += 1;
        }
        dest_row += buffer->pitch;
        source_row -= bitmap->width;
    }
}

void 
draw_rectangle(GameOffscreenBuffer *buffer, Vec2 v_min, Vec2 v_max, f32 r, f32 g, f32 b) {
    i32 min_x = round_f32_to_i32(v_min.x);
    i32 min_y = round_f32_to_i32(v_min.y);
    i32 max_x = round_f32_to_i32(v_max.x);
    i32 max_y = round_f32_to_i32(v_max.y);

    if (min_x < 0) {
        min_x = 0;
    }
    if (min_y < 0) {
        min_y = 0;
    }
    if (max_x > buffer->width) {
        max_x = buffer->width;
    }
    if (max_y > buffer->height) {
        max_y = buffer->height;
    }

    // TODO: Is this right?
    u32 color = (round_f32_to_u32(r * 255.0) << 16)
        | (round_f32_to_u32(g * 255.0) << 8)
        | (round_f32_to_u32(b * 255.0) << 0);
    
    u8 *row = (u8 *)buffer->memory + min_x * buffer->bytes_per_pixel + min_y * buffer->pitch;

    for (i32 y = min_y; y < max_y; y++) {
        u32 *pixel = (u32 *)row;
        for (i32 x = min_x; x < max_x; x++) {
            *pixel = color;
            pixel += 1;
        }
        row += buffer->pitch;
    }
}

extern "C" GAME_UPDATE_AND_RENDER(game_update_and_render) {
    // TODO: assert len(input.controllers[0].button_state.buttons) ==
    assert(sizeof(GameState) <= memory->permanent_storage_size);

    f32 player_height = 1.5;
    f32 player_width = player_height * 0.75;

    GameState *game_state = (GameState *)memory->permanent_storage;

    if (!memory->is_initialized) {
        game_state->backdrop = debug_load_bmp(thread, memory->debug_platform_read_entire_file, str8_lit("../test/test_background.bmp"));

        game_state->hero_bitmaps[0] = HeroBitmaps{
            .align_x = 22,
            .align_y = 56,
            .whole_player = debug_load_bmp(thread, memory->debug_platform_read_entire_file, str8_lit("../test/player.bmp")),
            .cape = 0,
        };
        game_state->hero_bitmaps[1] = HeroBitmaps{
            .align_x = 22,
            .align_y = 56,
            .whole_player = debug_load_bmp(thread, memory->debug_platform_read_entire_file, str8_lit("../test/player_cape_front.bmp")),
            .cape = 0,
        };

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

            printf("Screen (%u, %u, %u): Doors - Left: %d, Right: %d, Top: %d, Bottom: %d, AbsTileZ: %u\n",
                   screen_x, screen_y, abs_tile_z, door_left, door_right, door_top, door_bottom, abs_tile_z);

            for (u32 tile_y = 0; tile_y < tiles_per_height; tile_y++) {
                for (u32 tile_x = 0; tile_x < tiles_per_width; tile_x++) {
                    u32 abs_tile_x = (screen_x * tiles_per_width) + tile_x;
                    u32 abs_tile_y = (screen_y * tiles_per_height) + tile_y;

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

                    set_tile_value(
                        &game_state->world_arena,
                        world->tile_map,
                        abs_tile_x,
                        abs_tile_y,
                        abs_tile_z,
                        tile_value
                    );
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

    for (u64 controller_index = 0; controller_index < array_count(input->controllers); controller_index++) {
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
                game_state->hero_facing_direction = 1;
                dd_player.x = 1.0;
            }

            if (dd_player.x != 0.0 && dd_player.y != 0.0) {
                dd_player = dd_player * 0.707106781; // Sqrt(0.5) = 0.707106781
            }

            // Walking Speed
            f32 player_speed = 10.0; // meters/second^2
            if (controller->back.ended_down) {
                player_speed *= 3.0;
            }
            dd_player = dd_player * player_speed;

            dd_player = dd_player + (-game_state->d_player_p) * 3.5;

            TileMapPosition new_player_p = game_state->player_p;
            // 1/2 * a * dt + v * dt + p
            new_player_p.offset = dd_player * 0.5 * square(input->dt_for_frame) 
                + game_state->d_player_p * input->dt_for_frame 
                + new_player_p.offset;

            game_state->d_player_p = dd_player * input->dt_for_frame + game_state->d_player_p;

            new_player_p = recanonicalize_position(tile_map, new_player_p);

            TileMapPosition player_left = new_player_p;
            player_left.offset.x -= player_width * 0.5;
            player_left = recanonicalize_position(tile_map, player_left);

            TileMapPosition player_right = new_player_p;
            player_right.offset.x += player_width * 0.5;
            player_right = recanonicalize_position(tile_map, player_right);

            b32 collided = false;
            TileMapPosition col_p = {};
            
            if (!is_tile_map_point_empty(tile_map, new_player_p)) {
                col_p = new_player_p;
                collided = true;
            }

            if (!is_tile_map_point_empty(tile_map, player_left)) {
                col_p = player_left;
                collided = true;
            }

            if (!is_tile_map_point_empty(tile_map, player_right)) {
                col_p = player_right;
                collided = true;
            }

            if (collided) {
                Vec2 r = {};

                if (col_p.abs_tile_x < game_state->player_p.abs_tile_x) {
                    r = vec2(1.0, 0.0);
                }
                if (col_p.abs_tile_x > game_state->player_p.abs_tile_x) {
                    r = vec2(-1.0, 0.0);
                }
                if (col_p.abs_tile_y < game_state->player_p.abs_tile_y) {
                    r = vec2(0.0, 1.0);
                }
                if (col_p.abs_tile_y > game_state->player_p.abs_tile_y) {
                    r = vec2(0.0, -1.0);
                }
                game_state->d_player_p = game_state->d_player_p - r * vec2_dot(game_state->d_player_p, r) * 1.0;
            }
            else {
                if (!are_on_same_tile(&game_state->player_p, &new_player_p)) {
                    u32 new_tile_value = get_tile_value_from_map_pos(tile_map, new_player_p);
                    if (new_tile_value == 3) {
                        new_player_p.abs_tile_z += 1;
                    }
                    else if (new_tile_value == 4) {
                        new_player_p.abs_tile_z -= 1;
                    }
                }
                game_state->player_p = new_player_p;
                game_state->camera_p = new_player_p;
            }
        }
    }

    //Clear Screen
    draw_rectangle(
        buffer, 
        vec2(0.0, 0.0),
        vec2((f32)buffer->width, (f32)buffer->height),
        1.0,
        0.0,
        1.0
    );

    // Draw background
    // draw_bitmap(buffer, &game_state->backdrop, 0.0, 0.0, 0, 0);

    // Draw screen
    f32 screen_center_x = 0.5 * (f32)buffer->width;
    f32 screen_center_y = 0.5 * (f32)buffer->height;

    for (i32 rel_row = -100; rel_row < 100; rel_row++) {
        for (i32 rel_column = -200; rel_column < 200; rel_column++) {
            u32 column = game_state->player_p.abs_tile_x + rel_column;
            u32 row = game_state->camera_p.abs_tile_y + rel_row;
            
            u32 tile_id = get_tile_value(tile_map, column, row, game_state->camera_p.abs_tile_z);

            if (tile_id > 1) {
                f32 gray = 0.5;

                if (tile_id == 2) {
                    gray = 1.0;
                }

                if (tile_id > 2) {
                    gray = 0.25;
                }

                if (column == game_state->camera_p.abs_tile_x && row == game_state->camera_p.abs_tile_y) {
                    gray = 0.0;
                }

                Vec2 tile_side = vec2(0.5 * (f32)tile_side_in_pixels, 0.5 * (f32)tile_side_in_pixels);

                Vec2 cen = vec2(screen_center_x - meters_to_pixels * game_state->camera_p.offset.x + (f32)(rel_column * tile_side_in_pixels),
                                screen_center_y + meters_to_pixels * game_state->camera_p.offset.y - (f32)(rel_row * tile_side_in_pixels));
                
                Vec2 min = cen - tile_side;
                Vec2 max = cen + tile_side;
                
                draw_rectangle(buffer, min, max, gray, gray, gray);
            }
        }
    }

    // camera stuff that we might need? for now just scroll the screen.
    // Vec2 diff = subtract(tile_map, &game_state.player_p, &game_state.camera_p);

    f32 player_ground_point_x = screen_center_x;
    f32 player_ground_point_y = screen_center_y;

    Vec2 player_left_top = vec2(player_ground_point_x - 0.5 * player_width * meters_to_pixels,
                                player_ground_point_y - player_height * meters_to_pixels);
        
    Vec2 player_width_height = vec2(player_width, player_height);

    // Draw player
    draw_rectangle(buffer, player_left_top, player_left_top + meters_to_pixels * player_width_height, 1.0, 0.0, 0.0);

    HeroBitmaps *hero_bitmaps = &game_state->hero_bitmaps[game_state->hero_facing_direction];

    draw_bitmap(buffer, &hero_bitmaps->whole_player, 
                player_ground_point_x, player_ground_point_y, 
                hero_bitmaps->align_x, hero_bitmaps->align_y);
        
    draw_bitmap(buffer, &hero_bitmaps->cape, 
                player_ground_point_x, player_ground_point_y,
                hero_bitmaps->align_x, hero_bitmaps->align_y);
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
