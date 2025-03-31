#define UNICODE
#define _UNICODE

#include <stdio.h>
#include <intrin.h>
#include <windows.h>
#include <dsound.h>
#include "../base/base_include.h"
#include "../intrinsics.h"
#include "../game/tile.h"
#include "../game/game.h"
#include "win32_platform.h"

#include "../base/base_include.c"

b32 GLOBAL_RUNNING = true;
b32 GLOBAL_PAUSE = false;
i64 PERF_COUNTER_FREQ;
Win32OffscreenBuffer GLOBAL_BACKBUFFER;
LPDIRECTSOUNDBUFFER GLOBAL_SECONDARY_BUFFER;

Win32WindowDimension
win32_get_window_dimension(HWND window) {
    Win32WindowDimension result;

    RECT client_rect = {};
    GetClientRect(window, &client_rect);
    result.width  = client_rect.right - client_rect.left;
    result.height = client_rect.bottom - client_rect.top;

    return result;
}

WINDOWPLACEMENT g_wpPrev = { sizeof(g_wpPrev) };
void toggle_fullscreen(HWND window) {
    i32 dw_style = GetWindowLongW(window, GWL_STYLE);
    if (dw_style & WS_OVERLAPPEDWINDOW) {
        MONITORINFO mi = { sizeof(mi)};
        if (GetWindowPlacement(window, &g_wpPrev) && GetMonitorInfo(MonitorFromWindow(window, MONITOR_DEFAULTTOPRIMARY), &mi)) {
            SetWindowLong(window, GWL_STYLE, dw_style & ~WS_OVERLAPPEDWINDOW);
            SetWindowPos(window, HWND_TOP,
                         mi.rcMonitor.left, mi.rcMonitor.top,
                         mi.rcMonitor.right - mi.rcMonitor.left,
                         mi.rcMonitor.bottom - mi.rcMonitor.top,
                         SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
        }
    }
    else {
        SetWindowLong(window, GWL_STYLE, dw_style | WS_OVERLAPPEDWINDOW);
        SetWindowPlacement(window, &g_wpPrev);
        SetWindowPos(window, NULL, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED);
    }
}

#define DIRECT_SOUND_CREATE(name) HRESULT WINAPI name(LPCGUID pcGuidDevice, LPDIRECTSOUND *ppDS, LPUNKNOWN pUnkOuter)
typedef DIRECT_SOUND_CREATE(direct_sound_create);

void
win32_init_d_sound(HWND window, i32 samples_per_second, u32 buffer_size) {
    HMODULE d_sound_lib = LoadLibraryA("dsound.dll");
    if (d_sound_lib) {
        direct_sound_create *DirectSoundCreate = (direct_sound_create *)GetProcAddress(d_sound_lib, "DirectSoundCreate");

        LPDIRECTSOUND direct_sound;
        if (DirectSoundCreate && SUCCEEDED(DirectSoundCreate(0, &direct_sound, 0))) {
            WAVEFORMATEX wave_format = {};
            wave_format.wFormatTag = WAVE_FORMAT_PCM;
            wave_format.nChannels = 2;
            wave_format.nSamplesPerSec = samples_per_second;
            wave_format.wBitsPerSample = 16; 
            wave_format.nBlockAlign = (wave_format.nChannels * wave_format.wBitsPerSample) / 8;
            wave_format.nAvgBytesPerSec = wave_format.nBlockAlign * wave_format.nSamplesPerSec;
            wave_format.cbSize = 0;
            if (SUCCEEDED(direct_sound->SetCooperativeLevel(window, DSSCL_PRIORITY))) {
                DSBUFFERDESC buffer_description = {};
                buffer_description.dwSize = sizeof(buffer_description);
                buffer_description.dwFlags = DSBCAPS_PRIMARYBUFFER;
                
                LPDIRECTSOUNDBUFFER primary_buffer;

                if (SUCCEEDED(direct_sound->CreateSoundBuffer(&buffer_description, &primary_buffer, 0))) {
                    if (SUCCEEDED(primary_buffer->SetFormat(&wave_format))) {

                        DSBUFFERDESC buffer_description = {};
                        buffer_description.dwSize = sizeof(buffer_description);
                        buffer_description.dwFlags = DSBCAPS_GETCURRENTPOSITION2;
                        buffer_description.dwBufferBytes = buffer_size;
                        buffer_description.lpwfxFormat = &wave_format;

                        if (SUCCEEDED(direct_sound->CreateSoundBuffer(&buffer_description, &GLOBAL_SECONDARY_BUFFER, 0))) {
                        }
                    }
                }
            }
        }
    }
}

void
win32_clear_sound_buffer(Win32SoundOutput *sound_output) {
    void *region1;
    DWORD region1_size = 0;
    void *region2;
    DWORD region2_size = 0;

    GLOBAL_SECONDARY_BUFFER->Lock(0, sound_output->secondary_buffer_size, &region1, &region1_size, &region2, &region2_size, 0);
    
    u8 *dest_sample = (u8 *)region1;
    for (u64 i = 0; i < region1_size; i++) {
        *dest_sample = 0;
        dest_sample += 1;
    }

    dest_sample = (u8 *)region2;
    for (u64 i = 0; i < region2_size; i++) {
        *dest_sample = 0;
        dest_sample += 1;
    }

    GLOBAL_SECONDARY_BUFFER->Unlock(region1, region1_size, region2, region2_size);
}

void
win32_fill_sound_buffer(Win32SoundOutput *sound_output, u32 byte_to_lock, u32 bytes_to_write, GameSoundOutputBuffer source_buffer) {
    void* region1;
    DWORD region1_size = 0;
    void *region2;
    DWORD region2_size = 0;
    
    GLOBAL_SECONDARY_BUFFER->Lock(byte_to_lock, bytes_to_write, &region1, &region1_size, &region2, &region2_size, 0); 

    u32 region1_sample_count = region1_size / sound_output->bytes_per_sample; 
    i16 *dest_sample = (i16 *)region1;
    i16 *source_sample = source_buffer.samples;

    for (u32 i = 0; i < region1_sample_count; i++) {
        *dest_sample = *source_sample;
        source_sample += 1;
        dest_sample += 1; 
        *dest_sample = *source_sample;
        source_sample += 1;
        dest_sample += 1; 
        sound_output->running_sample_index++;
    }

    dest_sample = (i16 *)region2;
    u32 region2_sample_count = region2_size / sound_output->bytes_per_sample;

    for (u32 i = 0; i < region2_sample_count; i++) {
        *dest_sample = *source_sample;
        source_sample = dest_sample + 1;
        dest_sample = source_sample + 1;
        *dest_sample = *source_sample;
        source_sample = dest_sample + 1;
        dest_sample = source_sample + 1;
        sound_output->running_sample_index++;
    }

    GLOBAL_SECONDARY_BUFFER->Unlock(region1, region1_size, region2, region2_size);
}
FILETIME 
win32_get_last_write_time(String8 file_name) {
    WIN32_FILE_ATTRIBUTE_DATA data = {};
    GetFileAttributesExA((char *)file_name.str, GetFileExInfoStandard, &data);
    FILETIME last_write_time = data.ftLastWriteTime;
    return last_write_time;
}

Win32GameCode 
win32_load_game_code(String8 source_dll_name, String8 temp_dll_name) {
    Win32GameCode result = {};

    result.dll_last_write_time = win32_get_last_write_time(source_dll_name);
    CopyFileA((char *)source_dll_name.str, (char *)temp_dll_name.str, false);
    result.game_code_dll = LoadLibraryA((char *)temp_dll_name.str);

    if (result.game_code_dll) {
        result.update_and_render = (GameUpdateAndRender *)GetProcAddress(result.game_code_dll, "game_update_and_render");
        result.get_sound_samples = (GameGetSoundSamples *)GetProcAddress(result.game_code_dll, "game_get_sound_samples");
        result.is_valid = (result.update_and_render && result.get_sound_samples);
    }

    return result;
}

void
win32_unload_game_code(Win32GameCode *game_code) {
    if (game_code->game_code_dll) {
        FreeLibrary(game_code->game_code_dll);
    }

    game_code->game_code_dll = 0;
    game_code->get_sound_samples = 0;
    game_code->update_and_render = 0;
}

void
debug_platform_free_file_memory(ThreadContext *thread, void *memory) {
    if (memory) {
        VirtualFree(memory, 0, MEM_RELEASE);
    }
}

String8
debug_platform_read_entire_file(ThreadContext *thread, String8 file_name) {
    String8 result = {};

    HANDLE file_handle = CreateFileA((char *)file_name.str, GENERIC_READ, FILE_SHARE_READ, 0, OPEN_EXISTING, 0, 0);

    LARGE_INTEGER file_size = {};
    GetFileSizeEx(file_handle, &file_size);
    u32 file_size_u32 = safe_u64_to_u32(file_size.QuadPart);

    result.str = (u8 *)VirtualAlloc(0, file_size_u32, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);

    u8 *read_buffer = result.str;

    if (result.str) {
        DWORD bytes_read = 0;
        if (ReadFile(file_handle, read_buffer, file_size_u32, &bytes_read, 0)) {
            assert(bytes_read == file_size_u32);
            result.size = file_size_u32;
        }
        else {
            debug_platform_free_file_memory(thread, result.str);
            result.str = 0;
        }
    }
    CloseHandle(file_handle);

    return result;
}

void
debug_platform_write_entire_file(ThreadContext *thread, String8 file_name, void *memory, u32 memory_size) {
    HANDLE file_handle = CreateFileA((char *)file_name.str, GENERIC_WRITE, FILE_SHARE_WRITE, 0, CREATE_ALWAYS, 0, 0);

    u8 *write_buffer = (u8 *)memory;

    DWORD bytes_written;
    if(WriteFile(file_handle, write_buffer, memory_size, &bytes_written, 0)) {
        assert(bytes_written == memory_size);
    }
    else {

    }
    
    CloseHandle(file_handle);
}

void
win32_process_keyboard_message(GameButtonState *new_state, b32 is_down) {
    if (new_state->ended_down != is_down) {
        new_state->ended_down = is_down;
        new_state->half_transition_count += 1;
    }
}

u32 
win32_get_wall_clock() {
    LARGE_INTEGER result = {};
    QueryPerformanceCounter(&result);
    return result.QuadPart;
}

f32
win32_get_seconds_elapsed(i64 start, i64 end) {
    f32 result = (f32)(end - start) / PERF_COUNTER_FREQ;
    return result;
}

void
win32_get_exe_file_name(Arena *arena, Win32State *state) {
    u32 size = KB(32);
    u8 *buffer = push_array(arena, u8, size);
    u32 length = GetModuleFileNameA(0, (char *)buffer, size);
    String8 string = str8(buffer, length);
    
    state->exe_name = str8_last_slash_suffix(string);
    state->exe_dir_path = str8_last_slash_prefix(string);
}

// void
// win32_build_exe_path_file_name(Arena *arena, Win32State *state, String8 file_name, String8 *dest) {
    
// }

// void win32_get_input_file_location(Win32State *state, u32 slot_index, String8 *dest) {
//     assert(slot_index == 1);
//     win32_build_exe_path_file_name(state, str8_lit("loop_playback.pb"), dest);
// }

void
win32_get_replay_buffer(Win32State *state, u32 index, Win32ReplayBuffer *replay_buffer) {
    assert(index < array_count(state->replay_buffers));

    Win32ReplayBuffer *buffer = &state->replay_buffers[index];
    replay_buffer->memory_block = buffer->memory_block;
    replay_buffer->replay_file_name = buffer->replay_file_name;
}

void
win32_record_input(Win32State *win32_state, GameInput *new_input) {
    u32 bytes_written = 0;
    u32 memory_size = sizeof(GameInput);
    u8 *write_buffer = (u8 *)new_input;
    
    if (WriteFile(win32_state->recording_handle, write_buffer, memory_size, (LPDWORD)(&bytes_written), 0)) {
        assert(bytes_written == memory_size);
    }
    else {
        // Logging
    }
}

void
win32_begin_recording_input(Win32State *state, u32 input_recording_index) {
    assert(input_recording_index < array_count(state->replay_buffers));

    Win32ReplayBuffer replay_buffer = {};
    win32_get_replay_buffer(state, input_recording_index, &replay_buffer);

    if (replay_buffer.memory_block) {
        state->input_recording_index = input_recording_index;

        String8 file_name = {};
        // win32_get_input_file_location(state, input_recording_index, *file_name);
    }
}

void 
win32_process_pending_messages(Win32State *win32_state, GameControllerInput *keyboard_controller) {
    MSG message = {};
    
    while (PeekMessage(&message, 0, 0, 0, PM_REMOVE)) {
        
        switch (message.message) {
            case WM_QUIT: {
                GLOBAL_RUNNING = false;
                break;
            }
            case WM_SYSKEYDOWN:
            case WM_SYSKEYUP:
            case WM_KEYDOWN:
            case WM_KEYUP: {
                WPARAM vk_code = (u16)message.wParam;
                b32 was_down = (message.lParam & (1 << 30)) != 0;
                b32 is_down = (message.lParam & (1 << 31)) == 0;
                b32 alt_key_was_down = (message.lParam & (1 << 29)) != 0;
                
                if (was_down != is_down) {
                    switch (vk_code) {
                        case 'W': {
                            win32_process_keyboard_message(
                                &keyboard_controller->move_up,
                                is_down
                            );
                            break;
                        }
                        case 'A': {
                            win32_process_keyboard_message(
                                &keyboard_controller->move_left,
                                is_down
                            );
                            break;
                        }
                        case 'S': {
                            win32_process_keyboard_message(
                                &keyboard_controller->move_down,
                                is_down
                            );
                            break;
                        }
                        case 'D': {
                            win32_process_keyboard_message(
                                &keyboard_controller->move_right,
                                is_down
                            );
                            break;
                        }
                        case 'Q': {
                            win32_process_keyboard_message(
                                &keyboard_controller->right_shoulder,
                                is_down
                            );
                            break;
                        }
                        case 'E': {
                            win32_process_keyboard_message(
                                &keyboard_controller->left_shouder,
                                is_down
                            );
                            break;
                        }
                        case 'P': {
                            if (is_down) {
                                GLOBAL_PAUSE = !GLOBAL_PAUSE;
                            }
                            break;
                        }
                        case 'L': {
                            if (is_down) {
                                if (win32_state->input_playing_index == 0) {
                                    if (win32_state->input_recording_index == 0) {
                                        // win32_begin_recording_input(win32_state, 1);
                                    }
                                }
                            }
                            break;
                        }
                        case VK_ESCAPE: {
                            win32_process_keyboard_message(&keyboard_controller->start, is_down);
                            break;
                        }
                        case VK_SPACE: {
                            win32_process_keyboard_message(&keyboard_controller->back, is_down);
                            break;
                        }
                        case VK_RETURN: {
                            if (is_down && alt_key_was_down) {
                                toggle_fullscreen(message.hwnd);
                            }
                            break;
                        }
                        case VK_F4: {
                            if (alt_key_was_down) {
                                GLOBAL_RUNNING = false;
                            }
                            break;
                        }
                    }
                }
                break;
            }
            default: {
                TranslateMessage(&message);
                DispatchMessage(&message);
            }
        }    
    }
}

void
win32_resize_dib_section(Win32OffscreenBuffer *buffer, i32 width, i32 height) {
    if (buffer->memory) {
        VirtualFree(buffer->memory, 0, MEM_RELEASE);
    }

    buffer->width = width;
    buffer->height = height;
    buffer->bytes_per_pixel = 4;

    buffer->info.bmiHeader.biSize = sizeof(BITMAPINFOHEADER); 
    buffer->info.bmiHeader.biWidth = buffer->width;
    // Negative height means the bitmap is a top-down DIB and it's origin is the upper-left corner. Rows in window go sequentially from top to bottom.
    buffer->info.bmiHeader.biHeight = -buffer->height;
    buffer->info.bmiHeader.biPlanes = 1;
    buffer->info.bmiHeader.biBitCount = 32;
    buffer->info.bmiHeader.biCompression = BI_RGB;

    u64 bitmap_memory_size = ((width * height) * buffer->bytes_per_pixel);
    buffer->memory = VirtualAlloc(
        0,
        bitmap_memory_size,
        MEM_RESERVE | MEM_COMMIT,
        PAGE_READWRITE
    );

    buffer->pitch = (width * buffer->bytes_per_pixel);
}

void
win32_display_buffer_in_window(
    HDC device_context,
    Win32OffscreenBuffer *buffer,
    i32 window_height,
    i32 window_width
) {
        if (window_width >= buffer->width * 2 && window_height >= buffer->height * 2) {
            StretchDIBits(
                device_context,
                0,
                0,
                2 * buffer->width,  //window_width,
                2 * buffer->height, //window_height,
                0,
                0,
                buffer->width,
                buffer->height,
                buffer->memory,
                &buffer->info,
                DIB_RGB_COLORS,
                SRCCOPY
            );
        } 
        else {
            i32 offset_x = 10;
            i32 offset_y = 10;

            PatBlt(device_context, 0, 0, window_width, offset_y, BLACKNESS);
            PatBlt(
                device_context,
                0,
                offset_y + buffer->height,
                window_width,
                window_height,
                BLACKNESS
            );
            PatBlt(device_context, 0, 0, offset_x, window_height, BLACKNESS);
            PatBlt(
                device_context,
                offset_x + buffer->width,
                0,
                window_width,
                window_height,
                BLACKNESS
            );

            //TODO: Aspect ratio correction
            StretchDIBits(
                device_context,
                offset_x,
                offset_y,
                buffer->width,  //window_width,
                buffer->height, //window_height,
                0,
                0,
                buffer->width,
                buffer->height,
                buffer->memory,
                &buffer->info,
                DIB_RGB_COLORS,
                SRCCOPY
            );
        }
}

LRESULT CALLBACK
main_window_callback(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {

    LRESULT result = 0;

    switch (message) {
        case WM_SIZE: {
            break;
        }
        case WM_DESTROY: {
            GLOBAL_RUNNING = false;
            break;
        }
        case WM_CLOSE: {
            GLOBAL_RUNNING = false;
            break;
        }
        case WM_ACTIVATEAPP: {
            // WPARAM is true or false depending on if window is active or not
            // if (wparam == 1) {
            //     SetLayeredWindowAttributes(window, 0, 255, LWA_ALPHA);
            // } 
            // else { 
            //     SetLayeredWindowAttributes(window, 0, 128, LWA_ALPHA);
            // }
            break;
        }
        case WM_PAINT: {
            PAINTSTRUCT paint = {};
            HDC device_context = BeginPaint(window, &paint);

            Win32WindowDimension dimension = win32_get_window_dimension(window);
            win32_display_buffer_in_window(device_context, &GLOBAL_BACKBUFFER, dimension.height, dimension.width);
            EndPaint(window, &paint);
            break;
        }
        default: {
            result = DefWindowProc(window, message, wparam, lparam);
            break;
        }
    }

    return result;
}

int CALLBACK
WinMain(HINSTANCE instance, HINSTANCE prev_instance, LPSTR command_line, int show_code) {

    (void)command_line;
    (void)show_code;

    u64 global_arena_size = MB(1);
    u8 global_arena_buffer[global_arena_size];
    Arena global_arena = {};
    arena_init(&global_arena, global_arena_buffer, global_arena_size);

    Win32State win32_state = {};

    win32_get_exe_file_name(&global_arena, &win32_state);

    String8 source_game_code_dll_full_path = str8_concat(&global_arena, win32_state.exe_dir_path, str8_lit("\\game.dll"));

    String8 temp_game_code_dll_full_path = str8_concat(&global_arena, win32_state.exe_dir_path, str8_lit("\\temp_game.dll"));

    i32 desired_scheduler_ms = 1;
    b32 sleep_is_granular = timeBeginPeriod(desired_scheduler_ms) != TIMERR_NOERROR;

    WNDCLASS window_class = {};

    win32_resize_dib_section(&GLOBAL_BACKBUFFER, 960, 540);
    window_class.style = CS_HREDRAW|CS_VREDRAW;
    window_class.hInstance = instance;
    window_class.lpfnWndProc = main_window_callback;
    window_class.hCursor = LoadCursor(0, IDC_ARROW);
    window_class.lpszClassName = L"VoidWindowClass";

    if (RegisterClass(&window_class) != 0) {
        HWND window = CreateWindowEx(
            0,
            window_class.lpszClassName,
            L"Void",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            window_class.hInstance,
            0
        );
        if (window) {
            HDC refresh_dc = GetDC(window);
            u32 monitor_refresh_hz = 60;
            i32 win32_refresh_rate = GetDeviceCaps(refresh_dc, VREFRESH);
            ReleaseDC(window, refresh_dc);

            if (win32_refresh_rate > 1) {
                monitor_refresh_hz = win32_refresh_rate;
            }
            // TODO: If game_update_hz is too big, sound starts bugging. Ideally It should be monitor_refresh_hz / 2
            f32 game_update_hz = monitor_refresh_hz / 2.0;
            f32 target_seconds_per_frame = 1.0 / game_update_hz;

            Win32SoundOutput sound_output = {};
            sound_output.samples_per_second = 48000;
            sound_output.running_sample_index = 0;
            sound_output.bytes_per_sample = sizeof(i16) * 2;
            sound_output.secondary_buffer_size =
                sound_output.samples_per_second * sound_output.bytes_per_sample;
            sound_output.safety_bytes = (u32)(((f32)sound_output.samples_per_second
                * (f32)sound_output.bytes_per_sample
                / game_update_hz)
                / 3.0);

            win32_init_d_sound(window, sound_output.samples_per_second, sound_output.secondary_buffer_size);

            win32_clear_sound_buffer(&sound_output);

            GLOBAL_SECONDARY_BUFFER->Play(0, 0, DSBPLAY_LOOPING);

            GLOBAL_RUNNING = true;
            
            i16 *samples = (i16 *)VirtualAlloc(0, sound_output.secondary_buffer_size, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE);

            #if INTERNAL
                void *base_address = (void *)TB(2);
            #else
                void *base_address = 0;
            #endif //INTERNAL

            GameMemory game_memory = {};
            game_memory.is_initialized = false;
            game_memory.permanent_storage_size = MB(64);
            game_memory.permanent_storage = 0;
            game_memory.transient_storage_size = GB(1);
            game_memory.transient_storage = 0;
            game_memory.debug_platform_read_entire_file = debug_platform_read_entire_file;
            game_memory.debug_platform_write_entire_file = debug_platform_write_entire_file;
            game_memory.debug_platform_free_file_memory = debug_platform_free_file_memory;

            win32_state.total_size = game_memory.permanent_storage_size + game_memory.transient_storage_size;
            win32_state.game_memory_block = VirtualAlloc(
                base_address,
                win32_state.total_size,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_READWRITE
            );

            game_memory.permanent_storage = win32_state.game_memory_block;
            game_memory.transient_storage = ((u8 *)game_memory.permanent_storage + game_memory.permanent_storage_size);
            for (u64 i = 0; i < array_count(win32_state.replay_buffers); i++) {
                Win32ReplayBuffer replay_buffer = win32_state.replay_buffers[i];
                replay_buffer.memory_block = VirtualAlloc(
                    0,
                    win32_state.total_size,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_READWRITE
                );

                assert(replay_buffer.memory_block);
            }

            if (samples && game_memory.permanent_storage && game_memory.transient_storage) {

                GameInput input[2] = {};
                GameInput *new_input = &input[0];
                GameInput *old_input = &input[1];
                
                u32 last_counter = win32_get_wall_clock();
                u32 flip_wall_clock = win32_get_wall_clock();
               
                u64 debug_time_marker_index = 0;
                Win32DebugTimeMarker debug_time_markers[30];
                
                b32 sound_is_valid = false;
                i32 audio_latency_bytes = 0;
                f32 audio_latency_seconds = 0.0;

                u64 last_cycle_count = _rdtsc();

                Win32GameCode game = win32_load_game_code(
                    source_game_code_dll_full_path,
                    temp_game_code_dll_full_path
                );

                while (GLOBAL_RUNNING) {
                    new_input->dt_for_frame = target_seconds_per_frame;

                    FILETIME new_dll_write_time = {};
                    FILETIME last_write_time = win32_get_last_write_time(source_game_code_dll_full_path);
                    new_dll_write_time = last_write_time;

                    if (CompareFileTime(&new_dll_write_time, &game.dll_last_write_time) != 0) {
                        win32_unload_game_code(&game);
                        game = win32_load_game_code(source_game_code_dll_full_path, temp_game_code_dll_full_path);
                    }

                    GameControllerInput *old_keyboard_controller = get_controller(old_input, 0);
                    GameControllerInput *new_keyboard_controller = get_controller(new_input, 0);
                    memory_zero(new_keyboard_controller, sizeof(GameControllerInput));
                    new_keyboard_controller->is_connected = true; 
                    
                    for (u64 button_index = 0; button_index < array_count(new_keyboard_controller->buttons); button_index++) {
                        new_keyboard_controller->buttons[button_index].ended_down =
                        old_keyboard_controller->buttons[button_index].ended_down;
                    }

                    win32_process_pending_messages(&win32_state, new_keyboard_controller);

                    if (!GLOBAL_PAUSE) {
                        // Mouse input
                        POINT mouse_p = {};
                        GetCursorPos(&mouse_p);
                        
                        ScreenToClient(window, &mouse_p);
                        new_input->mouse_x = mouse_p.x;
                        new_input->mouse_y = mouse_p.y;
                        new_input->mouse_z = 0;
                        
                        for (u64 i = 0; i < array_count(new_input->mouse_buttons); i++) { 
                            win32_process_keyboard_message(
                                &new_input->mouse_buttons[i],
                                (GetKeyState(VK_LBUTTON) & (1 << 15)) != 0
                            );
                        }

                        GameOffscreenBuffer buffer = {};
                        buffer.width = GLOBAL_BACKBUFFER.width;
                        buffer.height = GLOBAL_BACKBUFFER.height;
                        buffer.memory = GLOBAL_BACKBUFFER.memory;
                        buffer.pitch = GLOBAL_BACKBUFFER.pitch;
                        buffer.bytes_per_pixel = GLOBAL_BACKBUFFER.bytes_per_pixel;

                        if (win32_state.input_recording_index == 1) {
                            win32_record_input(&win32_state, new_input);
                        }

                        if (win32_state.input_playing_index == 1) {
                            // win32_playback_input(&win32_state, new_input);
                        }
                        
                        ThreadContext thread = {};

                        if (game.update_and_render) {
                            (*game.update_and_render)(&thread, &buffer, new_input, &game_memory);
                        }

                        u32 audio_wall_clock = win32_get_wall_clock();
                        f32 from_begin_to_audio_seconds = win32_get_seconds_elapsed(flip_wall_clock, audio_wall_clock);

                        DWORD play_cursor = 0;
                        DWORD write_cursor = 0;

                        if (GLOBAL_SECONDARY_BUFFER->GetCurrentPosition(&play_cursor, &write_cursor) != DS_OK) {
                            sound_is_valid = false;
                        }

                        if (!sound_is_valid) {
                            sound_output.running_sample_index = write_cursor / sound_output.bytes_per_sample;
                            sound_is_valid = true;
                        }

                        u32 byte_to_lock = (sound_output.running_sample_index * sound_output.bytes_per_sample) 
                        % sound_output.secondary_buffer_size;

                        u32 expected_sound_bytes_per_frame = (u32)((f32)(sound_output.samples_per_second * sound_output.bytes_per_sample)
                        / game_update_hz);

                        f32 seconds_left_until_flip = target_seconds_per_frame - from_begin_to_audio_seconds;
                        u32 expected_bytes_until_flip = (u32)((seconds_left_until_flip / target_seconds_per_frame)
                        * (f32)expected_sound_bytes_per_frame);

                        u32 expected_frame_boundary_byte = play_cursor + expected_bytes_until_flip;

                        u32 safe_write_cursor = write_cursor;
                        if (safe_write_cursor < play_cursor) {
                            safe_write_cursor += sound_output.secondary_buffer_size;
                        }

                        assert(safe_write_cursor >= play_cursor);
                        safe_write_cursor += sound_output.safety_bytes;

                        b32 audio_card_is_low_latency = safe_write_cursor < expected_frame_boundary_byte;

                        u32 target_cursor;
                        if (audio_card_is_low_latency) {
                            target_cursor = expected_frame_boundary_byte + expected_sound_bytes_per_frame;
                        }
                        else {
                            target_cursor = write_cursor + expected_sound_bytes_per_frame + sound_output.safety_bytes;
                        }

                        target_cursor %= sound_output.secondary_buffer_size;

                        u32 bytes_to_write;
                        if (byte_to_lock > target_cursor) {
                            bytes_to_write = sound_output.secondary_buffer_size - byte_to_lock + target_cursor;
                        }
                        else {
                            bytes_to_write = target_cursor - byte_to_lock;
                        }

                        GameSoundOutputBuffer sound_buffer = {};
                        sound_buffer.samples_per_second = sound_output.samples_per_second;
                        sound_buffer.sample_count = bytes_to_write / sound_output.bytes_per_sample;
                        sound_buffer.samples = samples;

                        if (game.get_sound_samples) {
                            (*game.get_sound_samples)(&thread, &game_memory, &sound_buffer);
                        }

                        #if INTERNAL 
                        {
                            Win32DebugTimeMarker marker = debug_time_markers[debug_time_marker_index];
                            marker.output_play_cursor = play_cursor;
                            marker.output_write_cursor = write_cursor;
                            marker.output_location = byte_to_lock;
                            marker.output_byte_count = bytes_to_write;
                            marker.expected_flip_play_cursor = expected_frame_boundary_byte;
                            
                            u32 unwrapped_write_cursor = write_cursor;
                            if (unwrapped_write_cursor < play_cursor) {
                                unwrapped_write_cursor += sound_output.secondary_buffer_size;
                            }
                            audio_latency_bytes = unwrapped_write_cursor - play_cursor;
                            audio_latency_seconds = ((f32)audio_latency_bytes
                            / (f32)sound_output.bytes_per_sample
                            / (f32)sound_output.samples_per_second);
                            
                            // fprintf(stderr, "pc: {%ld} wc: {%ld} delta: {%d} {%f}",
                            //     play_cursor, write_cursor, audio_latency_bytes, audio_latency_seconds);
                        }
                        #endif // INTERNAL 
                        
                        win32_fill_sound_buffer(&sound_output, byte_to_lock, bytes_to_write, sound_buffer);

                        u32 work_counter = win32_get_wall_clock();
                        f32 work_seconds_elapsed = win32_get_seconds_elapsed(last_counter, work_counter);

                        f32 seconds_elapsed_for_frame = work_seconds_elapsed;

                        if (seconds_elapsed_for_frame < target_seconds_per_frame) {
                            if (sleep_is_granular) {
                                u32 sleep_ms = 1000.0 * (target_seconds_per_frame - seconds_elapsed_for_frame);
                                if (sleep_ms > 0) {
                                    Sleep(sleep_ms);
                                }

                                f32 test_seconds_elapsed_for_frame = win32_get_seconds_elapsed(last_counter, win32_get_wall_clock());

                                if (test_seconds_elapsed_for_frame < target_seconds_per_frame) {
                                    // LOGGING
                                }
                            }

                            while (seconds_elapsed_for_frame < target_seconds_per_frame) {
                                seconds_elapsed_for_frame = win32_get_seconds_elapsed(last_counter, win32_get_wall_clock());
                            }
                        }
                        else {
                            // LOGGING
                        }

                        u32 end_counter = win32_get_wall_clock();
                        f32 ms_per_frame = 1000.0 * win32_get_seconds_elapsed(last_counter, end_counter);
                        last_counter = end_counter;

                        #if INTERNAL 
                        {    
                            // win32_debug_sync_display();
                        }
                        #endif // INTERNAL

                        HDC device_context = GetDC(window);

                        Win32WindowDimension dim = win32_get_window_dimension(window);

                        win32_display_buffer_in_window(device_context, &GLOBAL_BACKBUFFER, dim.height, dim.width);
                        
                        ReleaseDC(window, device_context);

                        u32 flip_wall_clock = win32_get_wall_clock();

                        // TODO: Make function
                        GameInput *temp = new_input;
                        new_input = old_input;
                        old_input = temp;

                        u64 end_cycle_count = _rdtsc();
                        u64 cycles_elapsed = end_cycle_count - last_cycle_count;
                        last_cycle_count = end_cycle_count;

                        f32 fps = 0.0;
                        f32 megac_per_frame = cycles_elapsed / (1000 * 1000);
                        // fprintf(stderr, "%.02f ms/f, %.02f fps, %.02f mc/f\n", ms_per_frame, fps, megac_per_frame);

                        #if INTERNAL
                        {
                            debug_time_marker_index++;
                            if (debug_time_marker_index == array_count(debug_time_markers)) {
                                debug_time_marker_index = 0;
                            }
                        }
                        #endif         
                    }
                }
            }
            else {
                // Samples, Game Memory
            }
        }
        else {
            // Window
        }
    }

    return 0;
}
