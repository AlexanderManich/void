use game_definitions::*;
use std::{f32::consts::PI, ffi::c_void, mem::size_of};
use windows::core::{w, PCWSTR};

#[no_mangle]
fn get_controller(input: &mut GameInput, controller_index: usize) -> &mut GameControllerInput {
    assert!(controller_index < input.controllers.len());
    &mut input.controllers[controller_index]
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

                // Shift green up by 8 bits from bottom, to make them sit in the green channel and not overwrite the blue channel (little endian)
                *pixel = ((green << 8) | blue) as u32;
                pixel = pixel.add(1);
            }

            row = row.add(buffer.pitch);
        }
    }
}

pub fn game_output_sound(sound_buffer: &GameSoundOutputBuffer, tone_hz: u32) {
    static mut T_SINE: f32 = 0.0;
    let tone_volume: f32 = 200.0;
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
            if T_SINE > 2.0 * PI {
                T_SINE -= 2.0 * PI;
            }
        }
    }
}

#[no_mangle]
pub fn game_get_sound_samples(memory: &mut GameMemory, sound_buffer: &GameSoundOutputBuffer) {
    let game_state = memory.permanent_storage as *mut GameState;
    unsafe {
        game_output_sound(sound_buffer, (*game_state).tone_hz);
    }
}

#[no_mangle]
pub fn game_update_and_render(
    buffer: &GameOffscreenBuffer,
    input: &mut GameInput,
    memory: &mut GameMemory,
) {
    debug_assert!(
        size_of::<ConcreteGameButtonState>() / size_of::<GameButtonState>()
            == unsafe { input.controllers[0].button_state.buttons.len() }
    );
    debug_assert!(size_of::<GameState>() <= memory.permanent_storage_size.try_into().unwrap());

    unsafe {
        let game_state = memory.permanent_storage as *mut GameState;
        if !memory.is_initialized {
            #[cfg(feature = "gaming_internal")]
            {
                // TODO: File path of THIS file right now. change to something idk
                let filename = file!().encode_utf16().chain([0u16]).collect::<Vec<u16>>();
                let filename_pcwstr = PCWSTR(filename.as_ptr());
                // let filename_test = PCWSTR(filename.as_ptr());
                match (memory.debug_platform_read_entire_file)(filename_pcwstr) {
                    Ok(file) => {
                        (memory.debug_platform_write_entire_file)(
                            w!("src\\test.out"),
                            file.contents_size,
                            file.contents,
                        )
                        .unwrap_or_else(|e| {
                            dbg!(e);
                        });
                        (memory.debug_platform_free_file_memory)(file.contents);
                    }
                    Err(e) => {
                        dbg!(e);
                    }
                };
            }

            (*game_state).tone_hz = 256;
            memory.is_initialized = true;
        }

        for controller_index in 0..input.controllers.len() {
            let controller = get_controller(input, controller_index);
            if controller.is_analog {
                (*game_state).blue_offset += (4.0 * controller.stick_average_x) as i32;
                (*game_state).tone_hz = 256 + ((128.0 * controller.stick_average_y) as u32);
            } else {
                if controller
                    .button_state
                    .concrete_buttons
                    .move_left
                    .ended_down
                {
                    (*game_state).blue_offset -= 1;
                }

                if controller
                    .button_state
                    .concrete_buttons
                    .move_right
                    .ended_down
                {
                    (*game_state).blue_offset += 1;
                }
            }

            if controller
                .button_state
                .concrete_buttons
                .action_down
                .ended_down
            {
                (*game_state).green_offset += 1;
            }
        }

        render_gradient(
            buffer,
            (*game_state).blue_offset,
            (*game_state).green_offset,
        );
    }
}
