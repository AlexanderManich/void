pub mod gaming {
    use std::{f32::consts::PI, ffi::c_void};
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

    pub struct GameSoundOutputBuffer {
        pub samples_per_second: u32,
        pub sample_count: u32,
        pub samples: *mut i16,
    }

    impl Default for GameSoundOutputBuffer {
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
                    // Truncate to 8 bits using mask, because i couldn't figure out how to cast it :)
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

    pub fn game_output_sound(sound_buffer: &GameSoundOutputBuffer) {
        static mut T_SINE: f32 = 0.0;
        let tone_volume: f32 = 200.0;
        let tone_hz = 256;
        let wave_period = sound_buffer.samples_per_second / tone_hz;

        let mut sample_out = sound_buffer.samples;
        for _ in 0..sound_buffer.sample_count {
            unsafe {
                let sine_value: f32 = T_SINE.sin();
                let sample_value: i16 = (sine_value * tone_volume) as i16;
                *sample_out = sample_value;
                sample_out = sample_out.add(1);
                *sample_out = sample_value;
                sample_out = sample_out.add(1);

                T_SINE += 2.0 * PI / wave_period as f32;
            }
        }
    }

    pub fn game_update_and_render(
        buffer: &GameOffscreenBuffer,
        blue_offset: i32,
        green_offset: i32,
        sound_buffer: &GameSoundOutputBuffer,
    ) {
        game_output_sound(sound_buffer);
        render_gradient(buffer, blue_offset, green_offset);
    }
}
