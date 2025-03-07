mod chip8;
mod chip8_io;

use chip8::*;
use chip8_io::*;
use std::fs::File;
use std::{cell::RefCell, rc::Rc};

fn main() {
    const FRAME_RATE: u64 = 60;
    const FRAME_TIME_MICROSECONDS: u64 = 1000000 / FRAME_RATE;

    let font: [u8; FONT_SIZE] = [
        0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
        0x20, 0x60, 0x20, 0x20, 0x70, // 1
        0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
        0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
        0x90, 0x90, 0xF0, 0x10, 0x10, // 4
        0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
        0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
        0xF0, 0x10, 0x20, 0x40, 0x40, // 7
        0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
        0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
        0xF0, 0x90, 0xF0, 0x90, 0x90, // A
        0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
        0xF0, 0x80, 0x80, 0x80, 0xF0, // C
        0xE0, 0x90, 0x90, 0x90, 0xE0, // D
        0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
        0xF0, 0x80, 0xF0, 0x80, 0x80, // F
    ];
    let chip8_io = Rc::new(RefCell::new(Chip8IO::new(24)));
    let mut chip8_cpu = Chip8::new(&chip8_io, 0xFFFFFFFF, 0x000000FF);
    let mut rom_file = File::open("./roms/brick.ch8").unwrap();

    chip8_cpu.load_rom(&mut rom_file);
    chip8_cpu.load_font(&font[..], font.len());

    let target_frame_duration = std::time::Duration::from_micros(FRAME_TIME_MICROSECONDS);

    while chip8_io.borrow_mut().poll_input() {
        let frame_start = std::time::Instant::now();

        chip8_cpu.update_timers();

        for _ in 0..11 {
            if let Err(e) = chip8_cpu.run_cycle() {
                println!("{}", e);
                return;
            }
        }

        let frame_end = std::time::Instant::now();
        let time_elapsed = frame_end - frame_start;
        let sleep_time = target_frame_duration - time_elapsed;

        if sleep_time.as_micros() > 0u128 {
            std::thread::sleep(sleep_time);
        }

        chip8_io.borrow_mut().render_frame();
    }
}
