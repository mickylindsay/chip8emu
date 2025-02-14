
use std::io::{self, Read};
use std::os::unix::io::AsRawFd;
use std::thread;
use std::time::{Duration, Instant};
use termios::*;

mod chip;

use chip::Chip8;

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
    let test_binary: [u8; 8] = [0x63, 0x01, 0xF3, 0x29, 0xD0, 0x05, 0xF0, 0x0A];
    chip8.load(test_binary.to_vec()).unwrap();

 
    let emu_speed = Duration::from_secs_f64(1.0 / 60.0); // Default 60hz

    loop {
        let start_time = Instant::now();
 
        chip8.emulate();
        
        reader.read_exact(&mut buffer).unwrap();

        let elapsed_time = start_time.elapsed();
        if elapsed_time < emu_speed {
            thread::sleep(emu_speed - elapsed_time);
        }
        
    }
   
    // Reset terminal to orinal config - unreachable until input for stopping emulation
    // tcsetattr(stdin, TCSANOW, &original_term).unwrap();
}
