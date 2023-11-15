mod gaming_engine;
mod macros;
mod win32_platform;

use win32_platform::win32_main;
fn main() {
    #[cfg(target_os = "windows")]
    win32_main().unwrap_or_else(|e| {
        dbg!(e);
    });
}
