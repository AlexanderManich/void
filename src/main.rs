mod macros;
mod win32_platform;

use win32_platform::*;
fn main() {
    #[cfg(target_os = "windows")]
    win32_main().unwrap_or_else(|e| {
        dbg!(e);
    });
    #[cfg(target_os = "linux")]
    linux()
}

fn linux() {
    print!("Hi from linux");
}
