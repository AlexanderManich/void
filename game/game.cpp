#include "game.h"

void render_gradient(GameOffscreenBuffer *buffer, i32 blue_offset, i32 green_offset) {
    // Draw bitmap to window.
    u8 *row = (u8 *)buffer->memory;
    for (u64 y = 0; y < buffer->height; y++) {
        u32 *pixel = (u32 *)row;
        for (u64 x = 0; x < buffer->width; x++) {
            u8 blue = (x + blue_offset);
            u8 green = (y + green_offset);

            // Shift green up by 8 bits from bottom, to make them sit in the green channel and not overwrite the blue channel (little endian)
            *pixel++ = ((green << 8) | blue);
        }

        row += buffer->pitch;
    }
}

GameControllerInput *get_controller(GameInput *input, u64 controller_index) {
    // assert!(controller_index < input.controllers.len());
    return &input->controllers[controller_index];
}