#ifndef VOID_H 
#define VOID_H

#include "lib_base.h"
#include "lib_string.h"

// Game Input
typedef struct GameButtonState GameButtonState;
struct GameButtonState {
    i32 half_transition_count;
    b32 ended_down;
};

typedef struct ConcreteGameButtonState ConcreteGameButtonState;
struct ConcreteGameButtonState {
    GameButtonState move_up;
    GameButtonState move_down;
    GameButtonState move_left;
    GameButtonState move_right;

    GameButtonState action_up;
    GameButtonState action_down;
    GameButtonState action_left;
    GameButtonState action_right;

    GameButtonState left_shouder;
    GameButtonState right_shoulder;
    
    GameButtonState back;
    GameButtonState start;
};

typedef struct GameControllerInput GameControllerInput;
struct GameControllerInput {
    b32 is_analog;
    b32 is_connected;
    
    f32 stick_average_x;
    f32 stick_average_y;

    union {
        GameButtonState buttons[12];
        ConcreteGameButtonState concrete_buttons;
    };
};

typedef struct GameInput GameInput;
struct GameInput {
    GameButtonState mouse_buttons[5];
    i32 mouse_x;
    i32 mouse_y;
    i32 mouse_z;

    GameControllerInput controllers[5];

    f32 dt_for_frame;
};

typedef struct ThreadContext ThreadContext;
struct ThreadContext {
    i32 placeholder;
};

typedef String8 (*DebugPlatformReadEntireFile)(ThreadContext *thread, String8 file_name); 
typedef void (*DebugPlatformWriteEntireFile)(ThreadContext *thread, String8 file_name, void *memory, u32 memory_size); 
typedef void (*DebugPlatformFreeFileMemory)(ThreadContext *thread, void *memory); 

typedef struct GameMemory GameMemory;
struct GameMemory {
    b32 is_initialized;

    u64 permanent_storage_size;
    void *permanent_storage; // Required to be cleared to zero at startup

    u64 transient_storage_size;
    void *transient_storage; // Required to be cleared to zero at startup

    DebugPlatformReadEntireFile debug_platform_read_entire_file;
    DebugPlatformFreeFileMemory debug_platform_free_file_memory;
    DebugPlatformWriteEntireFile debug_platform_write_entire_file;
};


typedef struct GameSoundOutputBuffer GameSoundOutputBuffer;
struct GameSoundOutputBuffer {
    u32 samples_per_second;
    u32 sample_count;
    i16 *samples;
};

typedef struct GameOffscreenBuffer GameOffscreenBuffer;
struct GameOffscreenBuffer {
    i32 width;
    i32 height;
    void *memory;
    u64 pitch;
    i32 bytes_per_pixel;
};

typedef void (*GameGetSoundSamples)(ThreadContext *thread, GameMemory *memory, GameSoundOutputBuffer *sound_buffer);
typedef void (*GameUpdateAndRender)(ThreadContext *thread, GameOffscreenBuffer *buffer, GameInput* input, GameMemory* memory);

void render_gradient(GameOffscreenBuffer *buffer, i32 blue_offset, i32 green_offset);

GameControllerInput *get_controller(GameInput *input, u64 controller_index);

#endif // VOID_H