#ifndef GAME_H
#define GAME_H



typedef struct GameState GameState;
struct GameState {
    Arena world_arena;
    // World arena;

    // TileMapPosition camera_p;
    // TileMapPosition player_p;
    // Vec2 d_player_p;

    // Loaded_Bitmap backdrop;

    u64 hero_facing_direction;
    // HeroBitMaps hero_bitmaps[6];
};

// Game Input
typedef struct GameButtonState GameButtonState;
struct GameButtonState {
    i32 half_transition_count;
    b32 ended_down;
};

typedef struct GameControllerInput GameControllerInput;
struct GameControllerInput {
    b32 is_analog;
    b32 is_connected;
    
    f32 stick_average_x;
    f32 stick_average_y;

    union {
        GameButtonState buttons[12];
        struct {
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

typedef struct GameOffscreenBuffer GameOffscreenBuffer;
struct GameOffscreenBuffer {
    i32 width;
    i32 height;
    void *memory;
    u64 pitch;
    i32 bytes_per_pixel;
};

typedef struct GameSoundOutputBuffer GameSoundOutputBuffer;
struct GameSoundOutputBuffer {
    u32 samples_per_second;
    u32 sample_count;
    i16 *samples;
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


// #define GAME_UPDATE_AND_RENDER(name) void name(ThreadContext *thread, GameOffscreenBuffer *buffer, GameInput *input, GameMemory *memory)
// typedef GAME_UPDATE_AND_RENDER(GameUpdateAndRender);

// #define GAME_GET_SOUND_SAMPLES(name) void name(ThreadContext *thread, GameMemory *memory, GameSoundOutputBuffer *sound_buffer)
// typedef GAME_GET_SOUND_SAMPLES(GameGetSoundSamples);

typedef void (*GameGetSoundSamples)(ThreadContext *thread, GameMemory *memory, GameSoundOutputBuffer *sound_buffer);
typedef void (*GameUpdateAndRender)(ThreadContext *thread, GameOffscreenBuffer *buffer, GameInput* input, GameMemory* memory);

void render_gradient(GameOffscreenBuffer *buffer, i32 blue_offset, i32 green_offset);

inline GameControllerInput *get_controller(GameInput *input, u64 controller_index) {
    assert(controller_index < array_count(input->controllers));
    return &input->controllers[controller_index];
}

#endif // GAME_H