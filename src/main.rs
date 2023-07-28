extern crate sdl2;

use std::collections::hash_map::RandomState;
use std::env;
use std::error::Error;
use std::fs;
use std::ops::Range;
use std::path::Path;
use std::process;
use std::thread::current;
use std::{thread, time};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::libc::free;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;
use sdl2::sys::random;
use sdl2::video::Window;

use std::time::Duration;

const C8_DISPLAY_WIDTH: usize = 64;
const C8_DISPLAY_HEIGHT: usize = 32;
const C8_RAM_SIZE: usize = 4096;
const C8_STACK_SIZE: usize = 16;
const C8_PROGRAM_START: usize = 0x200;
const SCALE_FACTOR: usize = 8;

use rand::prelude::*;

pub const C8_FONT: [u8; 80] = [
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

struct Chip8 {
    ram: [u8; C8_RAM_SIZE],
    vram: [[u8; C8_DISPLAY_WIDTH]; C8_DISPLAY_HEIGHT],
    stack: [u16; C8_STACK_SIZE],
    pc: usize,
    idx: u16,
    sp: u16,
    delay_timer: u8,
    sound_timer: u8,
    registers: [u8; 16],
}

impl Chip8 {
    pub fn new() -> Self {
        let mut ram = [0u8; C8_RAM_SIZE];

        for i in 0..C8_FONT.len() {
            ram[i] = C8_FONT[i];
        }

        Chip8 {
            ram: ram,
            vram: [[0; C8_DISPLAY_WIDTH]; C8_DISPLAY_HEIGHT],
            stack: [0; C8_STACK_SIZE],
            pc: C8_PROGRAM_START,
            idx: 0,
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            registers: [0; 16],
        }
    }

    pub fn load_program(&mut self, file_path: &String) {
        let buffer: Vec<u8> = read_program(file_path).unwrap_or_else(|err| {
            println!("Can't read input file: {err}");
            process::exit(1);
        });

        //let free_space = C8_RAM_SIZE - C8_PROGRAM_START;
        let buffer_size = buffer.len();
        self.ram[C8_PROGRAM_START..C8_PROGRAM_START + buffer_size].copy_from_slice(&buffer);
    }

    pub fn fetch(&mut self) -> u16 {
        let instr: u16 = ((self.ram[self.pc] as u16) << 8) | self.ram[self.pc + 1] as u16;
        self.pc += 2;
        instr
    }

    fn decode(&mut self, instr: u16) {
        /*
        Instr = 0xABCD
        then:
            instr_type = A
            x = B (used to index registers)
            y = C (used to index registers)
            n = D (operand)
            nn = CD (operand)
            nnn = BCD (opeand)
        */
        let instr_type = ((instr & 0xF000) >> 12) as u8;
        println!("Type: {instr_type:x?}");

        let x = ((instr & 0x0F00) >> 8) as u8;
        let y = ((instr & 0x00F0) >> 4) as u8;
        let n = (instr & 0x000F) as u8;
        let nn = (instr & 0x00FF) as u8;
        let nnn = instr & 0x0FFF;

        match instr_type {
            0x0 => match n {
                0x0 => self.clear_screen(),
                0xE => self.ret(),
                _ => {
                    println!("Error: Unimplemented opcode {instr_type}");
                    process::exit(1);
                }
            },
            0x1 => self.jump(nnn),
            0x2 => self.call(nnn),
            0x3 => self.skip_eq(x, nn),
            0x4 => self.skip_ne(x, nn),
            0x5 => self.skipr_eq(x, y),
            0x6 => self.set(x, nn),
            0x7 => self.add(x, nn),
            0x8 => self.match_arithmatic(x, y, n),
            0x9 => self.skipr_ne(x, y),
            0xA => self.set_idx(nnn),
            // 0xB => (),
            0xC => self.random_and(x, nn),
            0xD => self.display(x, y, n),
            0xE => match nn {
                0x9E => self.skip_if_key(x),
                0xA1 => self.skip_if_not_key(x),
                _ => {
                    println!("Error: Unimplemented opcode {instr_type}");
                    process::exit(1);
                }
            },
            0xF => self.match_f(x, nn),
            _ => {
                println!("Error: Unimplemented opcode {instr_type}");
                process::exit(1);
            }
        }
    }

    fn match_arithmatic(&mut self, r1: u8, r2: u8, kind: u8) {
        match kind {
            0x0 => self.set_r(r1, r2),
            0x1 => self.or(r1, r2),
            0x2 => self.and(r1, r2),
            0x3 => self.xor(r1, r2),
            0x4 => self.add_r(r1, r2),
            0x5 => self.subtract_right(r1, r2),
            0x6 => self.shift_right(r1, r2),
            0x7 => self.subtract_left(r1, r2),
            0xE => self.shift_left(r1, r2),
            _ => {
                println!("Error: Unimplemented arithmetic opcode {kind}");
                process::exit(1);
            }
        }
    }

    fn match_f(&mut self, register: u8, kind: u8) {
        match kind {
            0x07 => self.set(register, self.delay_timer),
            0x15 => {
                self.delay_timer = self.registers[register as usize];
            }
            0x18 => {
                self.sound_timer = self.registers[register as usize];
            }
            0x1E => {
                self.idx += self.registers[register as usize] as u16;
            }
            // 0x0A => self.add_r(r1, r2),
            0x29 => {
                self.idx = (self.registers[register as usize]) as u16 * 5;
            }
            0x33 => self.split_into_three(register),
            0x55 => self.store(register),
            0x65 => self.load(register),
            _ => {
                println!("Error: Unimplemented F opcode {kind}");
                process::exit(1);
            }
        }
    }

    fn clear_screen(&mut self) {
        self.vram.iter().for_each(|&(mut x)| x.fill(0));
    }

    fn jump(&mut self, operand: u16) {
        self.pc = operand as usize;
    }

    fn call(&mut self, operand: u16) {
        self.stack[self.sp as usize] = self.pc as u16;
        self.sp += 1;
        self.pc = operand as usize;
    }

    fn ret(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize] as usize;
    }

    fn set(&mut self, register: u8, operand: u8) {
        self.registers[register as usize] = operand;
    }

    fn set_r(&mut self, r1: u8, r2: u8) {
        self.registers[r1 as usize] = self.registers[r2 as usize];
    }

    fn set_idx(&mut self, operand: u16) {
        self.idx = operand;
    }

    fn add(&mut self, register: u8, operand: u8) {
        self.registers[register as usize] = operand.wrapping_add(self.registers[register as usize]);
    }

    fn add_r(&mut self, r1: u8, r2: u8) {
        let lhs = self.registers[r1 as usize];
        let rhs = self.registers[r2 as usize];

        let res: u16 = lhs as u16 + rhs as u16;

        if res > 255 {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[r1 as usize] = lhs.wrapping_add(rhs);
    }

    fn subtract_left(&mut self, r1: u8, r2: u8) {
        let minuend = self.registers[r2 as usize];
        let subtrahend = self.registers[r1 as usize];

        if minuend > subtrahend {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[r1 as usize] = minuend.wrapping_sub(subtrahend);
    }

    fn subtract_right(&mut self, r1: u8, r2: u8) {
        let minuend = self.registers[r1 as usize];
        let subtrahend = self.registers[r2 as usize];

        if minuend > subtrahend {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[r1 as usize] = minuend.wrapping_sub(subtrahend);
    }

    fn shift_left(&mut self, r1: u8, r2: u8) {
        let _r2 = r2;

        let data = self.registers[r1 as usize];
        self.registers[0xF] = data >> 7;
        self.registers[r1 as usize] = data << 1;
    }

    fn shift_right(&mut self, r1: u8, r2: u8) {
        let _r2 = r2;

        let data = self.registers[r1 as usize];
        self.registers[0xF] = data & 1;
        self.registers[r1 as usize] = data >> 1;
    }

    fn and(&mut self, r1: u8, r2: u8) {
        self.registers[r1 as usize] &= self.registers[r2 as usize];
    }

    fn or(&mut self, r1: u8, r2: u8) {
        self.registers[r1 as usize] |= self.registers[r2 as usize];
    }

    fn xor(&mut self, r1: u8, r2: u8) {
        self.registers[r1 as usize] ^= self.registers[r2 as usize];
    }

    fn split_into_three(&mut self, register: u8) {
        let mut val = self.registers[register as usize];
        for i in (0..3).rev() {
            self.ram[self.idx as usize + i] = val % 10;
            val /= 10;
        }
    }

    fn skip_eq(&mut self, register: u8, operand: u8) {
        if self.registers[register as usize] == operand {
            self.pc += 2;
        }
    }

    fn skipr_eq(&mut self, r1: u8, r2: u8) {
        if self.registers[r1 as usize] == self.registers[r2 as usize] {
            self.pc += 2;
        }
    }

    fn skip_ne(&mut self, register: u8, operand: u8) {
        if self.registers[register as usize] != operand {
            self.pc += 2;
        }
    }

    fn skipr_ne(&mut self, r1: u8, r2: u8) {
        if self.registers[r1 as usize] != self.registers[r2 as usize] {
            self.pc += 2;
        }
    }

    fn skip_if_key(&mut self, register: u8) {
        return;
    }

    fn skip_if_not_key(&mut self, register: u8) {
        self.pc += 2;
    }

    fn random_and(&mut self, register: u8, operand: u8) {
        let random_number: u8 = rand::random();
        self.registers[register as usize] = (random_number & operand)
    }

    fn store(&mut self, register: u8) {
        let addr = self.idx;
        for i in 0..register + 1 {
            self.ram[(addr + i as u16) as usize] = self.registers[i as usize];
        }
    }

    fn load(&mut self, register: u8) {
        let addr = self.idx;
        for i in 0..register + 1 {
            self.registers[i as usize] = self.ram[(addr + i as u16) as usize];
        }
    }

    fn display(&mut self, r1: u8, r2: u8, height: u8) {
        // Starting position of sprite wraps
        let corner_x = self.registers[r1 as usize] % (C8_DISPLAY_WIDTH as u8);
        let corner_y = self.registers[r2 as usize] % (C8_DISPLAY_HEIGHT as u8);

        println!("Drawing sprite at {corner_x}, {corner_y} with height {height}");
        // Register VF set to 1 if any pixels are turned off, else 0
        self.registers[0xF] = 0;

        for byte in 0..height {
            let row = self.ram[(self.idx + byte as u16) as usize];
            for n in 0..8 {
                let pixel_x = corner_x + n;
                let pixel_y = corner_y + byte;
                if pixel_x >= (C8_DISPLAY_WIDTH as u8) {
                    continue;
                };
                if pixel_y >= (C8_DISPLAY_HEIGHT as u8) {
                    continue;
                };

                let new_pixel = row >> (7 - n) & 1;
                let current_pixel = self.get_pixel(pixel_x, pixel_y);

                self.registers[0xF] |= new_pixel & current_pixel;
                self.set_pixel(pixel_x, pixel_y, new_pixel);
            }
        }
    }

    fn get_pixel(&self, x: u8, y: u8) -> u8 {
        self.vram[y as usize][x as usize]
    }

    fn set_pixel(&mut self, x: u8, y: u8, value: u8) {
        self.vram[y as usize][x as usize] = value;
        let pixel = self.get_pixel(x, y);
        println!("Set pixel {x}, {y} to {pixel} ");
    }
}

fn read_program(file_path: &String) -> Result<Vec<u8>, Box<dyn Error>> {
    // let dir = env::current_dir().unwrap();
    // println!("Current dir: {dir:?}");
    let buffer = fs::read(file_path)?;
    Ok(buffer)
}

fn draw_screen(canvas: &mut Canvas<Window>, program: &Chip8) {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.set_draw_color(Color::RGB(255, 255, 255));

    for (i, row) in program.vram.iter().enumerate() {
        for (j, pixel) in row.iter().enumerate() {
            print!("{pixel}");
            if *pixel != 0 {
                canvas
                    .draw_rect(Rect::new(
                        (j * SCALE_FACTOR) as i32,
                        (i * SCALE_FACTOR) as i32,
                        SCALE_FACTOR as u32,
                        SCALE_FACTOR as u32,
                    ))
                    .expect("Shoudl print");
                // canvas
                //     .draw_point(Point::new(j as i32, i as i32))
                //     .expect("Should print");
            }
        }
        println!("");
    }
    canvas.present();
}

pub fn main() {
    let args: Vec<String> = env::args().collect();

    let file_path = &args[1];

    let mut program = Chip8::new();
    program.load_program(&file_path);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window(
            "rust-sdl2 demo",
            (C8_DISPLAY_WIDTH * SCALE_FACTOR) as u32,
            (C8_DISPLAY_HEIGHT * SCALE_FACTOR) as u32,
        )
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        let instr = program.fetch();
        println!("Instruction: {instr:x?}");
        program.decode(instr);
        draw_screen(&mut canvas, &program);
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
