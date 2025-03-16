#include "../base/base_include.h"
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
    u32 alpha_mask = !(red_mask | green_mask | blue_mask);


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
