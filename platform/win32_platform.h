#ifndef WIN32_PLATFORM_H
#define WIN32_PLATFORM_H

typedef struct Win32DebugTimeMarker Win32DebugTimeMarker;
struct Win32DebugTimeMarker {
    u32 output_play_cursor;
    u32 output_write_cursor;
    u32 output_location;
    u32 output_byte_count;
    u32 expected_flip_play_cursor;
    
    u32 flip_play_cursor;
    u32 flip_write_cursor;
};

// Replay
typedef struct Win32ReplayBuffer Win32ReplayBuffer;
struct Win32ReplayBuffer {
    String8 replay_file_name;
    void *memory_block;
};

typedef struct Win32State Win32State;
struct Win32State {
    u64 total_size;
    void *game_memory_block;
    Win32ReplayBuffer replay_buffers[4]; 

    HANDLE recording_handle;
    u64 input_recording_index;

    HANDLE playback_handle;
    u64 input_playing_index;

    String8 exe_dir_path;
    String8 exe_name;
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
struct Win32GameCode {
    HMODULE game_code_dll;
    FILETIME dll_last_write_time;
    GameGetSoundSamples *get_sound_samples;
    GameUpdateAndRender *update_and_render;
    b32 is_valid;
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

#endif // WIN32_PLATFORM_H