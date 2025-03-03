#[derive(Debug)]

pub struct Chip8IO {
    x: i32,
}

impl Chip8IO {
    pub fn new(x: i32) -> Self {
        Chip8IO { x }
    }
    pub fn draw(self: &mut Self, val: i32) {
        self.x = val;
    }
}
