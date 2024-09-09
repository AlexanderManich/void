mod macros;

#[cfg(target_os = "windows")]
mod win32_platform;

fn main() {
    #[cfg(target_os = "windows")]
    {
        use win32_platform::win32_main;
        win32_main().unwrap_or_else(|e| {
            dbg!(e);
        });
    }
    #[cfg(target_os = "linux")]
    println!("Hello From Linux");
}
