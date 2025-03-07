use crate::chip8_io;
use rand::distr::{Distribution, Uniform};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::{cell::RefCell, rc::Rc};

pub const FONT_SIZE: usize = 80;
const NUM_REGISTERS: usize = 0x10;
const MEMORY_SIZE: usize = 0x1000;
const ROM_START_ADDR: usize = 0x200;
const FONT_START_ADDR: usize = 0x50;

type Result<T> = std::result::Result<T, Chip8Error>;

#[derive(Debug)]
pub enum Chip8Error {
    InvaidOpcode(u16),
    StackUnderflow(Opcode), 
}

impl std::fmt::Display for Chip8Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chip8Error::InvaidOpcode(opcode) => {
                write!(f, "Unknown opcode error: {}", opcode)
            }
            Chip8Error::StackUnderflow(opcode) => {
                write!(f, "StackUnderflow error: opcode: {:#?}", opcode)
            }
        }
    }
}


#[derive(Debug, Clone)]
pub struct Opcode {
    raw: u16,
    op_type: u8,
    x: u8,
    y: u8,
    n: u8,
}

impl Opcode {
    fn new(raw: u16) -> Self {
        return Opcode {
            raw,
            op_type: ((raw & 0xF000) >> 12) as u8,
            x: ((raw & 0x0F00) >> 8) as u8,
            y: ((raw & 0x00F0) >> 4) as u8,
            n: (raw & 0x000F) as u8,
        };
    }

    fn get_nn(&self) -> u8 {
        (self.raw & 0x00FF) as u8
    }
    fn get_nnn(&self) -> u16 {
        (self.raw & 0x0FFF) as u16
    }
}

pub struct Chip8 {
    io: Rc<RefCell<chip8_io::Chip8IO>>,
    primary_color: u32,
    secondary_color: u32,
    pc: usize,
    i: usize,
    delay_timer: u8,
    sound_timer: u8,
    registers: [u8; NUM_REGISTERS],
    stack: Vec<usize>,
    memory: [u8; MEMORY_SIZE],
    rng: rand::rngs::ThreadRng,
    distrib: Uniform<u16>,
}

impl Chip8 {
    pub fn new(
        io: &Rc<RefCell<chip8_io::Chip8IO>>,
        primary_color: u32,
        secondary_color: u32,
    ) -> Self {
        return Chip8 {
            io: Rc::clone(io),
            primary_color,
            secondary_color,
            pc: ROM_START_ADDR,
            i: 0,
            delay_timer: 0,
            sound_timer: 0,
            registers: [0; NUM_REGISTERS],
            stack: Vec::new(),
            memory: [0; MEMORY_SIZE],
            rng: rand::rng(),
            distrib: Uniform::new(0, 256).unwrap(),
        };
    }

    fn set_vf(&mut self, value: u8) {
        self.registers[0xF] = value;
    }

    fn skip_pc(&mut self) {
        self.pc += 2;
    }

    fn skip_pc_back(&mut self) {
        self.pc -= 2;
    }

    fn exec_op_type0(&mut self, opcode: &Opcode) -> Result<()> {
        match opcode.get_nn() {
            0x0E0 => {
                for row in 0..chip8_io::DISPLAY_HEIGHT {
                    for col in 0..chip8_io::DISPLAY_WIDTH {
                        self.io
                            .borrow_mut()
                            .write_pixel(row, col, self.secondary_color);
                    }
                }
                Ok(())
            }
            0x0EE => {
                self.pc = self
                    .stack
                    .pop()
                    .ok_or(Chip8Error::StackUnderflow(opcode.clone()))?;
                Ok(())
            }
            _ => Err(Chip8Error::InvaidOpcode(opcode.raw)),
        }
    }

    fn exec_op_type1(&mut self, opcode: &Opcode) {
        self.pc = opcode.get_nnn() as usize;
    }

    fn exec_op_type2(&mut self, opcode: &Opcode) {
        self.stack.push(self.pc);
        self.pc = opcode.get_nnn() as usize;
    }

    fn exec_op_type3(&mut self, opcode: &Opcode) {
        if self.registers[opcode.x as usize] == opcode.get_nn() {
            self.skip_pc();
        }
    }

    fn exec_op_type4(&mut self, opcode: &Opcode) {
        if self.registers[opcode.x as usize] != opcode.get_nn() {
            self.skip_pc();
        }
    }

    fn exec_op_type5(&mut self, opcode: &Opcode) {
        if self.registers[opcode.x as usize] == self.registers[opcode.y as usize] {
            self.skip_pc();
        }
    }

    fn exec_op_type6(&mut self, opcode: &Opcode) {
        self.registers[opcode.x as usize] = opcode.get_nn();
    }

    fn exec_op_type7(&mut self, opcode: &Opcode) {
        self.registers[opcode.x as usize] =
            self.registers[opcode.x as usize].wrapping_add(opcode.get_nn());
    }

    fn exec_op_type8(&mut self, opcode: &Opcode) -> Result<()> {
        match opcode.n {
            0x0 => {
                self.registers[opcode.x as usize] = self.registers[opcode.y as usize];
                Ok(())
            }
            0x1 => {
                self.registers[opcode.x as usize] |= self.registers[opcode.y as usize];
                Ok(())
            }
            0x2 => {
                self.registers[opcode.x as usize] &= self.registers[opcode.y as usize];
                Ok(())
            }
            0x3 => {
                self.registers[opcode.x as usize] ^= self.registers[opcode.y as usize];
                Ok(())
            }
            0x4 => {
                let sum = self.registers[opcode.x as usize]
                    .wrapping_add(self.registers[opcode.y as usize]);
                let vf_value: u8;

                if sum < self.registers[opcode.x as usize] {
                    vf_value = 1;
                } else {
                    vf_value = 0;
                }
                self.registers[opcode.x as usize] = sum;
                self.set_vf(vf_value);
                Ok(())
            }
            0x5 => {
                let vf_value: u8;
                if self.registers[opcode.x as usize] >= self.registers[opcode.y as usize] {
                    vf_value = 1;
                } else {
                    vf_value = 0;
                }
                self.registers[opcode.x as usize] = self.registers[opcode.x as usize]
                    .wrapping_sub(self.registers[opcode.y as usize]);
                self.set_vf(vf_value);
                Ok(())
            }
            0x6 => {
                let vf_value = self.registers[opcode.x as usize] & 0x01;
                self.registers[opcode.x as usize] = self.registers[opcode.x as usize] >> 1;
                self.set_vf(vf_value);
                Ok(())
            }
            0x7 => {
                let vf_value: u8;
                if self.registers[opcode.y as usize] >= self.registers[opcode.x as usize] {
                    vf_value = 1;
                } else {
                    vf_value = 0;
                }
                self.registers[opcode.x as usize] = self.registers[opcode.y as usize]
                    .wrapping_sub(self.registers[opcode.x as usize]);
                self.set_vf(vf_value);
                Ok(())
            }
            0xE => {
                let vf_value = (self.registers[opcode.x as usize] & 0x80) >> 7;
                self.registers[opcode.x as usize] = self.registers[opcode.x as usize] << 1;
                self.set_vf(vf_value);
                Ok(())
            }
            _ => Err(Chip8Error::InvaidOpcode(opcode.raw)),
        }
    }

    fn exec_op_type9(&mut self, opcode: &Opcode) {
        if self.registers[opcode.x as usize] != self.registers[opcode.y as usize] {
            self.skip_pc();
        }
    }

    fn exec_op_type10(&mut self, opcode: &Opcode) {
        self.i = opcode.get_nnn() as usize;
    }

    fn exec_op_type11(&mut self, opcode: &Opcode) {
        self.pc = (opcode.get_nnn() + self.registers[0] as u16) as usize;
    }

    fn exec_op_type12(&mut self, opcode: &Opcode) {
        self.registers[opcode.x as usize] =
            (self.distrib.sample(&mut self.rng) as u8) & opcode.get_nn();
    }

    fn exec_op_type13(&mut self, opcode: &Opcode) {
        let x_coord = self.registers[opcode.x as usize] % (chip8_io::DISPLAY_WIDTH as u8);
        let y_coord = self.registers[opcode.y as usize] % (chip8_io::DISPLAY_HEIGHT as u8);
        self.set_vf(0);

        for i in 0..opcode.n {
            let new_y_coord = y_coord + i;
            if new_y_coord >= (chip8_io::DISPLAY_HEIGHT as u8) {
                continue;
            }
            for j in 0..8 {
                let new_x_coord = x_coord + j;
                if new_x_coord >= (chip8_io::DISPLAY_WIDTH as u8) {
                    continue;
                }
                let mask = 1 << (7 - j);
                let sprite_color = (self.memory[self.i + i as usize] & mask) >> (7 - j);
                let prev_frame_color = self
                    .io
                    .borrow_mut()
                    .get_pixel_color(new_y_coord as usize, new_x_coord as usize);
                if sprite_color == 1 {
                    if prev_frame_color == self.primary_color {
                        self.set_vf(1);
                        self.io.borrow_mut().write_pixel(
                            new_y_coord as usize,
                            new_x_coord as usize,
                            self.secondary_color,
                        );
                    } else {
                        self.io.borrow_mut().write_pixel(
                            new_y_coord as usize,
                            new_x_coord as usize,
                            self.primary_color,
                        );
                    }
                }
            }
        }
    }

    fn exec_op_type14(&mut self, opcode: &Opcode) -> Result<()> {
        match opcode.n {
            0x1 => {
                if !self
                    .io
                    .borrow_mut()
                    .is_key_pressed(self.registers[opcode.x as usize])
                {
                    self.skip_pc();
                }
                Ok(())
            }
            0xE => {
                if self
                    .io
                    .borrow_mut()
                    .is_key_pressed(self.registers[opcode.x as usize])
                {
                    self.skip_pc();
                }
                Ok(())
            }
            _ => Err(Chip8Error::InvaidOpcode(opcode.raw)),
        }
    }

    fn exec_op_type15(&mut self, opcode: &Opcode) -> Result<()> {
        match opcode.get_nn() {
            0x7 => {
                self.registers[opcode.x as usize] = self.delay_timer;
                Ok(())
            }
            0x15 => {
                self.delay_timer = self.registers[opcode.x as usize];
                Ok(())
            }
            0x18 => {
                self.sound_timer = self.registers[opcode.x as usize];
                Ok(())
            }
            0x1E => {
                self.i += self.registers[opcode.x as usize] as usize;
                if self.i >= 0x1000 {
                    self.set_vf(1);
                }
                self.i &= 0xFFF;
                Ok(())
            }
            0x0A => {
                let mut key_pressed = false;

                for i in 0..16 {
                    if self.io.borrow_mut().is_key_pressed(i) {
                        self.registers[opcode.x as usize] = i;
                        key_pressed = true;
                        break;
                    }
                }

                if key_pressed {
                    self.skip_pc_back();
                }
                Ok(())
            }
            0x29 => {
                self.i = FONT_START_ADDR + (self.registers[opcode.x as usize] * 5) as usize;
                Ok(())
            }
            0x33 => {
                let mut digits: Vec<u8> = Vec::new();
                let mut cur = self.registers[opcode.x as usize];

                while cur > 0 {
                    digits.push(cur % 10);
                    cur /= 10;
                }

                while digits.len() < 3 {
                    digits.push(0);
                }

                for (index, &value) in digits.iter().rev().enumerate() {
                    self.memory[self.i + index] = value;
                }
                Ok(())
            }
            0x55 => {
                for i in 0..opcode.x + 1 {
                    self.memory[self.i + (i as usize)] = self.registers[i as usize];
                }
                Ok(())
            }
            0x65 => {
                for i in 0..opcode.x + 1 {
                    self.registers[i as usize] = self.memory[self.i + i as usize];
                }
                Ok(())
            }
            _ => Err(Chip8Error::InvaidOpcode(opcode.raw)),
        }
    }

    pub fn load_rom(&mut self, rom_file: &mut File) {
        let len = rom_file.seek(SeekFrom::End(0)).expect("Failed to load ROM");

        rom_file
            .seek(SeekFrom::Start(0))
            .expect("Failed to load ROM");

        rom_file
            .read_exact(&mut self.memory[ROM_START_ADDR..ROM_START_ADDR + len as usize])
            .expect("Failed to load ROM");
    }

    pub fn load_font(&mut self, font_buffer: &[u8], font_size: usize) {
        self.memory[FONT_START_ADDR..FONT_START_ADDR + font_size]
            .as_mut()
            .copy_from_slice(font_buffer)
    }

    pub fn update_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    pub fn run_cycle(&mut self) -> Result<()> {
        let opcod_raw = ((self.memory[self.pc] as u16) << 8) | (self.memory[self.pc + 1] as u16);
        let opcode = Opcode::new(opcod_raw);
        self.skip_pc();

        match opcode.op_type {
            0x0 => self.exec_op_type0(&opcode)?,
            0x1 => self.exec_op_type1(&opcode),
            0x2 => self.exec_op_type2(&opcode),
            0x3 => self.exec_op_type3(&opcode),
            0x4 => self.exec_op_type4(&opcode),
            0x5 => self.exec_op_type5(&opcode),
            0x6 => self.exec_op_type6(&opcode),
            0x7 => self.exec_op_type7(&opcode),
            0x8 => self.exec_op_type8(&opcode)?,
            0x9 => self.exec_op_type9(&opcode),
            0xA => self.exec_op_type10(&opcode),
            0xB => self.exec_op_type11(&opcode),
            0xC => self.exec_op_type12(&opcode),
            0xD => self.exec_op_type13(&opcode),
            0xE => self.exec_op_type14(&opcode)?,
            0xF => self.exec_op_type15(&opcode)?,
            _ => Err(Chip8Error::InvaidOpcode(opcode.raw))?,
        }

        Ok(())
    }
}
