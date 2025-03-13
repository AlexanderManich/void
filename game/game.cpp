#include "../base/base_include.h"
#include "game.h"

void
game_update_and_render(ThreadContext *thread, GameOffscreenBuffer *buffer, GameInput *input, GameMemory *memory) {
    // TODO: assert len(input.controllers[0].button_state.buttons) ==
    // assert();

    assert(sizeof(GameState))
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
