#![allow(warnings)]

use crate::cartridge::ROM;
use crate::cpu::Memory;

const RAM_ADDRESS_SPACE_START: u16 = 0x0000;
const RAM_ADDRESS_SPACE_END: u16 = 0x1FFF;
const PPU_ADDRESS_SPACE_START: u16 = 0x2000;
const PPU_ADDRESS_SPACE_END: u16 = 0x3FFF;
const PRG_ADDRESS_SPACE_START : u16 = 0x8000;
const PRG_ADDRESS_SPACE_END: u16= 0xFFFF;

pub struct Bus {
    vram: [u8; 0x800],
    rom: ROM,
}

impl Memory for Bus {
    fn mem_read(&self, addr: u16) -> u8 {
        match addr {
            RAM_ADDRESS_SPACE_START..=RAM_ADDRESS_SPACE_END => {
                // CPU has 2KB of RAM which is addressable with 11 bits, but
                // reserves an address space of 8KB addressable with 13 bits,
                // therefore requests to memory need to mask bits 12 and 13.
                let mask = 0b11111111111;
                self.vram[(addr & mask) as usize]
            }
            PPU_ADDRESS_SPACE_START..=PPU_ADDRESS_SPACE_END => {
                // todo!("Implement ppu!");
                0
            }
            PRG_ADDRESS_SPACE_START..=PRG_ADDRESS_SPACE_END => self.read_prg_rom(addr),
            _ => {
                println!("Memory access at {:#04X?} ignored", addr);
                0
            }
        }
    }
    fn mem_write(&mut self, addr: u16, value: u8) {
        match addr {
            RAM_ADDRESS_SPACE_START..=RAM_ADDRESS_SPACE_END => {
                let mask = 0b11111111111;
                self.vram[(addr & mask) as usize] = value;
            }
            PPU_ADDRESS_SPACE_START..=PPU_ADDRESS_SPACE_END => {
                todo!("Implement ppu!");
            }
            PRG_ADDRESS_SPACE_START..=PRG_ADDRESS_SPACE_END => {
                panic!("Illegal write to cartridge ROM");
            }
            _ => {
                println!("Memory write at {:#04X?} ignored", addr);
            }
        }
    }
}

impl Bus {
    pub fn new(rom: ROM) -> Self {
        Bus {
            vram: [0; 0x800],
            rom,
        }
    }
    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        if self.rom.rom_prg.len() == 0x4000 && addr >= 0x4000 {
            addr = addr % 0x4000;
        }
        self.rom.rom_prg[addr as usize]
    }
}
