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
                sdl2::pixels::PixelFormatEnum::RGBA8888,
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

pub struct Chip8IO<'a> {
    key_pressed: HashMap<&'a str, bool>,
    display_buffer: [u8; DISPLAY_HEIGHT * DISPLAY_WIDTH * 4],
    sdl_mngr: Sdl2Mngr,
}

fn construct_color(pixels: &[u8]) -> u32 {
    ((pixels[0] as u32) << 24)
        | ((pixels[1] as u32) << 16)
        | ((pixels[2] as u32) << 8)
        | (pixels[3] as u32)
}

impl<'a> Chip8IO<'a> {
    pub fn new(scale_factor: u32) -> Self {
        return Self {
            key_pressed: HashMap::new(),
            display_buffer: [0; DISPLAY_WIDTH * DISPLAY_HEIGHT * 4],
            sdl_mngr: Sdl2Mngr::new(scale_factor),
        };
    }

    pub fn write_pixel(self: &mut Self, row: usize, col: usize, color: u32) {
        let index = ((row * DISPLAY_WIDTH) + col) * 4;
        self.display_buffer[index] = ((color >> 24) & 0xFF) as u8; // Red
        self.display_buffer[index + 1] = ((color >> 16) & 0xFF) as u8; // Green
        self.display_buffer[index + 2] = ((color >> 8) & 0xFF) as u8; // Blue
        self.display_buffer[index + 3] = (color & 0xFF) as u8; // Alpha
    }

    pub fn get_pixel_color(self: &Self, row: usize, col: usize) -> u32 {
        let index = ((row * DISPLAY_WIDTH) + col) * 4;
        construct_color(&self.display_buffer[index..index + 4])
    }

    pub fn render_frame(self: &mut Self) {
        self.sdl_mngr
            .texture
            .as_mut()
            .with_lock(
                None,
                |buffer: &mut [u8], _pitch: usize| {
                    buffer.copy_from_slice(&self.display_buffer);
                },
            )
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

    pub fn poll_input(self: &mut Self) -> bool {
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

    pub fn is_key_pressed(self: &Self, key_num: u8) -> bool {
        self.key_pressed[KEYS[key_num as usize]]
    }
}
