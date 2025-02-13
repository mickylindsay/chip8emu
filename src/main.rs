use rand::Rng;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::thread;
use std::time::{Duration, Instant};
use termios::*;

const MEMORY_SIZE: usize = 4096;
const NUM_REGISTERS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;
const DISPLAY_WIDTH: usize = 64;
const DISPLAY_HEIGHT: usize = 32;

pub struct Chip8 {
    memory: [u8; MEMORY_SIZE],      // 4k memory
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
            pc: 0x200, // First 512 bytes held chip8 interpreter
            stack: [0; STACK_SIZE],
            stack_pointer: 0,
            delay_timer: 0,
            sound_timer: 0,
            inputs: [false; NUM_KEYS],
            display: [[false; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
        };

        let font_set: [u8; 80] = [
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

        for i in 0..80 {
            chip8.memory[i] = font_set[i];
        }

        chip8
    }

    fn emulate(&mut self) {
        let opcode = ((self.memory[self.pc as usize] as u16) << 8)
            | self.memory[(self.pc + 1) as usize] as u16;

        // TODO separate graphics print
        self.print_graphics();

        self.pc += 2;
        self.execute(opcode);

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
            // Play sound whenever timer > 0. Probably just print ascii bell?
        }
    }

    fn execute(&mut self, opcode: u16) {
        let addr = opcode & 0x0FFF;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        let byte = (opcode & 0x00FF) as u8;
        let nibble = (opcode & 0x000F) as u8;

        match opcode & 0xF000 {
            0x0000 => match opcode {
                0x00E0 /* CLS */ => self.display = [[false; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
                0x00EE /* RET */ => self.return_subroutine(),
                _ => println!("Invalid operation {:X}", opcode),
            },
            0x1000 /* JP addr */ => self.pc = addr,
            0x2000 /* CALL addr */ => self.call_subroutine(addr),
            0x3000 /* SE Vx, byte */ => self.skip_if_condition(self.registers[x] == byte),
            0x4000 /* SNE Vx, byte */ => self.skip_if_condition(self.registers[x] != byte),
            0x5000 /* SE Vx, Vy */ => self.skip_if_condition(self.registers[x] == self.registers[y]),
            0x6000 /* LD Vx, byte */ => self.registers[x] = byte,
            0x7000 /* ADD Vx, byte */ => self.registers[x] += byte,
            0x8000 => match nibble {
                0x0 /* LD Vx, Vy */ => self.registers[x] = self.registers[y],
                0x1 /* OR Vx, Vy */ => self.registers[x] = self.registers[x] | self.registers[y],
                0x2 /* AND Vx, Vy */ => self.registers[x] = self.registers[x] & self.registers[y],
                0x3 /* XOR Vx, Vy */ => self.registers[x] = self.registers[x] ^ self.registers[y],
                0x4 /* ADD Vx, Vy */ => self.add_registers(x, y),
                0x5 /* SUB Vx, Vy */ => self.sub_registers(x, y),
                0x6 /* SHR Vx */ => self.shift_right(x),
                0x7 /* SUBN Vx */ => self.sub_registers_n(x, y),
                0xE /* SHR Vx */ => self.shift_left(x),
                _ => println!("Invalid operation {:X}", opcode),
            },
            0x9000 /* SNE Vx, Vy */ =>  self.skip_if_condition(self.registers[x] != self.registers[y]),
            0xA000 /* LD I, addr */ => self.index = addr,
            0xB000 /* JP V0, addr */ => self.pc = (addr + self.registers[0] as u16) & 0x3FFF,
            0xC000 /* RND Vx, byte */ => self.registers[x] = rand::thread_rng().gen::<u8>() & byte,
            0xD000 /* DRW Vx, Vy, nibble */ => self.draw_sprite(x, y, nibble as usize),
            0xE000 => match byte {
                0x9E /* SKP Vx */ => self.skip_if_condition(self.inputs[self.registers[x] as usize]),
                0xA1 /* SKNP Vx */ => self.skip_if_condition(!self.inputs[self.registers[x] as usize]),
                _ => println!("Invalid operation {:X}", opcode),
            },
            0xF000 => match byte {
                0x07 /* LD Vx, DT */ => self.registers[x] = self.delay_timer,
                0x0A /* LD Vx, K */ => self.wait_for_input(x),
                0x15 /* LD DT, Vx */ => self.delay_timer = self.registers[x],
                0x18 /* LD ST, Vx */ => self.sound_timer = self.registers[x],
                0x1E /* ADD I, Vx */ => self.index += self.registers[x] as u16,
                0x29 /* LD F, Vx */ => self.load_font(x),
                0x33 /* LD B, Vx */ => self.store_binary_coded_decimal(x),
                0x55 /* LD [I], Vx */ => self.store_many_registers(x),
                0x65 /* LD Vx, [I] */ => self.load_many_registers(x),
                _ => println!("Invalid operation {:X}", opcode),
            },
            _ => println!("Invalid operation {:X}", opcode),
        }
    }

    fn print_graphics(&mut self) {
        // TODO rather than print and clear each time, only print updated values/pixels
        // Stops the flashing from ANSI printing

        // Ansi clear screen
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

        // Print screen
        for row in self.display {
            let chars = row.map(|enabled| if enabled { '█' } else { ' ' });
            println!("{}", String::from_iter(chars));
        }

        // Print program counter and current op code next to screen
        print!("{esc}[1;67HPC:0x{pc:0>3X}", esc = 27 as char, pc = self.pc);
        let opcode = (self.memory[self.pc as usize] as u16) << 8
            | self.memory[(self.pc + 1) as usize] as u16;
        print!("{esc}[2;67HOp:0x{opcode:0>4X}", esc = 27 as char);
        print!(
            "{esc}[3;67HIndex:0x{index:0>4X}",
            esc = 27 as char,
            index = self.index
        );

        print!("{esc}[33;1H", esc = 27 as char);
        io::stdout().flush().unwrap()
    }

    // Op code functions

    fn return_subroutine(&mut self) {
        self.stack_pointer -= 1;
        self.pc = self.stack[self.stack_pointer as usize];
    }

    fn call_subroutine(&mut self, addr: u16) {
        self.stack_pointer += 1;
        self.stack[self.stack_pointer as usize] = addr
    }

    fn skip_if_condition(&mut self, con: bool) {
        if con {
            self.pc += 2
        }
    }

    fn add_registers(&mut self, x: usize, y: usize) {
        let (sum, carry) = self.registers[x].overflowing_add(self.registers[y]);
        self.registers[x] = sum;
        self.registers[0xF] = if carry { 1 } else { 0 };
    }

    fn sub_registers(&mut self, x: usize, y: usize) {
        self.registers[0xF] = if self.registers[x] >= self.registers[y] {
            1
        } else {
            0
        };
        self.registers[x] -= self.registers[y];
    }

    fn shift_right(&mut self, x: usize) {
        self.registers[0xF] = self.registers[x] & 0x1;
        self.registers[x] >>= 1;
    }

    fn sub_registers_n(&mut self, x: usize, y: usize) {
        self.registers[0xF] = if self.registers[y] >= self.registers[x] {
            1
        } else {
            0
        };
        self.registers[x] = self.registers[y] - self.registers[x];
    }

    fn shift_left(&mut self, x: usize) {
        self.registers[0xF] = (self.registers[x] & 0x80) >> 7;
        self.registers[x] <<= 1;
    }

    fn draw_sprite(&mut self, x_reg: usize, y_reg: usize, n: usize) {
        let mut carry = false;
        let x_coord = self.registers[x_reg] as usize;
        let y_coord = self.registers[y_reg] as usize;

        for y_offset in 0..n {
            let src = self.memory[self.index as usize + y_offset];
            for x_offset in 0..8 {
                if (src & (0x80 >> x_offset)) != 0 {
                    let y = (y_coord + y_offset) % DISPLAY_HEIGHT;
                    let x = (x_coord + x_offset) % DISPLAY_WIDTH;

                    if !carry && self.display[y][x] {
                        carry = true;
                    }
                    self.display[y][x] ^= true;
                }
            }
        }
        self.registers[0xF] = if carry { 1 } else { 0 };
    }

    fn wait_for_input(&mut self, x: usize) {
        let mut pressed = false;
        for index in 0..NUM_KEYS {
            if self.inputs[index] {
                self.registers[x] = index as u8;
                pressed = true;
                break;
            }
        }
        if !pressed {
            self.pc -= 2;
        }
    }

    fn load_font(&mut self, x: usize) {
        self.index = (self.registers[x] as u16) * 5;
    }

    fn store_binary_coded_decimal(&mut self, x: usize) {
        let val = self.registers[x];
        self.memory[self.index as usize] = val / 100;
        self.memory[(self.index + 1) as usize] = (val % 100) / 10;
        self.memory[(self.index + 2) as usize] = val % 10;
    }

    fn store_many_registers(&mut self, x: usize) {
        for reg_index in 0..x {
            self.memory[(self.index as usize) + reg_index] = self.registers[reg_index];
        }
    }

    fn load_many_registers(&mut self, x: usize) {
        for reg_index in 0..x {
            self.registers[reg_index] = self.memory[(self.index as usize) + reg_index]
        }
    }
}

fn main() {
    // Setup std in for keyboard input
    let stdin = io::stdin().as_raw_fd();
    let original_term = Termios::from_fd(stdin).unwrap();
    let mut termios = original_term.clone();
    // Change input to read buffer rather than line and remove echo
    termios.c_lflag &= !(ECHO | ICANON);
    tcsetattr(stdin, TCSANOW, &mut termios).unwrap();
    let mut reader = io::stdin();
    let mut buffer = [0; 1]; // read exactly one byte
 

    let mut chip8 = Chip8::new();
    // Temp manual rom to print sprite and wait for input
    chip8.memory[0x200] = 0x63;
    chip8.memory[0x201] = 0x01;
    chip8.memory[0x202] = 0xF3;
    chip8.memory[0x203] = 0x29;
    chip8.memory[0x204] = 0xD0;
    chip8.memory[0x205] = 0x05;
    chip8.memory[0x206] = 0xF0;
    chip8.memory[0x207] = 0x0A;

    /* Fill display with random pixels enabled/disabled
    for row in 0..DISPLAY_HEIGHT {
        for col in 0..DISPLAY_WIDTH {
            chip8.display[row][col] = rand::thread_rng().gen_bool(0.5);
        }
    }
    */
 
    let emu_speed = Duration::from_secs_f64(1.0 / 60.0); // Default 60hz

    loop {
        let start_time = Instant::now();
        reader.read_exact(&mut buffer).unwrap();
        println!("Value: {:?}", buffer);
 
        /* Temp remove emu to work on keyboard input
        chip8.emulate();

        let elapsed_time = start_time.elapsed();
        if elapsed_time < emu_speed {
            thread::sleep(emu_speed - elapsed_time);
        }
        */
    }
   
    // Reset terminal to orinal config
    tcsetattr(stdin, TCSANOW, &original_term).unwrap();
}
