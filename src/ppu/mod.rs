use crate::cartridge::MirroringType;
use reg_addr::PPUADDR;
use reg_controller::PPUCTRL;

pub mod reg_addr;
pub mod reg_controller;

pub struct PPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; 32],
    pub vram: [u8; 2048],
    pub oam: [u8; 256],
    pub mirroring: MirroringType,
    pub reg_v: u16,
    pub reg_t: u16,
    pub reg_x: u8,
    pub reg_w: bool,
    pub reg_address: PPUADDR,
    pub reg_controller: PPUCTRL,
}

impl PPU {
    /*
     * Accessing PPU memory
     *   1. CPU writes the address it wants to access in PPU memory space by writing 2 bytes to 0x2006.
     *   2. PPU accesses data stored at this address and stores it in its own internal buffer.
     *   3. PPU increments the address register, determined by the state of the control register
     *      @ 0x2000.
     *   4. CPU reads data register 0x2007, prompting the PPU to return the internal buffer data.
     */
    pub fn new(chr_rom: Vec<u8>, mirroring: MirroringType) -> Self {
        PPU {
            chr_rom,
            palette_table: [0; 32],
            vram: [0; 02048],
            oam: [0; 256],
            mirroring,
            reg_v: 0,
            reg_t: 0,
            reg_x: 0,
            reg_w: true,
            reg_address: PPUADDR::new(),
            reg_controller: PPUCTRL::from_bits_truncate(0b00000000),
        }
    }

    pub fn write_to_reg_addr(&mut self, value: u8) {
        self.reg_address.update(value);
    }

    pub fn write_to_reg_ctrl(&mut self, value: u8) {
        self.reg_controller = PPUCTRL::from_bits_truncate(value);
    }
}
