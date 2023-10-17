use windows::{core::*, Win32::UI::WindowsAndMessaging::*};

fn main() -> Result<()> {
    unsafe {
        MessageBoxA(None, s!("Ansi"), s!("Caption"), MB_OK);
    }

    Ok(())
}
