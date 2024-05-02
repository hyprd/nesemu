#![allow(warnings)]

use crate::cartridge::ROM;
use crate::cpu::Memory;
use crate::joypad::Joypad;
use crate::ppu::PPU;

const RAM_ADDRESS_SPACE_START: u16 = 0x0000;
const RAM_ADDRESS_SPACE_END: u16 = 0x1FFF;
const PPU_ADDRESS_SPACE_START: u16 = 0x2000;
const PPU_ADDRESS_SPACE_END: u16 = 0x3FFF;
const PRG_ADDRESS_SPACE_START: u16 = 0x8000;
const PRG_ADDRESS_SPACE_END: u16 = 0xFFFF;

pub struct Bus<'call> {
    vram: [u8; 0x800],
    rom: Vec<u8>,
    ppu: PPU,
    cycles: usize,
    callback: Box<dyn FnMut(&PPU, &mut Joypad) + 'call>,
    joypad: Joypad,
}

impl<'a> Bus<'a> {
    pub fn new<'call, F>(rom: ROM, callback: F) -> Bus<'call>
    where
        F: FnMut(&PPU, &mut Joypad) + 'call,
    {
        let ppu = PPU::new(rom.rom_chr, rom.mirroring_type);
        Bus {
            vram: [0; 0x800],
            rom: rom.rom_prg,
            ppu,
            cycles: 0,
            callback: Box::from(callback),
            joypad: Joypad::new(),
        }
    }

    pub fn tick(&mut self, cycles: u8) {
        self.cycles += cycles as usize;
        // Cheat way to render a screen is to read the screen state before
        // the CPu starts to render a new frame.
        let new_frame = self.ppu.tick(cycles * 3);
        if new_frame {
            (self.callback)(&self.ppu, &mut self.joypad);
        }
    }

    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        if self.rom.len() == 0x4000 && addr >= 0x4000 {
            addr = addr % 0x4000;
        }
        self.rom[addr as usize]
    }

    pub fn poll_nmi_status(&mut self) -> Option<u8> {
        self.ppu.nmi_interrupt.take()
    }
}

impl Memory for Bus<'_> {
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
                //panic!("Illegal read to write-only MMIO registers");
                0
            }
            0x2002 => self.ppu.read_status(),
            0x2004 => self.ppu.read_oam_data(),
            0x2007 => self.ppu.read_data(),
            0x2008..=PPU_ADDRESS_SPACE_END => {
                let mirror_down = addr & 0x2007;
                self.mem_read(mirror_down)
            }
            0x4016 => self.joypad.read(),
            0x4000..=0x4015 | 0x4017 => 0,
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
            0x2001 => {
                self.ppu.write_to_reg_mask(value);
            }
            0x2002 => {
                panic!("Illegl write to PPU status register: {:02x}", addr);
            }
            0x2003 => {
                self.ppu.write_to_oam_address(value);
            }
            0x2004 => {
                self.ppu.write_to_oam_data(value);
            }
            0x2005 => {
                self.ppu.write_to_reg_scroll(value);
            }
            0x2006 => {
                self.ppu.write_to_reg_addr(value);
            }
            0x2007 => {
                self.ppu.write_data(value);
            }
            0x4000..=0x4013 | 0x4015 | 0x4017 => {}
            0x4014 => {
                // OAMDMA
                // Writing anyhting to this register sends 0xNN00 -> 0xNNFF to
                // the PPU OAM table.
                //
                // This should only occur within VBLANK because OAM DRAM decays
                // when rendering is disabled.
                let mut buffer: [u8; 256] = [0; 256];
                let hi: u16 = (value as u16) << 8;
                for i in 0..256u16 {
                    buffer[i as usize] = self.mem_read(hi + i);
                }
                self.ppu.write_to_oam_dma(&buffer);
            }
            0x4016 => {
                self.joypad.write(value);
            }
            PPU_ADDRESS_SPACE_START..=PPU_ADDRESS_SPACE_END => {
                let mirror_down = addr & 0x2007;
                self.mem_write(mirror_down, value);
            }
            PRG_ADDRESS_SPACE_START..=PRG_ADDRESS_SPACE_END => {
                panic!("Illegal write to cartridge ROM: {:02x}", addr);
            }
            _ => {
                println!("Memory write at {:#04X?} ignored", addr);
            }
        }
    }
}
