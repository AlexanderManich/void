#include "void.h"

typedef struct Win32State Win32State;
struct Win32State {

};

typedef struct Win32OffScreenBuffer Win32OffScreenBuffer;
struct Win32OffScreenBuffer {
    i32 width;
    i32 height;
    void *memory;
    BITMAPINFO info;
    u64 pitch;
    i32 bytes_per_pixel;
};

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

typedef struct Win32WindowDimension Win32WindowDimension;
struct Win32WindowDimension {
    i32 width;
    i32 height;
};