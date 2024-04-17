#![allow(warnings)]

use crate::cartridge::ROM;
use crate::cpu::Memory;
use crate::ppu::PPU;

const RAM_ADDRESS_SPACE_START: u16 = 0x0000;
const RAM_ADDRESS_SPACE_END: u16 = 0x1FFF;
const PPU_ADDRESS_SPACE_START: u16 = 0x2000;
const PPU_ADDRESS_SPACE_END: u16 = 0x3FFF;
const PRG_ADDRESS_SPACE_START : u16 = 0x8000;
const PRG_ADDRESS_SPACE_END: u16= 0xFFFF;

pub struct Bus {
    vram: [u8; 0x800],
    rom: Vec<u8>,
    ppu: PPU,
}

impl Memory for Bus {
    fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            RAM_ADDRESS_SPACE_START..=RAM_ADDRESS_SPACE_END => {
                // CPU has 2KB of RAM which is addressable with 11 bits, but
                // reserves an address space of 8KB addressable with 13 bits,
                // therefore requests to memory need to mask bits 12 and 13.
                let mask = 0b11111111111;
                self.vram[(addr & mask) as usize]
            }
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 | 0x4014 => {
                panic!("Illegal write to MMIO registers");
            }
            0x2007 => self.ppu.read_data(),
            0x2008..=PPU_ADDRESS_SPACE_END => {
                let mirror_down = addr & 0x2007;
                self.mem_read(mirror_down)
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
            0x2000 => {
                self.ppu.write_to_reg_ctrl(value);
            }
            0x2006 => {
                self.ppu.write_to_reg_addr(value);
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
        let ppu = PPU::new(rom.rom_chr, rom.mirroring_type);
        Bus {
            vram: [0; 0x800],
            rom: rom.rom_prg,
            ppu,
        }
    }
    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        if self.rom.len() == 0x4000 && addr >= 0x4000 {
            addr = addr % 0x4000;
        }
        self.rom[addr as usize]
    }
}
