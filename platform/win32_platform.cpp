#define UNICODE
#define _UNICODE

#include <stdio.h>
#include <windows.h>
#include "win32_platform.h"
#include <intrin.h>

bool32 GLOBAL_RUNNING = true;
Win32OffscreenBuffer GLOBAL_BACKBUFFER = {};

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
        MONITORINFO mi = { sizeof(mi) };
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

void
win32_process_keyboard_message(GameButtonState *new_state, bool32 is_down) {
    if (new_state->ended_down != is_down) {
        new_state->ended_down = is_down;
        new_state->half_transition_count += 1;
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
                bool32 was_down = (message.lParam & (1 << 30)) != 0;
                bool32 is_down = (message.lParam & (1 << 31)) == 0;
                bool32 alt_key_was_down = (message.lParam & (1 << 29)) != 0;
                
                if (was_down != is_down) {
                    switch (vk_code) {
                        case 'W': {
                            fprintf(stderr, "W");
                            win32_process_keyboard_message(
                                &keyboard_controller->concrete_buttons.move_up,
                                is_down
                            );
                            break;
                        }
                        case 'A': {
                            win32_process_keyboard_message(
                                &keyboard_controller->concrete_buttons.move_left,
                                is_down
                            );
                            break;
                        }
                        case 'S': {
                            win32_process_keyboard_message(
                                &keyboard_controller->concrete_buttons.move_down,
                                is_down
                            );
                            break;
                        }
                        case 'D': {
                            win32_process_keyboard_message(
                                &keyboard_controller->concrete_buttons.move_down,
                                is_down
                            );
                            break;
                        }
                        case VK_RETURN: {
                            OutputDebugString(L"ENTER");
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
            OutputDebugString(L"WM_SIZE\n");
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
            OutputDebugString(L"WM_ACTIVATE_APP\n");
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

    Win32State win32_state = {};

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
            // sound_output.safety_bytes = ((sound_output.samples_per_second as f32
            //     * sound_output.bytes_per_sample as f32
            //     / game_update_hz)
            //     / 3.0) as u32;

            GLOBAL_RUNNING = true;
            
            #if INTERNAL
            void *base_address = (void *)TB(2);
            #else
            void *base_address = 0;
            #endif //INTERNAL
            
            GameMemory game_memory = {
                .is_initialized = false,
                .permanent_storage_size = MB(64),
                .permanent_storage = 0,
                .transient_storage_size = GB(1),
                .transient_storage = 0,
            };

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

            if (game_memory.permanent_storage && game_memory.transient_storage) {

                GameInput input[2] = {};
                GameInput *new_input = &input[0];
                GameInput *old_input = &input[1];
                
                // i64 last_counter = win32_get_wall_clock();
                // i64 flip_wall_clock = win32_get_wall_clock();
               
                u64 debug_time_marker_index = 0;
                // Win32DebugTimeMarker debug_time_markers[30];
                
                bool32 sound_is_valid = false;
                i32 audio_latency_bytes = 0;
                f32 autio_latency_seconds = 0.0;

                u64 last_cycle_count = _rdtsc();

                // Win32GameCode game = win32_load_game_code(
                //     source_game_code_dll_full_path,
                //     temp_game_code_dll_full_path
                // )


                i32 xoffset = 0;
                i32 yoffset = 0;
                while (GLOBAL_RUNNING) {
                    new_input->dt_for_frame = target_seconds_per_frame;

                    FILETIME new_dll_write_time = {};
                    // FILETIME last_write_time = win32_get_last_write_time(source_game_code_dll_full_path);
                    // if (last_write_time) {
                    //     new_dll_write_time = last_write_time;
                    // }

                    GameControllerInput *old_keyboard_controller = get_controller(old_input, 0);
                    GameControllerInput *new_keyboard_controller = get_controller(new_input, 0);
                    memory_zero(new_keyboard_controller, sizeof(GameControllerInput));
                    new_keyboard_controller->is_connected = true; 
                    
                    GameOffscreenBuffer buffer = {
                        .width = GLOBAL_BACKBUFFER.width,
                        .height = GLOBAL_BACKBUFFER.height,
                        .memory = GLOBAL_BACKBUFFER.memory,
                        .pitch = GLOBAL_BACKBUFFER.pitch,
                        .bytes_per_pixel = GLOBAL_BACKBUFFER.bytes_per_pixel,
                    };
                    
                    render_gradient(&buffer, xoffset, yoffset);
                    
                    HDC device_context = GetDC(window);
                    
                    Win32WindowDimension dim = win32_get_window_dimension(window);
                    win32_display_buffer_in_window(device_context, &GLOBAL_BACKBUFFER, dim.height, dim.width);
                    
                    ReleaseDC(window, device_context);
                    
                    xoffset -= 1;
                    yoffset -= 1;
                    
                    win32_process_pending_messages(&win32_state, new_keyboard_controller);
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
