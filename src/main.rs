use rand::Rng;
use std::io::{self, Read, Write};

const MEMORY_SIZE: usize = 4096;
const NUM_REGISTERS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;
const DISPLAY_WIDTH: usize = 64;
const DISPLAY_HEIGHT: usize = 32;

pub struct Chip8 {
    memory: [u8; MEMORY_SIZE], // 4k memory
    registers: [u8; NUM_REGISTERS], // registers V0 -> VF
    index: u16,
    pc: u16,
    stack: [u16; STACK_SIZE],
    stack_pointer: u8,
    delay_timer: u8,
    sound_timer: u8,
    inputs: [bool; NUM_KEYS],
    display: [[bool; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
}

impl Chip8 {
    pub fn new() -> Self {
        let mut chip8 = Chip8 {
            memory: [0; MEMORY_SIZE],
            registers: [0; NUM_REGISTERS],
            index: 0,
            pc: 0x200, // Original 512 bytes held chip8 interpreter, programs start after that
            stack: [0; STACK_SIZE],
            stack_pointer: 0,
            delay_timer: 0,
            sound_timer: 0,
            inputs: [false; NUM_KEYS],
            display: [[false; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
        };

        // Temp fill display with random pixels enabled/disabled
        for row in 0..DISPLAY_HEIGHT {
            for col in 0..DISPLAY_WIDTH {
                chip8.display[row][col] = rand::thread_rng().gen_bool(0.5);
            }
        }

        chip8
    }

    fn execute(&mut self, opcode: u16) {
        let nnn = opcode & 0x0FFF;
        match opcode & 0xF000 {
            0x1000 /* JP addr */ => self.pc = nnn,
            0x6000 /* LD Vx, byte */ => self.registers[((opcode & 0xF00) >> 8) as usize] = (opcode & 0xFF) as u8,
            0x7000 /* ADD Vx, byte */ => self.registers[((opcode & 0xF00) >> 8) as usize] += (opcode & 0xFF) as u8,
            0x8000 /* LD Vx, Vy */ => self.registers[((opcode & 0xF00) >> 8) as usize] = self.registers[((opcode & 0xF0) >> 4) as usize],
            _ => println!("Unknown opcode {:X}", opcode),
        }
    }

    pub fn print_graphics(&mut self) {
        // Ansi clear screen
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

        // Print screen
        for row in self.display {
            let chars = row.map(|enabled| if enabled { '█' } else {  ' ' });
            println!("{}", String::from_iter(chars));
        }

        // Print program counter and current op code next to screen
        print!("{esc}[1;67HPC:0x{pc:0>3X}", esc = 27 as char, pc = self.pc);
        let opcode = (self.memory[self.pc as usize] as u16) << 8
            | self.memory[(self.pc + 1) as usize] as u16;
        print!("{esc}[2;67HOp:0x{opcode:0>4X}", esc = 27 as char);

        print!("{esc}[33;1H", esc = 27 as char);
        io::stdout().flush().unwrap()
    }
   
}

fn main() {
    let mut stdin = io::stdin();

    let mut chip_emu = Chip8::new();
    chip_emu.print_graphics();

    // Push enter to continues
    let _ = stdin.read(&mut [0u8]).unwrap();

}