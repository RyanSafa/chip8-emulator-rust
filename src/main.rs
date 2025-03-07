mod chip8;
mod chip8_io;

use chip8::*;
use chip8_io::*;
use clap::Parser;

const FRAME_RATE: u64 = 60;
const FRAME_TIME_MICROSECONDS: u64 = 1000000 / FRAME_RATE;
const FONT_SIZE: usize = 80;
const FONT: [u8; FONT_SIZE] = [
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

/// Custom parser for hex color strings.
/// This function accepts strings like "0xFF0000FF" and parses them into a u32.
fn parse_hex_color(s: &str) -> Result<u32, String> {
    let s = s.trim();
    let s = if s.starts_with("0x") || s.starts_with("0X") {
        &s[2..]
    } else {
        s
    };
    u32::from_str_radix(s, 16).map_err(|e| format!("Invalid hex color '{}': {}", s, e))
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Chip8 emulator in Rust", long_about = None)]
struct Args {
    /// Path to a ROM
    path_to_rom: std::path::PathBuf,

    /// Scale factor for the original 64 x 32 screen size
    #[arg(long, default_value_t = 24)]
    scale_factor: u32,

    /// The number of instructions that should be performed in one frame
    #[arg(long, default_value_t = 11)]
    instructions_per_second: u32,

    /// Primary color in rgba format
    /// Accepts hex values like "0xFF0000FF".
    #[arg(long, default_value = "0xFFFFFFFF", value_parser = parse_hex_color )]
    primary_color: u32,

    /// Secondary color in rgba format
    /// Accepts hex values like "0x000000FF".
    #[arg(long, default_value = "0x000000FF", value_parser = parse_hex_color)]
    secondary_color: u32,
}

fn main() {
    let args = Args::parse();
    let chip8_io = std::rc::Rc::new(std::cell::RefCell::new(Chip8IO::new(
        args.scale_factor,
        args.primary_color,
        args.secondary_color,
    )));
    let mut chip8_cpu = Chip8::new(&chip8_io);
    let mut rom_file = std::fs::File::open(args.path_to_rom).expect("Failed to open ROM file");

    chip8_cpu.load_rom(&mut rom_file);
    chip8_cpu.load_font(&FONT[..], FONT_SIZE);

    let target_frame_duration = std::time::Duration::from_micros(FRAME_TIME_MICROSECONDS);

    while chip8_io.borrow_mut().poll_input() {
        let frame_start = std::time::Instant::now();

        chip8_cpu.update_timers();

        for _ in 0..args.instructions_per_second {
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
