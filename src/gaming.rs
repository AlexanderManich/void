pub mod gaming {
    use std::ffi::c_void;
    pub struct GameOffscreenBuffer {
        pub width: i32,
        pub height: i32,
        pub memory: *mut c_void,
        pub pitch: usize,
    }

    impl Default for GameOffscreenBuffer {
        fn default() -> Self {
            unsafe { ::core::mem::zeroed() }
        }
    }

    pub fn render_gradient(buffer: &GameOffscreenBuffer, blue_offset: i32, green_offset: i32) {
        unsafe {
            // Draw bitmap to window.
            let mut row: *mut u8 = buffer.memory.cast();
            for y in 0..buffer.height {
                let mut pixel: *mut u32 = row.cast();
                for x in 0..buffer.width {
                    //truncate to 8 bits using mask, because i couldn't figure out how to cast it :)
                    let blue = (x + blue_offset) & 0xFF;
                    let green = (y + green_offset) & 0xFF;

                    // Shift green up by 8 bits from bottom, to make them sit in the green channel and not overwriting the blue channel (little endian)
                    *pixel = ((green << 8) | blue) as u32;
                    pixel = pixel.add(1);
                }

                row = row.add(buffer.pitch);
            }
        }
    }

    pub fn game_update_and_render(
        buffer: &GameOffscreenBuffer,
        blue_offset: i32,
        green_offset: i32,
    ) {
        render_gradient(buffer, blue_offset, green_offset);
    }
}
