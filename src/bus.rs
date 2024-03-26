#![allow(warnings)]

use crate::cpu::Memory;

const RAM_START: u16 = 0x0000;
const RAM_END: u16 = 0x1FFF;
const PPU_START: u16 = 0x2000;
const PPU_END: u16 = 0x3FFF;

pub struct Bus {
    vram: [u8; 0x800],
}

impl Memory for Bus {
    fn mem_read(&self, addr: u16) -> u8 {
        match addr {
            RAM_START..=RAM_END => {
                // CPU has 2KB of RAM which is addressable with 11 bits, but
                // reserves an address space of 8KB addressable with 13 bits,
                // therefore requests to memory need to mask bits 12 and 13.
                let mask = 0b11111111111;
                self.vram[(addr & mask) as usize]
            }
            PPU_START..=PPU_END => {
                todo!("Implement ppu!");
            }
            _ => {
                println!("Memory access at {:#04X?} ignored", addr);
                0
            }
        }
    }
    fn mem_write(&mut self, addr: u16, value: u8) {
        match addr {
            RAM_START..=RAM_END => {
                let mask = 0b11111111111;
                self.vram[(addr & mask) as usize] = value;
            }
            PPU_START..=PPU_END => {
                todo!("Implement ppu!");
            }
            _ => {
                println!("Memory write at {:#04X?} ignored", addr);
            }
        }
    }
}

impl Bus {
    pub fn new() -> Self {
        Bus { vram: [0; 0x800] }
    }
}
