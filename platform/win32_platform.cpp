#define UNICODE
#define _UNICODE

#include <windows.h>
#include "win32_platform.h"

bool32 GLOBAL_RUNNING = true;
Win32OffScreenBuffer GLOBAL_BACKBUFFER;

Win32WindowDimension
win32_get_window_dimension(HWND window) {
    Win32WindowDimension result;

    RECT client_rect = {};
    GetClientRect(window, &client_rect);
    result.width  = client_rect.right - client_rect.left;
    result.height = client_rect.bottom - client_rect.top;

    return result;
}

void
win32_process_keyboard_message(GameButtonState *new_state, bool32 is_down) {
    if (new_state->ended_down != is_down) {
        new_state->ended_down = is_down;
        new_state->half_transition_count += 1;
    }
}

void 
win32_process_pending_messages(Win32State win32_state, GameControllerInput keyboard_controller) {
    MSG message = {};

    while (PeekMessage(&message, 0, 0, 0, PM_REMOVE)) {
        switch (message.message) {
            case WM_QUIT: {
                GLOBAL_RUNNING = false;
                break;
            }
            case WM_SYSKEYDOWN | WM_SYSKEYUP | WM_KEYDOWN | WM_KEYUP: {
                WPARAM vk_code = message.wParam;
                bool32 was_down = (message.lParam & (1 << 30)) != 0;
                bool32 is_down = (message.lParam & (1 << 31)) == 0;
                bool32 alt_key_was_down = (message.lParam & (1 << 29)) != 0;
                
                if (was_down != is_down) {
                    switch (vk_code) {
                        case 'W': {
                            win32_process_keyboard_message(
                                &keyboard_controller.concrete_buttons.move_up,
                                is_down
                            );
                            break;
                        }
                        case 'A': {
                            win32_process_keyboard_message(
                                &keyboard_controller.concrete_buttons.move_left,
                                is_down
                            );
                            break;
                        }
                        case 'S': {
                            win32_process_keyboard_message(
                                &keyboard_controller.concrete_buttons.move_down,
                                is_down
                            );
                            break;
                        }
                        case 'D': {
                            win32_process_keyboard_message(
                                &keyboard_controller.concrete_buttons.move_down,
                                is_down
                            );
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
win32_resize_dib_section(Win32OffScreenBuffer *buffer, i32 width, i32 height) {
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
    Win32OffScreenBuffer *buffer,
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
        }
        default: {
            result = DefWindowProc(window, message, wparam, lparam);
        }
    }

    return result;
}

int CALLBACK
WinMain(HINSTANCE instance, HINSTANCE prev_instance, LPSTR command_line, int show_code) {

    WNDCLASS window_class = {};

    win32_resize_dib_section(&GLOBAL_BACKBUFFER, 960, 540);
    window_class.style = CS_HREDRAW|CS_VREDRAW;
    window_class.hInstance = instance;
    window_class.lpfnWndProc = main_window_callback;
    window_class.hCursor = LoadCursor(0, IDC_ARROW);
    window_class.lpszClassName = L"VoidWindowClass";

    if (RegisterClass(&window_class) != 0) {
        HWND window_handle = CreateWindowEx(
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
        if (window_handle) {
            MSG message;
            while (1) {
                HDC refresh_dc = GetDC(window_handle);
                u32 monitor_refresh_hz = 60;
                i32 win32_refresh_rate = GetDeviceCaps(refresh_dc, VREFRESH);
                ReleaseDC(window_handle, refresh_dc);

                if (win32_refresh_rate > 1) {
                    monitor_refresh_hz = win32_refresh_rate;
                }

                GLOBAL_RUNNING = true;

                // win32_process_pending_messages(win32_state, new_keyboard_controller);
            }
        }
        else {

        }
    }
    else {

    }

    return 0;
}
