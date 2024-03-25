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
    fn mem_read(&self, addr: u16) -> u8 {0}
    fn mem_write(&mut self, addr: u16, value: u8) {}
}

impl Bus {
    pub fn new() -> Self {
        Bus { vram: [0; 0x800] }
    }
}
