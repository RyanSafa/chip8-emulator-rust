use sdl2::{audio::*, render::*, video::*};
use std::collections::HashMap;

pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;
const NUM_KEYS: usize = 16;
const KEYS: [&str; NUM_KEYS] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F",
];

/* Pros of using unsafe_texture:
 * 1. Don't need to initialize texture_creator and texture in main
 * 2. No liftimes needed for texture and textue_creator
 */
struct DroppableTexture {
    texture: Option<Texture>, // Wrap Option around Texture because we need a default value for std::mem::take
}
impl DroppableTexture {
    fn new(texture: Texture) -> Self {
        Self {
            texture: Some(texture),
        }
    }
}
impl AsRef<Texture> for DroppableTexture {
    fn as_ref(&self) -> &Texture {
        self.texture.as_ref().expect("Missing texture")
    }
}
impl AsMut<Texture> for DroppableTexture {
    fn as_mut(&mut self) -> &mut Texture {
        self.texture.as_mut().expect("Missing texture")
    }
}
impl Drop for DroppableTexture {
    fn drop(&mut self) {
        unsafe {
            std::mem::take(&mut self.texture)
                .expect("Missing texture")
                .destroy()
        };
    }
}

struct SquareWave {
    phase: f32,
    phase_increment: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = i16;

    fn callback(&mut self, buffer: &mut [Self::Channel]) {
        for i in buffer.iter_mut() {
            self.phase += self.phase_increment;
            if self.phase >= 1f32 {
                self.phase -= 1f32
            }
            let sample = if self.phase < 0.5 {
                (i16::max_value() as f32) * self.volume
            } else {
                (i16::min_value() as f32) * self.volume
            };
            *i = sample as i16;
        }
    }
}

pub struct Sdl2Mngr {
    sdl_context: sdl2::Sdl,
    canvas: Canvas<Window>,
    texture: DroppableTexture,
    audio_device: Option<AudioDevice<SquareWave>>,
}

fn create_audio_device(sdl_context: &sdl2::Sdl) -> Option<AudioDevice<SquareWave>> {
    let audio_subsystem = sdl_context.audio().ok()?;

    let desired_spec = AudioSpecDesired {
        freq: Some(44_100),
        channels: Some(1),
        samples: Some(4096),
    };

    Some(
        audio_subsystem
            .open_playback(None, &desired_spec, |spec| SquareWave {
                phase: 0.0,
                phase_increment: 440.0 / spec.freq as f32,
                volume: 0.05,
            })
            .ok()?,
    )
}

fn create_window(sdl_context: &sdl2::Sdl, scale_factor: u32) -> Window {
    let video_subsystem = sdl_context
        .video()
        .expect("Failed to initialze the video subsystem.");

    video_subsystem
        .window(
            "Chip8 Window",
            (DISPLAY_WIDTH as u32) * scale_factor,
            (DISPLAY_HEIGHT as u32) * scale_factor,
        )
        .position_centered()
        .build()
        .expect("Failed to create a window.")
}

impl Sdl2Mngr {
    fn new(scale_factor: u32) -> Self {
        let sdl_context = sdl2::init().expect("Failed to intialize the SDL2 Library.");
        let window = create_window(&sdl_context, scale_factor);
        let canvas = window
            .into_canvas()
            .build()
            .expect("Failed to create canvas.");
        let texture_creator = canvas.texture_creator();
        let texture = texture_creator
            .create_texture_streaming(
                sdl2::pixels::PixelFormatEnum::RGBA32,
                DISPLAY_WIDTH as u32,
                DISPLAY_HEIGHT as u32,
            )
            .expect("Failed to create texture.");

        let audio_device = create_audio_device(&sdl_context);

        return Self {
            sdl_context,
            canvas,
            texture: DroppableTexture::new(texture),
            audio_device,
        };
    }
}

pub struct Chip8IO {
    pub primary_color: u32,
    pub secondary_color: u32,
    keys_pressed: HashMap<&'static str, bool>,
    display_buffer: [u8; DISPLAY_HEIGHT * DISPLAY_WIDTH * 4],
    sdl_mngr: Sdl2Mngr,
}

fn construct_color_from_slice(pixels: &[u8]) -> u32 {
    let mut color: u32 = 0;
    color |= (pixels[0] as u32) << 24;
    color |= (pixels[1] as u32) << 16;
    color |= (pixels[2] as u32) << 8;
    color |= pixels[3] as u32;
    color
}

fn write_color_to_slice(pixels: &mut [u8], color: u32) {
    pixels[0] = ((color & 0xFF000000) >> 24) as u8;
    pixels[1] = ((color & 0x00FF0000) >> 16) as u8;
    pixels[2] = ((color & 0x0000FF00) >> 8) as u8;
    pixels[3] = (color & 0x000000FF) as u8;
}

impl Chip8IO {
    pub fn new(scale_factor: u32, primary_color: u32, secondary_color: u32) -> Self {
        let mut display_buffer = [0u8; DISPLAY_WIDTH * DISPLAY_HEIGHT * 4];
        for i in 0..DISPLAY_HEIGHT {
            for j in 0..DISPLAY_WIDTH {
                let index = ((i * DISPLAY_WIDTH) + j) * 4;
                write_color_to_slice(&mut display_buffer[index..index + 4], secondary_color);
            }
        }

        return Self {
            primary_color,
            secondary_color,
            keys_pressed: KEYS
                .iter()
                .enumerate()
                .map(|(_, &value)| (value, false))
                .collect(),
            display_buffer,
            sdl_mngr: Sdl2Mngr::new(scale_factor),
        };
    }

    pub fn write_pixel(&mut self, row: usize, col: usize, primary_color: bool) {
        let index = ((row * DISPLAY_WIDTH) + col) * 4;
        write_color_to_slice(
            &mut self.display_buffer[index..index + 4],
            if primary_color {
                self.primary_color
            } else {
                self.secondary_color
            },
        );
    }

    pub fn get_pixel_color(&self, row: usize, col: usize) -> u32 {
        let index = ((row * DISPLAY_WIDTH) + col) * 4;
        construct_color_from_slice(&self.display_buffer[index..index + 4])
    }

    pub fn render_frame(&mut self) {
        self.sdl_mngr
            .texture
            .as_mut()
            .with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                buffer.copy_from_slice(&self.display_buffer);
            })
            .expect("Locking texture failed");
        self.sdl_mngr
            .canvas
            .copy(self.sdl_mngr.texture.as_mut(), None, None)
            .expect("Copying texture failed");
        self.sdl_mngr.canvas.present();
    }

    pub fn poll_input(&mut self) -> bool {
        let mut events = self
            .sdl_mngr
            .sdl_context
            .event_pump()
            .expect("Error polling input");
        loop {
            for event in events.poll_iter() {
                match event {
                    sdl2::event::Event::Quit { .. } => {
                        return false;
                    }
                    sdl2::event::Event::KeyUp { scancode, .. } => {
                        let key_name = scancode.expect("Missing scancode").name();
                        if self.keys_pressed.contains_key(key_name) {
                            self.keys_pressed.insert(key_name, false);
                        }
                    }
                    sdl2::event::Event::KeyDown { scancode, .. } => {
                        let key_name = scancode.expect("Missing scancode").name();
                        if self.keys_pressed.contains_key(key_name) {
                            self.keys_pressed.insert(key_name, true);
                        }
                    }
                    _ => {}
                }
            }
            break;
        }
        return true;
    }

    pub fn is_key_pressed(&self, key_num: u8) -> bool {
        self.keys_pressed[KEYS[key_num as usize]]
    }

    pub fn play_audio(&self) {
        if let Some(audio_device) = self.sdl_mngr.audio_device.as_ref() {
            audio_device.resume()
        }
    }

    pub fn pause_audio(&self) {
        if let Some(audio_device) = self.sdl_mngr.audio_device.as_ref() {
            audio_device.pause()
        }
    }
}
