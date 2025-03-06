#include <stdint.h>

#if SLOW
#define assert(expr) if (!(expr)) {*(int *)0 = 0;}
#endif // SLOW

typedef int8_t   i8;
typedef int16_t  i16;
typedef int32_t  i32;
typedef int64_t  i64;
typedef uint8_t  u8;
typedef uint16_t u16;
typedef uint32_t u32;
typedef uint64_t u64;
typedef float    f32;
typedef double   f64;

typedef int32_t bool32;
#define false 0
#define true 1

#define KB(n) ((n) * 1024LL)
#define MB(n) ((KB(n)) * 1024LL)
#define GB(n) ((MB(n)) * 1024LL)
#define TB(n) ((GB(n)) * 1024LL)

#define memory_zero(s, n) (memset(s, 0, n))
#define array_count(arr) ((sizeof(arr)) / (sizeof((arr[0]))))

// Game Input
typedef struct GameButtonState GameButtonState;
struct GameButtonState {
    i32 half_transition_count;
    bool32 ended_down;
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
    bool32 is_analog;
    bool32 is_connected;
    
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

typedef struct GameMemory GameMemory;
struct GameMemory {
    bool32 is_initialized;

    u64 permanent_storage_size;
    void *permanent_storage; // Required to be cleared to zero at startup

    u64 transient_storage_size;
    void *transient_storage; // Required to be cleared to zero at startup

    // debug_platform_read_entire_file: DebugPlatformReadEntireFile,
    // debug_platform_free_file_memory: DebugPlatformFreeFileMemory,
    // debug_platform_write_entire_file: DebugPlatformWriteEntireFile,
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

typedef void (*GameGetSoundSample)(ThreadContext *thread, GameMemory *memory, GameSoundOutputBuffer *sound_buffer);
typedef void (*GameUpdateAndRender)(ThreadContext *thread, GameOffscreenBuffer *buffer, GameInput* input, GameMemory* memory);

void render_gradient(GameOffscreenBuffer *buffer, i32 blue_offset, i32 green_offset);

GameControllerInput *get_controller(GameInput *input, u64 controller_index);