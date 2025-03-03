use rand::Rng;
use rand::distr::{Distribution, Uniform};
use std::rc::Rc;

use crate::chip8_io::Chip8IO;

const NUM_REGISTERS: usize = 0x10;
const MEMORY_SIZE: usize = 0x1000;
const ROM_START_ADDR: usize = 0x200;
const FONT_START_ADDR: usize = 0x50;

#[derive(Debug)]

struct Opcode {
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

    fn get_nn(self: &Self) -> u8 {
        (self.raw & 0x00FF) as u8
    }
    fn get_nnn(self: &Self) -> u16 {
        (self.raw & 0x0FFF) as u16
    }
}

#[derive(Debug)]
pub struct Chip8<'a> {
    pc: usize,
    i: usize,
    delay_timer: u8,
    sound_timer: u8,
    registers: [u8; NUM_REGISTERS],
    stack: Vec<usize>,
    memory: [u8; MEMORY_SIZE],
    font: &'a [u32],
    io: Rc<Chip8IO>,
    rng: rand::rngs::ThreadRng,
    distrib: Uniform<u16>,
}

impl<'a> Chip8<'a> {
    pub fn new(io: Rc<Chip8IO>, font: &'a [u32]) -> Self {
        return Chip8 {
            i: 0,
            delay_timer: 0,
            sound_timer: 0,
            registers: [0; NUM_REGISTERS],
            stack: Vec::new(),
            memory: [0; MEMORY_SIZE],
            pc: ROM_START_ADDR,
            font,
            io: Rc::clone(&io),
            rng: rand::rng(),
            distrib: Uniform::new(0, 256).unwrap(),
        };
    }

    fn set_vf(self: &mut Self, value: u8) {
        self.registers[0xF] = value;
    }

    fn skip_pc(self: &mut Self) {
        self.pc += 2;
    }

    fn exec_op_type0(self: &mut Self, opcode: &Opcode) {
        match opcode.get_nn() {
            0x0E0 => {
                // fix later
            }
            0x0EE => {
                self.pc = self.stack.pop().unwrap_or_else(|| {
                    panic!("Opcode: {:#?} - Popped of stack when emtpy", opcode)
                });
            }
            _ => {
                panic!("Did not recognize opcode: {:#?}", opcode)
            }
        }
    }

    fn exec_op_type1(self: &mut Self, opcode: &Opcode) {
        self.pc = opcode.get_nnn() as usize;
    }

    fn exec_op_type2(self: &mut Self, opcode: &Opcode) {
        self.stack.push(self.pc);
        self.pc = opcode.get_nnn() as usize;
    }

    fn exec_op_type3(self: &mut Self, opcode: &Opcode) {
        if self.registers[opcode.x as usize] == self.registers[opcode.get_nnn() as usize] {
            self.skip_pc();
        }
    }

    fn exec_op_type4(self: &mut Self, opcode: &Opcode) {
        if self.registers[opcode.x as usize] != self.registers[opcode.get_nnn() as usize] {
            self.skip_pc();
        }
    }

    fn exec_op_type5(self: &mut Self, opcode: &Opcode) {
        if self.registers[opcode.x as usize] != self.registers[opcode.y as usize] {
            self.skip_pc();
        }
    }

    fn exec_op_type6(self: &mut Self, opcode: &Opcode) {
        self.registers[opcode.x as usize] = self.registers[opcode.get_nn() as usize];
    }

    fn exec_op_type7(self: &mut Self, opcode: &Opcode) {
        self.registers[opcode.x as usize] =
            self.registers[opcode.x as usize] + self.registers[opcode.get_nn() as usize]
    }

    fn exec_op_type8(self: &mut Self, opcode: &Opcode) {
        match opcode.n {
            0x0 => self.registers[opcode.x as usize] = self.registers[opcode.y as usize],
            0x1 => self.registers[opcode.x as usize] |= self.registers[opcode.y as usize],
            0x2 => self.registers[opcode.x as usize] &= self.registers[opcode.y as usize],
            0x3 => self.registers[opcode.x as usize] ^= self.registers[opcode.y as usize],
            0x4 => {
                let sum = self.registers[opcode.x as usize] + self.registers[opcode.y as usize];
                let vf_value: u8;

                if sum < self.registers[opcode.x as usize] {
                    vf_value = 1;
                } else {
                    vf_value = 0;
                }
                self.registers[opcode.x as usize] = sum;
                self.set_vf(vf_value);
            }
            0x5 => {
                let vf_value: u8;
                if self.registers[opcode.x as usize] >= self.registers[opcode.y as usize] {
                    vf_value = 1;
                } else {
                    vf_value = 0;
                }
                self.registers[opcode.x as usize] -= self.registers[opcode.y as usize];
                self.set_vf(vf_value);
            }
            0x6 => {
                let vf_value = self.registers[opcode.x as usize] & 0x01;
                self.registers[opcode.x as usize] = self.registers[opcode.x as usize] >> 1;
                self.set_vf(vf_value);
            }
            0x7 => {
                let vf_value: u8;
                if self.registers[opcode.y as usize] >= self.registers[opcode.x as usize] {
                    vf_value = 1;
                } else {
                    vf_value = 0;
                }
                self.registers[opcode.x as usize] =
                    self.registers[opcode.y as usize] - self.registers[opcode.x as usize];
                self.set_vf(vf_value);
            }
            0xE => {
                let vf_value = (self.registers[opcode.x as usize] & 0x80) >> 7;
                self.registers[opcode.x as usize] = self.registers[opcode.x as usize] << 1;
                self.set_vf(vf_value);
            }
            _ => {}
        }
    }

    fn exec_op_type9(self: &mut Self, opcode: &Opcode) {
        if self.registers[opcode.x as usize] != self.registers[opcode.y as usize] {
            self.skip_pc();
        }
    }

    fn exec_op_type10(self: &mut Self, opcode: &Opcode) {
        self.pc = opcode.get_nnn() as usize;
    }

    fn exec_op_type11(self: &mut Self, opcode: &Opcode) {
        self.pc = (opcode.get_nnn() + self.registers[0] as u16) as usize;
    }

    fn exec_op_type12(self: &mut Self, opcode: &Opcode) {
        self.registers[opcode.x as usize] =
            (self.distrib.sample(&mut self.rng) as u8) & opcode.get_nn();
    }

    fn exec_op_type13(self: &mut Self, opcode: &Opcode) {
        // skip
    }

    fn exec_op_type14(self: &mut Self, opcode: &Opcode) {
        match opcode.n {
            0x1 => {
                //
            }
            0xE => {
                //
            }
            _ => {
                println!("fucked up")
            }
        }
    }

    fn exec_op_type15(self: &mut Self, opcode: &Opcode) {
        match opcode.get_nn() {
            0x7 => {
                self.registers[opcode.x as usize] = self.delay_timer;
            }
            0x15 => {
                self.delay_timer = self.registers[opcode.x as usize];
            }
            0x18 => {
                self.sound_timer = self.registers[opcode.x as usize];
            }
            0x1E => {
                self.i = self.registers[opcode.x as usize] as usize;
                if self.i >= 0x1000 {
                    self.set_vf(1);
                }
            }
            0x0A => {
                // skip
            }
            0x29 => {
                self.i = FONT_START_ADDR + (self.registers[opcode.x as usize] * 5) as usize;
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
                    self.registers[self.i + index] = value;
                }
            }
            0x55 => {
                for (i, value) in (0..opcode.x + 1).enumerate() {
                    self.registers[self.i + i] = value;
                }
            }
            0x65 => {
                for i in 0..opcode.x + 1 {
                    self.registers[i as usize] = self.registers[self.i + i as usize];
                }
            }
            _ => {}
        }
    }

    pub fn update_timers(self: &mut Self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    pub fn run_cycle(self: &mut Self) {
        let opcod_raw = ((self.memory[self.pc] >> 8) | self.memory[self.pc + 1]) as u16;
        let opcode = Opcode::new(opcod_raw);

        self.skip_pc();

        match opcode.op_type {
            0x0 => {
                self.exec_op_type0(&opcode);
            }
            0x1 => {
                self.exec_op_type1(&opcode);
            }
            0x2 => {
                self.exec_op_type2(&opcode);
            }
            0x3 => {
                self.exec_op_type3(&opcode);
            }
            0x4 => {
                self.exec_op_type4(&opcode);
            }
            0x5 => {
                self.exec_op_type5(&opcode);
            }
            0x6 => {
                self.exec_op_type6(&opcode);
            }
            0x7 => {
                self.exec_op_type7(&opcode);
            }
            0x8 => {
                self.exec_op_type8(&opcode);
            }
            0x9 => {
                self.exec_op_type9(&opcode);
            }
            0xA => {
                self.exec_op_type10(&opcode);
            }
            0xB => {
                self.exec_op_type11(&opcode);
            }
            0xC => {
                self.exec_op_type12(&opcode);
            }
            0xD => {
                self.exec_op_type13(&opcode);
            }
            0xE => {
                self.exec_op_type14(&opcode);
            }
            0xF => {
                self.exec_op_type15(&opcode);
            }
            _ => {}
        }
    }
}
