#include "void.h"

typedef struct Win32State Win32State;
struct Win32State {

};

typedef struct Win32OffscreenBuffer Win32OffscreenBuffer;
struct Win32OffscreenBuffer {
    i32 width;
    i32 height;
    void *memory;
    BITMAPINFO info;
    u64 pitch;
    i32 bytes_per_pixel;
};


typedef struct Win32WindowDimension Win32WindowDimension;
struct Win32WindowDimension {
    i32 width;
    i32 height;
};

typedef struct Win32GameCode Win32GameCode;
typedef struct Win32GameCode {
    HMODULE game_code_dll;
    FILETIME dll_last_write_time;
    GameGetSoundSample get_sound_samples;
    GameUpdateAndRender update_and_render;
};

// Sound
typedef struct Win32SoundOutput Win32SoundOutput;
struct Win32SoundOutput {
    u32 samples_per_second;
    u32 running_sample_index;
    u32 bytes_per_sample;
    u32 secondary_buffer_size;
    u32 latency_sample_count;
    u32 safety_bytes;
};