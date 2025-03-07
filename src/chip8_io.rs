extern crate sdl2;
use ::std::collections::HashMap;
use sdl2::{rect::*, render::*, video::*};

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
        self.texture.as_ref().unwrap()
    }
}
impl AsMut<Texture> for DroppableTexture {
    fn as_mut(&mut self) -> &mut Texture {
        self.texture.as_mut().unwrap()
    }
}
impl Drop for DroppableTexture {
    fn drop(&mut self) {
        unsafe { std::mem::take(&mut self.texture).unwrap().destroy() };
    }
}

pub struct Sdl2Mngr {
    sdl_context: sdl2::Sdl,
    canvas: Canvas<Window>,
    _texture_creator: TextureCreator<WindowContext>,
    texture: DroppableTexture,
    src_rect: Rect,
    dst_rect: Rect,
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

        return Self {
            sdl_context,
            canvas,
            _texture_creator: texture_creator,
            texture: DroppableTexture::new(texture),
            src_rect: Rect::new(
                0,
                0,
                DISPLAY_WIDTH.try_into().unwrap(),
                DISPLAY_HEIGHT.try_into().unwrap(),
            ),
            dst_rect: Rect::new(
                0,
                0,
                <usize as TryInto<u32>>::try_into(DISPLAY_WIDTH).unwrap() * scale_factor,
                <usize as TryInto<u32>>::try_into(DISPLAY_HEIGHT).unwrap() * scale_factor,
            ),
        };
    }
}

pub struct Chip8IO {
    key_pressed: HashMap<&'static str, bool>,
    display_buffer: [u8; DISPLAY_HEIGHT * DISPLAY_WIDTH * 4],
    sdl_mngr: Sdl2Mngr,
}

fn construct_color(pixels: &[u8]) -> u32 {
    let mut color: u32 = 0;
    color |= (pixels[0] as u32) << 24;
    color |= (pixels[1] as u32) << 16;
    color |= (pixels[2] as u32) << 8;
    color |= pixels[3] as u32;
    color
}

fn deconstruct_color(pixels: &mut [u8], color: u32) {
    pixels[0] = ((color & 0xFF000000) >> 24) as u8;
    pixels[1] = ((color & 0x00FF0000) >> 16) as u8;
    pixels[2] = ((color & 0x0000FF00) >> 8) as u8;
    pixels[3] = (color & 0x000000FF) as u8;
}

impl Chip8IO {
    pub fn new(scale_factor: u32) -> Self {
        return Self {
            key_pressed: KEYS
                .iter()
                .enumerate()
                .map(|(_, &value)| (value, false))
                .collect(),
            display_buffer: [0; DISPLAY_WIDTH * DISPLAY_HEIGHT * 4],
            sdl_mngr: Sdl2Mngr::new(scale_factor),
        };
    }

    pub fn write_pixel(&mut self, row: usize, col: usize, color: u32) {
        let index = ((row * DISPLAY_WIDTH) + col) * 4;
        deconstruct_color(&mut self.display_buffer[index..index + 4], color);
    }

    pub fn get_pixel_color(&self, row: usize, col: usize) -> u32 {
        let index = ((row * DISPLAY_WIDTH) + col) * 4;
        construct_color(&self.display_buffer[index..index + 4])
    }

    pub fn render_frame(&mut self) {
        self.sdl_mngr
            .texture
            .as_mut()
            .with_lock(None, |buffer: &mut [u8], _pitch: usize| {
                buffer.copy_from_slice(&self.display_buffer);
            })
            .unwrap();
        self.sdl_mngr
            .canvas
            .copy(
                self.sdl_mngr.texture.as_mut(),
                self.sdl_mngr.src_rect,
                self.sdl_mngr.dst_rect,
            )
            .unwrap();
        self.sdl_mngr.canvas.present();
    }

    pub fn poll_input(&mut self) -> bool {
        let mut events = self.sdl_mngr.sdl_context.event_pump().unwrap();
        loop {
            for event in events.poll_iter() {
                match event {
                    sdl2::event::Event::Quit { .. } => {
                        return false;
                    }
                    sdl2::event::Event::KeyUp { scancode, .. } => {
                        let key_name = scancode.unwrap().name();
                        if self.key_pressed.contains_key(key_name) {
                            self.key_pressed.insert(key_name, false);
                        }
                    }
                    sdl2::event::Event::KeyDown { scancode, .. } => {
                        let key_name = scancode.unwrap().name();
                        if self.key_pressed.contains_key(key_name) {
                            self.key_pressed.insert(key_name, true);
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
        self.key_pressed[KEYS[key_num as usize]]
    }
}
