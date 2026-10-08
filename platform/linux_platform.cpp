#include <X11/Xlib.h>
#include <../base/base_include.h>

b32 GLOBAL_RUNNING = true;

int main(int argc, char **argv) {
    Display *display = XOpenDisplay(NULL);

    Window window = XCreateSimpleWindow(
        display,
        XDefaultRootWindow(display),
        0, 0,
        800, 600,
        0,
        0x00000000,
        0x00000000
    );

    XStoreName(display, window, "WindowTitle");

    XSelectInput(display, window, KeyPressMask | KeyReleaseMask);

    XMapWindow(display, window);

    while(GLOBAL_RUNNING) {
        while(XPending(display) > 0) {
            XEvent event = {};
            XNextEvent(display, &event);
            if (event.type == KeyPress) {
                GLOBAL_RUNNING = false;
            }
        }
    }
}